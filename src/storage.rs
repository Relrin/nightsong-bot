use std::sync::Arc;

use serenity::model::id::ApplicationId;
use serenity::prelude::TypeMapKey;

use crate::commands::giveaway::manager::GiveawayManager;

// TODO: Remove this???
pub struct GiveawayStorage;

impl TypeMapKey for GiveawayStorage {
    type Value = Arc<GiveawayManager>;
}

pub struct BotIdStorage;

impl TypeMapKey for BotIdStorage {
    type Value = Arc<ApplicationId>;
}
