use poise::Context as PoiseContext;

// User data, which is stored and accessible in all command invocations
pub struct UserData {}

// Generic context available across Poise commands
pub type Context<'a> = PoiseContext<'a, UserData, crate::error::Error>;
