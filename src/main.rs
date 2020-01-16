pub mod bot;
pub mod commands;
pub mod error;
pub mod state;

use crate::bot::run_discord_bot;

fn main() {
    run_discord_bot();
}
