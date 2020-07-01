use std::sync::{Arc, Mutex};

use crate::commands::giveaway::models::Giveaway;
use crate::error::{Error, ErrorKind, Result};

#[derive(Debug)]
#[non_exhaustive]
pub struct BotState {
    giveaways: Arc<Mutex<Vec<Arc<Box<Giveaway>>>>>,
}

impl BotState {
    pub fn new() -> Self {
        BotState {
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
                let message = format!("The requested giveaway was not found or doesn't exist.");
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
    use crate::commands::giveaway::models::Giveaway;
    use crate::error::{Error, ErrorKind};
    use crate::state::BotState;

    #[test]
    fn test_read_an_new_state() {
        let state = BotState::new();
        let giveaways = state.get_giveaways();

        assert_eq!(giveaways.len(), 0);
    }

    #[test]
    fn test_read_after_giveaway_update() {
        let state = BotState::new();

        let mut giveaways = state.get_giveaways();
        assert_eq!(giveaways.len(), 0);

        let giveaway = Giveaway::default().with_description("test giveaway");
        state.add_giveaway(giveaway);
        giveaways = state.get_giveaways();
        assert_eq!(giveaways.len(), 1);
    }

    #[test]
    fn test_get_error_for_invalid_index() {
        let state = BotState::new();

        let result_new = state.get_giveaway_by_index(10);
        assert_eq!(result_new.is_err(), true);
        assert_eq!(
            result_new.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "The requested giveaway was not found or doesn't exist."
            )))
        );
    }
}
