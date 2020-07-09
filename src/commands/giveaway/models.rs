use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serenity::model::user::User as DiscordUser;

use crate::commands::giveaway::utils::parse_message;
use crate::error::{Error, ErrorKind, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Participant {
    user_id: u64,
    username: String,
}

impl Participant {
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }
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
    active: Arc<AtomicBool>,
    owner: Participant,
    description: String,
    giveaway_objects: Arc<Mutex<Box<Vec<Arc<Box<GiveawayObject>>>>>>,
}

impl Giveaway {
    pub fn new(discord_user: &DiscordUser) -> Self {
        Giveaway {
            active: Arc::new(AtomicBool::new(false)),
            owner: Participant::from(discord_user.clone()),
            description: String::from(""),
            giveaway_objects: Arc::new(Mutex::new(Box::new(Vec::new()))),
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn owner(&self) -> &Participant {
        &self.owner
    }

    pub fn is_activated(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn activate(&self) {
        self.active.store(true, Ordering::SeqCst)
    }

    pub fn deactivate(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn get_giveaway_objects(&self) -> Vec<Arc<Box<GiveawayObject>>> {
        self.giveaway_objects
            .clone()
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    pub fn add_giveaway_object(&self, obj: &GiveawayObject) {
        self.giveaway_objects
            .clone()
            .lock()
            .unwrap()
            .push(Arc::new(Box::new(obj.clone())));
    }

    pub fn remove_giveaway_object_by_index(&self, index: usize) -> Result<()> {
        let ref_giveaways = self.giveaway_objects.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();

        match index > 0 && index < guard_giveaways.len() + 1 {
            true => {
                guard_giveaways.remove(index - 1);
            }
            false => {
                let message = format!("The requested prize was not found.");
                return Err(Error::from(ErrorKind::Giveaway(message)));
            }
        };

        Ok(())
    }

    pub fn pretty_print(&self) -> String {
        format!(
            "{} [owner: <@{}>]",
            self.description,
            self.owner.get_user_id(),
        )
    }
}

impl Eq for Giveaway {}

impl PartialEq for Giveaway {
    fn eq(&self, other: &Self) -> bool {
        let self_giveaway_objects;
        {
            self_giveaway_objects = self.giveaway_objects.lock().unwrap().clone();
        }

        let other_giveaway_objects;
        {
            other_giveaway_objects = other.giveaway_objects.lock().unwrap().clone();
        }

        self.description == other.description && self_giveaway_objects == other_giveaway_objects
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

    pub fn get_description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn get_object_type(&self) -> ObjectType {
        self.object_type
    }

    pub fn get_object_state(&self) -> ObjectState {
        self.object_state
    }

    pub fn set_object_state(&mut self, state: ObjectState) {
        self.object_state = state;
    }

    pub fn detailed_print(&self) -> String {
        match self.object_type {
            ObjectType::Key => {
                let key = match self.object_info.clone() {
                    Some(info) => format!("{} {}", self.value, info),
                    None => format!("{}", self.value),
                };

                format!(
                    "{} -> {}",
                    key,
                    self.description.clone().unwrap_or(String::from("")),
                )
            }
            ObjectType::Other => format!(
                "{}{}",
                self.value,
                self.description.clone().unwrap_or(String::from("")),
            ),
        }
    }

    pub fn pretty_print(&self) -> String {
        let text = match self.object_type {
            // Different output of the key, depends on the current state
            ObjectType::Key => {
                let key = match self.object_info.clone() {
                    Some(info) => format!("{} {}", self.value, info),
                    None => format!("{}", self.value),
                };

                match self.object_state {
                    // When is Activated show what was hidden behind the key
                    ObjectState::Activated => format!(
                        "{}{} -> {}",
                        self.object_state.as_str(),
                        key,
                        self.description.clone().unwrap_or(String::from("")),
                    ),
                    // For Unused/Pending states print minimal amount of info
                    _ => format!("{} {}", self.object_state.as_str(), key),
                }
            }
            // Print any non-keys as is
            ObjectType::Other => format!(
                "{} {}{}",
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

#[cfg(test)]
mod tests {
    use serenity::model::id::UserId;
    use serenity::model::user::{CurrentUser, User as DiscordUser};

    use crate::commands::giveaway::models::{Giveaway, GiveawayObject, ObjectState, ObjectType};

    fn get_user(user_id: u64, username: &str) -> DiscordUser {
        let mut current_user = CurrentUser::default();
        current_user.id = UserId(user_id);
        current_user.name = username.to_owned();
        DiscordUser::from(current_user)
    }

    // ---- Giveaway struct tests ----

    #[test]
    fn test_get_giveaway_objects() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let giveaway_object_1 = GiveawayObject::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        let giveaway_object_2 = GiveawayObject::new("BBBBB-CCCCC-DDDDD-FFFF [Store] -> Some game");
        let giveaway_object_3 = GiveawayObject::new("CCCCC-DDDDD-FFFFF-EEEE [Store] -> Some game");
        giveaway.add_giveaway_object(&giveaway_object_1);
        giveaway.add_giveaway_object(&giveaway_object_2);
        giveaway.add_giveaway_object(&giveaway_object_3);

        let giveaway_objects = giveaway
            .get_giveaway_objects()
            .iter()
            .map(|obj| obj.pretty_print())
            .collect::<Vec<String>>();
        assert_eq!(
            giveaway_objects.contains(&giveaway_object_1.pretty_print()),
            true
        );
        assert_eq!(
            giveaway_objects.contains(&giveaway_object_2.pretty_print()),
            true
        );
        assert_eq!(
            giveaway_objects.contains(&giveaway_object_3.pretty_print()),
            true
        );
    }

    #[test]
    fn test_get_giveaway_objects_for_a_new_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);

        let giveaway_objects = giveaway.get_giveaway_objects();
        assert_eq!(giveaway_objects.is_empty(), true);
    }

    #[test]
    fn test_add_giveaway_object_to_the_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let giveaway_object = GiveawayObject::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");

        let old_giveaway_objects = giveaway.get_giveaway_objects();
        assert_eq!(old_giveaway_objects.is_empty(), true);

        giveaway.add_giveaway_object(&giveaway_object);
        let updated_giveaway_objects = giveaway
            .get_giveaway_objects()
            .iter()
            .map(|obj| obj.pretty_print())
            .collect::<Vec<String>>();
        assert_eq!(
            updated_giveaway_objects.contains(&giveaway_object.pretty_print()),
            true
        );
    }

    #[test]
    fn test_remove_giveaway_object_by_index_from_the_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let giveaway_object = GiveawayObject::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");

        let old_giveaway_objects = giveaway.get_giveaway_objects();
        assert_eq!(old_giveaway_objects.is_empty(), true);

        giveaway.add_giveaway_object(&giveaway_object);
        let updated_giveaway_objects = giveaway
            .get_giveaway_objects()
            .iter()
            .map(|obj| obj.pretty_print())
            .collect::<Vec<String>>();
        assert_eq!(
            updated_giveaway_objects.contains(&giveaway_object.pretty_print()),
            true
        );

        giveaway.remove_giveaway_object_by_index(1).unwrap();
        let latest_giveaway_objects = giveaway
            .get_giveaway_objects()
            .iter()
            .map(|obj| obj.pretty_print())
            .collect::<Vec<String>>();
        assert_eq!(
            latest_giveaway_objects.contains(&giveaway_object.pretty_print()),
            false
        );
        assert_eq!(latest_giveaway_objects.is_empty(), true);
    }

    // ---- GiveawayObject struct tests ----

    #[test]
    fn test_get_giveaway_object_value() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(
            giveaway_object.get_value().as_str(),
            "AAAAA-BBBBB-CCCCC-DDDD"
        )
    }

    #[test]
    fn test_get_giveaway_object_value_for_other_type() {
        let text = "just a text";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_value().as_str(), text);
    }

    #[test]
    fn test_get_giveaway_object_type() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_type(), ObjectType::Key)
    }

    #[test]
    fn test_get_giveaway_object_type_value_for_other_type() {
        let text = "just a text";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_type(), ObjectType::Other);
    }

