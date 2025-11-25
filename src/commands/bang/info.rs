//! Info bang commands: !help, !status, !version, !uptime

use crate::features;

/// Generate help response
pub fn help() -> String {
    super::get_help_text()
}

/// Generate status response
pub fn status(start_time: std::time::Instant) -> String {
    let uptime = start_time.elapsed();
    let hours = uptime.as_secs() / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    format!(
        "**Bot Status**\n\
        ✅ Online and operational\n\
        ⏱️ Uptime: {}h {}m {}s\n\
        📦 Version: {}",
        hours,
        minutes,
        seconds,
        features::get_bot_version()
    )
}

/// Generate version response
pub fn version() -> String {
    let mut output = format!("**Persona Bot v{}**\n\n", features::get_bot_version());
    output.push_str("**Feature Versions:**\n");

    for feature in features::get_features() {
        output.push_str(&format!("• {} v{}\n", feature.name, feature.version));
    }

    output
}

/// Generate uptime response
pub fn uptime(start_time: std::time::Instant) -> String {
    let uptime = start_time.elapsed();
    let days = uptime.as_secs() / 86400;
    let hours = (uptime.as_secs() % 86400) / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    if days > 0 {
        format!("⏱️ Uptime: {days}d {hours}h {minutes}m {seconds}s")
    } else if hours > 0 {
        format!("⏱️ Uptime: {hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("⏱️ Uptime: {minutes}m {seconds}s")
    } else {
        format!("⏱️ Uptime: {seconds}s")
    }
}
