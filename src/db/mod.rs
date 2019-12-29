#[macro_use]
pub mod jsonb;
pub mod models;
pub mod schema;
pub mod util;

pub use crate::db::util::establish_connection;
