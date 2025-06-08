use std::sync::Arc;
use serenity::builder::EditMessage;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use tracing::error;

use crate::commands::giveaway::manager::GIVEAWAY_MANAGER;

pub async fn update_giveaway_message(
    ctx: crate::commands::context::Context<'_>,
    index: usize,
) {
    let giveaway = match GIVEAWAY_MANAGER.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            error!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    let update_msg = match GIVEAWAY_MANAGER.pretty_print_giveaway(index) {
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
            match ctx
                .channel_id()
                .edit_message(&ctx.http(), message_id, EditMessage::new().content(&update_msg))
                .await
            {
                Ok(_) => (),
                Err(err) => {
                    error!(
                        "Can't retrieve formatted giveaway state: {}",
                        err.to_string()
                    );
                    return;
                }
            }
        }
        // Send a new message in the chat (if it was missing by some reason)
        None => {
            match ctx.channel_id().say(&ctx.http(), &update_msg).await {
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
    ctx: crate::commands::context::Context<'_>,
    index: usize,
) {
    let giveaway = match GIVEAWAY_MANAGER.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            error!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    if giveaway.is_required_state_output() {
        giveaway.reset_actions_processed();

        match GIVEAWAY_MANAGER.pretty_print_giveaway(index) {
            Ok(response) => {
                match ctx.channel_id().say(&ctx.http(), &response).await {
                    Ok(_) => (),
                    Err(err) => {
                        error!(
                            "Can't send the message to the channel: {}",
                            err.to_string()
                        );
                    }
                }
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
