//! # Feature: Startup Notification
//!
//! Sends combined rich embed notifications when bots come online.
//! Multiple bots are grouped into a single message that updates progressively.
//! Supports DM to bot owner and/or specific guild channels.
//! Configuration is stored in the database and managed via /set_guild_setting.
//!
//! - **Version**: 2.0.0
//! - **Since**: 0.4.0
//! - **Toggleable**: true
//!
//! ## Changelog
//! - 2.0.0: Add combined multi-bot notifications with session-based progressive updates
//! - 1.1.0: Moved configuration from env vars to database
//! - 1.0.0: Initial release with DM and channel support, rich embeds

use crate::config::BotConfig;
use crate::database::{Database, StartedBot};
use crate::features::get_bot_version;
use anyhow::Result;
use log::{info, warn};
use serenity::builder::CreateEmbed;
use serenity::http::Http;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::utils::Color;
use std::sync::Arc;

/// Handles sending startup notifications to configured destinations
pub struct StartupNotifier {
    database: Arc<Database>,
}

impl StartupNotifier {
    /// Creates a new StartupNotifier with database access
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Sends startup notifications if enabled (with session-based progressive updates)
    pub async fn send_if_enabled(&self, http: &Http, ready: &Ready, bot_config: &BotConfig) {
        // 1. Check per-bot toggle
        if !bot_config.startup_notification_enabled.unwrap_or(true) {
            info!("Startup notifications disabled for bot {}", bot_config.name);
            return;
        }

        // 2. Check global enabled flag
        let enabled = self
            .database
            .get_bot_setting("startup_notification")
            .await
            .ok()
            .flatten()
            .map(|v| v == "enabled")
            .unwrap_or(false);

        if !enabled {
            info!("Startup notifications globally disabled");
            return;
        }

        // 3. Get destinations
        let owner_id = self
            .database
            .get_bot_setting("startup_notify_owner_id")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u64>().ok());

        let channel_id = self
            .database
            .get_bot_setting("startup_notify_channel_id")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u64>().ok());

        if owner_id.is_none() && channel_id.is_none() {
            info!("No notification destinations configured");
            return;
        }

        // 4. Get or create session
        let session_id = match get_or_create_session(&self.database).await {
            Ok(id) => id,
            Err(e) => {
                warn!("Failed to get/create session: {}", e);
                return;
            }
        };

        // 5. Add this bot to the session
        if let Err(e) = self
            .database
            .add_bot_to_startup_session(
                &session_id,
                &ready.user.id.to_string(),
                &bot_config.name,
                &get_bot_version(),
                ready.guilds.len() as i64,
                ready.shard.map(|s| format!("{}/{}", s[0] + 1, s[1])),
            )
            .await
        {
            warn!("Failed to add bot to session: {}", e);
            return;
        }

        // 6. Get all bots in this session
        let bots = match self.database.get_bots_in_session(&session_id).await {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to get bots in session: {}", e);
                return;
            }
        };

        // 7. Build combined embed
        let embed = build_combined_embed(&bots);

        // 8. Send or update messages
        let session_info = self.database.get_startup_session(&session_id).await.ok();

        if bots.len() == 1 {
            // First bot - send initial messages
            if let Some(oid) = owner_id {
                match send_to_owner(http, oid, embed.clone()).await {
                    Ok(msg_id) => {
                        let _ = self
                            .database
                            .update_session_owner_message(&session_id, &msg_id)
                            .await;
                    }
                    Err(e) => warn!("Failed to send startup DM: {}", e),
                }
            }

            if let Some(cid) = channel_id {
                match send_to_channel(http, cid, embed).await {
                    Ok(msg_id) => {
                        let _ = self
                            .database
                            .update_session_channel_message(&session_id, &msg_id)
                            .await;
                    }
                    Err(e) => warn!("Failed to send startup channel message: {}", e),
                }
            }
        } else {
            // Subsequent bot - update existing messages
            if let Some(oid) = owner_id {
                if let Some(msg_id) = session_info
                    .as_ref()
                    .and_then(|i| i.owner_message_id.as_ref())
                {
                    if let Err(e) = update_owner_message(http, oid, msg_id, embed.clone()).await {
                        warn!("Failed to update owner message: {}. Sending new.", e);
                        // Fallback: send new message
                        if let Ok(new_msg_id) = send_to_owner(http, oid, embed.clone()).await {
                            let _ = self
                                .database
                                .update_session_owner_message(&session_id, &new_msg_id)
                                .await;
                        }
                    }
                }
            }

            if let Some(cid) = channel_id {
                if let Some(msg_id) = session_info
                    .as_ref()
                    .and_then(|i| i.channel_message_id.as_ref())
                {
                    if let Err(e) = update_channel_message(http, cid, msg_id, embed.clone()).await {
                        warn!("Failed to update channel message: {}. Sending new.", e);
                        // Fallback: send new message
                        if let Ok(new_msg_id) = send_to_channel(http, cid, embed).await {
                            let _ = self
                                .database
                                .update_session_channel_message(&session_id, &new_msg_id)
                                .await;
                        }
                    }
                }
            }
        }

        // 9. Update session timestamp
        let _ = self.database.touch_startup_session(&session_id).await;
    }
}

