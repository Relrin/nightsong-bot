#[macro_use]
mod jsonb;
mod models;
mod schema;
mod util;

pub use crate::db::util::establish_connection;
