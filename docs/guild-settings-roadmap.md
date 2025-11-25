# Guild Settings Roadmap

This document outlines the guild-level settings for the Persona Discord Bot, including what's implemented and planned future additions.

## Currently Implemented

| Setting | Values | Command | Description |
|---------|--------|---------|-------------|
| `default_verbosity` | concise, normal, detailed | `/set_guild_setting` | Guild-wide default response length |
| `bot_admin_role` | Role ID | `/admin_role` | Role that can manage bot settings |

## Planned Settings

### High Priority

#### `default_persona`
- **Values:** obi, muppet, chef, teacher, analyst
- **Description:** Guild-wide default persona for users who haven't set their own
- **Current behavior:** Falls back to `PERSONA` env var or "obi"
- **Implementation:**
  - Add to `set_guild_setting` autocomplete
  - Update `database.get_user_persona()` to check guild default before system default

#### `conflict_mediation`
- **Values:** enabled, disabled
- **Description:** Toggle conflict detection and mediation per-guild
- **Current behavior:** Controlled only by `CONFLICT_MEDIATION_ENABLED` env var
- **Implementation:**
  - Check guild setting in `check_and_mediate_conflicts()`
  - Falls back to env var if not set

#### `conflict_sensitivity`
- **Values:** low, medium, high, ultra
- **Description:** How aggressively the bot detects conflicts
- **Current behavior:** Controlled only by `CONFLICT_SENSITIVITY` env var
- **Thresholds:**
  - low: 0.7 (only obvious conflicts)
  - medium: 0.5 (default)
  - high: 0.35 (more sensitive)
  - ultra: 0.3 (maximum sensitivity)
- **Implementation:**
  - Check guild setting in conflict detection
  - Falls back to env var if not set

#### `mediation_cooldown`
- **Values:** 1, 5, 10, 15, 30, 60 (minutes)
- **Description:** Minimum time between mediation attempts in same channel
- **Current behavior:** Controlled only by `MEDIATION_COOLDOWN_MINUTES` env var (default 5)
- **Implementation:**
  - Check guild setting in `conflict_mediator.can_intervene()`
  - Falls back to env var if not set

### Medium Priority

#### `max_context_messages`
- **Values:** 10, 20, 40, 60
- **Description:** How many conversation history messages to include in AI context
- **Current behavior:** Hardcoded to 40
- **Implementation:**
  - Pass to `get_conversation_history()` calls
  - Affects memory/token usage

#### `audio_transcription`
- **Values:** enabled, disabled
- **Description:** Toggle audio file transcription feature
- **Current behavior:** Always enabled if OpenAI API key is set
- **Implementation:**
  - Check before processing audio attachments in `handle_audio_attachments()`

#### `mention_responses`
- **Values:** enabled, disabled
- **Description:** Whether bot responds when @mentioned in channels
- **Current behavior:** Always responds to mentions
- **Implementation:**
  - Check in `handle_message()` before `handle_mention_message_with_id()`

### Lower Priority

#### `response_language`
- **Values:** en, es, fr, de, ja, etc.
- **Description:** Preferred language for bot responses
- **Current behavior:** Responds in English or matches user's language
- **Implementation:**
  - Add language instruction to system prompt suffix
  - Similar to verbosity suffix system

#### `dm_responses`
- **Values:** enabled, disabled
- **Description:** Whether bot responds in direct messages
- **Current behavior:** Always responds in DMs
- **Implementation:**
  - Check in `handle_message()` before `handle_dm_message_with_id()`

## Implementation Checklist

For each new setting:

1. [ ] Add to `set_guild_setting` autocomplete choices in:
   - `src/slash_commands.rs` (command definition)
   - `src/bin/bot.rs` (gateway autocomplete)

2. [ ] Add validation in `handle_set_guild_setting()` in `src/commands.rs`

3. [ ] Add display in `/settings` command output

4. [ ] Implement the actual behavior change in relevant handler

5. [ ] Update `docs/verbosity-and-permissions.md` with new settings

## Autocomplete Structure

When adding new settings, update the autocomplete handlers:

```rust
// In bot.rs
match setting {
    "default_verbosity" => /* existing */,
    "default_persona" => /* obi, muppet, chef, teacher, analyst */,
    "conflict_mediation" => /* enabled, disabled */,
    "conflict_sensitivity" => /* low, medium, high, ultra */,
    "mediation_cooldown" => /* 1, 5, 10, 15, 30, 60 */,
    // ... etc
    _ => /* empty */
}
```

## Database

All guild settings use the existing `guild_settings` table:

```sql
CREATE TABLE guild_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    setting_key TEXT NOT NULL,
    setting_value TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guild_id, setting_key)
);
```

No schema changes needed for new settings.

## Fallback Behavior

Settings follow this cascade:
1. Guild-specific setting (if set)
2. Environment variable (if applicable)
3. System default

This allows:
- Per-guild customization
- Server-wide defaults via env vars
- Sensible defaults when nothing is configured
