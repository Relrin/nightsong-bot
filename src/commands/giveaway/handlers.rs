use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::Args;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use serenity::utils::MessageBuilder;

use crate::commands::giveaway::models::Giveaway as GiveawayInstance;
use crate::storage::GiveawayStore;

#[group]
#[commands(
    list_giveaways,
    new_giveaway,
    start_giveaway,
    deactivate_giveaway,
    finish_giveaway
)]
#[description = "Commands for managing giveaways"]
struct Giveaway;

#[command("giveaways")]
#[aliases("gs")]
#[description = "Get a list of available giveaways"]
fn list_giveaways(ctx: &mut Context, msg: &Message) -> CommandResult {
    let giveaway_manager = ctx
        .data
        .read()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    let giveaways = giveaway_manager
        .get_giveaways()
        .iter()
        .enumerate()
        .map(|(index, giveaway)| format!("{}. {}", index + 1, giveaway.pretty_print()))
        .collect::<Vec<String>>();

    let content = match giveaways.len() {
        0 => "There are no active giveaways.".to_string(),
        _ => format!("Giveaways:\n{}", giveaways.join("\n")),
    };

    let message = MessageBuilder::new().push(content).build();
    msg.channel_id.say(&ctx.http, message)?;

    Ok(())
}

#[command("new-giveaway")]
#[aliases("nga")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!new-giveaway <\"description\">")]
#[description = "Create a new giveaway"]
fn new_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let description = args
        .single::<String>()
        .unwrap_or("".to_string())
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();

    let giveaway = GiveawayInstance::new(&msg.author).with_description(&description);

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    giveaway_manager.add_giveaway(giveaway);
    msg.channel_id
        .say(&ctx.http, "The giveaway has been added!")?;

    Ok(())
}

#[command("start-giveaway")]
#[aliases("sga")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!start-giveaway <number>")]
#[description = "Start a giveaway"]
fn start_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `start-giveaway` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.activate_giveaway(&msg.author, index) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The giveaway has been started.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("deactivate-giveaway")]
#[aliases("dga")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!deactivate-giveaway <number>")]
#[description = "Start a giveaway by the given number"]
fn deactivate_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `deactivate-giveaway` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.deactivate_giveaway(&msg.author, index) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The giveaway has been deactivated.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("finish-giveaway")]
#[aliases("fga")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!finish-giveaway <number>")]
#[description = "Finish and delete a giveaway by the given number"]
fn finish_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `finish-giveaway` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.delete_giveaway(&msg.author, index) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The giveaway has been finished.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}
