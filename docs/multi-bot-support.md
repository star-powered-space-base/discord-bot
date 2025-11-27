# Multi-Discord-App Support Implementation Plan

## Executive Summary

This document outlines a comprehensive plan to enable the persona bot to support multiple Discord applications simultaneously. Currently, the bot is designed for a single Discord application with one token and one identity. This plan details the architectural changes needed to run multiple bots concurrently while sharing resources efficiently.

**Estimated Timeline**: 2 weeks
**Complexity**: Medium
**Risk Level**: Medium (primarily database migration)

---

## Current Architecture Analysis

### What Works Well (No Changes Needed)

1. **Persona System** ([src/personas.rs](../src/personas.rs))
   - Already stateless and shareable across multiple bots
   - Loads personas from `/prompt/*.md` files
   - No bot-specific state
   - ✅ Ready for multi-bot use

2. **OpenAI Integration**
   - Stateless API client
   - Can be shared across all bots
   - ✅ Ready for multi-bot use

3. **Modular Architecture**
   - Clean separation of concerns
   - Use of `Arc<>` for shared resources
   - Good foundation for multi-bot support

### Critical Blockers

#### 1. Database Schema ([src/database.rs](../src/database.rs))

**Problem**: No bot identity tracking

Current tables lack `bot_id` column:
- `user_preferences`: Keyed only by `user_id`
  - Issue: Same user across multiple bots would conflict
- `conversation_history`: Keyed by `user_id` + `channel_id`
  - Issue: Can't distinguish which bot had which conversation
- `guild_settings`: Keyed by `guild_id` + `setting_key`
  - Issue: Same guild could have different settings per bot
- `usage_stats`: No bot identifier
  - Issue: Can't track usage per bot

**Impact**: HIGH - Requires schema migration and ~50 method updates

#### 2. Configuration System ([src/config.rs](../src/config.rs))

**Problem**: Hardcoded single bot design

```rust
pub struct Config {
    pub discord_token: String,          // Only ONE token
    pub openai_api_key: String,
    pub database_path: String,
    pub log_level: String,
}
```

- Loads from `DISCORD_MUPPET_FRIEND` env variable
- No concept of multiple bot identities
- No structure for per-bot configuration

**Impact**: HIGH - Needs complete redesign

#### 3. Entry Point ([src/bin/bot.rs](../src/bin/bot.rs))

**Problem**: Single synchronous client

```rust
let mut client = Client::builder(&config.discord_token, intents)
    .event_handler(handler)
    .await?;

client.start().await?;  // Blocks forever - can't start another bot
```

**Impact**: MEDIUM - Needs async task spawning

#### 4. Command Handler ([src/commands.rs](../src/commands.rs))

**Problem**: No bot context awareness

- All database calls lack `bot_id` parameter
- Rate limiting per user, not per bot-user
- No way to distinguish which bot is handling a command

**Impact**: MEDIUM - Needs context propagation

---

## Implementation Plan

### Phase 1: Database Multi-Tenancy ⚠️ CRITICAL FIRST STEP

#### 1.1 Schema Migration

Add `bot_id TEXT NOT NULL` to all tables:

```sql
-- Migration script
ALTER TABLE user_preferences ADD COLUMN bot_id TEXT NOT NULL DEFAULT 'default';
ALTER TABLE conversation_history ADD COLUMN bot_id TEXT NOT NULL DEFAULT 'default';
ALTER TABLE guild_settings ADD COLUMN bot_id TEXT NOT NULL DEFAULT 'default';
ALTER TABLE usage_stats ADD COLUMN bot_id TEXT NOT NULL DEFAULT 'default';

-- Update primary keys
-- user_preferences: (user_id) -> (bot_id, user_id)
-- conversation_history: (id) -> keep id, add index on (bot_id, user_id, channel_id)
-- guild_settings: (guild_id, setting_key) -> (bot_id, guild_id, setting_key)
```

#### 1.2 Database Method Updates

Update all methods in `src/database.rs` to accept `bot_id` parameter:

