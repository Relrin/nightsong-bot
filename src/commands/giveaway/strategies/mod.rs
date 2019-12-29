use crate::error::Result;

struct RollOptions {
    input: Option<String>,
}

trait GiveawayStrategy {
    fn roll(options: &RollOptions) -> Result<String>;
}
