//! # Command System
//!
//! Unified command handling for slash commands (/) and bang commands (!).
//!
//! - **Version**: 1.0.0
//! - **Since**: 0.2.0
//! - **Toggleable**: false
//!
//! ## Changelog
//! - 1.0.0: Initial reorganization with modular command structure

pub mod bang;
pub mod slash;

// Re-export the CommandHandler from the handler module
pub use crate::command_handler::CommandHandler;

// Re-export commonly used items from submodules
pub use bang::{parse_bang_command, BangCommand};
pub use slash::{
    create_context_menu_commands, create_slash_commands, filter_slash_commands,
    get_channel_option, get_integer_option, get_role_option, get_string_option,
    register_global_commands, register_guild_commands,
};
