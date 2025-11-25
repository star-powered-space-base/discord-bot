//! Multi-bot Discord Gateway entry point
//!
//! Supports running multiple Discord bots concurrently from a single process.
//! Configuration can come from:
//! - config.yaml (multi-bot mode)
//! - Environment variables (single-bot legacy mode)

use anyhow::Result;
use dotenvy::dotenv;
use futures::future::join_all;
use log::{error, info, warn};
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use persona::commands::{register_global_commands, register_guild_commands, CommandHandler};
use persona::config::{populate_application_ids, BotConfig, MultiConfig};
use persona::database::Database;
use persona::message_components::MessageComponentHandler;
use persona::personas::PersonaManager;
use persona::reminder_scheduler::ReminderScheduler;
use persona::startup_notification::StartupNotifier;
use persona::system_info::metrics_collection_loop_with_bot_id;
use persona::usage_tracker::UsageTracker;

/// Tracks which bots have sent their first Ready notification (vs reconnects)
static NOTIFIED_BOTS: once_cell::sync::Lazy<Mutex<HashSet<String>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashSet::new()));

/// Handler for a single bot's Discord events
struct Handler {
    bot_id: String,
    bot_name: String,
    bot_config: BotConfig,
    command_handler: Arc<CommandHandler>,
    component_handler: Arc<MessageComponentHandler>,
    guild_id: Option<GuildId>,
    startup_notifier: StartupNotifier,
}

