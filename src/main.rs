pub mod commands;
pub mod error;
pub mod storage;

use std::env;
use std::sync::Arc;
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions};
use poise::builtins::register_globally;
use poise::serenity_prelude::GatewayIntents;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use tracing::{error, info};

use crate::commands::{help, list_giveaways};
use crate::commands::context::UserData;
use crate::commands::giveaway::{create_giveaway, start_giveaway};
use crate::commands::giveaway::manager::GIVEAWAY_MANAGER;
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

            match GIVEAWAY_MANAGER.get_giveaway_by_index(index) {
                Ok(giveaway) => { giveaway.set_message_id(Some(msg.id)); }
                Err(err) => error!("Can't get the giveaway by index: {}", err.to_string()),
            };
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let framework = Framework::<UserData, Error>::builder()
        .options(FrameworkOptions {
            commands: vec![
                help(),
                list_giveaways(),
                create_giveaway(),
                start_giveaway(),
            ],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                Ok(UserData {})
            })
        })
        .build();

    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Cannot create a Discord client");

    let bot_id = match client.http.get_current_application_info().await {
        Ok(info) => info.id,
        Err(err) => panic!("Could not access application info: {:?}", err),
    };
    {
        let mut data = client.data.write().await;
        data.insert::<BotIdStorage>(Arc::new(bot_id));
    }

    if let Err(err) = client.start().await {
        error!("Client error: {:?}", err);
    }
}
