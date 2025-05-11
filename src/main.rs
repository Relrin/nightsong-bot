pub mod commands;
pub mod error;
pub mod storage;

use std::env;
use std::sync::Arc;
use poise::serenity_prelude::GatewayIntents;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
//use serenity::framework::standard::{StandardFramework, Configuration};
use serenity::framework::standard::macros::{hook};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use tracing::{error, info, instrument};

use crate::commands::UserData;
use crate::commands::giveaway::manager::GiveawayManager;
use crate::commands::{GET_COMMANDS_LIST, GIVEAWAY_GROUP};
use crate::error::Error;
use crate::storage::{BotIdStorage, GiveawayStorage};


pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let bot_id = ctx
            .data
            .read()
            .await
            .get::<BotIdStorage>()
            .cloned()
            .expect("Expected BotId in ShareMap.");

        if msg.author.id.get() == bot_id.get() && msg.content.starts_with("Giveaway #") {
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
                .await
                .get::<GiveawayStorage>()
                .cloned()
                .expect("Expected GiveawayManager in ShareMap.");

            match giveaway_manager.get_giveaway_by_index(index) {
                Ok(giveaway) => {
                    giveaway.set_message_id(Some(msg.id));
                }
                Err(err) => error!("Can't get the giveaway by index: {}", err.to_string()),
            };
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[hook]
#[instrument]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!("Got command '{}' by user '{}'", command_name, msg.author.name);
    true
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let framework = poise::Framework::<UserData, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(UserData {})
            })
        })
        .build();

    //let framework = StandardFramework::new()
    //   .before(before)
    //   .help(&GET_COMMANDS_LIST)
    //   .group(&GIVEAWAY_GROUP);
    //framework.configure(Configuration::new().prefix("!"));

    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let intents = GatewayIntents::non_privileged();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Cannot create a Discord client");

    let bot_id = match client.http.get_current_application_info().await {
        Ok(info) => info.id,
        Err(why) => panic!("Could not access application info: {:?}", why),
    };
    {
        let mut data = client.data.write().await;
        data.insert::<GiveawayStorage>(Arc::new(GiveawayManager::new()));
        data.insert::<BotIdStorage>(Arc::new(bot_id));
    }

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
