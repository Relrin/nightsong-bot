use serenity::all::CreateMessage;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::Args;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::utils::MessageBuilder;

use crate::error::Error;
use crate::commands::context::Context;
use crate::commands::giveaway::models::Giveaway as GiveawayInstance;
use crate::commands::giveaway::utils::{periodic_giveaway_state_output, update_giveaway_message};
use crate::commands::giveaway::manager::GIVEAWAY_MANAGER;
use crate::storage::GiveawayStorage;

// Giveaway management
// - [x] list_giveaways,
// - [ ] create_giveaway,
// - [ ] start_giveaway,
// - [ ] deactivate_giveaway,
// - [ ] finish_giveaway,
//
// Giveaway rewards management
// - [ ] list_rewards,
// - [ ] add_reward,
// - [ ] add_multiple_rewards,
// - [ ] remove_reward,
//
// Interaction with the giveaway
// - [ ] roll_reward,
// - [ ] confirm_reward,
// - [ ] deny_reward,

#[poise::command(prefix_command, rename="glist")]
/// Get a list of available giveaways
pub async fn list_giveaways(ctx: Context<'_>) -> Result<(), Error> {
    let giveaways = GIVEAWAY_MANAGER
        .get_giveaways()
        .iter()
        .enumerate()
        .map(|(index, giveaway)| format!("{}. {}", index + 1, giveaway.pretty_print()))
        .collect::<Vec<String>>();

    let content = match giveaways.len() {
        0 => "There are no active giveaways.".to_string(),
        _ => format!("Giveaways:\n{}", giveaways.join("\n")),
    };

    let message = CreateMessage::new().content(content);
    ctx.channel_id().send_message(&ctx.http(), message).await?;

    Ok(())
}

#[command("gcreate")]
#[min_args(1)]
#[help_available]
#[usage("<description>")]
#[example("My new Steam / EGS games giveaway.")]
#[description = "Create a new giveaway"]
async fn create_giveaway(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let description = args.message();
    let giveaway = GiveawayInstance::new(&msg.author).with_description(description);

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStorage>()
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
#[usage("<giveaway-number>")]
#[example("1")]
#[description = "Start the certain giveaway"]
async fn start_giveaway(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gstart` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.activate_giveaway(&msg.author, index) {
        Ok(_) => {
            let response = giveaway_manager.pretty_print_giveaway(index)?;
            msg.channel_id.say(&ctx.http, &response)?;
        }
        Err(err) => {
            msg.channel_id.say(&ctx.http, format!("{}", err))?;
        }
    };

    Ok(())
}

#[command("gdeactivate")]
#[min_args(1)]
#[max_args(1)]
#[help_available]
#[usage("<giveaway-number>")]
#[example("1")]
#[description = "Deactivates the giveaway by the given number"]
async fn deactivate_giveaway(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gdeactivate` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStorage>()
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
#[usage("<giveaway-number>")]
#[example("1")]
#[description = "Finishes and deletes the giveaway by the given number"]
async fn finish_giveaway(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gfinish` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .get::<GiveawayStorage>()
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
#[max_args(1)]
#[help_available]
#[usage("<giveaway-number>")]
#[example("1")]
#[description = "Display detailed info about the rewards in the giveaway for the owner."]
async fn list_rewards(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gitems` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.get_giveaway_rewards(&msg.author, index) {
        Ok(items) => {
            let giveaway = giveaway_manager.get_giveaway_by_index(index).unwrap();
            let reward_formatter = giveaway.reward_formatter();
            let content = match items.len() {
                0 => "There are no added rewards.".to_string(),
                _ => format!(
                    "Rewards:\n{}",
                    items
                        .iter()
                        .enumerate()
                        .map(|(index, obj)| format!(
                            "{}. {}",
                            index + 1,
                            reward_formatter.debug_print(obj)
                        ))
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
#[usage("<giveaway-number> <description>")]
#[example("1 Just a plain text with some description about the reward")]
#[example("1 AAAAA-BBBBB-CCCCC-DDDD [Store name] -> Game name")]
#[description = "Adds a new reward to the certain giveaway"]
async fn add_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gadd` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let data = args.rest();

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.add_giveaway_reward(&msg.author, index, data) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The reward has been added to the giveaway.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("gaddm")]
#[min_args(2)]
#[help_available]
#[usage("<giveaway-number> <description>")]
#[description = "Adds a new reward to the certain giveaway, parsed from the single message. The separator for rewards is the new line"]
async fn add_multiple_rewards(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gaddm` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let data = args.rest();

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.add_multiple_giveaway_rewards(&msg.author, index, data) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The reward has been added to the giveaway.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("gremove")]
#[min_args(2)]
#[max_args(2)]
#[help_available]
#[usage("<giveaway-number> <reward-to-remove>")]
#[example("1 1")]
#[description = "Removes the reward from the certain giveaway"]
async fn remove_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gremove` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let reward_index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `reward-to-remove` argument for the `gremove` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.remove_giveaway_reward(&msg.author, index, reward_index) {
        Ok(_) => msg
            .channel_id
            .say(&ctx.http, "The reward has been removed from the giveaway.")?,
        Err(err) => msg.channel_id.say(&ctx.http, format!("{}", err))?,
    };

    Ok(())
}

#[command("groll")]
#[min_args(1)]
#[help_available]
#[usage("<giveaway-number> <reward-number>")]
#[example("1 1")]
#[description = "Roll the reward from the certain giveaway"]
async fn roll_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `groll` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.roll_reward(&msg.author, index, args.rest()) {
        Ok(response) => match response {
            Some(reward) => {
                msg.channel_id.say(&ctx.http, &reward)?;
            }
            None => (),
        },
        Err(err) => {
            msg.channel_id.say(&ctx.http, format!("{}", err))?;
        }
    };

    update_giveaway_message(ctx, msg, &giveaway_manager, index);
    periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
    Ok(())
}

#[command("gconfirm")]
#[min_args(2)]
#[max_args(2)]
#[help_available]
#[usage("<giveaway-number> <reward-number>")]
#[example("1 1")]
#[description = "Confirm that the reward was activated from the certain giveaway"]
async fn confirm_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gconfirm` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let reward_index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `reward-number` argument for the `gconfirm` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.confirm_reward(&msg.author, index, reward_index) {
        Ok(_) => (),
        Err(err) => {
            msg.reply(&ctx.http, format!("{}", err))?;
        }
    };

    update_giveaway_message(ctx, msg, &giveaway_manager, index);
    periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
    Ok(())
}

#[command("gdeny")]
#[min_args(2)]
#[max_args(2)]
#[help_available]
#[usage("<giveaway-number> <reward-number>")]
#[example("1 1")]
#[description = "Return the reward back that can't be activated"]
async fn deny_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `giveaway-number` argument for the `gdeny` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };
    let reward_index = match args.single::<usize>() {
        Ok(value) => value,
        Err(_) => {
            msg.channel_id.say(
                &ctx.http,
                "The `reward-number` argument for the `gdeny` command must be a positive integer.",
            )?;
            return Ok(());
        }
    };

    let giveaway_manager = ctx
        .data
        .write()
        .await
        .get::<GiveawayStorage>()
        .cloned()
        .expect("Expected GiveawayManager in ShareMap.");

    match giveaway_manager.deny_reward(&msg.author, index, reward_index) {
        Ok(_) => (),
        Err(err) => {
            msg.reply(&ctx.http, format!("{}", err))?;
        }
    };

    update_giveaway_message(ctx, msg, &giveaway_manager, index);
    periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
    Ok(())
}
