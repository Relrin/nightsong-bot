use chrono::NaiveDateTime;
use diesel::sql_types::Jsonb;
use serde::{Deserialize, Serialize};
use serenity::model::user::User as DiscordUser;

use crate::db::schema::{giveaway, giveaway_object};

#[derive(Clone, FromSqlRow, AsExpression, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[sql_type = "Jsonb"]
pub struct Participant {
    pub user_id: u64,
    pub username: String,
}

impl_jsonb_boilerplate!(Participant);

impl From<DiscordUser> for Participant {
    fn from(discord_user: DiscordUser) -> Self {
        Participant {
            user_id: discord_user.id.0,
            username: discord_user.name,
        }
    }
}

#[derive(Clone, Identifiable, Queryable, Debug)]
#[table_name = "giveaway"]
pub struct Giveaway {
    pub id: i32,
    pub description: String,
    pub participants: Vec<Participant>,
    pub finished: bool,
    pub created_at: NaiveDateTime,
}

impl Eq for Giveaway {}

impl PartialEq for Giveaway {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.description == other.description
            && self.participants == other.participants
            && self.finished == other.finished
            && self.created_at == self.created_at
    }
}

#[derive(Clone, Queryable, Associations, Debug, Eq, PartialEq)]
#[belongs_to(Giveaway)]
#[table_name = "giveaway_object"]
pub struct GiveawayObject {
    pub id: i32,
    pub giveaway_id: i32,
    pub value: String,
    pub object_type: String,
    pub object_state: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ObjectType {
    Key,
    Other,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Key => "Key",
            ObjectType::Other => "Other",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ObjectState {
    Activated,
    Pending,
    Unused,
}

impl ObjectState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectState::Activated => "Activated",
            ObjectState::Pending => "Pending",
            ObjectState::Unused => "Unused",
        }
    }
}
