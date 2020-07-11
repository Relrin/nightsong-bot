use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use serenity::model::id::MessageId;
use serenity::model::user::User as DiscordUser;
use uuid::Uuid;

use crate::commands::giveaway::models::{
    Giveaway, ObjectState, Participant, ParticipantStats, Reward,
};
use crate::commands::giveaway::strategies::RollOptions;
use crate::error::{Error, ErrorKind, Result};

#[derive(Debug)]
#[non_exhaustive]
pub struct GiveawayManager {
    giveaways: Arc<Mutex<Vec<Arc<Box<Giveaway>>>>>,
}

impl GiveawayManager {
    pub fn new() -> Self {
        GiveawayManager {
            giveaways: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // Returns all current giveaways (started and on a pause).
    pub fn get_giveaways(&self) -> Vec<Arc<Box<Giveaway>>> {
        let ref_giveaways = self.giveaways.clone();
        let guard_giveaways = ref_giveaways.lock().unwrap();
        guard_giveaways.to_vec()
    }

    // Returns a giveaway by the given index.
    pub fn get_giveaway_by_index(&self, index: usize) -> Result<Arc<Box<Giveaway>>> {
        let ref_giveaways = self.giveaways.clone();
        let guard_giveaways = ref_giveaways.lock().unwrap();

        match index > 0 && index < guard_giveaways.len() + 1 {
            true => Ok(guard_giveaways[index - 1].clone()),
            false => {
                let message = format!("The requested giveaway was not found.");
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
        let ref_giveaways = self.giveaways.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();

        match index > 0 && index < guard_giveaways.len() + 1 {
            true => {
                if user.id.0 != guard_giveaways[index - 1].owner().get_user_id() {
                    let message = format!("For deleting this giveaway you need to be its owner.");
                    return Err(Error::from(ErrorKind::Giveaway(message)));
                }

                guard_giveaways.remove(index - 1);
                Ok(())
            }
            false => {
                let message = format!("The requested giveaway was not found.");
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    // Adds a new giveaway.
    pub fn add_giveaway(&self, giveaway: Giveaway) {
        let ref_giveaways = self.giveaways.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();
        guard_giveaways.push(Arc::new(Box::new(giveaway)));
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

    // Returns a reward from the requested giveaway in according with the set strategy.
    pub fn roll_reward(
        &self,
        user: &DiscordUser,
        index: usize,
        raw_message: &str,
    ) -> Result<Option<String>> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_is_active(&giveaway)?;

        let participant = Participant::from(user.clone());
        let stats = giveaway.stats();
        let rewards = giveaway.raw_rewards();
        let roll_options = RollOptions::new(&participant, &rewards, raw_message, &stats);
        let strategy = giveaway.strategy();
        let selected_reward = strategy.roll(&roll_options)?;

        let user_id = participant.get_user_id();
        match stats.get_mut(&user_id) {
            Some(mut data) => {
                data.add_pending_reward(selected_reward.id());
            }
            None => {
                stats.insert(user_id, ParticipantStats::new());
                let mut data = stats.get_mut(&user_id).unwrap();
                data.add_pending_reward(selected_reward.id());
            }
        };
        selected_reward.set_object_state(ObjectState::Pending);

        let response = strategy.to_message(selected_reward);
        Ok(response)
    }

    pub fn pretty_print_giveaway(
        &self,
        giveaway_index: usize,
    ) -> Result<(Option<MessageId>, String)> {
        let giveaway = self.get_giveaway_by_index(giveaway_index)?;
        let stats = giveaway.stats();

        let pending_rewards = self.extract_pending_rewards(&stats);
        let retrieved_rewards = self.extract_retrieved_rewards(&stats);

        let rewards_output = giveaway
            .raw_rewards()
            .clone()
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, obj)| {
                let reward_id = obj.id();
                let is_pending = pending_rewards.contains_key(&reward_id);
                let is_retrieved = retrieved_rewards.contains_key(&reward_id);

                match (is_pending, is_retrieved) {
                    (true, false) => {
                        let user_id = pending_rewards.get(&reward_id).unwrap();
                        format!(
                            "{}. {}  [taken by <@{}>]",
                            index + 1,
                            obj.pretty_print(),
                            user_id
                        )
                    }
                    (false, true) => {
                        let user_id = retrieved_rewards.get(&reward_id).unwrap();
                        format!(
                            "{}. {}  [activated by <@{}>]",
                            index + 1,
                            obj.pretty_print(),
                            user_id
                        )
                    }
                    _ => format!("{}. {}", index + 1, obj.pretty_print()),
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        let message_id = giveaway.get_message_id();
        let response = format!("Giveaway #{}:\n{}", giveaway_index, rewards_output);
        Ok((message_id, response))
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
        if user.id.0 != giveaway.owner().get_user_id() {
            let message = format!("For interacting with this giveaway you need to be its owner.");
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        Ok(())
    }

    fn check_giveaway_is_active(&self, giveaway: &Giveaway) -> Result<()> {
        if !giveaway.is_activated() {
            let message =
                format!("The giveaway hasn't started yet or has been suspended by the owner.");
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
    use crate::commands::giveaway::models::{Giveaway, ObjectState, Reward};
    use crate::error::{Error, ErrorKind};

    fn get_user(user_id: u64, username: &str) -> DiscordUser {
        let mut current_user = CurrentUser::default();
        current_user.id = UserId(user_id);
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For deleting this giveaway you need to be its owner."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "The requested reward was not found."
            )))
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
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
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

        let result = manager.roll_reward(&owner, 1, "1");
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), None);
        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        let updated_rewards = updated_giveaway.get_available_rewards();
        assert_eq!(updated_rewards[0].get_object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_get_error_for_inactive_giveaway_on_roll_reward() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.roll_reward(&owner, 1, "1");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The giveaway hasn't started yet or has been suspended by the owner."
            )))
        );
    }
}
