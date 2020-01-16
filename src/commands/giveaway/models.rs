use std::collections::HashMap;

use serenity::model::user::User as DiscordUser;

use crate::commands::giveaway::util::parse_message;

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
    description: String,
    participants: HashMap<u64, Box<Participant>>,
    giveaway_objects: Box<Vec<Giveaway>>,
}

impl Giveaway {
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }
}

impl Default for Giveaway {
    fn default() -> Self {
        Giveaway {
            description: String::from(""),
            participants: HashMap::new(),
            giveaway_objects: Box::new(Vec::new()),
        }
    }
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
    value: String,
    description: Option<String>,
    object_info: Option<String>,
    object_type: ObjectType,
    object_state: ObjectState,
}

impl GiveawayObject {
    pub fn new(value: &str) -> Self {
        let parse_result = parse_message(value);

        GiveawayObject {
            value: parse_result.value.clone(),
            description: parse_result.description.clone(),
            object_info: parse_result.object_info.clone(),
            object_type: parse_result.object_type,
            object_state: ObjectState::Unused,
        }
    }

    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    pub fn get_object_type(&self) -> ObjectType {
        self.object_type
    }

    pub fn get_object_state(&self) -> ObjectState {
        self.object_state
    }

    pub fn set_object_state(mut self, state: ObjectState) {
        self.object_state = state;
    }

    pub fn pretty_print(&self) -> String {
        let text = match self.object_type {
            // Different output of the key, depends on the current state
            ObjectType::Key => {
                match self.object_state {
                    // When is Activated show what was hidden behind the key
                    ObjectState::Activated => format!(
                        "{}{}{} -> {}",
                        self.object_state.as_str(),
                        self.value,
                        self.object_info.clone().unwrap_or(String::from("")),
                        self.description.clone().unwrap_or(String::from("")),
                    ),
                    // For Unused/Pending states print minimal amount of info
                    _ => format!(
                        "{}{}{}",
                        self.object_state.as_str(),
                        self.value,
                        self.object_info.clone().unwrap_or(String::from("")),
                    ),
                }
            }
            // Print any non-keys as is
            ObjectType::Other => format!(
                "{}{}{}",
                self.object_state.as_str(),
                self.value,
                self.description.clone().unwrap_or(String::from("")),
            ),
        };

        // If the object was taken by someone, then cross out the text
        match self.object_state == ObjectState::Activated {
            true => format!("~~{}~~", text),
            false => text,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectType {
    Key,
    Other,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectState {
    Activated,
    Pending,
    Unused,
}

impl ObjectState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectState::Activated => "[+]",
            ObjectState::Pending => "[?]",
            ObjectState::Unused => "[ ]",
        }
    }
}