impl Handler {
    fn new(
        bot_id: String,
        bot_name: String,
        bot_config: BotConfig,
        command_handler: CommandHandler,
        component_handler: MessageComponentHandler,
        guild_id: Option<GuildId>,
        startup_notifier: StartupNotifier,
    ) -> Self {
        Handler {
            bot_id,
            bot_name,
            bot_config,
            command_handler: Arc::new(command_handler),
            component_handler: Arc::new(component_handler),
            guild_id,
            startup_notifier,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if let Err(e) = self.command_handler.handle_message(&ctx, &msg).await {
            error!("[{}] Error handling message: {e}", self.bot_name);
            if let Err(why) = msg
                .channel_id
                .say(
                    &ctx.http,
                    "Sorry, I encountered an error processing your message.",
                )
                .await
            {
                error!("[{}] Failed to send error message: {why}", self.bot_name);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(
            "[{}] {} is connected and ready!",
            self.bot_name, ready.user.name
        );
        info!("[{}] Connected to {} guilds", self.bot_name, ready.guilds.len());
        info!("[{}] Bot ID: {}", self.bot_name, ready.user.id);

        // Log shard information
        if let Some(shard) = ready.shard {
            info!("[{}] Shard: {}/{}", self.bot_name, shard[0] + 1, shard[1]);
        }

        // Register slash commands
        if let Some(guild_id) = self.guild_id {
            info!(
                "[{}] Development mode: Registering commands for guild {guild_id}",
                self.bot_name
            );
            if let Err(e) = register_guild_commands(&ctx, guild_id, self.bot_config.commands.as_deref()).await {
                error!(
                    "[{}] Failed to register guild slash commands: {e}",
                    self.bot_name
                );
            } else {
                info!(
                    "[{}] Successfully registered slash commands for guild {guild_id}",
                    self.bot_name
                );
            }
        } else {
            info!(
                "[{}] Production mode: Registering commands globally",
                self.bot_name
            );
            if let Err(e) = register_global_commands(&ctx, self.bot_config.commands.as_deref()).await {
                error!(
                    "[{}] Failed to register global slash commands: {e}",
                    self.bot_name
                );
            } else {
                info!(
                    "[{}] Successfully registered slash commands globally",
                    self.bot_name
                );
            }
        }

        // Send startup notification only on first Ready (not reconnects)
        // Use per-bot tracking instead of global static
        {
            let mut notified = NOTIFIED_BOTS.lock().await;
            if notified.contains(&self.bot_id) {
                info!(
                    "[{}] Skipping startup notification (reconnect)",
                    self.bot_name
                );
            } else {
                notified.insert(self.bot_id.clone());
                drop(notified); // Release lock before async call
                self.startup_notifier
                    .send_if_enabled(&ctx.http, &ready, &self.bot_config)
                    .await;
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                if let Err(e) = self
                    .command_handler
                    .handle_slash_command(&ctx, &command)
                    .await
                {
                    error!(
                        "[{}] Error handling slash command '{}': {}",
                        self.bot_name, command.data.name, e
                    );

                    let error_message =
                        if e.to_string().contains("timeout") || e.to_string().contains("OpenAI") {
                            "Sorry, the AI service is taking longer than expected. Please try again."
                        } else {
                            "Sorry, I encountered an error processing your command. Please try again."
                        };

                    #[allow(clippy::redundant_pattern_matching)]
                    if let Err(_) = command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(error_message)
                        })
                        .await
                    {
                        let _ = command
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| {
                                        message.content(error_message)
                                    })
                            })
                            .await;
                    }
                }
            }
            Interaction::MessageComponent(component) => {
                if let Err(e) = self
                    .component_handler
                    .handle_component_interaction(&ctx, &component)
                    .await
                {
                    error!(
                        "[{}] Error handling component interaction '{}': {}",
                        self.bot_name, component.data.custom_id, e
                    );

                    let error_message =
                        "Sorry, I encountered an error processing your interaction. Please try again.";

                    #[allow(clippy::redundant_pattern_matching)]
                    if let Err(_) = component
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(serenity::model::application::interaction::InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|message| {
                                    message.content(error_message)
                                })
                        })
                        .await
                    {
                        let _ = component
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| {
                                        message.content(error_message)
                                    })
                            })
                            .await;
                    }
                }
            }
            Interaction::ModalSubmit(modal) => {
                if let Err(e) = self
                    .component_handler
                    .handle_modal_submit(&ctx, &modal)
                    .await
                {
                    error!(
                        "[{}] Error handling modal submit '{}': {}",
                        self.bot_name, modal.data.custom_id, e
                    );

                    let error_message =
                        if e.to_string().contains("timeout") || e.to_string().contains("OpenAI") {
                            "Sorry, the AI service is taking longer than expected. Please try again."
                        } else {
                            "Sorry, I encountered an error processing your submission. Please try again."
                        };

                    #[allow(clippy::redundant_pattern_matching)]
                    if let Err(_) = modal
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(error_message)
                        })
                        .await
                    {
                        let _ = modal
                            .create_interaction_response(&ctx.http, |response| {
                                response
                                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                    .interaction_response_data(|message| {
                                        message.content(error_message)
                                    })
                            })
                            .await;
                    }
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                let _ = self.handle_autocomplete(&ctx, &autocomplete).await;
            }
            Interaction::Ping(_) => {
                info!("[{}] Ping interaction received", self.bot_name);
            }
        }
    }
}

