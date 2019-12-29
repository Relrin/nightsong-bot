use chrono::NaiveDateTime;
use diesel::sql_types::Jsonb;
use serde::{Deserialize, Serialize};
use serenity::model::user::User as DiscordUser;

use crate::db::schema::{giveaway, giveaway_object};

#[derive(FromSqlRow, AsExpression, Serialize, Deserialize, Debug)]
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

#[derive(Identifiable, Queryable, Debug)]
#[table_name = "giveaway"]
pub struct Giveaway {
    id: i32,
    description: String,
    participants: Vec<Participant>,
    finished: bool,
    created_at: NaiveDateTime,
}

#[derive(Queryable, Associations, Debug)]
#[belongs_to(Giveaway)]
#[table_name = "giveaway_object"]
pub struct GiveawayObject {
    id: i32,
    giveaway_id: i32,
    value: String,
    object_type: String,
    object_state: String,
}
