use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::prelude::Context;

use crate::commands::giveaway::manager::GiveawayManager;

pub fn update_giveaway_message(
    ctx: &mut Context,
    msg: &Message,
    giveaway_manager: &Arc<GiveawayManager>,
    index: usize,
) {
    let giveaway = match giveaway_manager.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            println!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    let update_msg = match giveaway_manager.pretty_print_giveaway(index) {
        Ok(output) => output,
        Err(err) => {
            println!(
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
                .edit_message(&ctx.http, message_id, |m| m.content(&update_msg))
            {
                Ok(_) => (),
                Err(_) => {
                    msg.channel_id.say(&ctx.http, &update_msg).unwrap();
                }
            }
        }
        // Send a new message in the chat (if it was missing by some reason)
        None => match msg.channel_id.say(&ctx.http, &update_msg) {
            Ok(_) => (),
            Err(err) => {
                println!(
                    "Impossible to output the giveaway message in the channel. Reason: {}",
                    err.to_string()
                );
            }
        },
    }
}

pub fn periodic_giveaway_state_output(
    ctx: &mut Context,
    msg: &Message,
    giveaway_manager: &Arc<GiveawayManager>,
    index: usize,
) {
    let giveaway = match giveaway_manager.get_giveaway_by_index(index) {
        Ok(giveaway) => giveaway,
        Err(err) => {
            println!("Can't get giveaway by index: {}", err.to_string());
            return;
        }
    };

    if giveaway.is_required_state_output() {
        giveaway.reset_actions_processed();

        match giveaway_manager.pretty_print_giveaway(index) {
            Ok(response) => {
                msg.channel_id.say(&ctx.http, &response).unwrap();
            }
            Err(err) => {
                println!(
                    "Can't retrieve formatted giveaway state: {}",
                    err.to_string()
                );
            }
        }
    };
}
