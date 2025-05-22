use std::collections::VecDeque;

use crate::logic::civilization::Civilization;
use crate::models::ruleset::RulesetObject;
use crate::models::ruleset::unique::{Unique, UniqueTarget, UniqueTriggerActivation, UniqueType};
use crate::models::ruleset::validation::UniqueValidator;
use crate::models::translations::{fill_placeholders, get_placeholder_text};
use crate::ui::screens::devconsole::cli_input::{CliInput, Method};
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Container for console access to UniqueTriggerActivation.triggerUnique.
///
/// # Arguments
///
/// * `top_level_command` - For the beginning of the format string. Also used to control syntax checks.
pub struct ConsoleTriggerAction {
    top_level_command: String,
}

impl ConsoleTriggerAction {
    /// Create a new ConsoleTriggerAction
    pub fn new(top_level_command: &str) -> Self {
        Self {
            top_level_command: top_level_command.to_string(),
        }
    }

    /// Get the action function for the console command
    fn get_action(top_level_command: &str) -> impl Fn(&mut DevConsolePopup, &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
        let top_level_command = top_level_command.to_string();

        move |console: &mut DevConsolePopup, params: &[CliInput]| {
            let mut param_stack = VecDeque::from(params.to_vec());

            // The city and tile blocks could be written shorter without try-catch, but this way the error message is easily kept in one place
            let city = match console.get_selected_city() {
                Ok(city) => Some(city),
                Err(ex) => {
                    if top_level_command == "city" {
                        return Err(ex);
                    }
                    None
                }
            };

            let unit = match console.get_selected_unit() {
                Ok(unit) => Some(unit),
                Err(ex) => {
                    if top_level_command == "unit" {
                        return Err(ex);
                    }
                    None
                }
            };

            let tile = match console.get_selected_tile() {
                Ok(tile) => Some(tile),
                Err(ex) => {
                    if top_level_command == "tile" {
                        return Err(ex);
                    }
                    None
                }
            };

            let civ = get_civ(console, &top_level_command, &mut param_stack)
                .or_else(|| city.as_ref().map(|c| c.civ.clone()))
                .or_else(|| unit.as_ref().map(|u| u.civ.clone()))
                .or_else(|| tile.as_ref().and_then(|t| t.get_owner()))
                .ok_or_else(|| ConsoleErrorException::new("A trigger command needs a Civilization from some source"))?;

            let unique = get_unique(console, &mut param_stack)?;

            if UniqueTriggerActivation::trigger_unique(&unique, &civ, city.as_ref(), unit.as_ref(), tile.as_ref(), None, "due to cheating") {
                Ok(DevConsoleResponse::OK)
            } else {
                Ok(DevConsoleResponse::error("The `triggerUnique` call failed"))
            }
        }
    }

    /// Get the civilization for the command
    fn get_civ(console: &DevConsolePopup, top_level_command: &str, param_stack: &mut VecDeque<CliInput>) -> Option<Civilization> {
        if top_level_command != "civ" {
            return None;
        }

        // Came from `civ activatetrigger`: We want a civ, but on the command line it should be an optional parameter, defaulting to WorldScreen selected
        let name = param_stack.front()?;
        let civ = console.get_civ_by_name_or_null(name)
            .unwrap_or_else(|| console.screen.selected_civ.clone());

        // name was good - remove from deque
        param_stack.pop_front();
        Some(civ)
    }

    /// Get the unique for the command
    fn get_unique(console: &DevConsolePopup, param_stack: &mut VecDeque<CliInput>) -> Result<Unique, Box<dyn std::error::Error>> {
        let mut unique_text = param_stack.pop_front()
            .ok_or_else(|| ConsoleErrorException::new("Parameter triggeredUnique missing"))?
            .to_method(Method::Quoted)?;

        let unique_type = get_unique_type(&unique_text)?;

        if !param_stack.is_empty() && unique_text.to_string() == unique_type.text() {
            // Simplification: You either specify a fully formatted Unique as one parameter or the default text and a full set of replacements
            let params: Vec<String> = param_stack.iter()
                .map(|p| p.original_unquoted())
                .collect();

            unique_text = CliInput::new(
                fill_placeholders(&unique_type.placeholder_text(), &params),
                Method::Quoted
            );
        }

        let unique = Unique::new(unique_text.to_string(), UniqueTarget::Triggerable, "DevConsole".to_string());
        let validator = UniqueValidator::new(&console.game_info.ruleset);
        let errors = validator.check_unique(&unique, false, &ConsoleRulesetObject::new(), true);

        if !errors.is_ok() {
            return Err(Box::new(ConsoleErrorException::new(errors.get_error_text(true))));
        }

        Ok(unique)
    }

    /// Get the unique type from a parameter
    fn get_unique_type(param: &CliInput) -> Result<UniqueType, Box<dyn std::error::Error>> {
        let filter_text = CliInput::new(
            get_placeholder_text(&param.to_string()),
            param.method()
        );

        let unique_types: Vec<UniqueType> = UniqueType::iter()
            .filter(|ut| {
                CliInput::new(ut.placeholder_text(), param.method()) == filter_text
            })
            .take(4)
            .collect();

        if unique_types.is_empty() {
            return Err(Box::new(ConsoleErrorException::new(
                format!("`{}` not found in UniqueTypes", param)
            )));
        }

        if unique_types.len() > 1 {
            let type_names: Vec<String> = unique_types.iter()
                .take(3)
                .map(|ut| ut.text())
                .collect();

            return Err(Box::new(ConsoleErrorException::new(
                format!("`{}` has ambiguous UniqueType: {}?", param, type_names.join(", "))
            )));
        }

        let unique_type = unique_types[0];
        if unique_type.can_accept_unique_target(UniqueTarget::Triggerable) {
            Ok(unique_type)
        } else {
            Err(Box::new(ConsoleErrorException::new(
                format!("`{}` is not a Triggerable", param)
            )))
        }
    }
}

impl ConsoleAction for ConsoleTriggerAction {
    fn format(&self) -> String {
        format!("{} activatetrigger <triggeredUnique|triggeredUniqueTemplate> [uniqueParam]...", self.top_level_command)
    }

    fn execute(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
        let action = Self::get_action(&self.top_level_command);
        action(console, params)
    }
}

/// Ruleset object for the console
struct ConsoleRulesetObject {
    name: String,
}

impl ConsoleRulesetObject {
    fn new() -> Self {
        Self {
            name: "DevConsole".to_string(),
        }
    }
}

impl RulesetObject for ConsoleRulesetObject {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Triggerable
    }

    fn make_link(&self) -> String {
        String::new()
    }
}