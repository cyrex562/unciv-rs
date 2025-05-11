use std::collections::HashMap;

use crate::constants::Constants;
use crate::logic::city::City;
use crate::logic::civilization::{LocationAction, Notification, NotificationCategory, NotificationIcon};
use crate::logic::map::mapgenerator::RiverGenerator;
use crate::logic::map::mapgenerator::RiverGenerator::RiverDirections;
use crate::logic::map::tile::Tile;
use crate::models::ruleset::tile::Terrain;
use crate::models::ruleset::tile::TerrainType;
use crate::ui::screens::devconsole::cli_input::DevConsolePopupExt;
use crate::ui::screens::devconsole::console_command::ConsoleAction;
use crate::ui::screens::devconsole::console_command_node::ConsoleCommandNode;
use crate::ui::screens::devconsole::console_error::ConsoleErrorException;
use crate::ui::screens::devconsole::console_hint::ConsoleHintException;
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;

/// Commands for managing tiles in the dev console
pub struct ConsoleTileCommands;

impl ConsoleCommandNode for ConsoleTileCommands {
    fn subcommands(&self) -> HashMap<String, Box<dyn ConsoleAction>> {
        let mut commands = HashMap::new();

        // checkfilter command
        commands.insert(
            "checkfilter".to_string(),
            Box::new(ConsoleAction::new("tile checkfilter <tileFilter>", |console, params| {
                let selected_tile = console.get_selected_tile();
                Ok(DevConsoleResponse::hint(selected_tile.matches_filter(params[0].original_unquoted()).to_string()))
            })),
        );

        // setimprovement command
        commands.insert(
            "setimprovement".to_string(),
            Box::new(ConsoleAction::new("tile setimprovement <improvementName> [civName]", |console, params| {
                let selected_tile = console.get_selected_tile();
                let improvement = params[0].find(&console.game_info.ruleset.tile_improvements.values)?;
                let civ = params.get(1).map(|p| console.get_civ_by_name(p))?;

                selected_tile.improvement_functions.set_improvement(improvement.name.clone(), civ);
                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // removeimprovement command
        commands.insert(
            "removeimprovement".to_string(),
            Box::new(ConsoleAction::new("tile removeimprovement", |console, _| {
                let selected_tile = console.get_selected_tile();
                selected_tile.improvement_functions.set_improvement(None, None);
                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // setpillaged command
        commands.insert(
            "setpillaged".to_string(),
            Box::new(ConsoleAction::new("tile setpillaged <boolean>", |console, params| {
                let selected_tile = console.get_selected_tile();
                let set_pillaged = params[0].to_boolean()?;

                if set_pillaged {
                    selected_tile.set_pillaged();
                } else {
                    selected_tile.set_repaired();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // removeroad command
        commands.insert(
            "removeroad".to_string(),
            Box::new(ConsoleAction::new("tile removeroad", |console, _| {
                let selected_tile = console.get_selected_tile();
                selected_tile.remove_road();

                // This covers many cases but not all - do we really need to loop over all civs?
                if let Some(owner) = selected_tile.get_owner() {
                    owner.cache.update_cities_connected_to_capital();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // addfeature command
        commands.insert(
            "addfeature".to_string(),
            Box::new(ConsoleAction::new("tile addfeature <featureName>", |console, params| {
                let selected_tile = console.get_selected_tile();
                let feature = get_terrain_feature(console, &params[0])?;

                if feature.name == Constants::RIVER {
                    RiverGenerator::continue_river_on(selected_tile);
                } else {
                    selected_tile.add_terrain_feature(feature.name.clone());
                }

                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // removefeature command
        commands.insert(
            "removefeature".to_string(),
            Box::new(ConsoleAction::new("tile removefeature <featureName>", |console, params| {
                let selected_tile = console.get_selected_tile();
                let feature = get_terrain_feature(console, &params[0])?;

                if feature.name == Constants::RIVER {
                    return Err(Box::new(ConsoleHintException::new(
                        "Rivers cannot be removed like a terrain feature - use tile removeriver <direction>"
                    )));
                }

                selected_tile.remove_terrain_feature(feature.name.clone());
                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // setterrain command
        commands.insert(
            "setterrain".to_string(),
            Box::new(ConsoleAction::new("tile setterrain <terrainName>", |console, params| {
                let selected_tile = console.get_selected_tile();
                let terrain = params[0].find(&console.game_info.ruleset.terrains.values)?;

                if terrain.terrain_type == TerrainType::NaturalWonder {
                    set_natural_wonder(selected_tile, terrain)
                } else {
                    set_base_terrain(selected_tile, terrain)
                }
            })),
        );

        // setresource command
        commands.insert(
            "setresource".to_string(),
            Box::new(ConsoleAction::new("tile setresource <resourceName>", |console, params| {
                let selected_tile = console.get_selected_tile();
                let resource = params[0].find(&console.game_info.ruleset.tile_resources.values)?;

                selected_tile.resource = Some(resource.name.clone());
                selected_tile.set_terrain_transients();

                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // removeresource command
        commands.insert(
            "removeresource".to_string(),
            Box::new(ConsoleAction::new("tile removeresource", |console, _| {
                let selected_tile = console.get_selected_tile();
                selected_tile.resource = None;
                selected_tile.set_terrain_transients();

                if let Some(city) = selected_tile.get_city() {
                    city.reassign_population();
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // addriver command
        commands.insert(
            "addriver".to_string(),
            Box::new(ConsoleRiverAction::new("tile addriver <direction>", true)),
        );

        // removeriver command
        commands.insert(
            "removeriver".to_string(),
            Box::new(ConsoleRiverAction::new("tile removeriver <direction>", false)),
        );

        // setowner command
        commands.insert(
            "setowner".to_string(),
            Box::new(ConsoleAction::new("tile setowner [civName|cityName]", |console, params| {
                let selected_tile = console.get_selected_tile();
                let old_owner = selected_tile.get_city();
                let new_owner = get_owner_city(console, params, selected_tile)?;

                // For simplicity, treat assign to civ without cities same as un-assign
                if let Some(old_owner) = old_owner {
                    old_owner.expansion.relinquish_ownership(selected_tile);
                }

                if let Some(new_owner) = new_owner {
                    new_owner.expansion.take_ownership(selected_tile);
                }

                Ok(DevConsoleResponse::OK)
            })),
        );

        // find command
        commands.insert(
            "find".to_string(),
            Box::new(ConsoleAction::new("tile find <tileFilter>", |console, params| {
                let filter = &params[0];
                let locations: Vec<_> = console.game_info.tile_map.tile_list
                    .iter()
                    .filter(|tile| tile.matches_filter(filter.to_string()))
                    .map(|tile| tile.position)
                    .collect();

                if locations.is_empty() {
                    Ok(DevConsoleResponse::hint("None found".to_string()))
                } else {
                    let notification = Notification::new(
                        format!("tile find [{}]", filter),
                        vec![NotificationIcon::Spy],
                        LocationAction::new(locations).as_iterable(),
                        NotificationCategory::General
                    );

                    console.screen.notifications_scroll.one_time_notification = Some(notification.clone());
                    notification.execute(&console.screen);

                    Ok(DevConsoleResponse::OK)
                }
            })),
        );

        commands
    }
}

/// Helper function to set base terrain
fn set_base_terrain(tile: &mut Tile, terrain: &Terrain) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
    if terrain.terrain_type != tile.get_base_terrain().terrain_type {
        return Err(Box::new(ConsoleErrorException::new("Changing terrain type is not allowed")));
    }

    set_base_terrain_by_name(tile, &terrain.name);
    Ok(DevConsoleResponse::OK)
}

/// Helper function to set base terrain by name
fn set_base_terrain_by_name(tile: &mut Tile, terrain_name: &str) {
    tile.base_terrain = terrain_name.to_string();
    tile.set_terrain_transients();

    if let Some(city) = tile.get_city() {
        city.reassign_population();
    }
}

/// Helper function to set natural wonder
fn set_natural_wonder(tile: &mut Tile, wonder: &Terrain) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
    tile.remove_terrain_features();
    tile.natural_wonder = Some(wonder.name.clone());

    let turns_into = wonder.turns_into.as_ref().unwrap_or(&tile.base_terrain);
    set_base_terrain_by_name(tile, turns_into);

    for civ in &tile.tile_map.game_info.civilizations {
        if civ.natural_wonders.contains(&wonder.name) {
            continue;
        }

        if civ.is_defeated() || civ.is_barbarian || civ.is_spectator() {
            continue;
        }

        if !civ.has_explored(tile) {
            continue;
        }

        civ.cache.discover_natural_wonders();
        civ.update_stats_for_next_turn();
    }

    Ok(DevConsoleResponse::OK)
}

/// Helper function to get terrain feature
fn get_terrain_feature(console: &DevConsolePopup, param: &CliInput) -> Result<&Terrain, Box<dyn std::error::Error>> {
    param.find(
        console.game_info.ruleset.terrains.values
            .iter()
            .filter(|t| t.terrain_type == TerrainType::TerrainFeature)
    )
}

/// Helper function to get owner city
fn get_owner_city(console: &DevConsolePopup, params: &[CliInput], selected_tile: &Tile) -> Result<Option<&City>, Box<dyn std::error::Error>> {
    let param = params.get(0)?;
    if param.is_empty() {
        return Ok(None);
    }

    // Look for a city by name to assign the Tile to
    let named_city = param.find_or_null(
        console.game_info.civilizations.iter()
            .flat_map(|civ| civ.cities.iter())
    )?;

    if let Some(named_city) = named_city {
        return Ok(Some(named_city));
    }

    // If the user didn't specify a City, they must have given us a Civilization instead
    let named_civ = console.get_civ_by_name_or_null(param)?;
    if named_civ.is_none() {
        return Err(Box::new(ConsoleErrorException::new(
            format!("{} is neither a city nor a civilization", param)
        )));
    }

    let named_civ = named_civ.unwrap();
    let closest_city = named_civ.cities.iter()
        .min_by_key(|city| {
            city.get_center_tile().aerial_distance_to(selected_tile) +
            if city.is_being_razed { 5 } else { 0 }
        });

    Ok(closest_city)
}

/// Console action for river-related commands
struct ConsoleRiverAction {
    format: String,
    new_value: bool,
}

impl ConsoleRiverAction {
    fn new(format: &str, new_value: bool) -> Self {
        Self {
            format: format.to_string(),
            new_value,
        }
    }
}

impl ConsoleAction for ConsoleRiverAction {
    fn format(&self) -> &str {
        &self.format
    }

    fn execute(&self, console: &mut DevConsolePopup, params: &[CliInput]) -> Result<DevConsoleResponse, Box<dyn std::error::Error>> {
        let selected_tile = console.get_selected_tile();
        let direction = params[0].enum_value::<RiverDirections>()?;

        let other_tile = direction.get_neighbor_tile(selected_tile)
            .ok_or_else(|| ConsoleErrorException::new(
                format!("tile has no neighbor to the {}", direction.name())
            ))?;

        if !other_tile.is_land {
            return Err(Box::new(ConsoleErrorException::new(
                format!("there's no land to the {}", direction.name())
            )));
        }

        selected_tile.set_connected_by_river(other_tile, self.new_value);
        Ok(DevConsoleResponse::OK)
    }
}