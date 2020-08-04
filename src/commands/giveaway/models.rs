use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam::atomic::AtomicCell;
use dashmap::DashMap;
use serenity::model::id::MessageId;
use serenity::model::user::User as DiscordUser;
use uuid::Uuid;

use crate::commands::giveaway::formatters::{DefaultRewardFormatter, RewardFormatter};
use crate::commands::giveaway::parser::parse_message;
use crate::commands::giveaway::strategies::{GiveawayStrategy, ManualSelectStrategy};
use crate::error::{Error, ErrorKind, Result};

pub type ConcurrencyReward = Arc<Box<Reward>>;
pub type ConcurrencyRewardsVec = Arc<Mutex<Box<Vec<ConcurrencyReward>>>>;
pub const OUTPUT_AFTER_GIVEAWAY_COMMANDS: u64 = 15;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Participant {
    user_id: u64,
    username: String,
}

impl Participant {
    // Returns a unique identifier in Discord
    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }

    // Returns a username in the Discord room
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
pub struct ParticipantStats {
    pending_rewards: HashSet<Uuid>,
    retrieved_rewards: HashSet<Uuid>,
}

impl ParticipantStats {
    pub fn new() -> Self {
        ParticipantStats {
            pending_rewards: HashSet::new(),
            retrieved_rewards: HashSet::new(),
        }
    }

    // Returns set of rewards which aren't activated but was received by the user.
    pub fn pending_rewards(&self) -> HashSet<Uuid> {
        self.pending_rewards.clone()
    }

    // Returns a set of rewards successfully retrieved by the user.
    pub fn retrieved_rewards(&self) -> HashSet<Uuid> {
        self.retrieved_rewards.clone()
    }

    // Adds id of the reward that was taken (but haven't acked yet) by the user
    pub fn add_pending_reward(&mut self, value: Uuid) {
        self.pending_rewards.insert(value);
    }

    // Deletes pending reward from the hashset
    pub fn remove_pending_reward(&mut self, value: Uuid) {
        self.pending_rewards.remove(&value);
    }

    // Adds id of the reward that was taken by the user.
    pub fn add_retrieved_reward(&mut self, value: Uuid) {
        self.retrieved_rewards.insert(value);
    }
}

#[derive(Clone)]
pub struct Giveaway {
    // A flag that determines that current phase of the giveaway.
    // true - The giveaway in active phase
    // false - The giveaway in edit / pause phase
    active: Arc<AtomicBool>,
    // A reference to the owner / create of the giveaway
    owner: Participant,
    // A giveaway description.
    description: String,
    // A list of attached rewards
    rewards: ConcurrencyRewardsVec,
    // Collected stats for each users participated in the giveaway
    stats: Arc<DashMap<u64, ParticipantStats>>,
    // Determines the algorithm for distributing rewards.
    strategy: Arc<Box<dyn GiveawayStrategy>>,
    // A reference to the message which needs to update during the
    // active giveaway phase.
    message_id: Arc<AtomicCell<Option<MessageId>>>,
    // Defines how many actions are required for printing the current
    // state of the giveaway.
    actions_required_to_output: u64,
    // An internal counter for periodic output the state of
    // the giveaway.
    actions_processed: Arc<AtomicU64>,
    // The formatter instance used for generating output for each
    // added or updated reward.
    reward_formatter: Arc<Box<dyn RewardFormatter + Send + Sync>>,
}

impl Giveaway {
    pub fn new(discord_user: &DiscordUser) -> Self {
        Giveaway {
            active: Arc::new(AtomicBool::new(false)),
            owner: Participant::from(discord_user.clone()),
            description: String::from(""),
            rewards: Arc::new(Mutex::new(Box::new(Vec::new()))),
            stats: Arc::new(DashMap::new()),
            strategy: Arc::new(Box::new(ManualSelectStrategy::new())),
            message_id: Arc::new(AtomicCell::new(None)),
            actions_required_to_output: OUTPUT_AFTER_GIVEAWAY_COMMANDS,
            actions_processed: Arc::new(AtomicU64::new(0)),
            reward_formatter: Arc::new(Box::new(DefaultRewardFormatter::new())),
        }
    }

    // Returns a text description about the giveaway.
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    // Returns information about who created the giveaway.
    pub fn owner(&self) -> &Participant {
        &self.owner
    }

    // Returns latest statistics in according with the requested giveaway.
    pub fn stats(&self) -> Arc<DashMap<u64, ParticipantStats>> {
        self.stats.clone()
    }

    // Returns a list of all rewards as concurrent structure
    pub fn raw_rewards(&self) -> ConcurrencyRewardsVec {
        self.rewards.clone()
    }

