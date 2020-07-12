# nightsong-bot
The Discord bot for managing server rooms

## Features
- Giveaways management:
    - Ability to manage multiple giveaways simultaneously
    - Rewards management (adding, deletion, etc.) for each giveaway
    - Pretty-print of the giveaways

### List of available commands
Each command must be called with the `!` prefix in the beginning of each command (e.g. `!glist`) 

- Giveaway management
    - `glist` - Get a list of available giveaways
    - `gcreate` - Create a new giveaway
    - `gstart` - Start the certain giveaway
    - `gdeactivate` - Deactivate (or suspend) the giveaway
    - `gfinish` - Finish and delete the giveaway
    - `gitems` - Display detailed info about the rewards in the giveaway
    - `gadd` - Add a new reward to the certain giveaway
    - `gremove` - Remove the reward from the certain giveaway
    - `groll` - Roll the reward from the certain giveaway
    - `gconfirm` - Confirm that the reward was activated from the certain giveaway
    - `gdeny` - Return the reward back that can't be activated

For more information call the help command via `!help <command-name>` in a discord channel.

## License
The nightsong-bot project is published under the BSD license. For more details read the [LICENSE](https://github.com/Relrin/nightsong-bot/blob/master/LICENSE) file.
