use std::sync::Arc;

use serenity::model::id::ApplicationId;
use serenity::prelude::TypeMapKey;

pub struct BotIdStorage;

impl TypeMapKey for BotIdStorage {
    type Value = Arc<ApplicationId>;
}