    // Returns a reference to the message that must be updated
    pub fn get_message_id(&self) -> Option<MessageId> {
        self.message_id.load()
    }

    // Overrides the message reference.
    pub fn set_message_id(&self, message_id: Option<MessageId>) {
        self.message_id.store(message_id)
    }

    // Returns a current strategy for distributing rewards.
    pub fn strategy(&self) -> Arc<Box<dyn GiveawayStrategy>> {
        self.strategy.clone()
    }

    // Checks that the giveaway has been started by the owner.
    pub fn is_activated(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    // Starts the giveaway.
    pub fn activate(&self) {
        self.active.store(true, Ordering::SeqCst)
    }

    // Disables the giveaway (which is actually means "a pause state").
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::SeqCst);
        self.reset_actions_processed();
    }

    // Increase the action processed counter by one.
    pub fn update_actions_processed(&self) {
        let current_value = self.actions_processed.load(Ordering::SeqCst);
        self.actions_processed
            .store(current_value + 1, Ordering::SeqCst);
    }

    // Resets the action processed counter to zero.
    pub fn reset_actions_processed(&self) {
        self.actions_processed.store(0, Ordering::SeqCst);
    }

    // Checks that the `action_processed` counter is equal to the
    // defined limits stored in `actions_required_to_output` field.
    pub fn is_required_state_output(&self) -> bool {
        let current_value = self.actions_processed.load(Ordering::SeqCst);
        current_value == self.actions_required_to_output
    }

    // Return a reward formatter.
    pub fn reward_formatter(&self) -> Arc<Box<dyn RewardFormatter + Send + Sync>> {
        self.reward_formatter.clone()
    }

    // Returns a list of all available rewards.
    pub fn get_available_rewards(&self) -> Vec<Arc<Box<Reward>>> {
        self.rewards
            .clone()
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    // Adds a new reward in the list of available rewards.
    pub fn add_reward(&self, obj: &Reward) {
        self.rewards
            .clone()
            .lock()
            .unwrap()
            .push(Arc::new(Box::new(obj.clone())));
    }

    // Removes the reward by index from the list of available rewards.
    pub fn remove_reward_by_index(&self, index: usize) -> Result<()> {
        let ref_giveaways = self.rewards.clone();
        let mut guard_giveaways = ref_giveaways.lock().unwrap();

        match index > 0 && index < guard_giveaways.len() + 1 {
            true => {
                guard_giveaways.remove(index - 1);
            }
            false => {
                let message = format!("The requested reward was not found.");
                return Err(Error::from(ErrorKind::Giveaway(message)));
            }
        };

        Ok(())
    }

    // Pretty-print of the giveaway in the text messages.
    pub fn pretty_print(&self) -> String {
        format!(
            "{} [owner: <@{}>]",
            self.description,
            self.owner.get_user_id(),
        )
    }
}

impl fmt::Debug for Giveaway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Giveaway")
            .field("active", &self.active.clone())
            .field("owner", &self.owner.clone())
            .field("description", &self.description.clone())
            .field("stats", &self.stats.clone())
            .finish()
    }
}

impl Eq for Giveaway {}

impl PartialEq for Giveaway {
    fn eq(&self, other: &Self) -> bool {
        let self_giveaway_rewards;
        {
            self_giveaway_rewards = self.rewards.lock().unwrap().clone();
        }

        let other_giveaway_rewards;
        {
            other_giveaway_rewards = other.rewards.lock().unwrap().clone();
        }

        self.description == other.description && self_giveaway_rewards == other_giveaway_rewards
    }
}

#[derive(Debug)]
pub struct Reward {
    // A unique identifier of the reward in the giveaway(s)
    id: Uuid,
    // The actual prize.
    value: Arc<String>,
    // Reward description
    description: Option<String>,
    // Store an additional information about the reward (e.g. the online store,
    // where the key can be activated)
    object_info: Option<String>,
    // Determines the is the type of the reward. The games / store keys requires
    // a different output rather then a plain text.
    object_type: ObjectType,
    // Current state of the rewards (was activated, unused, etc.)
    object_state: AtomicCell<ObjectState>,
}

impl Reward {
    pub fn new(value: &str) -> Self {
        let parse_result = parse_message(value);

        Reward {
            id: Uuid::new_v4(),
            value: Arc::new(parse_result.value.clone()),
            description: parse_result.description.clone(),
            object_info: parse_result.object_info.clone(),
            object_type: parse_result.object_type,
            object_state: AtomicCell::new(ObjectState::Unused),
        }
    }

    // Returns a unique identifier of the reward.
    pub fn id(&self) -> Uuid {
        self.id.clone()
    }

    // Returns the reward's store key or a plain text
    pub fn value(&self) -> Arc<String> {
        self.value.clone()
    }

