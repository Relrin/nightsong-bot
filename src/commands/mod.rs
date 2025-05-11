pub mod giveaway;
pub mod help;

// Re-exports for the later usage in bot.rs
pub use crate::commands::giveaway::GIVEAWAY_GROUP;
pub use crate::commands::help::GET_COMMANDS_LIST;

// Poise specific data
// User data, which is stored and accessible in all command invocations
pub struct UserData {}
pub type Context<'a> = poise::Context<'a, UserData, crate::error::Error>;
