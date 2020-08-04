use std::sync::Arc;

use crate::commands::giveaway::models::Reward;

pub trait RewardFormatter {
    // Returns detailed info for the giveaway owner when necessary
    // to update the giveaway.
    fn debug_print(&self, reward: &Arc<Box<Reward>>) -> String;
    // Stylized print for the users in the channel when the giveaways
    // has been started.
    fn pretty_print(&self, reward: &Arc<Box<Reward>>) -> String;
}
