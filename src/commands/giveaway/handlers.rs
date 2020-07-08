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
    // Giveaway management
    list_giveaways,
    create_giveaway,
    start_giveaway,
    deactivate_giveaway,
    finish_giveaway,

    // Giveaway objects management
    list_giveaway_objects,
    add_giveaway_object,
    remove_giveaway_object,
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
                "The `number` argument for the `gstart` command must be a positive integer.",
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
                "The `number` argument for the `gdeactivate` command must be a positive integer.",
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
                "The `number` argument for the `gfinish` command must be a positive integer.",
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

#[command("gitems")]
#[min_args(1)]
#[min_args(1)]
#[help_available]
#[example("!gitems <number>")]
#[description = "Display detailed info about the prizes in the giveaway for the owner."]
fn list_giveaway_objects(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `number` argument for the `gitems` command must be a positive integer.",
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

    match giveaway_manager.get_giveaway_objects(&msg.author, index) {
        Ok(items) => {
            let content = match items.len() {
                0 => "There are no added prizes.".to_string(),
                _ => format!(
                    "Prizes:\n{}",
                    items
                        .iter()
                        .enumerate()
                        .map(|(index, obj)| format!("{}. {}", index + 1, obj.detailed_print()))
                        .collect::<Vec<String>>()
                        .join("\n")
                ),
            };

            let message = MessageBuilder::new().push(content).build();
            msg.channel_id.say(&ctx.http, message)?
        }
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("gadd")]
#[min_args(2)]
#[help_available]
#[example("!gadd <number> <description>")]
#[description = "Adds a new prize to the certain giveaway"]
fn add_giveaway_object(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `number` argument for the `gadd` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let data = args.rest();

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStore>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.add_giveaway_object(&msg.author, index, data) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The prize has been added to the giveaway.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("gremove")]
#[min_args(2)]
#[min_args(2)]
#[help_available]
#[example("!gremove <number> <prize-to-remove>")]
#[description = "Removes the prize from the certain giveaway"]
fn remove_giveaway_object(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `number` argument for the `gremove` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let prize_index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `prize-to-remove` argument for the `gremove` command must be a positive integer.",
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

    match giveaway_manager.remove_giveaway_object(&msg.author, index, prize_index) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The prize has been removed from the giveaway.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}