**Before**:
```rust
pub async fn get_user_persona(&self, user_id: &str) -> Result<Option<String>>
```

**After**:
```rust
pub async fn get_user_persona(&self, bot_id: &str, user_id: &str) -> Result<Option<String>>
```

Affected methods (~50 total):
- `get_user_persona`
- `set_user_persona`
- `get_conversation_history`
- `store_message`
- `clear_conversation_history`
- `get_guild_setting`
- `set_guild_setting`
- `record_command_usage`
- All other database operations

#### 1.3 Migration Strategy

**Option A**: Assign existing data to default bot
- Set `bot_id = 'default'` for all existing records
- New bots get unique IDs (e.g., 'muppet', 'chef', 'teacher')
- Pros: Preserves existing data
- Cons: Migration required for live databases

**Option B**: Fresh start
- Drop and recreate tables with new schema
- Pros: Clean implementation
- Cons: Lose all conversation history

**Recommendation**: Option A with SQL migration script

#### 1.4 Deliverables

- [ ] SQL migration script: `migrations/001_add_bot_id.sql`
- [ ] Updated database schema in `database.rs`
- [ ] All database methods accept `bot_id` parameter
- [ ] Integration tests for multi-bot data isolation
- [ ] Migration guide for production databases

**Estimated Time**: 3-5 days

---

### Phase 2: Configuration System Redesign

#### 2.1 New Configuration Structures

```rust
// src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    /// Unique identifier for this bot instance
    pub bot_id: String,

    /// Friendly name for logging
    pub name: String,

    /// Discord bot token
    pub discord_token: String,

    /// Optional: Default persona for this bot
    pub default_persona: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultiConfig {
    /// List of bot configurations
    pub bots: Vec<BotConfig>,

    /// Shared OpenAI API key
    pub openai_api_key: String,

    /// Shared database path
    pub database_path: String,

    /// Logging configuration
    pub log_level: String,
}

impl MultiConfig {
    /// Load from YAML/JSON file
    pub fn from_file(path: &str) -> Result<Self> {
        // Implementation
    }

    /// Load from environment variables (backward compatible)
    pub fn from_env_single_bot() -> Result<Self> {
        // Creates MultiConfig with single bot from DISCORD_MUPPET_FRIEND
    }
}
```

#### 2.2 Configuration File Format

**config.yaml**:
```yaml
bots:
  - bot_id: "muppet"
    name: "Muppet Friend"
    discord_token: "${DISCORD_MUPPET_TOKEN}"
    default_persona: "muppet"

  - bot_id: "chef"
    name: "Chef Bot"
    discord_token: "${DISCORD_CHEF_TOKEN}"
    default_persona: "chef"

  - bot_id: "teacher"
    name: "Teacher Bot"
    discord_token: "${DISCORD_TEACHER_TOKEN}"
    default_persona: "teacher"

openai_api_key: "${OPENAI_API_KEY}"
database_path: "./persona.db"
log_level: "info"
```

#### 2.3 Backward Compatibility

Support both old and new configuration methods:

```rust
// Option 1: New multi-bot config file
let config = MultiConfig::from_file("config.yaml")?;

// Option 2: Legacy single-bot env vars
let config = MultiConfig::from_env_single_bot()?;
```

#### 2.4 Deliverables

- [ ] New config structures in `config.rs`
- [ ] YAML/JSON file parsing support
- [ ] Environment variable interpolation
- [ ] Backward compatibility layer
- [ ] Example `config.yaml` file
- [ ] Configuration validation

**Estimated Time**: 1-2 days

---

### Phase 3: Multi-Client Gateway Architecture

#### 3.1 Refactor Entry Point

