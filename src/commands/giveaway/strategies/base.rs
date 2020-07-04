use crate::error::Result;

pub struct RollOptions {
    input: Option<String>,
}

pub trait GiveawayStrategy {
    fn roll(options: &RollOptions) -> Result<String>;
}