impl Handler {
    /// Handle autocomplete interactions
    async fn handle_autocomplete(
        &self,
        ctx: &Context,
        autocomplete: &serenity::model::application::interaction::autocomplete::AutocompleteInteraction,
    ) -> Result<(), serenity::Error> {
        match autocomplete.data.name.as_str() {
            "set_guild_setting" => {
                let setting = autocomplete
                    .data
                    .options
                    .iter()
                    .find(|opt| opt.name == "setting")
                    .and_then(|opt| opt.value.as_ref())
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                autocomplete
                    .create_autocomplete_response(&ctx.http, |response| {
                        match setting {
                            "default_verbosity" => response
                                .add_string_choice("concise - Brief responses", "concise")
                                .add_string_choice("normal - Balanced responses", "normal")
                                .add_string_choice("detailed - Comprehensive responses", "detailed"),
                            "default_persona" => response
                                .add_string_choice("obi - Obi-Wan Kenobi", "obi")
                                .add_string_choice("muppet - Enthusiastic Muppet", "muppet")
                                .add_string_choice("chef - Cooking expert", "chef")
                                .add_string_choice("teacher - Patient educator", "teacher")
                                .add_string_choice("analyst - Step-by-step analyst", "analyst"),
                            "conflict_mediation" => response
                                .add_string_choice("enabled", "enabled")
                                .add_string_choice("disabled", "disabled"),
                            "conflict_sensitivity" => response
                                .add_string_choice("low", "low")
                                .add_string_choice("medium", "medium")
                                .add_string_choice("high", "high")
                                .add_string_choice("ultra", "ultra"),
                            "mediation_cooldown" => response
                                .add_string_choice("1 minute", "1")
                                .add_string_choice("5 minutes", "5")
                                .add_string_choice("10 minutes", "10")
                                .add_string_choice("15 minutes", "15")
                                .add_string_choice("30 minutes", "30"),
                            "max_context_messages" => response
                                .add_string_choice("10 messages", "10")
                                .add_string_choice("20 messages", "20")
                                .add_string_choice("40 messages", "40")
                                .add_string_choice("60 messages", "60"),
                            "audio_transcription" | "mention_responses" | "startup_notification" => {
                                response
                                    .add_string_choice("enabled", "enabled")
                                    .add_string_choice("disabled", "disabled")
                            }
                            "audio_transcription_mode" => response
                                .add_string_choice("always", "always")
                                .add_string_choice("mention_only", "mention_only"),
                            "audio_transcription_output" => response
                                .add_string_choice("transcription_only", "transcription_only")
                                .add_string_choice("with_commentary", "with_commentary"),
                            _ => response,
                        }
                    })
                    .await
            }
            _ => {
                autocomplete
                    .create_autocomplete_response(&ctx.http, |response| response)
                    .await
            }
        }
    }
}

/// Run a single bot with retry logic
async fn run_bot(
    bot_config: BotConfig,
    multi_config: &MultiConfig,
    database: Arc<Database>,
    persona_manager: Arc<PersonaManager>,
) -> Result<()> {
    let bot_name = bot_config.name.clone();
    let max_retries = 5;
    let mut retry_count = 0;

    loop {
        info!("[{}] Starting bot (attempt {}/{})", bot_name, retry_count + 1, max_retries);

        match run_bot_inner(&bot_config, multi_config, database.clone(), persona_manager.clone()).await {
            Ok(()) => {
                info!("[{}] Bot exited normally", bot_name);
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    error!(
                        "[{}] Bot failed after {} retries: {}",
                        bot_name, max_retries, e
                    );
                    return Err(e);
                }

                let delay = Duration::from_secs(5 * retry_count as u64);
                warn!(
                    "[{}] Bot failed: {}. Retrying in {:?}...",
                    bot_name, e, delay
                );
                tokio::time::sleep(delay).await;
            }
        }
    }

    Ok(())
}

