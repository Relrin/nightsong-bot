use std::collections::HashSet;
use std::sync::Arc;

use serenity::framework::standard::{Args, Delimiter};

use crate::commands::giveaway::models::{ConcurrencyReward, ObjectState, Reward};
use crate::commands::giveaway::strategies::base::{GiveawayStrategy, RollOptions};
use crate::error::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct ManualSelectStrategy;

impl ManualSelectStrategy {
    pub fn new() -> Self {
        ManualSelectStrategy {}
    }

    fn check_rewards_are_defined(&self, options: &RollOptions) -> Result<()> {
        if options.rewards().lock().unwrap().len() == 0 {
            let message = format!(
                "The giveaway doesn't have any rewards. Please, add rewards \
                or ask to do an owner."
            );
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }

    fn check_user_has_pending_rewards(&self, options: &RollOptions) -> Result<()> {
        let user_id = options.user().get_user_id();
        let pending_rewards = match options.stats().get(&user_id) {
            Some(pair) => pair.value().pending_rewards(),
            None => HashSet::new(),
        };

        let pending_rewards = options
            .rewards()
            .clone()
            .lock()
            .unwrap()
            .iter()
            .filter(|obj| {
                let reward_id = obj.id();
                let is_pending = obj.get_object_state() == ObjectState::Pending;
                is_pending && pending_rewards.contains(&reward_id)
            })
            .map(|reward| reward.clone())
            .collect::<Vec<ConcurrencyReward>>();

        if pending_rewards.len() > 0 {
            let message = format!(
                "It's not possible to have more than one reward in \
                the pending state. Please, activate previous reward, \
                or invoke the `!greroll` command."
            );
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }

    fn check_no_unused_rewards(&self, options: &RollOptions) -> Result<()> {
        let no_unused_rewards = options
            .rewards()
            .clone()
            .lock()
            .unwrap()
            .iter()
            .filter(|obj| obj.get_object_state() == ObjectState::Unused)
            .map(|reward| reward.clone())
            .collect::<Vec<ConcurrencyReward>>()
            .is_empty();

        if no_unused_rewards {
            let message = format!("All possible rewards have been handed out.");
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }

    fn get_reward(&self, options: &RollOptions) -> Result<Arc<Box<Reward>>> {
        let mut args = Args::new(options.raw_message(), &[Delimiter::Single(' ')]);
        let index = match args.single::<usize>() {
            Ok(value) => value,
            Err(_) => {
                let message = format!("The request reward index must be a positive integer.");
                return Err(Error::from(ErrorKind::Giveaway(message)));
            }
        };

        let ref_rewards = options.rewards().clone();
        let guard_rewards = ref_rewards.lock().unwrap();
        match index > 0 && index < guard_rewards.len() + 1 {
            true => {
                let reward = guard_rewards[index - 1].clone();

                if reward.get_object_state() != ObjectState::Unused {
                    let message = format!("This reward has already been taken by someone.");
                    return Err(Error::from(ErrorKind::Giveaway(message)));
                }

                Ok(reward)
            }
            false => {
                let message = format!("The requested reward was not found.");
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }
}

impl GiveawayStrategy for ManualSelectStrategy {
    fn roll(&self, options: &RollOptions) -> Result<Arc<Box<Reward>>> {
        self.check_rewards_are_defined(options)?;
        self.check_user_has_pending_rewards(options)?;
        self.check_no_unused_rewards(options)?;
        let reward = self.get_reward(options)?;
        Ok(reward)
    }

    fn to_message(&self, _reward: Arc<Box<Reward>>) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashmap::DashMap;
    use serenity::model::id::UserId;
    use serenity::model::user::{CurrentUser, User as DiscordUser};

    use crate::commands::giveaway::models::{ObjectState, Participant, ParticipantStats, Reward};
    use crate::commands::giveaway::strategies::{
        GiveawayStrategy, ManualSelectStrategy, RollOptions,
    };
    use crate::error::{Error, ErrorKind};

    fn get_user(user_id: u64, username: &str) -> DiscordUser {
        let mut current_user = CurrentUser::default();
        current_user.id = UserId(user_id);
        current_user.name = username.to_owned();
        DiscordUser::from(current_user)
    }

    #[test]
    fn test_get_reward() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);
        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        let rewards = Arc::new(Mutex::new(Box::new(vec![reward_1.clone()])));
        let stats = Arc::new(DashMap::new());
        let options = RollOptions::new(&participant, &rewards, "1", &stats);

        let strategy = ManualSelectStrategy::new();
        let roll = strategy.roll(&options).unwrap();
        assert_eq!(roll, reward_1);
    }

    #[test]
    fn test_get_error_for_empty_rewards() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);
        let rewards = Arc::new(Mutex::new(Box::new(vec![])));
        let stats = Arc::new(DashMap::new());
        let options = RollOptions::new(&participant, &rewards, "1", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The giveaway doesn't have any rewards. Please, add rewards \
                or ask to do an owner."
            )))
        );
    }

    #[test]
    fn test_get_error_for_when_user_has_pending_reward() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);

        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        reward_1.set_object_state(ObjectState::Pending);
        let reward_2 = Arc::new(Box::new(Reward::new("reward #2")));
        let rewards = Arc::new(Mutex::new(Box::new(vec![
            reward_1.clone(),
            reward_2.clone(),
        ])));

        let mut participant_1_stats = ParticipantStats::new();
        participant_1_stats.add_pending_reward(reward_1.id());
        let stats = Arc::new(DashMap::new());
        stats.insert(participant.get_user_id(), participant_1_stats);

        let options = RollOptions::new(&participant, &rewards, "2", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "It's not possible to have more than one reward in \
                the pending state. Please, activate previous reward, \
                or invoke the `!greroll` command."
            )))
        );
    }

    #[test]
    fn test_get_error_for_no_available_reward_and_they_were_all_taken() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);

        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        reward_1.set_object_state(ObjectState::Activated);
        let reward_2 = Arc::new(Box::new(Reward::new("reward #2")));
        reward_2.set_object_state(ObjectState::Activated);
        let rewards = Arc::new(Mutex::new(Box::new(vec![
            reward_1.clone(),
            reward_2.clone(),
        ])));

        let mut participant_1_stats = ParticipantStats::new();
        participant_1_stats.add_retrieved_reward(reward_1.id());
        participant_1_stats.add_retrieved_reward(reward_2.id());
        let stats = Arc::new(DashMap::new());
        stats.insert(participant.get_user_id(), participant_1_stats);

        let options = RollOptions::new(&participant, &rewards, "2", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "All possible rewards have been handed out."
            )))
        );
    }

    #[test]
    fn test_get_error_for_invalid_argument_input() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);
        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        let rewards = Arc::new(Mutex::new(Box::new(vec![reward_1.clone()])));
        let stats = Arc::new(DashMap::new());
        let options = RollOptions::new(&participant, &rewards, "", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The request reward index must be a positive integer."
            )))
        );
    }

    #[test]
    fn test_get_error_for_already_taken_reward_by_someone() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);

        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        reward_1.set_object_state(ObjectState::Activated);
        let reward_2 = Arc::new(Box::new(Reward::new("reward #2")));
        let rewards = Arc::new(Mutex::new(Box::new(vec![
            reward_1.clone(),
            reward_2.clone(),
        ])));

        let mut participant_1_stats = ParticipantStats::new();
        participant_1_stats.add_retrieved_reward(reward_1.id());
        let stats = Arc::new(DashMap::new());
        stats.insert(participant.get_user_id(), participant_1_stats);

        let options = RollOptions::new(&participant, &rewards, "1", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "This reward has already been taken by someone."
            )))
        );
    }

    #[test]
    fn test_get_error_for_invalid_index_value() {
        let user = get_user(1, "Test");
        let participant = Participant::from(user);
        let reward_1 = Arc::new(Box::new(Reward::new("reward #1")));
        let rewards = Arc::new(Mutex::new(Box::new(vec![reward_1.clone()])));
        let stats = Arc::new(DashMap::new());
        let options = RollOptions::new(&participant, &rewards, "2", &stats);

        let strategy = ManualSelectStrategy::new();
        let result = strategy.roll(&options);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The requested reward was not found."
            )))
        );
    }
}
