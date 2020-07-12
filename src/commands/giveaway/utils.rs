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
    match giveaway_manager.pretty_print_giveaway(index) {
        Ok((option_message_id, update_msg)) => {
            match option_message_id {
                // Try to edit the existing message instead of printing a new one
                Some(message_id) => {
                    match msg
                        .channel_id
                        .edit_message(&ctx.http, message_id, |m| m.content(&update_msg))
                    {
                        Ok(_msg) => (),
                        Err(err) => {
                            msg.channel_id.say(&ctx.http, &update_msg).unwap();
                        }
                    }
                }
                // Send a new message in the chat (if it was missing by some reason)
                None => match msg.channel_id.say(&ctx.http, &update_msg) {
                    Ok(_msg) => (),
                    Err(err) => {
                        println!(
                            "Impossible to output the giveaway message in the channel. Reason: {}",
                            err.to_string()
                        );
                        ()
                    }
                },
            };
        }
        Err(err) => println!("Cant't output the giveaway update: {}", err.to_string()),
    }
}
