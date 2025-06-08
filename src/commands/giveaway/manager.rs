use std::collections::HashMap;
use std::sync::Arc;

use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use lazy_static::lazy_static;
use orx_concurrent_vec::ConcurrentVec;
use serenity::model::user::User as DiscordUser;
use uuid::Uuid;

use crate::commands::giveaway::models::{
    Giveaway, ObjectState, Participant, ParticipantStats, Reward
};
use crate::commands::giveaway::strategies::RollOptions;
use crate::error::{Error, ErrorKind, Result};

// TODO: Filter out invalid or delete giveaways
lazy_static! {
    pub static ref GIVEAWAY_MANAGER: GiveawayManager = GiveawayManager::new();
}

#[derive(Debug)]
#[non_exhaustive]
pub struct GiveawayManager {
    giveaways: Arc<ConcurrentVec<Arc<Box<Giveaway>>>>,
}

impl GiveawayManager {
    pub fn new() -> Self {
        GiveawayManager {
            giveaways: Arc::new(ConcurrentVec::new()),
        }
    }

    // Returns all current giveaways (started and on a pause).
    pub fn get_giveaways(&self) -> Vec<Arc<Box<Giveaway>>> {
        self.giveaways.clone_to_vec()
    }

    // Returns a giveaway by the given index.
    pub fn get_giveaway_by_index(&self, index: usize) -> Result<Arc<Box<Giveaway>>> {
        match index > 0 && index < self.giveaways.len() + 1 {
            true => {
                let giveaway = self.giveaways.get(index - 1).unwrap();
                Ok(giveaway.cloned())
            },
            false => {
                let message = "The requested giveaway was not found.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    // Sets the giveaway to the "active" state. Available only for the owner.
    pub fn activate_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        giveaway.activate();
        Ok(())
    }

    // Sets the giveaway to the "pause" state. Available only for the owner.
    pub fn deactivate_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?.clone();
        self.check_giveaway_owner(user, &giveaway)?;

        giveaway.deactivate();
        Ok(())
    }

    // Deletes the giveaway. Available only for the owner.
    pub fn delete_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        match index > 0 && index < self.giveaways.len() + 1 {
            true => {
                let giveaway = self.giveaways.get(index - 1).unwrap().cloned();

                if user.id.get() != giveaway.owner().get_user_id() {
                    let message = "For deleting this giveaway you need to be its owner.".to_string();
                    return Err(Error::from(ErrorKind::Giveaway(message)));
                }
                
                giveaway.mark_as_deleted();
                Ok(())
            }
            false => {
                let message = "The requested giveaway was not found.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    // Adds a new giveaway.
    pub fn add_giveaway(&self, giveaway: Giveaway) {
        self.giveaways.push(Arc::new(Box::new(giveaway)));
    }

    // Returns a list of reward for the certain giveaway. Mostly used for checks
    // before the beginning and debugging. Available only for the owner.
    pub fn get_giveaway_rewards(
        &self,
        user: &DiscordUser,
        index: usize,
    ) -> Result<Vec<Arc<Box<Reward>>>> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        let rewards = giveaway
            .get_available_rewards()
            .iter()
            .cloned()
            .collect::<Vec<Arc<Box<Reward>>>>();
        Ok(rewards)
    }

    // Parses the messages into the certain type of reward and adds to the certain
    // giveaway. Owners can add rewards only for their own giveaways.
    pub fn add_giveaway_reward(&self, user: &DiscordUser, index: usize, data: &str) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        let reward = Reward::new(data);
        giveaway.add_reward(&reward);

        Ok(())
    }

    // Parses the given message into multiple reward and then adds them to the
    // certain giveaway. The separator is the `\n` (just a new line) for each
    // declared reward. Owners can add rewards only for their own giveaways.
    pub fn add_multiple_giveaway_rewards(
        &self,
        user: &DiscordUser,
        index: usize,
        data: &str,
    ) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        for raw_reward_data in data.split("\n") {
            let reward = Reward::new(raw_reward_data);
            giveaway.add_reward(&reward);
        }

        Ok(())
    }

