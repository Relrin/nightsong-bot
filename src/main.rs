#[macro_use]
extern crate diesel;

pub mod bot;
pub mod commands;
pub mod db;
pub mod error;

use crate::bot::run_discord_bot;

fn main() {
    run_discord_bot();
}
