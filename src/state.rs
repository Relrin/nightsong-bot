use std::sync::{Arc, RwLock};

use crate::db::models::Giveaway;
use crate::error::{Error, ErrorKind, Result};

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
        let field_ref = self.giveaway.clone();
        let value = field_ref.try_read()?;
        Ok(value.clone())
    }

    pub fn set_giveaway(&self, giveaway: Option<Giveaway>) -> Result<&Self> {
        let old_giveaway = self.get_giveaway()?;

        if let Some(current_giveaway) = old_giveaway {
            if !current_giveaway.finished {
                let message = format!(
                    "Can't create a new giveaway: the previous \
                     giveaway must be marked as finished before starting another one."
                );
                return Err(Error::from(ErrorKind::Giveaway(message)));
            }
        }

        let field_ref = self.giveaway.clone();
        let mut write_lock = field_ref.try_write()?;
        *write_lock = giveaway;
        Ok(&self)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::db::models::Giveaway;
    use crate::error::{Error, ErrorKind};
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
        let giveaway = Giveaway {
            id: 1,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: false,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        state.set_giveaway(Some(giveaway.clone())).unwrap();

        let result = state.get_giveaway();
        assert_eq!(result.is_ok(), true);
        let read_giveaway = result.unwrap();
        assert_eq!(read_giveaway.is_some(), true);
        assert_eq!(giveaway, read_giveaway.unwrap());
    }

    #[test]
    fn test_set_giveaway_for_a_new_state() {
        let state = BotState::new();
        let giveaway = Giveaway {
            id: 1,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: false,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        let result = state.set_giveaway(Some(giveaway));

        assert_eq!(result.is_ok(), true);
        let value = result.unwrap();
        assert_eq!(value, &state);
    }

    #[test]
    fn test_set_giveaway_after_the_previous_finished_giveaway() {
        let state = BotState::new();
        let giveaway_old = Giveaway {
            id: 1,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: true,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        state.set_giveaway(Some(giveaway_old.clone())).unwrap();

        let result_old = state.get_giveaway();
        assert_eq!(result_old.is_ok(), true);
        let read_giveaway_old = result_old.unwrap();
        assert_eq!(read_giveaway_old.is_some(), true);
        assert_eq!(giveaway_old, read_giveaway_old.unwrap());

        let giveaway_new = Giveaway {
            id: 2,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: false,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        state.set_giveaway(Some(giveaway_new.clone())).unwrap();

        let result_new = state.get_giveaway();
        assert_eq!(result_new.is_ok(), true);
        let read_giveaway_new = result_new.unwrap();
        assert_eq!(read_giveaway_new.is_some(), true);
        assert_eq!(giveaway_new, read_giveaway_new.unwrap());
    }

    #[test]
    fn test_set_giveaway_returns_error_for_unfinished_old_giveaway() {
        let state = BotState::new();
        let giveaway_old = Giveaway {
            id: 1,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: false,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        state.set_giveaway(Some(giveaway_old.clone())).unwrap();

        let result_old = state.get_giveaway();
        assert_eq!(result_old.is_ok(), true);
        let read_giveaway_old = result_old.unwrap();
        assert_eq!(read_giveaway_old.is_some(), true);
        assert_eq!(giveaway_old, read_giveaway_old.unwrap());

        let giveaway_new = Giveaway {
            id: 2,
            description: String::from("test giveaway"),
            participants: Vec::new(),
            finished: false,
            created_at: NaiveDateTime::new(
                NaiveDate::from_ymd(2020, 1, 1),
                NaiveTime::from_hms_milli(12, 00, 00, 000),
            ),
        };
        let result_new = state.set_giveaway(Some(giveaway_new.clone()));

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
