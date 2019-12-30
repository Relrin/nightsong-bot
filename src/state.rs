use std::sync::{Arc, RwLock};

use crate::error::{Error, ErrorKind, Result};
use crate::models::Giveaway;

#[derive(Debug)]
#[non_exhaustive]
pub struct BotState {
    giveaway: Arc<RwLock<Option<Giveaway>>>,
}

impl Eq for BotState {}

impl PartialEq for BotState {
    fn eq(&self, other: &Self) -> bool {
        let self_giveaway_ref = self.giveaway.clone();
        let self_giveaway = self_giveaway_ref.try_read().unwrap();
        let other_giveaway_ref = other.giveaway.clone();
        let other_giveaway = other_giveaway_ref.try_read().unwrap();
        *self_giveaway == *other_giveaway
    }
}

impl BotState {
    pub fn new() -> Self {
        BotState {
            giveaway: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_giveaway(&self) -> Result<Option<Giveaway>> {
        let current_giveaway = self.giveaway.try_read()?;
        Ok(current_giveaway.clone())
    }

    pub fn set_giveaway(&self, giveaway: &Giveaway) -> Result<&Self> {
        if self.get_giveaway()?.is_some() {
            let message = format!(
                "Can't create a new giveaway: the previous \
                 giveaway must be marked as finished before starting another one."
            );
            return Err(Error::from(ErrorKind::Giveaway(message)));
        }

        let mut current_giveaway = self.giveaway.try_write()?;
        *current_giveaway = Some(giveaway.to_owned());
        Ok(&self)
    }

    pub fn finish_giveaway(&self) -> Result<bool> {
        let mut current_giveaway = self.giveaway.try_write()?;
        *current_giveaway = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::{Error, ErrorKind};
    use crate::models::Giveaway;
    use crate::state::BotState;

    #[test]
    fn test_read_an_empty_state() {
        let state = BotState::new();
        let giveaway = state.get_giveaway();

        assert_eq!(giveaway.is_ok(), true);
        let value = giveaway.unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_read_after_giveaway_update() {
        let state = BotState::new();
        let giveaway = Giveaway::default().with_description("test giveaway");
        state.set_giveaway(&giveaway).unwrap();

        let result = state.get_giveaway();
        assert_eq!(result.is_ok(), true);
        let read_giveaway = result.unwrap();
        assert_eq!(read_giveaway.is_some(), true);
        assert_eq!(giveaway, read_giveaway.unwrap());
    }

    #[test]
    fn test_set_giveaway_for_a_new_state() {
        let state = BotState::new();
        let giveaway = Giveaway::default().with_description("test giveaway");
        let result = state.set_giveaway(&giveaway);

        assert_eq!(result.is_ok(), true);
        let value = result.unwrap();
        assert_eq!(value, &state);
    }

    #[test]
    fn test_set_giveaway_after_the_previous_finished_giveaway() {
        let state = BotState::new();
        let giveaway_old = Giveaway::default().with_description("test giveaway");
        state.set_giveaway(&giveaway_old).unwrap();

        let result_old = state.get_giveaway();
        assert_eq!(result_old.is_ok(), true);
        let read_giveaway_old = result_old.unwrap();
        assert_eq!(read_giveaway_old.is_some(), true);
        assert_eq!(giveaway_old, read_giveaway_old.unwrap());

        state.finish_giveaway().unwrap();
        let giveaway_new = Giveaway::default().with_description("test giveaway #2");
        state.set_giveaway(&giveaway_new).unwrap();

        let result_new = state.get_giveaway();
        assert_eq!(result_new.is_ok(), true);
        let read_giveaway_new = result_new.unwrap();
        assert_eq!(read_giveaway_new.is_some(), true);
        assert_eq!(giveaway_new, read_giveaway_new.unwrap());
    }

    #[test]
    fn test_set_giveaway_returns_error_for_unfinished_old_giveaway() {
        let state = BotState::new();
        let giveaway_old = Giveaway::default().with_description("test giveaway");
        state.set_giveaway(&giveaway_old).unwrap();

        let result_old = state.get_giveaway();
        assert_eq!(result_old.is_ok(), true);
        let read_giveaway_old = result_old.unwrap();
        assert_eq!(read_giveaway_old.is_some(), true);
        assert_eq!(giveaway_old, read_giveaway_old.unwrap());

        let giveaway_new = Giveaway::default().with_description("test giveaway #2");
        let result_new = state.set_giveaway(&giveaway_new);

        assert_eq!(result_new.is_err(), true);
        assert_eq!(
            result_new.unwrap_err(),
            Error::from(ErrorKind::Giveaway(format!(
                "Can't create a new giveaway: the previous \
                 giveaway must be marked as finished before starting another one."
            )))
        );
    }
}
