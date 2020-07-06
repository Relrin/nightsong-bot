use std::sync::Arc;

use serenity::prelude::TypeMapKey;

use crate::commands::giveaway::manager::GiveawayManager;

pub struct GiveawayStore;

impl TypeMapKey for GiveawayStore {
    type Value = Arc<GiveawayManager>;
}
