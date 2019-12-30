use std::fmt::{self, Display};
use std::result;

use failure::{Backtrace, Context, Fail};
use std::sync::TryLockError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        self.inner.get_context().clone()
    }
}

impl Eq for Error {}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        let self_error_kind = self.kind();
        let other_error_kind = other.kind();
        self_error_kind == other_error_kind
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "{}", _0)]
    RwLock(String),
    #[fail(display = "{}", _0)]
    Giveaway(String),
    #[fail(display = "{}", description)]
    Other { description: String },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl<T> From<TryLockError<T>> for Error {
    fn from(err: TryLockError<T>) -> Error {
        let description = match err {
            TryLockError::Poisoned(e) => format!("The RwLock poisoned for {:?}.", e),
            TryLockError::WouldBlock => format!("Can't acquire RwLock for read/write."),
        };
        let kind = ErrorKind::RwLock(description);
        Error::from(Context::new(kind))
    }
}
