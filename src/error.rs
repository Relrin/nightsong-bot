use std::result;
use std::sync::TryLockError;

use thiserror::Error as ThisError;
use serenity::prelude::SerenityError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone, Eq, PartialEq, ThisError)]
pub enum Error {
    #[error("{0}")]
    SerenityError(String),
    #[error("{0}")]
    RwLock(String),
    #[error("{0}")]
    Giveaway(String),
}

impl From<SerenityError> for Error {
    fn from(err: SerenityError) -> Error {
        let description = err.to_string();
        Error::SerenityError(description)
    }
}

impl<T> From<TryLockError<T>> for Error {
    fn from(err: TryLockError<T>) -> Error {
        let description = match err {
            TryLockError::Poisoned(e) => format!("The RwLock poisoned for {:?}.", e),
            TryLockError::WouldBlock => "Can't acquire RwLock for read/write.".to_string(),
        };
        Error::RwLock(description)
    }
}

