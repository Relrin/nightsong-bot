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
    create_giveaway,
    start_giveaway,
    deactivate_giveaway,
    finish_giveaway
)]
#[description = "Commands for managing giveaways"]
struct Giveaway;

#[command("glist")]
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

#[command("gcreate")]
#[min_args(1)]
#[help_available]
#[example("!gcreate <description>")]
#[description = "Create a new giveaway"]
fn create_giveaway(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let description = args.message();
    let giveaway = GiveawayInstance::new(&msg.author).with_description(description);

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    giveaway_manager.add_giveaway(giveaway);
    msg.channel_id
        .say(&ctx.http, "The giveaway has been created!")?;

    Ok(())
}

#[command("gstart")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!gstart <number>")]
#[description = "Start the certain giveaway"]
fn start_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `gstart` command must be a positive integer.",
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

#[command("gdeactivate")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!gdeactivate <number>")]
#[description = "Deactivates the giveaway by the given number"]
fn deactivate_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `gdeactivate` command must be a positive integer.",
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

#[command("gfinish")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[example("!gfinish <number>")]
#[description = "Finishes and deletes the giveaway by the given number"]
fn finish_giveaway(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "An argument for the `gfinish` command must be a positive integer.",
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