**Current** ([src/bin/bot.rs](../src/bin/bot.rs)):
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    // Single client only
    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;  // Blocks forever
}
```

**New**:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load multi-bot configuration
    let config = if Path::new("config.yaml").exists() {
        MultiConfig::from_file("config.yaml")?
    } else {
        MultiConfig::from_env_single_bot()?
    };

    // Shared resources
    let database = Arc::new(Database::new(&config.database_path).await?);
    let persona_manager = Arc::new(PersonaManager::new());
    let openai_api_key = config.openai_api_key.clone();

    // Spawn one task per bot
    let mut handles = vec![];

    for bot_config in config.bots {
        let db = Arc::clone(&database);
        let pm = Arc::clone(&persona_manager);
        let api_key = openai_api_key.clone();

        let handle = tokio::spawn(async move {
            run_bot(bot_config, db, pm, api_key).await
        });

        handles.push(handle);
    }

    // Wait for all bots (or first failure)
    let results = futures::future::join_all(handles).await;

    // Handle errors
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(())) => info!("Bot {} exited successfully", i),
            Ok(Err(e)) => error!("Bot {} failed: {}", i, e),
            Err(e) => error!("Bot {} task panicked: {}", i, e),
        }
    }

    Ok(())
}

async fn run_bot(
    bot_config: BotConfig,
    database: Arc<Database>,
    persona_manager: Arc<PersonaManager>,
    openai_api_key: String,
) -> Result<()> {
    info!("Starting bot: {} ({})", bot_config.name, bot_config.bot_id);

    let command_handler = CommandHandler::new(
        bot_config.bot_id.clone(),  // NEW: Pass bot_id
        persona_manager,
        database,
        openai_api_key,
    );

    let handler = Handler {
        command_handler: Arc::new(command_handler),
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&bot_config.discord_token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;

    Ok(())
}
```

#### 3.2 Error Handling & Restart Logic

```rust
// Add retry logic for individual bot failures
async fn run_bot_with_retry(/* ... */) -> Result<()> {
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 5;

    loop {
        match run_bot(/* ... */).await {
            Ok(()) => break,
            Err(e) if retry_count < MAX_RETRIES => {
                error!("Bot {} failed: {}. Retrying ({}/{})",
                    bot_config.name, e, retry_count + 1, MAX_RETRIES);
                retry_count += 1;
                tokio::time::sleep(Duration::from_secs(5 * retry_count as u64)).await;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
```

#### 3.3 Graceful Shutdown

```rust
use tokio::signal;

// In main()
tokio::select! {
    _ = signal::ctrl_c() => {
        info!("Received Ctrl+C, shutting down all bots...");
        // Cancel all bot tasks
    }
    results = futures::future::join_all(handles) => {
        // Handle normal completion
    }
}
```

#### 3.4 Deliverables

- [ ] Refactored `bin/bot.rs` with multi-client spawning
- [ ] Shared resource management (Arc-wrapped)
- [ ] Per-bot error handling and logging
- [ ] Graceful shutdown mechanism
- [ ] Bot restart logic for transient failures
- [ ] Structured logging with bot_id context

**Estimated Time**: 2-4 days

---

### Phase 4: Context Propagation

#### 4.1 Update CommandHandler

**Current**:
```rust
pub struct CommandHandler {
    persona_manager: PersonaManager,
    database: Database,
    rate_limiter: RateLimiter,
    audio_transcriber: AudioTranscriber,
    openai_api_key: String,
}
```

**New**:
```rust
pub struct CommandHandler {
    bot_id: String,  // NEW: Bot identity
    persona_manager: PersonaManager,
    database: Database,
    rate_limiter: RateLimiter,
    audio_transcriber: AudioTranscriber,
    openai_api_key: String,
}

impl CommandHandler {
    pub fn new(
        bot_id: String,  // NEW parameter
        persona_manager: Arc<PersonaManager>,
        database: Arc<Database>,
        openai_api_key: String,
    ) -> Self {
        Self {
            bot_id,
            persona_manager: (*persona_manager).clone(),
            database: (*database).clone(),
            rate_limiter: RateLimiter::new(),
            audio_transcriber: AudioTranscriber::new(),
            openai_api_key,
        }
    }
}
```

#### 4.2 Update All Command Methods

