use std::sync::Arc;

use serenity::prelude::TypeMapKey;

use crate::commands::giveaway::storage::GiveawayManager;

pub struct GiveawayStore;

impl TypeMapKey for GiveawayStore {
    type Value = Arc<GiveawayManager>;
}
