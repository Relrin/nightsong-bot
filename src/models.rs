use std::collections::HashMap;

use serenity::model::user::User as DiscordUser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Participant {
    pub user_id: u64,
    pub username: String,
}

impl From<DiscordUser> for Participant {
    fn from(discord_user: DiscordUser) -> Self {
        Participant {
            user_id: discord_user.id.0,
            username: discord_user.name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Giveaway {
    pub description: String,
    pub participants: HashMap<u64, Box<Participant>>,
    pub giveaway_objects: Box<Vec<Giveaway>>,
}

impl Eq for Giveaway {}

impl PartialEq for Giveaway {
    fn eq(&self, other: &Self) -> bool {
        self.description == other.description
            && self.participants == other.participants
            && self.giveaway_objects == other.giveaway_objects
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GiveawayObject {
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