/// Inner bot run function (single attempt)
async fn run_bot_inner(
    bot_config: &BotConfig,
    multi_config: &MultiConfig,
    database: Arc<Database>,
    persona_manager: Arc<PersonaManager>,
) -> Result<()> {
    let bot_id = bot_config.bot_id().to_string();
    let bot_name = bot_config.name.clone();

    // Create usage tracker for this bot
    let usage_tracker = UsageTracker::with_bot_id(bot_id.clone(), (*database).clone());

    // Get effective settings (per-bot overrides or global defaults)
    let openai_model = bot_config.effective_model(&multi_config.openai_model);
    let conflict_enabled = bot_config.effective_conflict_enabled(multi_config.conflict_mediation_enabled);
    let conflict_sensitivity = bot_config.effective_conflict_sensitivity(&multi_config.conflict_sensitivity);
    let mediation_cooldown = bot_config.effective_mediation_cooldown(multi_config.mediation_cooldown_minutes);

    // Create command handler with bot_id
    let command_handler = CommandHandler::with_bot_id(
        bot_id.clone(),
        (*database).clone(),
        multi_config.openai_api_key.clone(),
        openai_model.clone(),
        conflict_enabled,
        &conflict_sensitivity,
        mediation_cooldown,
        usage_tracker.clone(),
    );

    // Create component handler
    let component_handler = MessageComponentHandler::with_bot_id(
        bot_id.clone(),
        command_handler.clone(),
        (*persona_manager).clone(),
        (*database).clone(),
    );

    // Parse guild ID for dev mode
    let guild_id = bot_config
        .discord_guild_id
        .as_ref()
        .and_then(|id| id.parse::<u64>().ok())
        .map(GuildId);

    // Create startup notifier
    let startup_notifier = StartupNotifier::new(database.clone());

    // Create handler
    let handler = Handler::new(
        bot_id.clone(),
        bot_name.clone(),
        bot_config.clone(),
        command_handler,
        component_handler,
        guild_id,
        startup_notifier,
    );

    // Configure gateway intents
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Build Discord client
    let mut client = Client::builder(&bot_config.discord_token, intents)
        .event_handler(handler)
        .await
        .map_err(|e| {
            error!("[{}] Failed to create Discord client: {e}", bot_name);
            anyhow::anyhow!("Client creation failed: {}", e)
        })?;

    // Start reminder scheduler for this bot
    let scheduler = ReminderScheduler::with_bot_id(
        bot_id.clone(),
        (*database).clone(),
        openai_model.clone(),
        usage_tracker,
    );
    let http = client.cache_and_http.http.clone();
    tokio::spawn(async move {
        scheduler.run(http).await;
    });

    // Start metrics collection for this bot
    let metrics_db = database.clone();
    let db_path = multi_config.database_path.clone();
    let metrics_bot_id = bot_id.clone();
    tokio::spawn(async move {
        metrics_collection_loop_with_bot_id(metrics_db, db_path, metrics_bot_id).await;
    });

    info!("[{}] Connecting to Discord gateway...", bot_name);

    // Start the bot (blocks until disconnect)
    client.start().await.map_err(|e| {
        error!("[{}] Gateway connection failed: {e}", bot_name);
        anyhow::anyhow!("Gateway connection failed: {}", e)
    })?;

    Ok(())
}

/// Graceful shutdown flag
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Auto-load configuration (YAML file or legacy env vars)
    let mut config = MultiConfig::auto_load()?;

    // Set up logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.log_level),
    )
    .init();

    info!("Starting Persona Discord Bot (multi-bot mode)...");
    info!("Configured bots: {}", config.bots.len());

    // Ensure OPENAI_API_KEY is set for the openai crate
    std::env::set_var("OPENAI_API_KEY", &config.openai_api_key);
    std::env::set_var("OPENAI_KEY", &config.openai_api_key);

    // Fetch application IDs from Discord API
    info!("Fetching application IDs from Discord API...");
    populate_application_ids(&mut config).await?;

    // Create shared resources
    let database = Arc::new(Database::new(&config.database_path).await?);
    let persona_manager = Arc::new(PersonaManager::new());

    // Log bot configurations
    for bot in &config.bots {
        info!(
            "  - {} (ID: {}, persona: {:?})",
            bot.name,
            bot.bot_id(),
            bot.default_persona
        );
    }

    // Spawn a task for each bot
    let mut handles = vec![];

    for bot_config in config.bots.clone() {
        let db = database.clone();
        let pm = persona_manager.clone();
        let cfg = config.clone();

        let handle = tokio::spawn(async move {
            run_bot(bot_config, &cfg, db, pm).await
        });

        handles.push(handle);
    }

    // Set up Ctrl+C handler for graceful shutdown
    let shutdown_handle = tokio::spawn(async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating graceful shutdown...");
                SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                error!("Failed to listen for Ctrl+C: {}", e);
            }
        }
    });

    // Wait for all bots to complete (or first failure in single-bot mode)
    let results = join_all(handles).await;

    // Cancel shutdown handler
    shutdown_handle.abort();

    // Report results
    let mut all_ok = true;
    for (i, result) in results.into_iter().enumerate() {
        let bot_name = config.bots.get(i).map(|b| b.name.as_str()).unwrap_or("unknown");
        match result {
            Ok(Ok(())) => {
                info!("[{}] Bot exited successfully", bot_name);
            }
            Ok(Err(e)) => {
                error!("[{}] Bot failed: {}", bot_name, e);
                all_ok = false;
            }
            Err(e) => {
                error!("[{}] Bot task panicked: {}", bot_name, e);
                all_ok = false;
            }
        }
    }

    if all_ok {
        info!("All bots shut down successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!("One or more bots failed"))
    }
}
