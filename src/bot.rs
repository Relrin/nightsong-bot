use std::env;

use serenity::framework::standard::StandardFramework;
use serenity::model::gateway::Ready;
use serenity::prelude::{Client, Context, EventHandler};

use crate::commands::GET_COMMANDS_LIST;

pub struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub fn run_discord_bot() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let mut client = Client::new(&token, Handler).expect("Cannot create a Discord client");

    let bot_id = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => info.id,
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.with_whitespace(false)
                    .on_mention(Some(bot_id))
                    .prefix("!")
            })
            .help(&GET_COMMANDS_LIST),
    );

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
