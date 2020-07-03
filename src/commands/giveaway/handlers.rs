use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::prelude::Context;

use crate::storage::GiveawayStore;

#[group]
#[commands(list_giveaways, new_giveaway, finish_giveaway)]
struct Giveaway;

#[command("giveaways")]
#[aliases("gs")]
fn list_giveaways(ctx: &mut Context, msg: &Message) -> CommandResult {
    let giveaway_manager = ctx
        .data
        .read()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    let giveaways = giveaway_manager.get_giveaways();
    msg.reply(ctx, "list")?;

    Ok(())
}

#[command("new-giveaway")]
#[aliases("nga")]
fn new_giveaway(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "new")?;
    Ok(())
}

#[command("finish-giveaway")]
#[aliases("fga")]
fn finish_giveaway(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "finish")?;
    Ok(())
}