    // Returns the description of the item (if has any)
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    // Returns an additional object information (e.g. for what store the key is)
    pub fn object_info(&self) -> Option<String> {
        self.object_info.clone()
    }

    // Returns the object type. It can be a game / store key or just a plain text.
    pub fn object_type(&self) -> ObjectType {
        self.object_type
    }

    // Returns the current object state.
    pub fn object_state(&self) -> ObjectState {
        self.object_state.load()
    }

    // Overrides the object state onto the new one.
    pub fn set_object_state(&self, state: ObjectState) {
        self.object_state.store(state);
    }

    // Checks that the reward has been defined as the pre-order type.
    pub fn is_preorder(&self) -> bool {
        match self.object_type {
            ObjectType::KeyPreorder => true,
            _ => false,
        }
    }
}

impl Clone for Reward {
    fn clone(&self) -> Self {
        Reward {
            id: self.id.clone(),
            value: self.value.clone(),
            description: self.description.clone(),
            object_info: self.object_info.clone(),
            object_type: self.object_type,
            object_state: AtomicCell::new(self.object_state.load()),
        }
    }
}

impl Eq for Reward {}

impl PartialEq for Reward {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectType {
    Key,
    KeyPreorder,
    Other,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectState {
    // The reward has been activated by someone an works without any issues.
    Activated,
    // The reward was taken by someone, but not verified yet.
    Pending,
    // The reward hasn't been taken by anyone.
    Unused,
}

impl ObjectState {
    // Pretty-print for the object state in text messages
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
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use serenity::model::id::UserId;
    use serenity::model::user::{CurrentUser, User as DiscordUser};

    use crate::commands::giveaway::models::{
        Giveaway, ObjectState, ObjectType, Reward, OUTPUT_AFTER_GIVEAWAY_COMMANDS,
    };

    fn get_user(user_id: u64, username: &str) -> DiscordUser {
        let mut current_user = CurrentUser::default();
        current_user.id = UserId(user_id);
        current_user.name = username.to_owned();
        DiscordUser::from(current_user)
    }

    // ---- Giveaway struct tests ----

    #[test]
    fn test_get_giveaway_rewards() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let formatter = giveaway.reward_formatter();
        let reward_1 = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        let reward_2 = Reward::new("BBBBB-CCCCC-DDDDD-FFFF [Store] -> Some game");
        let reward_3 = Reward::new("CCCCC-DDDDD-FFFFF-EEEE [Store] -> Some game");
        let concurrecy_reward_1 = Arc::new(Box::new(reward_1.clone()));
        let concurrecy_reward_2 = Arc::new(Box::new(reward_2.clone()));
        let concurrecy_reward_3 = Arc::new(Box::new(reward_3.clone()));
        let expected_item_1 = formatter.pretty_print(&concurrecy_reward_1);
        let expected_item_2 = formatter.pretty_print(&concurrecy_reward_2);
        let expected_item_3 = formatter.pretty_print(&concurrecy_reward_3);
        giveaway.add_reward(&reward_1);
        giveaway.add_reward(&reward_2);
        giveaway.add_reward(&reward_3);

        let rewards = giveaway
            .get_available_rewards()
            .iter()
            .map(|obj| formatter.pretty_print(obj))
            .collect::<Vec<String>>();
        assert_eq!(rewards.contains(&expected_item_1), true);
        assert_eq!(rewards.contains(&expected_item_2), true);
        assert_eq!(rewards.contains(&expected_item_3), true);
    }

    #[test]
    fn test_get_giveaway_rewards_for_a_new_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);

