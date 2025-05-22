use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::ui::screens::devconsole::cli_input::{CliInput, get_autocomplete_string, or_empty};
use crate::ui::screens::devconsole::console_parameter_type::ConsoleParameterType;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// An Exception representing a minor user error in DevConsolePopup input.
/// hint is user-readable but never translated and should help understanding how to fix the mistake.
#[derive(Debug)]
pub struct ConsoleHintException {
    pub hint: String,
}

impl fmt::Display for ConsoleHintException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hint)
    }
}

impl Error for ConsoleHintException {}

impl ConsoleHintException {
    pub fn new(hint: &str) -> Self {
        Self {
            hint: hint.to_string(),
        }
    }
}

/// An Exception representing a user error in DevConsolePopup input.
/// error is user-readable but never translated.
#[derive(Debug)]
pub struct ConsoleErrorException {
    pub error: String,
}

impl fmt::Display for ConsoleErrorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl Error for ConsoleErrorException {}

impl ConsoleErrorException {
    pub fn new(error: &str) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

/// Interface for console commands
pub trait ConsoleCommand: Send + Sync {
    /// Handle the command execution
    fn handle(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn Error>>;

    /// Returns the string to replace the last parameter of the existing command with.
    /// None means no change due to no options.
    /// The function should add a space at the end if and only if the "match" is an unambiguous choice!
    fn autocomplete(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Option<String> {
        None
    }
}

/// Base implementation of a console command
pub struct ConsoleAction {
    format: String,
    action: Box<dyn Fn(&mut DevConsolePopup, &[CliInput]) -> Result<DevConsoleResponse, Box<dyn Error>> + Send + Sync>,
}

impl ConsoleAction {
    pub fn new<F>(format: &str, action: F) -> Self
    where
        F: Fn(&mut DevConsolePopup, &[CliInput]) -> Result<DevConsoleResponse, Box<dyn Error>> + Send + Sync + 'static,
    {
        Self {
            format: format.to_string(),
            action: Box::new(action),
        }
    }
}

impl ConsoleCommand for ConsoleAction {
    fn handle(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn Error>> {
        match self.validate_format(&self.format, params) {
            Ok(_) => {
                match (self.action)(console, params) {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        if let Some(hint_ex) = e.downcast_ref::<ConsoleHintException>() {
                            Ok(DevConsoleResponse::hint(&hint_ex.hint))
                        } else if let Some(error_ex) = e.downcast_ref::<ConsoleErrorException>() {
                            Ok(DevConsoleResponse::error(&error_ex.error))
                        } else {
                            Err(e)
                        }
                    }
                }
            },
            Err(e) => {
                if let Some(hint_ex) = e.downcast_ref::<ConsoleHintException>() {
                    Ok(DevConsoleResponse::hint(&hint_ex.hint))
                } else {
                    Err(e)
                }
            }
        }
    }

    fn autocomplete(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Option<String> {
        let format_params: Vec<String> = self.format
            .split(' ')
            .skip(2)
            .map(|s| {
                let mut s = s.to_string();
                if s.starts_with('<') && s.ends_with('>') {
                    s = s[1..s.len()-1].to_string();
                }
                if s.starts_with('[') && s.ends_with(']') {
                    s = s[1..s.len()-1].to_string();
                }
                if s.starts_with('"') && s.ends_with('"') {
                    s = s[1..s.len()-1].to_string();
                }
                s
            })
            .collect();

        if format_params.is_empty() {
            return None; // nothing to autocomplete - for example "history " + tab
        }

        if format_params.len() < params.len() {
            return None; // format has no definition, so there are no options to choose from
        }

        // It is possible we're here *with* another format parameter but an *empty* params (e.g. `tile addriver` and hit tab) -> see below
        let (format_param, last_param) = if params.len() > 0 && params.len() - 1 < format_params.len() {
            (format_params[params.len() - 1].clone(), params.last())
        } else {
            (format_params[0].clone(), None)
        };

        let options = ConsoleParameterType::multi_options(&format_param, console);
        let result = get_autocomplete_string(
            last_param.map_or_else(|| CliInput::empty(), |p| p.clone()),
            &options,
            console
        );

        if last_param.is_none() && result.is_some() {
            // we got the situation described above and something to add: The caller will ultimately replace the second subcommand, so add it back
            // border case, only happens right after the second token, not after the third: Don't optimize the double split call
            let subcommand = self.format.split(' ').nth(1).unwrap_or("");
            return Some(format!("{} {}", subcommand, result.unwrap()));
        }

        result
    }

    fn validate_format(&self, format: &str, params: &[CliInput]) -> Result<(), Box<dyn Error>> {
        let all_params: Vec<&str> = format.split(' ').collect();
        let required_params_amount = all_params.iter().filter(|s| s.starts_with('<')).count();
        let optional_params_amount = if format.ends_with("]...") {
            999999
        } else {
            all_params.iter().filter(|s| s.starts_with('[')).count()
        };

        // For this check, ignore an empty token caused by a trailing blank
        let params_size = if params.is_empty() {
            0
        } else if params.last().map_or(false, |p| p.is_empty()) {
            params.len() - 1
        } else {
            params.len()
        };

        if params_size < required_params_amount || params_size > required_params_amount + optional_params_amount {
            return Err(Box::new(ConsoleHintException::new(&format!("Format: {}", format))));
        }

        Ok(())
    }
}

/// Interface for console command nodes that contain subcommands
pub trait ConsoleCommandNode: ConsoleCommand {
    /// Get the subcommands for this node
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleCommand>>;

    fn handle(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn Error>> {
        if params.is_empty() {
            let commands: Vec<&str> = self.subcommands().keys().map(|s| s.as_str()).collect();
            return Ok(DevConsoleResponse::hint(&format!("Available commands: {}", commands.join(", "))));
        }

        let handler = self.subcommands().get(params[0].to_string().as_str());
        match handler {
            Some(handler) => handler.handle(console, &params[1..]),
            None => {
                let commands: Vec<&str> = self.subcommands().keys().map(|s| s.as_str()).collect();
                let error_msg = format!(
                    "Invalid command.\nAvailable commands:{}",
                    commands.iter().map(|s| format!("\n- {}", s)).collect::<Vec<String>>().join("")
                );
                Ok(DevConsoleResponse::error(&error_msg))
            }
        }
    }

    fn autocomplete(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Option<String> {
        let first_param = params.first().map_or_else(|| CliInput::empty(), |p| p.clone());
        let handler = self.subcommands().get(first_param.to_string().as_str());

        match handler {
            Some(handler) => handler.autocomplete(console, &params[1..]),
            None => {
                let commands: Vec<&str> = self.subcommands().keys().map(|s| s.as_str()).collect();
                get_autocomplete_string(first_param, &commands, console)
            }
        }
    }
}

/// Root command node that contains all top-level commands
pub struct ConsoleCommandRoot;

impl ConsoleCommandNode for ConsoleCommandRoot {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleCommand>> {
        let mut commands = HashMap::new();

        commands.insert(
            "unit".to_string(),
            Box::new(super::console_unit_commands::ConsoleUnitCommands),
        );

        commands.insert(
            "city".to_string(),
            Box::new(super::console_city_commands::ConsoleCityCommands),
        );

        commands.insert(
            "tile".to_string(),
            Box::new(super::console_tile_commands::ConsoleTileCommands),
        );

        commands.insert(
            "civ".to_string(),
            Box::new(super::console_civ_commands::ConsoleCivCommands),
        );

        commands.insert(
            "history".to_string(),
            Box::new(ConsoleAction::new("history", |console, _| {
                console.show_history();
                Ok(DevConsoleResponse::hint("")) // Trick console into staying open
            })),
        );

        commands.insert(
            "game".to_string(),
            Box::new(super::console_game_commands::ConsoleGameCommands),
        );

        commands
    }
}