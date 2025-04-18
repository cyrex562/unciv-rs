use std::collections::HashMap;

use crate::models::ruleset::Building;
use crate::ui::screens::devconsole::cli_input::DevConsolePopupExt;
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_command_node::ConsoleCommandNode;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Commands for managing cities in the dev console
pub struct ConsoleCityCommands;

impl ConsoleCommandNode for ConsoleCityCommands {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleAction>> {
        let mut commands = HashMap::new();

        // checkfilter command
        commands.insert(
            "checkfilter".to_string(),
            Box::new(ConsoleAction::new("city checkfilter <cityFilter>", |console, params| {
                let city = console.get_selected_city();
                DevConsoleResponse::hint(city.matches_filter(params[0].original_unquoted()).to_string());
            })),
        );

        // add command
        commands.insert(
            "add".to_string(),
            Box::new(ConsoleAction::new("city add <civName>", |console, params| {
                let civ = console.get_civ_by_name(params[0].original_unquoted());
                if !civ.is_major_civ() && !civ.is_city_state() {
                    return Err(ConsoleErrorException::new("Can only add cities to major civs or city states"));
                }
                let selected_tile = console.get_selected_tile();
                if selected_tile.is_city_center() {
                    return Err(ConsoleErrorException::new("Tile already contains a city center"));
                }
                civ.add_city(selected_tile.position);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // remove command
        commands.insert(
            "remove".to_string(),
            Box::new(ConsoleAction::new("city remove", |console, _| {
                let city = console.get_selected_city();
                city.destroy_city(true); // override_safeties = true
                Ok(DevConsoleResponse::OK)
            })),
        );

        // setpop command
        commands.insert(
            "setpop".to_string(),
            Box::new(ConsoleAction::new("city setpop <amount>", |console, params| {
                let city = console.get_selected_city();
                let new_pop = params[0].to_int()?;
                if new_pop < 1 {
                    return Err(ConsoleErrorException::new("Population must be at least 1"));
                }
                city.population.set_population(new_pop);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // setname command
        commands.insert(
            "setname".to_string(),
            Box::new(ConsoleAction::new("city setname <\"name\">", |console, params| {
                let city = console.get_selected_city();
                city.name = params[0].original_unquoted();
                Ok(DevConsoleResponse::OK)
            })),
        );

        // addtile command
        commands.insert(
            "addtile".to_string(),
            Box::new(ConsoleAction::new("city addtile <cityName> [radius]", |console, params| {
                let selected_tile = console.get_selected_tile();
                let city = console.get_city(params[0].original_unquoted())?;

                if !selected_tile.neighbors.iter().any(|tile| tile.get_city() == Some(&city)) {
                    return Err(ConsoleErrorException::new("Tile is not adjacent to any tile already owned by the city"));
                }

                if selected_tile.is_city_center() {
                    return Err(ConsoleErrorException::new("Cannot transfer city center"));
                }

                let radius = params.get(1).map(|p| p.to_int()).unwrap_or(Ok(0))?;

                for tile in selected_tile.get_tiles_in_distance(radius) {
                    if tile.get_city() != Some(&city) && !tile.is_city_center() {
                        city.expansion.take_ownership(tile);
                    }
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // removetile command
        commands.insert(
            "removetile".to_string(),
            Box::new(ConsoleAction::new("city removetile", |console, _| {
                let selected_tile = console.get_selected_tile();
                let city = console.get_selected_city();
                city.expansion.relinquish_ownership(selected_tile);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // religion command
        commands.insert(
            "religion".to_string(),
            Box::new(ConsoleAction::new("city religion <religionName> <±pressure>", |console, params| {
                let city = console.get_selected_city();
                let religion_name = params[0].original_unquoted();
                let religion = console.game_info.religions.keys()
                    .find(|&r| r == religion_name)
                    .ok_or_else(|| ConsoleErrorException::new(&format!("'{}' is not a known religion", religion_name)))?;

                let pressure = params[1].to_int()?;
                let current_pressure = city.religion.get_pressures().get(religion).copied().unwrap_or(0);
                let min_pressure = -current_pressure;

                city.religion.add_pressure(religion, pressure.max(min_pressure));
                city.religion.update_pressure_on_population_change(0);

                Ok(DevConsoleResponse::OK)
            })),
        );

        // sethealth command
        commands.insert(
            "sethealth".to_string(),
            Box::new(ConsoleAction::new("city sethealth [amount]", |console, params| {
                let city = console.get_selected_city();
                let max_health = city.get_max_health();
                let health = params.first().map(|p| p.to_int()).unwrap_or(Ok(max_health))?;

                if health < 1 || health > max_health {
                    return Err(ConsoleErrorException::new("Number out of range"));
                }

                city.health = health;
                Ok(DevConsoleResponse::OK)
            })),
        );

        // addbuilding command
        commands.insert(
            "addbuilding".to_string(),
            Box::new(ConsoleAction::new("city addbuilding <buildingName>", |console, params| {
                let city = console.get_selected_city();
                let building_name = params[0].original_unquoted();
                let building = console.find_cli_input::<Building>(&params[0])
                    .ok_or_else(|| ConsoleErrorException::new("Unknown building"))?;

                city.city_constructions.add_building(building);
                Ok(DevConsoleResponse::OK)
            })),
        );

        // removebuilding command
        commands.insert(
            "removebuilding".to_string(),
            Box::new(ConsoleAction::new("city removebuilding <buildingName>", |console, params| {
                let city = console.get_selected_city();
                let building_name = params[0].original_unquoted();
                let building = console.find_cli_input::<Building>(&params[0])
                    .ok_or_else(|| ConsoleErrorException::new("Unknown building"))?;

                city.city_constructions.remove_building(building);
                Ok(DevConsoleResponse::OK)
            })),
        );

        commands
    }
}