**Example - handle_chat**:
```rust
// Before
pub async fn handle_chat(&self, ctx: &Context, msg: &Message) -> Result<()> {
    let persona = self.database.get_user_persona(&msg.author.id.to_string()).await?;
    // ...
}

// After
pub async fn handle_chat(&self, ctx: &Context, msg: &Message) -> Result<()> {
    let persona = self.database
        .get_user_persona(&self.bot_id, &msg.author.id.to_string())
        .await?;
    // ...
}
```

Apply this pattern to all methods:
- `handle_chat`
- `handle_persona_command`
- `handle_clear_command`
- `handle_help_command`
- `handle_stats_command`
- All other command handlers

#### 4.3 Update Rate Limiter

**Current**:
```rust
// Rate limiter keyed by user_id only
pub struct RateLimiter {
    last_interaction: HashMap<String, Instant>,
}
```

**New**:
```rust
// Rate limiter keyed by (bot_id, user_id)
pub struct RateLimiter {
    last_interaction: HashMap<(String, String), Instant>,
}

impl RateLimiter {
    pub fn check_rate_limit(&mut self, bot_id: &str, user_id: &str) -> bool {
        let key = (bot_id.to_string(), user_id.to_string());
        // ... rest of logic
    }
}
```

#### 4.4 Update All Database Calls

Systematically update every database call to include `bot_id`:

```rust
// Pattern: Add &self.bot_id as first parameter
self.database.method_name(&self.bot_id, /* other params */).await?;
```

#### 4.5 Deliverables

- [ ] Add `bot_id` field to `CommandHandler`
- [ ] Update all command handler methods
- [ ] Update rate limiter to use composite keys
- [ ] Update all database calls with bot_id
- [ ] Add integration tests for context isolation
- [ ] Verify no conversation bleeding between bots

**Estimated Time**: 2-3 days

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bot_data_isolation() {
        let db = Database::new(":memory:").await.unwrap();

        // Set persona for bot1
        db.set_user_persona("bot1", "user123", "muppet").await.unwrap();

        // Set different persona for same user on bot2
        db.set_user_persona("bot2", "user123", "chef").await.unwrap();

        // Verify isolation
        assert_eq!(
            db.get_user_persona("bot1", "user123").await.unwrap(),
            Some("muppet".to_string())
        );
        assert_eq!(
            db.get_user_persona("bot2", "user123").await.unwrap(),
            Some("chef".to_string())
        );
    }

    #[tokio::test]
    async fn test_conversation_history_isolation() {
        // Similar test for conversation history
    }

    #[test]
    fn test_rate_limiter_per_bot() {
        let mut limiter = RateLimiter::new();

        // User should be rate limited per bot, not globally
        assert!(limiter.check_rate_limit("bot1", "user123"));
        assert!(limiter.check_rate_limit("bot2", "user123"));  // Different bot, should pass
    }
}
```

### Integration Tests

1. **Multi-Bot Startup**: Verify all bots connect successfully
2. **Data Isolation**: Send commands to different bots, verify no data bleeding
3. **Concurrent Operations**: Stress test with simultaneous requests to all bots
4. **Bot Failure Recovery**: Kill one bot, verify others continue
5. **Configuration Loading**: Test both YAML and env var configurations

### Manual Testing Checklist

- [ ] Start 2+ bots with different tokens
- [ ] Send DM to each bot, verify separate conversation histories
- [ ] Set different personas on same user across bots
- [ ] Verify guild settings are per-bot
- [ ] Check usage stats tracked separately
- [ ] Test rate limiting per bot
- [ ] Verify graceful shutdown
- [ ] Test bot restart after crash

---

## Migration Guide

### For Existing Deployments

#### Step 1: Backup Database
```bash
cp persona.db persona.db.backup
```

#### Step 2: Run Migration Script
```bash
sqlite3 persona.db < migrations/001_add_bot_id.sql
```

#### Step 3: Update Configuration

Create `config.yaml`:
```yaml
bots:
  - bot_id: "default"  # Match migration default
    name: "Main Bot"
    discord_token: "${DISCORD_MUPPET_FRIEND}"
    default_persona: "muppet"

