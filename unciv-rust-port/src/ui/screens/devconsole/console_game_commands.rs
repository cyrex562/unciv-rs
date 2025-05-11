use std::collections::HashMap;

use crate::ui::screens::devconsole::cli_input::DevConsolePopupExt;
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_command_node::ConsoleCommandNode;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Commands for managing game settings in the dev console
pub struct ConsoleGameCommands;

impl ConsoleCommandNode for ConsoleGameCommands {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleAction>> {
        let mut commands = HashMap::new();

        // setdifficulty command
        commands.insert(
            "setdifficulty".to_string(),
            Box::new(ConsoleAction::new("game setdifficulty <difficulty>", |console, params| {
                let difficulty = console.game_info.ruleset.difficulties.values()
                    .find(|&d| d.name == params[0].original_unquoted())
                    .ok_or_else(|| ConsoleErrorException::new("Unrecognized difficulty"))?;

                console.game_info.difficulty = difficulty.name.clone();
                console.game_info.set_transients();

                Ok(DevConsoleResponse::OK)
            })),
        );

        commands
    }
}