    #[test]
    fn test_get_giveaway_object_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_state(), ObjectState::Unused)
    }

    #[test]
    fn test_get_giveaway_object_state_for_other_type() {
        let text = "just a text";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_state(), ObjectState::Unused);
    }

    #[test]
    fn test_set_giveaway_object_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let mut giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_state(), ObjectState::Unused);
        giveaway_object.set_object_state(ObjectState::Pending);
        assert_eq!(giveaway_object.get_object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_set_giveaway_object_state_for_other_type() {
        let text = "just a text";
        let mut giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.get_object_state(), ObjectState::Unused);
        giveaway_object.set_object_state(ObjectState::Pending);
        assert_eq!(giveaway_object.get_object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_pretty_print_for_the_giveaway_object_in_the_unused_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(
            giveaway_object.pretty_print(),
            "[ ] AAAAA-BBBBB-CCCCC-DDDD [Store]"
        );
    }

    #[test]
    fn test_pretty_print_for_the_giveaway_object_in_the_pending_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let mut giveaway_object = GiveawayObject::new(text);

        giveaway_object.set_object_state(ObjectState::Pending);
        assert_eq!(
            giveaway_object.pretty_print(),
            "[?] AAAAA-BBBBB-CCCCC-DDDD [Store]"
        );
    }

    #[test]
    fn test_pretty_print_for_the_giveaway_object_in_the_activated_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let mut giveaway_object = GiveawayObject::new(text);

        giveaway_object.set_object_state(ObjectState::Activated);
        assert_eq!(
            giveaway_object.pretty_print(),
            "~~[+]AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game~~"
        );
    }

    #[test]
    fn test_pretty_print_for_an_unknown_object_in_the_unused_state() {
        let text = "just a text";
        let giveaway_object = GiveawayObject::new(text);

        assert_eq!(giveaway_object.pretty_print(), "[ ] just a text");
    }

    #[test]
    fn test_pretty_print_for_an_unknown_object_in_the_activated_state() {
        let text = "just a text";
        let mut giveaway_object = GiveawayObject::new(text);

        giveaway_object.set_object_state(ObjectState::Activated);
        assert_eq!(giveaway_object.pretty_print(), "~~[+] just a text~~");
    }
}