    // Removed the giveaway from the certain giveaways. Owners can remove rewards
    // only for their own giveaways.
    pub fn remove_giveaway_reward(
        &self,
        user: &DiscordUser,
        index: usize,
        reward_index: usize,
    ) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;
        giveaway.remove_reward_by_index(reward_index)?;
        Ok(())
    }

    // Returns a reward from the requested giveaway in according to the set strategy.
    pub fn roll_reward(
        &self,
        user: &DiscordUser,
        index: usize,
        reward_number: usize,
    ) -> Result<Option<String>> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_is_active(&giveaway)?;

        giveaway.update_actions_processed();

        let participant = Participant::from(user.clone());
        let stats = giveaway.stats();
        let rewards = giveaway.raw_rewards();
        let roll_options = RollOptions::new(&participant, &rewards, reward_number, &stats);
        let strategy = giveaway.strategy();
        let selected_reward = strategy.roll(&roll_options)?;

        let user_id = participant.get_user_id();
        let next_state = match stats.get_mut(&user_id) {
            Some(mut data) => self.get_next_reward_state_after_roll(&selected_reward, &mut data),
            None => {
                stats.insert(user_id, ParticipantStats::new());
                let mut data = stats.get_mut(&user_id).unwrap();
                self.get_next_reward_state_after_roll(&selected_reward, &mut data)
            }
        };
        selected_reward.set_object_state(next_state);

        let response = strategy.to_message(selected_reward);
        Ok(response)
    }

    // Returns a next state that needs to be set for the rolled reward. Also
    // updates user's statistics for tracking what have been taken.
    fn get_next_reward_state_after_roll(
        &self,
        reward: &Arc<Box<Reward>>,
        user_data: &mut RefMut<u64, ParticipantStats>,
    ) -> ObjectState {
        match reward.is_preorder() {
            // Any pre-order goes to activated instantly after the roll
            true => {
                user_data.add_retrieved_reward(reward.id());
                ObjectState::Activated
            }
            // All other types needs activated manually
            false => {
                user_data.add_pending_reward(reward.id());
                ObjectState::Pending
            }
        }
    }

    // Confirm that the reward was received and has been activated.
    pub fn confirm_reward(
        &self,
        user: &DiscordUser,
        index: usize,
        reward_index: usize,
    ) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_is_active(&giveaway)?;

        giveaway.update_actions_processed();

        let ref_rewards = giveaway.raw_rewards().clone();
        let guard_rewards = ref_rewards.lock().unwrap();

        match reward_index > 0 && reward_index < guard_rewards.len() + 1 {
            true => {
                let participant = Participant::from(user.clone());
                let stats = giveaway.stats();
                let user_id = participant.get_user_id();
                let selected_reward = guard_rewards[reward_index - 1].clone();

                let user_stats = stats.get_mut(&user_id);
                match user_stats {
                    Some(mut data) => self.move_reward_to_retrieved(&mut data, &selected_reward),
                    None => {
                        stats.insert(user_id, ParticipantStats::new());
                        let message = "The reward must be rolled before confirming.".to_string();
                        Err(Error::from(ErrorKind::Giveaway(message)))
                    }
                }
            }
            false => {
                let message = "The requested reward was not found.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    // Return the certain reward to the unused state and cleanup the user's stats
    pub fn deny_reward(&self, user: &DiscordUser, index: usize, reward_index: usize) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_is_active(&giveaway)?;

        giveaway.update_actions_processed();

        let ref_rewards = giveaway.raw_rewards().clone();
        let guard_rewards = ref_rewards.lock().unwrap();

        match reward_index > 0 && reward_index < guard_rewards.len() + 1 {
            true => {
                let participant = Participant::from(user.clone());
                let stats = giveaway.stats();
                let user_id = participant.get_user_id();
                let selected_reward = guard_rewards[reward_index - 1].clone();

                let user_stats = stats.get_mut(&user_id);
                match user_stats {
                    Some(mut data) => self.rollback_reward_to_unused(&mut data, &selected_reward),
                    None => {
                        stats.insert(user_id, ParticipantStats::new());
                        let message = "The reward must be rolled before return.".to_string();
                        Err(Error::from(ErrorKind::Giveaway(message)))
                    }
                }
            }
            false => {
                let message = "The requested reward was not found.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    // Checks that whether the certain giveaway needs to be printed out
    pub fn is_required_state_output(&self, index: usize) -> Result<bool> {
        let giveaway = self.get_giveaway_by_index(index)?;
        Ok(giveaway.is_required_state_output())
    }

    // Returns a pretty print of the giveaway state
    pub fn pretty_print_giveaway(&self, giveaway_index: usize) -> Result<String> {
        let giveaway = self.get_giveaway_by_index(giveaway_index)?;
        let stats = giveaway.stats();

        let pending_rewards = self.extract_pending_rewards(&stats);
        let retrieved_rewards = self.extract_retrieved_rewards(&stats);

        let reward_formatter = giveaway.reward_formatter();
        let rewards_output = giveaway
            .raw_rewards()
            .clone()
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, reward)| {
                let reward_id = reward.id();
                let is_pending = pending_rewards.contains_key(&reward_id);
                let is_retrieved = retrieved_rewards.contains_key(&reward_id);

                let reward_output = reward_formatter.pretty_print(reward);
                match (is_pending, is_retrieved) {
                    (true, false) => {
                        let user_id = pending_rewards.get(&reward_id).unwrap();
                        format!(
                            "{}. {}  [taken by <@{}>]",
                            index + 1,
                            reward_output,
                            user_id
                        )
                    }
                    (false, true) => {
                        let user_id = retrieved_rewards.get(&reward_id).unwrap();
                        format!(
                            "{}. {}  [activated by <@{}>]",
                            index + 1,
                            reward_output,
                            user_id
                        )
                    }
                    _ => format!("{}. {}", index + 1, reward_output),
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        let response = format!("Giveaway #{}:\n{}", giveaway_index, rewards_output);
        Ok(response)
    }

    // A special wrapper to help with moving the reward in the retrieved group in stats
    fn move_reward_to_retrieved(
        &self,
        data: &mut RefMut<u64, ParticipantStats>,
        reward: &Arc<Box<Reward>>,
    ) -> Result<()> {
        let pending_rewards = data.pending_rewards();

        match reward.object_state() {
            ObjectState::Activated => {
                let message = "The reward has been activated already.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
            ObjectState::Pending => match pending_rewards.contains(&reward.id()) {
                true => {
                    data.remove_pending_reward(reward.id());
                    data.add_retrieved_reward(reward.id());
                    reward.set_object_state(ObjectState::Activated);
                    Ok(())
                }
                false => {
                    let message = "This reward can't be activated by others.".to_string();
                    Err(Error::from(ErrorKind::Giveaway(message)))
                }
            },
            ObjectState::Unused => {
                let message = "The reward must be rolled before confirming.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    fn rollback_reward_to_unused(
        &self,
        data: &mut RefMut<u64, ParticipantStats>,
        reward: &Arc<Box<Reward>>,
    ) -> Result<()> {
        let pending_rewards = data.pending_rewards();

        match reward.object_state() {
            ObjectState::Activated => {
                let message = "The reward has been activated already.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
            ObjectState::Pending => match pending_rewards.contains(&reward.id()) {
                true => {
                    data.remove_pending_reward(reward.id());
                    reward.set_object_state(ObjectState::Unused);
                    Ok(())
                }
                false => {
                    let message = "This reward can't be returned by others.".to_string();
                    Err(Error::from(ErrorKind::Giveaway(message)))
                }
            },
            ObjectState::Unused => {
                let message = "The reward must be rolled before return.".to_string();
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    fn extract_pending_rewards(
        &self,
        stats: &Arc<DashMap<u64, ParticipantStats>>,
    ) -> HashMap<Uuid, u64> {
        stats
            .iter()
            .map(|pair| {
                let user_id = pair.key().clone();

                let mut vec = Vec::new();
                for reward_uuid in pair.value().pending_rewards() {
                    vec.push((reward_uuid, user_id));
                }

                vec
            })
            .flatten()
            .collect()
    }

    fn extract_retrieved_rewards(
        &self,
        stats: &Arc<DashMap<u64, ParticipantStats>>,
    ) -> HashMap<Uuid, u64> {
        stats
            .iter()
            .map(|pair| {
                let user_id = pair.key().clone();

                let mut vec = Vec::new();
                for reward_uuid in pair.value().retrieved_rewards() {
                    vec.push((reward_uuid, user_id));
                }

                vec
            })
            .flatten()
            .collect()
    }

    fn check_giveaway_owner(&self, user: &DiscordUser, giveaway: &Giveaway) -> Result<()> {
        if user.id.get() != giveaway.owner().get_user_id() {
            let message = "For interacting with this giveaway you need to be its owner.".to_string();
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }

    fn check_giveaway_is_active(&self, giveaway: &Giveaway) -> Result<()> {
        if !giveaway.is_activated() {
            let message =
                "The giveaway hasn't started yet or has been suspended by the owner.".to_string();
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serenity::model::id::UserId;
    use serenity::model::user::{CurrentUser, User as DiscordUser};

    use crate::commands::giveaway::manager::GiveawayManager;
    use crate::commands::giveaway::models::{
        Giveaway, ObjectState, Reward, OUTPUT_AFTER_GIVEAWAY_COMMANDS,
    };
    use crate::error::{Error, ErrorKind};

    fn get_user(user_id: u64, username: &str) -> DiscordUser {
        let mut current_user = CurrentUser::default();
        current_user.id = UserId::new(user_id);
        current_user.name = username.to_owned();
        DiscordUser::from(current_user)
    }

    #[test]
    fn test_read_an_new_state() {
        let manager = GiveawayManager::new();
        let giveaways = manager.get_giveaways();

        assert_eq!(giveaways.len(), 0);
    }

    #[test]
    fn test_read_after_giveaway_update() {
        let manager = GiveawayManager::new();

        let mut giveaways = manager.get_giveaways();
        assert_eq!(giveaways.len(), 0);

        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);
        giveaways = manager.get_giveaways();
        assert_eq!(giveaways.len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_read() {
        let manager = GiveawayManager::new();

        let result = manager.get_giveaway_by_index(10);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_delete_giveaway() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.delete_giveaway(&user, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_deletion() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let user = get_user(2, "Test");
        let result = manager.delete_giveaway(&user, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For deleting this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_index_on_deletion() {
        let manager = GiveawayManager::new();

        let user = get_user(1, "Test");
        let result = manager.delete_giveaway(&user, 10);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_activate_giveaway() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.activate_giveaway(&user, 1);
        assert_eq!(result.is_ok(), true);

        let giveaway_after_changes = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(giveaway_after_changes.is_activated(), true);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_activate() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.activate_giveaway(&user, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_activate() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.activate_giveaway(&user, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_deactivate_giveaway() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.deactivate_giveaway(&user, 1);
        assert_eq!(result.is_ok(), true);

        let giveaway_after_changes = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(giveaway_after_changes.is_activated(), false);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_deactivate() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.deactivate_giveaway(&user, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_deactivate() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.deactivate_giveaway(&user, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_get_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_reward(&user, 1, "test").unwrap();
        let result = manager.get_giveaway_rewards(&user, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_get_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.get_giveaway_rewards(&user, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_get_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.get_giveaway_rewards(&user, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_add_giveaway_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_reward(&owner, 1, "test");
        assert_eq!(result.is_ok(), true);

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.get_available_rewards().len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_add_new_giveaway_reward() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_reward(&user, 2, "");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_add_giveaway_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_reward(&user, 1, "test");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_add_multiple_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);
        let text = "reward #1 \n reward #2 \n reward #3";

        let result = manager.add_multiple_giveaway_rewards(&owner, 1, text);
        assert_eq!(result.is_ok(), true);

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.get_available_rewards().len(), 3);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_add_multiple_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_multiple_giveaway_rewards(&user, 2, "");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_add_multiple_giveaway_rewards() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_multiple_giveaway_rewards(&user, 1, "test");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_remove_reward() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_reward(&user, 1, "test").unwrap();
        let reward_before_deletion = manager.get_giveaway_rewards(&user, 1).unwrap();
        assert_eq!(reward_before_deletion.len(), 1);

        manager.remove_giveaway_reward(&user, 1, 1).unwrap();
        let reward_after_deletion = manager.get_giveaway_rewards(&user, 1).unwrap();
        assert_eq!(reward_after_deletion.len(), 0);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_remove_reward() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_reward(&user, 1, "test").unwrap();
        let result = manager.remove_giveaway_reward(&user, 1, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested reward was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_remove_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.remove_giveaway_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("For interacting with this giveaway you need to be its owner.".to_string()))
        );
    }

    #[test]
    fn test_roll_reward_with_manual_select_strategy_by_default() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        let reward = Reward::new("something");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.roll_reward(&owner, 1, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), None);
        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        let updated_rewards = updated_giveaway.get_available_rewards();
        assert_eq!(updated_rewards[0].object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_roll_preorder_reward_with_manual_select_strategy_by_default() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        let reward = Reward::new("AAAAA-BBBBB-CCCCC -> Pre-order something");
        assert_eq!(reward.is_preorder(), true);

        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.roll_reward(&owner, 1, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), None);
        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        let updated_rewards = updated_giveaway.get_available_rewards();
        assert_eq!(updated_rewards[0].object_state(), ObjectState::Activated);
    }

    #[test]
    fn test_get_error_for_inactive_giveaway_on_roll_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.roll_reward(&owner, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The giveaway hasn't started yet or has been suspended by the owner.".to_string()))
        );
    }

    #[test]
    fn test_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        let reward = Reward::new("something");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        let result = manager.confirm_reward(&owner, 1, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn test_get_error_for_invalid_giveaway_index_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.confirm_reward(&owner, 2, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_giveaway_in_the_inactive_state_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.confirm_reward(&owner, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The giveaway hasn't started yet or has been suspended by the owner.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_reward_index_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.confirm_reward(&owner, 1, 10);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested reward was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_already_activated_reward_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        manager.confirm_reward(&owner, 1, 1).unwrap();
        let result = manager.confirm_reward(&owner, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward has been activated already.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_user_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "SomeUser");
        let reward_1 = Reward::new("something");
        let reward_2 = Reward::new("something else");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward_1);
        giveaway.add_reward(&reward_2);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        manager.roll_reward(&user, 1, 2).unwrap();
        let result = manager.confirm_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("This reward can't be activated by others.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_unused_reward_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward_1 = Reward::new("something");
        let reward_2 = Reward::new("something else");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward_1);
        giveaway.add_reward(&reward_2);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        let result = manager.confirm_reward(&owner, 1, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward must be rolled before confirming.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_first_command_by_user_in_giveaway_on_confirm_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "SomeUser");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        let result = manager.confirm_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward must be rolled before confirming.".to_string()))
        );
    }

    #[test]
    fn test_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        let reward = Reward::new("something");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        let result = manager.deny_reward(&owner, 1, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn test_get_error_for_invalid_giveaway_index_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.deny_reward(&owner, 2, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested giveaway was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_giveaway_in_the_inactive_state_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.deny_reward(&owner, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The giveaway hasn't started yet or has been suspended by the owner.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_reward_index_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.deny_reward(&owner, 1, 10);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The requested reward was not found.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_already_activated_reward_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        manager.confirm_reward(&owner, 1, 1).unwrap();
        let result = manager.deny_reward(&owner, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward has been activated already.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_invalid_user_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "SomeUser");
        let reward_1 = Reward::new("something");
        let reward_2 = Reward::new("something else");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward_1);
        giveaway.add_reward(&reward_2);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        manager.roll_reward(&user, 1, 2).unwrap();
        let result = manager.deny_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("This reward can't be returned by others.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_unused_reward_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "SomeUser");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        let result = manager.deny_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward must be rolled before return.".to_string()))
        );
    }

    #[test]
    fn test_get_error_for_first_command_by_user_in_giveaway_on_deny_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "SomeUser");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        manager.roll_reward(&owner, 1, 1).unwrap();
        let result = manager.deny_reward(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway("The reward must be rolled before return.".to_string()))
        );
    }

    #[test]
    fn test_actions_processing_is_growing_after_roll_command() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        for _ in 0..OUTPUT_AFTER_GIVEAWAY_COMMANDS {
            manager.roll_reward(&owner, 1, 1).ok();
        }

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.is_required_state_output(), true);
    }

    #[test]
    fn test_actions_processing_is_growing_after_confirm_command() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        for _ in 0..OUTPUT_AFTER_GIVEAWAY_COMMANDS {
            manager.confirm_reward(&owner, 1, 1).ok();
        }

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.is_required_state_output(), true);
    }

    #[test]
    fn test_actions_processing_is_growing_after_deny_command() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let reward = Reward::new("something");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        giveaway.add_reward(&reward);
        giveaway.activate();
        manager.add_giveaway(giveaway);

        for _ in 0..OUTPUT_AFTER_GIVEAWAY_COMMANDS {
            manager.deny_reward(&owner, 1, 1).ok();
        }

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.is_required_state_output(), true);
    }
}
