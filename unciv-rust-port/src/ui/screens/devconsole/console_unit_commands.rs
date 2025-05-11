use std::collections::HashMap;

use crate::ui::screens::devconsole::cli_input::{CliInput, get_autocomplete_string, or_empty};
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_command_node::ConsoleCommandNode;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Commands for managing units in the dev console
pub struct ConsoleUnitCommands;

impl ConsoleCommandNode for ConsoleUnitCommands {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleAction>> {
        let mut commands = HashMap::new();

        // checkfilter command
        commands.insert(
            "checkfilter".to_string(),
            Box::new(ConsoleAction::new("unit checkfilter <unitFilter>", |console, params| {
                let unit = console.get_selected_unit()?;
                Ok(DevConsoleResponse::hint(unit.matches_filter(params[0].original_unquoted()).to_string()))
            })),
        );

        // add command
        commands.insert(
            "add".to_string(),
            Box::new(ConsoleAction::new("unit add <civName> <unitName>", |console, params| {
                let selected_tile = console.get_selected_tile()?;
                let civ = console.get_civ_by_name(&params[0])?;
                let base_unit = params[1].find(&console.game_info.ruleset.units.values)?;
                civ.units.place_unit_near_tile(selected_tile.position, base_unit);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // remove command
        commands.insert(
            "remove".to_string(),
            Box::new(ConsoleAction::new("unit remove [all]", |console, params| {
                if !params.is_empty() && params[0].to_string() == "all" {
                    for civ in &console.game_info.civilizations {
                        for unit in civ.units.get_civ_units() {
                            unit.destroy();
                        }
                    }
                } else {
                    let unit = console.get_selected_unit()?;
                    unit.destroy();
                }
                Ok(DevConsoleResponse::OK)
            })),
        );

        // addpromotion command
        commands.insert(
            "addpromotion".to_string(),
            Box::new(ConsoleActionWithAutocomplete::new(
                "unit addpromotion <promotionName>",
                |console, params| {
                    let unit = console.get_selected_unit()?;
                    let promotion = params[0].find(&console.game_info.ruleset.unit_promotions.values)?;
                    unit.promotions.add_promotion(promotion.name.clone(), true);
                    Ok(DevConsoleResponse::OK)
                },
                |console, params| {
                    // Note: filtering by unit.type.name in promotion.unitTypes sounds good (No [Zero]-Ability on an Archer),
                    // but would also prevent promotions that can be legally obtained like Morale and Rejuvenation
                    let promotions = console.get_selected_unit()?.promotions.promotions.clone();
                    let options: Vec<String> = console.game_info.ruleset.unit_promotions.keys()
                        .iter()
                        .filter(|it| !promotions.contains(it))
                        .map(|it| it.replace("[", "").replace("]", ""))
                        .collect();

                    get_autocomplete_string(params.last().map_or_else(|| CliInput::empty(), |p| p.clone()), &options, console)
                }
            )),
        );

        // removepromotion command
        commands.insert(
            "removepromotion".to_string(),
            Box::new(ConsoleActionWithAutocomplete::new(
                "unit removepromotion <promotionName>",
                |console, params| {
                    let unit = console.get_selected_unit()?;
                    let promotion = params[0].find_or_null(&unit.promotions.get_promotions())?
                        .ok_or_else(|| ConsoleErrorException::new("Promotion not found on unit"))?;

                    // No such action in-game so we need to manually update
                    unit.promotions.promotions.remove(&promotion.name);
                    unit.update_uniques();
                    unit.update_visible_tiles();
                    Ok(DevConsoleResponse::OK)
                },
                |console, params| {
                    get_autocomplete_string(
                        params.last().map_or_else(|| CliInput::empty(), |p| p.clone()),
                        &console.get_selected_unit()?.promotions.promotions,
                        console
                    )
                }
            )),
        );

        // setmovement command
        commands.insert(
            "setmovement".to_string(),
            Box::new(ConsoleAction::new("unit setmovement [amount]", |console, params| {
                // Note amount defaults to maxMovement, but is not limited by it - it's an arbitrary choice to allow that
                let unit = console.get_selected_unit()?;
                let movement = params.first()
                    .and_then(|p| if !p.is_empty() { Some(p.to_float()?) } else { None })
                    .unwrap_or_else(|| unit.get_max_movement() as f32);

                if movement < 0.0 {
                    return Err(Box::new(ConsoleErrorException::new("Number out of range")));
                }

                unit.current_movement = movement;
                Ok(DevConsoleResponse::OK)
            })),
        );

        // sethealth command
        commands.insert(
            "sethealth".to_string(),
            Box::new(ConsoleAction::new("unit sethealth [amount]", |console, params| {
                let health = params.first()
                    .and_then(|p| if !p.is_empty() { Some(p.to_int()?) } else { None })
                    .unwrap_or(100);

                if !(1..=100).contains(&health) {
                    return Err(Box::new(ConsoleErrorException::new("Number out of range")));
                }

                let unit = console.get_selected_unit()?;
                unit.health = health;
                Ok(DevConsoleResponse::OK)
            })),
        );

        // setxp command
        commands.insert(
            "setxp".to_string(),
            Box::new(ConsoleAction::new("unit setxp [amount]", |console, params| {
                let xp = params.first()
                    .ok_or_else(|| ConsoleErrorException::new("No XP provided"))?
                    .to_int()?;

                if xp < 0 {
                    return Err(Box::new(ConsoleErrorException::new("Number out of range")));
                }

                let unit = console.get_selected_unit()?;
                unit.promotions.xp = xp;
                Ok(DevConsoleResponse::OK)
            })),
        );

        commands
    }
}

/// Console action with autocomplete functionality
struct ConsoleActionWithAutocomplete {
    format: String,
    execute_fn: Box<dyn Fn(&mut DevConsolePopup, &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> + Send + Sync>,
    autocomplete_fn: Box<dyn Fn(&mut DevConsolePopup, &[CliInput]) -> Option<String> + Send + Sync>,
}

impl ConsoleActionWithAutocomplete {
    fn new<F, G>(format: &str, execute_fn: F, autocomplete_fn: G) -> Self
    where
        F: Fn(&mut DevConsolePopup, &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> + Send + Sync + 'static,
        G: Fn(&mut DevConsolePopup, &[CliInput]) -> Option<String> + Send + Sync + 'static,
    {
        Self {
            format: format.to_string(),
            execute_fn: Box::new(execute_fn),
            autocomplete_fn: Box::new(autocomplete_fn),
        }
    }
}

impl ConsoleAction for ConsoleActionWithAutocomplete {
    fn format(&self) -> &str {
        &self.format
    }

    fn execute(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
        (self.execute_fn)(console, params)
    }

    fn autocomplete(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Option<String> {
        (self.autocomplete_fn)(console, params)
    }
}