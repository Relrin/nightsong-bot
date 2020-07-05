use std::sync::{Arc, Mutex};

use serenity::model::user::User as DiscordUser;

use crate::commands::giveaway::models::Giveaway;
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
            true => Ok(guard_giveaways[index].clone()),
            false => {
                let message = format!("The requested giveaway was not found.");
                Err(Error::from(ErrorKind::Giveaway(message)))
            }
        }
    }

    pub fn delete_giveaway(&self, user: &DiscordUser, index: usize) -> Result<()> {
        let ref_giveaways = self.giveaways.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();

        match index > 0 && index < guard_giveaways.len() + 1 {
            true => {
                if user.id.0 != guard_giveaways[index - 1].owner().get_user_id() {
                    let message =
                        format!("For deleting this giveaway you need to be an its owner.");
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
}

#[cfg(test)]
mod tests {
    use serenity::model::id::UserId;
    use serenity::model::user::{CurrentUser, User as DiscordUser};

    use crate::commands::giveaway::models::Giveaway;
    use crate::commands::giveaway::storage::GiveawayManager;
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
                "For deleting this giveaway you need to be an its owner."
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
}