openai_api_key: "${OPENAI_API_KEY}"
database_path: "./persona.db"
log_level: "info"
```

#### Step 4: Deploy New Version

```bash
cargo build --release
./target/release/bot  # Will auto-detect config.yaml
```

#### Step 5: Add Additional Bots

Edit `config.yaml` to add more bot configurations, then restart.

---

## Monitoring & Observability

### Structured Logging

```rust
use tracing::{info, error, warn};

// Log with bot context
info!(
    bot_id = %self.bot_id,
    user_id = %user_id,
    "Processing chat command"
);
```

### Metrics to Track

Per bot:
- Active connections
- Messages processed
- Commands executed
- Rate limit hits
- Errors encountered
- API latency (OpenAI, Discord)

### Health Checks

```rust
// Optional: Add health check endpoint
async fn health_check(registry: Arc<BotRegistry>) -> Json<HealthStatus> {
    let status = registry.bots.iter().map(|(id, bot)| {
        (id.clone(), bot.is_connected())
    }).collect();

    Json(HealthStatus { bots: status })
}
```

---

## Performance Considerations

### Resource Usage

**Per Bot**:
- 1 WebSocket connection to Discord Gateway
- ~10-50 MB memory (depending on cache size)
- Minimal CPU (event-driven)

**Shared**:
- SQLite database (single file, thread-safe)
- OpenAI HTTP client (connection pool)
- Persona manager (lightweight, in-memory)

**Scaling**: Should easily support 5-10 bots on modest hardware (2 CPU, 4GB RAM)

### Rate Limits

Discord API limits (per bot):
- 50 requests/second global
- 5 requests/second per channel
- 1 gateway connection per shard (5000 guilds)

**Mitigation**: Each bot has independent rate limits since they're separate applications.

### Database Contention

SQLite handles concurrent reads well but serializes writes. With multiple bots:
- Use WAL mode: `PRAGMA journal_mode=WAL;`
- Keep transactions short
- Consider connection pool if needed

---

## Risk Assessment

### High Risk

1. **Database Migration Failure**
   - Mitigation: Mandatory backup, rollback script, test on copy first

2. **Data Leakage Between Bots**
   - Mitigation: Extensive integration tests, code review on all database calls

### Medium Risk

1. **Bot Crash Affecting Others**
   - Mitigation: Isolated async tasks, error boundaries, restart logic

2. **Configuration Errors**
   - Mitigation: Validation on load, clear error messages, schema validation

### Low Risk

1. **Performance Degradation**
   - Mitigation: Monitoring, load testing before production

2. **Discord API Changes**
   - Mitigation: Pin serenity version, gradual upgrades

---

## Rollback Plan

If multi-bot deployment fails:

1. **Stop New Version**
   ```bash
   killall bot
   ```

2. **Restore Database Backup** (if migration was run)
   ```bash
   mv persona.db.backup persona.db
   ```

3. **Deploy Previous Version**
   ```bash
   git checkout <previous-tag>
   cargo build --release
   ./target/release/bot
   ```

4. **Revert to Env Var Configuration**
   ```bash
   export DISCORD_MUPPET_FRIEND=<token>
   ```

---

## Future Enhancements

### Phase 6+: Advanced Features

1. **Dynamic Bot Management**
   - Add/remove bots without restart
   - Hot-reload configuration
   - Admin API for bot management

2. **Per-Bot Customization**
   - Custom personas per bot
   - Different OpenAI models per bot
   - Bot-specific rate limits

3. **Cross-Bot Features**
   - User preferences that follow across bots
   - Shared conversation context (opt-in)
   - Bot-to-bot communication

4. **Scaling**
   - PostgreSQL for high-concurrency deployments
   - Redis for distributed rate limiting
   - Separate processes for Gateway vs HTTP bots

5. **Monitoring Dashboard**
   - Real-time bot status
   - Usage analytics per bot
   - Cost tracking per bot (OpenAI API)

---

## Open Questions for Discussion

Before implementing, we should decide:

1. **Data Sharing Philosophy**
   - Should user preferences be per-bot or global with bot-specific overrides?
   - Should conversation history ever be shared between bots?

2. **Bot Identification**
   - Use Discord application_id or custom bot_id?
   - How to handle bot_id in logs/metrics?

3. **Configuration Management**
   - Require config file or support 100% env vars?
   - Support remote config (HTTP, S3, etc.)?

4. **Deployment Model**
   - Single process for all bots or ability to run separately?
   - Docker container per bot or monolithic?

---

## Appendix A: File Change Summary

### Major Changes Required

| File | Changes | Lines | Complexity |
|------|---------|-------|------------|
| `src/config.rs` | Complete rewrite | ~100 | High |
| `src/database.rs` | Add bot_id to all methods | ~300 | High |
| `src/bin/bot.rs` | Multi-client spawning | ~150 | Medium |
| `src/commands.rs` | Add bot_id context | ~200 | Medium |
| `src/rate_limiter.rs` | Composite keys | ~50 | Low |

### New Files Needed

- `migrations/001_add_bot_id.sql` - Database migration
- `config.yaml.example` - Example configuration
- `docs/multi-bot-setup.md` - User-facing setup guide
- `tests/integration/multi_bot_tests.rs` - Integration tests

### No Changes Required

- `src/personas.rs` - Already multi-bot compatible ✅
- `src/audio.rs` - Stateless ✅
- `src/message_components.rs` - Minor context updates only

---

## Appendix B: Database Schema (After Migration)

```sql
-- User preferences (after migration)
CREATE TABLE user_preferences (
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    persona TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (bot_id, user_id)
);

