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
use crate::error::ErrorKind::Giveaway;
use crate::storage::GiveawayStorage;

// Giveaway management
// - [x] list_giveaways,
// - [x] create_giveaway,
// - [x] start_giveaway,
// - [x] deactivate_giveaway,
// - [x] finish_giveaway,
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

#[poise::command(prefix_command, rename="gcreate")]
/// Create a new giveaway
pub async fn create_giveaway(
    ctx: Context<'_>,
    #[min_length = 1]
    #[description = "Shown message about the giveaway"]
    #[rest]
    description: String
) -> Result<(), Error> {
    let author = ctx.author();
    let giveaway = GiveawayInstance::new(&author).with_description(&description);
    GIVEAWAY_MANAGER.add_giveaway(giveaway);

    let message = CreateMessage::new().content("The giveaway has been created!");
    ctx.channel_id().send_message(&ctx.http(), message).await?;
    Ok(())
}

#[poise::command(prefix_command, rename="gstart")]
/// Start the certain giveaway
pub async fn start_giveaway(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to activate"]
    giveaway_number: usize
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.activate_giveaway(ctx.author(), giveaway_number) {
        Ok(_) => GIVEAWAY_MANAGER.pretty_print_giveaway(giveaway_number)?,
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gdeactivate")]
/// Deactivates the giveaway by the given number
pub async fn deactivate_giveaway(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to deactivate"]
    giveaway_number: usize
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.deactivate_giveaway(ctx.author(), giveaway_number) {
        Ok(_) => String::from("The giveaway has been deactivated."),
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gfinish")]
/// Finishes and deletes the giveaway by the given number
pub async fn finish_giveaway(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to finish and delete"]
    giveaway_number: usize
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.delete_giveaway(ctx.author(), giveaway_number) {
        Ok(_) => String::from("The giveaway has been finished."),
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gitems")]
/// Display detailed info about the rewards in the giveaway for the owner.
pub async fn list_rewards(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to finish and delete"]
    giveaway_number: usize
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.get_giveaway_rewards(ctx.author(), giveaway_number) {
        Ok(items) => {
            let giveaway = GIVEAWAY_MANAGER.get_giveaway_by_index(giveaway_number)?;
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

           MessageBuilder::new().push(content).build()
        }
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gadd")]
/// Adds a new reward to the giveaway
pub async fn add_reward(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to add a reward"]
    giveaway_number: usize,
    #[min_length = 1]
    #[description = "An item to be added to the giveaway. Can be a plain text or platform key in the `AAAAA-BBBBB-CCCCC-DDDD [Store name] -> Game name` format"]
    #[rest]
    reward: String
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.add_giveaway_reward(ctx.author(), giveaway_number, &reward) {
        Ok(_) => String::from("The reward has been added to the giveaway."),
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gaddm")]
/// Adds a new reward to the giveaway, parsed from the single message. The separator for rewards is the new line
pub async fn add_multiple_rewards(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to add multiple rewards"]
    giveaway_number: usize,
    #[min_length = 1]
    #[description = "List of rewards as the single message. The separator for rewards is the new line"]
    #[rest]
    rewards: String
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.add_multiple_giveaway_rewards(ctx.author(), giveaway_number, &rewards) {
        Ok(_) => String::from("The reward has been added to the giveaway."),
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

#[poise::command(prefix_command, rename="gremove")]
/// Removes the reward from the giveaway
pub async fn remove_reward(
    ctx: Context<'_>,
    #[min = 1]
    #[max = 255]
    #[description = "Number of the giveaway to interact with the reward"]
    giveaway_number: usize,
    #[min_length = 1]
    #[description = "Number of the reward within the list"]
    #[min = 1]
    #[max = 255]
    reward_index: usize
) -> Result<(), Error> {
    let message = match GIVEAWAY_MANAGER.remove_giveaway_reward(ctx.author(), giveaway_number, reward_index) {
        Ok(_) => String::from("The reward has been removed from the giveaway."),
        Err(err) => format!("{}", err),
    };
    ctx.channel_id().say(&ctx.http(), message).await?;

    Ok(())
}

// #[command("groll")]
// #[min_args(1)]
// #[help_available]
// #[usage("<giveaway-number> <reward-number>")]
// #[example("1 1")]
// #[description = "Roll the reward from the certain giveaway"]
// async fn roll_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     let index = match args.single::<usize>() {
//         Ok(value) => value,
//         Err(_) => {
//             msg.channel_id.say(
//                 &ctx.http,
//                 "The `giveaway-number` argument for the `groll` command must be a positive integer.",
//             )?;
//             return Ok(());
//         }
//     };
//
//     let giveaway_manager = ctx
//         .data
//         .write()
//         .await
//         .get::<GiveawayStorage>()
//         .cloned()
//         .expect("Expected GiveawayManager in ShareMap.");
//
//     match giveaway_manager.roll_reward(&msg.author, index, args.rest()) {
//         Ok(response) => match response {
//             Some(reward) => {
//                 msg.channel_id.say(&ctx.http, &reward)?;
//             }
//             None => (),
//         },
//         Err(err) => {
//             msg.channel_id.say(&ctx.http, format!("{}", err))?;
//         }
//     };
//
//     update_giveaway_message(ctx, msg, &giveaway_manager, index);
//     periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
//     Ok(())
// }
//
// #[command("gconfirm")]
// #[min_args(2)]
// #[max_args(2)]
// #[help_available]
// #[usage("<giveaway-number> <reward-number>")]
// #[example("1 1")]
// #[description = "Confirm that the reward was activated from the certain giveaway"]
// async fn confirm_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     let index = match args.single::<usize>() {
//         Ok(value) => value,
//         Err(_) => {
//             msg.channel_id.say(
//                 &ctx.http,
//                 "The `giveaway-number` argument for the `gconfirm` command must be a positive integer.",
//             )?;
//             return Ok(());
//         }
//     };
//     let reward_index = match args.single::<usize>() {
//         Ok(value) => value,
//         Err(_) => {
//             msg.channel_id.say(
//                 &ctx.http,
//                 "The `reward-number` argument for the `gconfirm` command must be a positive integer.",
//             )?;
//             return Ok(());
//         }
//     };
//
//     let giveaway_manager = ctx
//         .data
//         .write()
//         .await
//         .get::<GiveawayStorage>()
//         .cloned()
//         .expect("Expected GiveawayManager in ShareMap.");
//
//     match giveaway_manager.confirm_reward(&msg.author, index, reward_index) {
//         Ok(_) => (),
//         Err(err) => {
//             msg.reply(&ctx.http, format!("{}", err))?;
//         }
//     };
//
//     update_giveaway_message(ctx, msg, &giveaway_manager, index);
//     periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
//     Ok(())
// }
//
// #[command("gdeny")]
// #[min_args(2)]
// #[max_args(2)]
// #[help_available]
// #[usage("<giveaway-number> <reward-number>")]
// #[example("1 1")]
// #[description = "Return the reward back that can't be activated"]
// async fn deny_reward(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     let index = match args.single::<usize>() {
//         Ok(value) => value,
//         Err(_) => {
//             msg.channel_id.say(
//                 &ctx.http,
//                 "The `giveaway-number` argument for the `gdeny` command must be a positive integer.",
//             )?;
//             return Ok(());
//         }
//     };
//     let reward_index = match args.single::<usize>() {
//         Ok(value) => value,
//         Err(_) => {
//             msg.channel_id.say(
//                 &ctx.http,
//                 "The `reward-number` argument for the `gdeny` command must be a positive integer.",
//             )?;
//             return Ok(());
//         }
//     };
//
//     let giveaway_manager = ctx
//         .data
//         .write()
//         .await
//         .get::<GiveawayStorage>()
//         .cloned()
//         .expect("Expected GiveawayManager in ShareMap.");
//
//     match giveaway_manager.deny_reward(&msg.author, index, reward_index) {
//         Ok(_) => (),
//         Err(err) => {
//             msg.reply(&ctx.http, format!("{}", err))?;
//         }
//     };
//
//     update_giveaway_message(ctx, msg, &giveaway_manager, index);
//     periodic_giveaway_state_output(ctx, msg, &giveaway_manager, index);
//     Ok(())
// }