        let rewards = giveaway.get_available_rewards();
        assert_eq!(rewards.is_empty(), true);
    }

    #[test]
    fn test_add_giveaway_reward_to_the_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        let formatter = giveaway.reward_formatter();
        let concurrecy_reward = Arc::new(Box::new(reward.clone()));
        let expected_item = formatter.pretty_print(&concurrecy_reward);

        let old_giveaway_rewards = giveaway.get_available_rewards();
        assert_eq!(old_giveaway_rewards.is_empty(), true);

        giveaway.add_reward(&reward);
        let updated_giveaway_rewards = giveaway
            .get_available_rewards()
            .iter()
            .map(|obj| formatter.pretty_print(obj))
            .collect::<Vec<String>>();
        assert_eq!(updated_giveaway_rewards.contains(&expected_item), true);
    }

    #[test]
    fn test_remove_giveaway_reward_by_index_from_the_giveaway() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        let formatter = giveaway.reward_formatter();
        let concurrecy_reward = Arc::new(Box::new(reward.clone()));
        let expected_item = formatter.pretty_print(&concurrecy_reward);

        let old_giveaway_rewards = giveaway.get_available_rewards();
        assert_eq!(old_giveaway_rewards.is_empty(), true);

        giveaway.add_reward(&reward);
        let updated_giveaway_rewards = giveaway
            .get_available_rewards()
            .iter()
            .map(|obj| formatter.pretty_print(obj))
            .collect::<Vec<String>>();
        assert_eq!(updated_giveaway_rewards.contains(&expected_item), true);

        giveaway.remove_reward_by_index(1).unwrap();
        let latest_giveaway_rewards = giveaway
            .get_available_rewards()
            .iter()
            .map(|obj| formatter.pretty_print(obj))
            .collect::<Vec<String>>();
        assert_eq!(latest_giveaway_rewards.contains(&expected_item), false);
        assert_eq!(latest_giveaway_rewards.is_empty(), true);
    }

    #[test]
    fn test_update_giveaway_actions_processed_counter() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        giveaway.add_reward(&reward);
        giveaway.activate();

        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);

        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_reset_giveaway_actions_processed() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        giveaway.add_reward(&reward);
        giveaway.activate();

        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);

        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 3);

        giveaway.reset_actions_processed();
        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_reset_giveaway_actions_processed_after_deactivate() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        giveaway.add_reward(&reward);
        giveaway.activate();

        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);

        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        giveaway.update_actions_processed();
        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 3);

        giveaway.deactivate();
        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_is_required_giveaway_state_output_before_reaching_limits_is_false() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        giveaway.add_reward(&reward);
        giveaway.activate();

        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);

        let commands_count = OUTPUT_AFTER_GIVEAWAY_COMMANDS - 1;
        for _ in 0..commands_count {
            giveaway.update_actions_processed();
        }

        assert_eq!(giveaway.is_required_state_output(), false);
        assert_eq!(
            giveaway.actions_processed.load(Ordering::SeqCst),
            commands_count
        );
    }

    #[test]
    fn test_is_required_giveaway_state_output_after_reaching_limits_is_true() {
        let user = get_user(1, "Test");
        let giveaway = Giveaway::new(&user);
        let reward = Reward::new("AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game");
        giveaway.add_reward(&reward);
        giveaway.activate();

        assert_eq!(giveaway.actions_processed.load(Ordering::SeqCst), 0);

        for _ in 0..OUTPUT_AFTER_GIVEAWAY_COMMANDS {
            giveaway.update_actions_processed();
        }

        assert_eq!(giveaway.is_required_state_output(), true);
        assert_eq!(
            giveaway.actions_processed.load(Ordering::SeqCst),
            OUTPUT_AFTER_GIVEAWAY_COMMANDS
        );
    }

    // ---- GiveawayObject struct tests ----

    #[test]
    fn test_get_reward_value() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Reward::new(text);

        assert_eq!(reward.value().as_str(), "AAAAA-BBBBB-CCCCC-DDDD")
    }

    #[test]
    fn test_get_reward_value_for_other_type() {
        let text = "just a text";
        let reward = Reward::new(text);

        assert_eq!(reward.value().as_str(), text);
    }

    #[test]
    fn test_get_reward_object_type() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Reward::new(text);

        assert_eq!(reward.object_type(), ObjectType::Key)
    }

    #[test]
    fn test_get_reward_type_value_for_other_type() {
        let text = "just a text";
        let reward = Reward::new(text);

        assert_eq!(reward.object_type(), ObjectType::Other);
    }

    #[test]
    fn test_get_reward_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Reward::new(text);

        assert_eq!(reward.object_state(), ObjectState::Unused)
    }

    #[test]
    fn test_get_reward_state_for_other_type() {
        let text = "just a text";
        let reward = Reward::new(text);

        assert_eq!(reward.object_state(), ObjectState::Unused);
    }

    #[test]
    fn test_set_reward_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Reward::new(text);

        assert_eq!(reward.object_state(), ObjectState::Unused);
        reward.set_object_state(ObjectState::Pending);
        assert_eq!(reward.object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_set_reward_state_for_other_type() {
        let text = "just a text";
        let reward = Reward::new(text);

        assert_eq!(reward.object_state(), ObjectState::Unused);
        reward.set_object_state(ObjectState::Pending);
        assert_eq!(reward.object_state(), ObjectState::Pending);
    }

    #[test]
    fn test_is_pre_order_key_returns_true() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD -> Preorder game key";
        let reward = Reward::new(text);

        assert_eq!(reward.is_preorder(), true);
    }

    #[test]
    fn test_is_pre_order_key_returns_false() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD -> Just a regular game key";
        let reward = Reward::new(text);

        assert_eq!(reward.is_preorder(), false);
    }
}