-- Conversation history (after migration)
CREATE TABLE conversation_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX idx_conversation ON conversation_history(bot_id, user_id, channel_id, timestamp);

-- Guild settings (after migration)
CREATE TABLE guild_settings (
    bot_id TEXT NOT NULL,
    guild_id TEXT NOT NULL,
    setting_key TEXT NOT NULL,
    setting_value TEXT NOT NULL,
    PRIMARY KEY (bot_id, guild_id, setting_key)
);

-- Usage statistics (after migration)
CREATE TABLE usage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    command TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX idx_usage ON usage_stats(bot_id, timestamp);
```

---

## Appendix C: Example Multi-Bot Config

```yaml
# config.yaml - Full example with 4 bots

bots:
  # Muppet personality bot
  - bot_id: "muppet"
    name: "Muppet Friend"
    discord_token: "${DISCORD_MUPPET_TOKEN}"
    default_persona: "muppet"

  # Chef personality bot
  - bot_id: "chef"
    name: "Chef Bot"
    discord_token: "${DISCORD_CHEF_TOKEN}"
    default_persona: "chef"

  # Teacher personality bot
  - bot_id: "teacher"
    name: "Teacher Bot"
    discord_token: "${DISCORD_TEACHER_TOKEN}"
    default_persona: "teacher"

  # Analyst personality bot
  - bot_id: "analyst"
    name: "Analyst Bot"
    discord_token: "${DISCORD_ANALYST_TOKEN}"
    default_persona: "analyst"

# Shared configuration
openai_api_key: "${OPENAI_API_KEY}"
database_path: "./persona.db"
log_level: "info"
```

---

## Conclusion

This implementation plan provides a comprehensive roadmap to enable multi-Discord-app support. The phased approach minimizes risk while delivering incremental value. The architecture maintains the existing persona system's elegance while adding the flexibility to run multiple bot identities simultaneously.

**Key Success Factors**:
- Careful database migration with rollback plan
- Comprehensive testing at each phase
- Backward compatibility during transition
- Clear separation of shared vs. per-bot resources

**Estimated Total Effort**: ~2 weeks for core implementation (Phases 1-4 using Gateway mode)

**Questions?** Review the "Open Questions" section and make decisions before beginning implementation.
