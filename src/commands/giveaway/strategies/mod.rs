pub mod base;
pub mod manual;

pub use crate::commands::giveaway::strategies::base::{GiveawayStrategy, RollOptions};
pub use crate::commands::giveaway::strategies::manual::ManualSelectStrategy;
