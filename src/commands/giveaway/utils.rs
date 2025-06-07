use std::sync::Arc;
use serenity::builder::EditMessage;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use tracing::error;

use crate::commands::giveaway::manager::GiveawayManager;

pub async fn update_giveaway_message(
    ctx: &mut Context,
    msg: &Message,
    giveaway_manager: &Arc<GiveawayManager>,
    index: usize,
) {
    let giveaway = match giveaway_manager.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            error!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    let update_msg = match giveaway_manager.pretty_print_giveaway(index) {
        Ok(output) => output,
        Err(err) => {
            error!(
                "Can't retrieve formatted giveaway state: {}",
                err.to_string()
            );
            return;
        }
    };

    match giveaway.get_message_id() {
        // Try to edit the existing message instead of printing a new one
        Some(message_id) => {
            match msg
                .channel_id
                .edit_message(&ctx.http, message_id, EditMessage::new().content(&update_msg))
                .await
            {
                Ok(_) => (),
                Err(_) => {
                    msg.channel_id.say(&ctx.http, &update_msg).await.unwrap();
                }
            }
        }
        // Send a new message in the chat (if it was missing by some reason)
        None => {
            match msg.channel_id.say(&ctx.http, &update_msg).await {
                Ok(_) => (),
                Err(err) => {
                    error!(
                    "Impossible to output the giveaway message in the channel. Reason: {}",
                    err.to_string()
                );
                }
            }
        },
    }
}

pub async fn periodic_giveaway_state_output(
    ctx: &mut Context,
    msg: &Message,
    giveaway_manager: &Arc<GiveawayManager>,
    index: usize,
) {
    let giveaway = match giveaway_manager.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            error!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    if giveaway.is_required_state_output() {
        giveaway.reset_actions_processed();

        match giveaway_manager.pretty_print_giveaway(index) {
            Ok(response) => {
                msg.channel_id.say(&ctx.http, &response).await.unwrap();
            }
            Err(err) => {
                error!(
                    "Can't retrieve formatted giveaway state: {}",
                    err.to_string()
                );
            }
        }
    };
}
