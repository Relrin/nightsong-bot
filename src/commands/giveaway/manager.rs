use std::sync::{Arc, Mutex};

use serenity::model::user::User as DiscordUser;

use crate::commands::giveaway::models::{Giveaway, GiveawayObject};
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

    pub fn get_giveaways(&self) -> Vec<Arc<Box<Giveaway>>> {
        let ref_giveaways = self.giveaways.clone();
        let guard_giveaways = ref_giveaways.lock().unwrap();
        guard_giveaways.to_vec()
    }

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

    pub fn activate_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        giveaway.activate();
        Ok(())
    }

    pub fn deactivate_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?.clone();
        self.check_giveaway_owner(user, &giveaway)?;

        giveaway.deactivate();
        Ok(())
    }

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

    pub fn add_giveaway(&self, giveaway: Giveaway) {
        let ref_giveaways = self.giveaways.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();
        guard_giveaways.push(Arc::new(Box::new(giveaway)));
    }

    pub fn get_giveaway_objects(
        &self,
        user: &DiscordUser,
        index: usize,
    ) -> Result<Vec<Arc<Box<GiveawayObject>>>> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        let giveaway_objects = giveaway
            .get_giveaway_objects()
            .iter()
            .cloned()
            .collect::<Vec<Arc<Box<GiveawayObject>>>>();
        Ok(giveaway_objects)
    }

    pub fn add_giveaway_object(&self, user: &DiscordUser, index: usize, data: &str) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;

        let giveaway_object = GiveawayObject::new(data);
        giveaway.add_giveaway_object(&giveaway_object);

        Ok(())
    }

    pub fn remove_giveaway_object(
        &self,
        user: &DiscordUser,
        index: usize,
        prize_index: usize,
    ) -> Result<()> {
        let giveaway = self.get_giveaway_by_index(index)?;
        self.check_giveaway_owner(user, &giveaway)?;
        giveaway.remove_giveaway_object_by_index(prize_index)?;
        Ok(())
    }

    fn check_giveaway_owner(&self, user: &DiscordUser, giveaway: &Giveaway) -> Result<()> {
        if user.id.0 != giveaway.owner().get_user_id() {
            let message = format!("For interacting with this giveaway you need to be its owner.");
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
    use crate::commands::giveaway::models::Giveaway;
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
    fn test_get_giveaway_objects() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_object(&user, 1, "test").unwrap();
        let result = manager.get_giveaway_objects(&user, 1);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_get_giveaway_objects() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.get_giveaway_objects(&user, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_get_giveaway_objects() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.get_giveaway_objects(&user, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
        );
    }

    #[test]
    fn test_add_giveaway_object() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_object(&owner, 1, "test");
        assert_eq!(result.is_ok(), true);

        let updated_giveaway = manager.get_giveaway_by_index(1).unwrap();
        assert_eq!(updated_giveaway.get_giveaway_objects().len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_add_new_giveaway_object() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_object(&user, 2, "");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found."
            )))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_add_giveaway_object() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.add_giveaway_object(&user, 1, "test");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
        );
    }

    #[test]
    fn test_remove_giveaway_object() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_object(&user, 1, "test").unwrap();
        let prizes_before_deletion = manager.get_giveaway_objects(&user, 1).unwrap();
        assert_eq!(prizes_before_deletion.len(), 1);

        manager.remove_giveaway_object(&user, 1, 1).unwrap();
        let prizes_after_deletion = manager.get_giveaway_objects(&user, 1).unwrap();
        assert_eq!(prizes_after_deletion.len(), 0);
    }

    #[test]
    fn test_get_error_for_invalid_index_on_remove_giveaway_object() {
        let manager = GiveawayManager::new();
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        manager.add_giveaway_object(&user, 1, "test").unwrap();
        let result = manager.remove_giveaway_object(&user, 1, 2);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The requested prize was not found."
            )))
        );
    }

    #[test]
    fn test_get_error_for_invalid_owner_on_remove_giveaway_object() {
        let manager = GiveawayManager::new();
        let owner = get_user(1, "Owner");
        let user = get_user(2, "Test");
        let giveaway = Giveaway::new(&owner).with_description("test giveaway");
        manager.add_giveaway(giveaway);

        let result = manager.remove_giveaway_object(&user, 1, 1);
        assert_eq!(result.is_err(), true);
        assert_eq!(
            result.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "For interacting with this giveaway you need to be its owner."
            )))
        );
    }
}