/// Get or create a startup session
/// Sessions expire after 5 minutes of inactivity
async fn get_or_create_session(db: &Database) -> Result<String> {
    // Try to get a session updated in last 5 minutes
    if let Some(session_id) = db.get_recent_startup_session(5).await? {
        return Ok(session_id);
    }

    // Create new session with UUID
    let session_id = uuid::Uuid::new_v4().to_string();
    db.create_startup_session(&session_id).await?;
    Ok(session_id)
}

/// Builds the combined embed for multiple bots
fn build_combined_embed(bots: &[StartedBot]) -> CreateEmbed {
    let mut embed = CreateEmbed::default();

    let bot_count = bots.len();
    let title = if bot_count == 1 {
        format!("{} is Online!", bots[0].bot_name)
    } else {
        format!("{} Bots are Online!", bot_count)
    };

    embed
        .title(title)
        .color(Color::from_rgb(87, 242, 135))
        .description(format!(
            "System startup at {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

    // Add a field for each bot (max 10 due to Discord limit)
    for bot in bots.iter().take(10) {
        let mut value = format!("**Version**: `{}`\n**Guilds**: {}", bot.version, bot.guilds_count);

        if let Some(ref shard) = bot.shard_info {
            value.push_str(&format!("\n**Shard**: {}", shard));
        }

        value.push_str(&format!("\n**Started**: <t:{}:R>", bot.started_at.timestamp()));

        embed.field(&bot.bot_name, value, true);
    }

    // If more than 10 bots, show count in footer
    if bot_count > 10 {
        embed.footer(|f| f.text(format!("+ {} more bots", bot_count - 10)));
    } else {
        embed.footer(|f| f.text("All systems operational"));
    }

    embed.timestamp(chrono::Utc::now().to_rfc3339());
    embed
}

/// Send initial message to owner DM, return message ID
async fn send_to_owner(http: &Http, owner_id: u64, embed: CreateEmbed) -> Result<String> {
    let user = UserId(owner_id);
    let dm = user.create_dm_channel(http).await?;
    let msg = dm.send_message(http, |m| m.set_embed(embed)).await?;
    info!("Sent startup notification to owner {} via DM", owner_id);
    Ok(msg.id.to_string())
}

/// Send initial message to channel, return message ID
async fn send_to_channel(http: &Http, channel_id: u64, embed: CreateEmbed) -> Result<String> {
    let channel = ChannelId(channel_id);
    let msg = channel.send_message(http, |m| m.set_embed(embed)).await?;
    info!("Sent startup notification to channel {}", channel_id);
    Ok(msg.id.to_string())
}

/// Update existing owner DM message
async fn update_owner_message(
    http: &Http,
    owner_id: u64,
    message_id: &str,
    embed: CreateEmbed,
) -> Result<()> {
    let user = UserId(owner_id);
    let dm = user.create_dm_channel(http).await?;
    let msg_id = MessageId(message_id.parse()?);

    dm.edit_message(http, msg_id, |m| m.set_embed(embed)).await?;

    info!("Updated startup notification DM for owner {}", owner_id);
    Ok(())
}

/// Update existing channel message
async fn update_channel_message(
    http: &Http,
    channel_id: u64,
    message_id: &str,
    embed: CreateEmbed,
) -> Result<()> {
    let channel = ChannelId(channel_id);
    let msg_id = MessageId(message_id.parse()?);

    channel.edit_message(http, msg_id, |m| m.set_embed(embed)).await?;

    info!("Updated startup notification in channel {}", channel_id);
    Ok(())
}
