use std::env;
use std::sync::Arc;

use serenity::framework::standard::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::{Client, Context, EventHandler};

use crate::commands::giveaway::manager::GiveawayManager;
use crate::commands::{GET_COMMANDS_LIST, GIVEAWAY_GROUP};
use crate::storage::{BotIdStorage, GiveawayStorage};

pub struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        let bot_id = ctx
            .data
            .read()
            .get::<BotIdStorage>()
            .cloned()
            .expect("Expected BotId in ShareMap.");

        if msg.author.id.0 == bot_id.0 && msg.content.starts_with("Giveaway #") {
            let substrings: Vec<&str> = msg.content.split_terminator("\n").collect();
            if substrings.len() < 1 {
                return;
            }

            let index = substrings[0]
                .trim_start_matches("Giveaway #")
                .trim_end_matches(":")
                .parse::<usize>()
                .unwrap();

            let giveaway_manager = ctx
                .data
                .write()
                .get::<GiveawayStorage>()
                .cloned()
                .expect("Expected GiveawayManager in ShareMap.");

            match giveaway_manager.get_giveaway_by_index(index) {
                Ok(giveaway) => {
                    giveaway.set_message_id(Some(msg.id));
                }
                Err(err) => println!("Cant't get the giveaway by index: {}", err.to_string()),
            };
        }
    }

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

    {
        let mut data = client.data.write();
        data.insert::<GiveawayStorage>(Arc::new(GiveawayManager::new()));
        data.insert::<BotIdStorage>(Arc::new(bot_id));
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.with_whitespace(false)
                    .on_mention(Some(bot_id))
                    .prefix("!")
            })
            .help(&GET_COMMANDS_LIST)
            .group(&GIVEAWAY_GROUP),
    );

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
