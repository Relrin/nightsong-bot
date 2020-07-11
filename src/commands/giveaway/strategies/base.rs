use std::sync::Arc;

use dashmap::DashMap;

use crate::commands::giveaway::models::{
    ConcurrencyRewardsVec, Participant, ParticipantStats, Reward,
};
use crate::error::Result;

pub struct RollOptions<'a> {
    user: &'a Participant,
    rewards: &'a ConcurrencyRewardsVec,
    raw_message: &'a str,
    stats: Arc<DashMap<u64, ParticipantStats>>,
}

impl<'a> RollOptions<'a> {
    pub fn new(
        user: &'a Participant,
        rewards: &'a ConcurrencyRewardsVec,
        raw_message: &'a str,
        stats: &Arc<DashMap<u64, ParticipantStats>>,
    ) -> Self {
        RollOptions {
            user,
            rewards,
            raw_message,
            stats: stats.clone(),
        }
    }

    // Returns the initiator of the roll command.
    pub fn user(&self) -> &'a Participant {
        self.user
    }

    // Returns a list of reward of the giveaway.
    pub fn rewards(&self) -> &'a ConcurrencyRewardsVec {
        self.rewards
    }

    // Returns the raw message as is for later usage.
    pub fn raw_message(&self) -> &'a str {
        self.raw_message
    }

    // Returns latest statistics in according with the requested giveaway.
    pub fn stats(&self) -> Arc<DashMap<u64, ParticipantStats>> {
        self.stats.clone()
    }
}

pub trait GiveawayStrategy: Send + Sync {
    // Returns a reward in according with the passed roll options.
    fn roll(&self, options: &RollOptions) -> Result<Arc<Box<Reward>>>;

    // Converts the reward instance into the text message. Returns None when
    // no need to send a message to user.
    fn to_message(&self, reward: Arc<Box<Reward>>) -> Option<String>;
}
