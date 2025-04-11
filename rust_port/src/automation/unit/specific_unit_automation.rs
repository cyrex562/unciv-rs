use std::collections::HashSet;
use crate::models::civilization::diplomacy::DiplomaticModifiers;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::city::City;
use crate::models::UnitActionType;
use crate::models::ruleset::Building;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::unique::UniqueType;
use crate::models::stats::Stat;
use crate::automation::Automation;
use crate::automation::unit::civilian_unit_automation::CivilianUnitAutomation;
use crate::automation::unit::city_location_tile_ranker::CityLocationTileRanker;
use crate::battle::GreatGeneralImplementation;
use crate::ui::screens::worldscreen::unit::actions::{UnitActions, UnitActionsFromUniques};
use crate::models::ruleset::unique::LocalUniqueCache;

/// Contains automation logic for specific unit types
pub struct SpecificUnitAutomation;

impl SpecificUnitAutomation {
    /// Automates Great General movement and actions
    pub fn automate_great_general(unit: &mut MapUnit) -> bool {
        // Try to follow nearby units. Do not garrison in city if possible
        let max_affected_troops_tile = match GreatGeneralImplementation::get_best_affected_troops_tile(unit) {
            Some(tile) => tile,
            None => return false,
        };

        unit.movement.head_towards(max_affected_troops_tile);
        true
    }

    /// Automates Citadel placement
    pub fn automate_citadel_placer(unit: &mut MapUnit) -> bool {
        // Keep at least 2 generals alive
        if unit.has_unique(UniqueType::StrengthBonusInRadius)
            && unit.civ.units.get_civ_units().iter()
                .filter(|u| u.has_unique(UniqueType::StrengthBonusInRadius))
                .count() < 3 {
            return false;
        }

        // Try to revenge and capture their tiles
        let enemy_cities: Vec<_> = unit.civ.get_known_civs().iter()
            .filter(|civ| unit.civ.get_diplomacy_manager(civ)
                .map_or(false, |dm| dm.has_modifier(DiplomaticModifiers::StealingTerritory)))
            .flat_map(|civ| civ.cities.iter())
            .collect();

        // Find the suitable tiles (or their neighbours)
        let tile_to_steal = enemy_cities.iter()
            .flat_map(|city| city.get_tiles()) // City tiles
            .filter(|tile| tile.neighbors.iter()
                .any(|neighbor| neighbor.get_owner() != Some(&unit.civ))) // Edge city tiles
            .flat_map(|tile| tile.neighbors.iter()) // Neighbors of edge city tiles
            .filter(|tile| {
                unit.civ.viewable_tiles.contains(tile) // We can see them
                    && tile.neighbors.iter()
                        .any(|neighbor| neighbor.get_owner() == Some(&unit.civ)) // They are close to our borders
            })
            .min_by_key(|tile| {
                // Get closest tiles and prioritize valuable ones
                let distance = tile.aerial_distance_to(unit.current_tile);
                match tile.get_owner() {
                    Some(owner) => {
                        let priority = owner.get_worker_automation()
                            .get_base_priority(tile, unit)
                            .round() as i32;
                        distance - priority
                    }
                    None => distance,
                }
            })
            .filter(|tile| unit.movement.can_reach(tile)); // canReach is performance-heavy and always a last resort

        // If there is a good tile to steal - go there
        if let Some(tile) = tile_to_steal {
            unit.movement.head_towards(tile);
            if unit.has_movement() && unit.current_tile == tile {
                if let Some(action) = UnitActionsFromUniques::get_improvement_construction_actions_from_general_unique(unit, tile)
                    .first() {
                    action.action();
                }
            }
            return true;
        }

        // Try to build a citadel for defensive purposes
        if unit.civ.get_worker_automation().evaluate_fort_placement(unit.current_tile, true) {
            if let Some(action) = UnitActionsFromUniques::get_improvement_construction_actions_from_general_unique(unit, unit.current_tile)
                .first() {
                action.action();
            }
            return true;
        }
        false
    }

    /// Handles Great General fallback behavior when no units to follow
    pub fn automate_great_general_fallback(unit: &mut MapUnit) {
        let reachable_test = |tile: &Tile| {
            tile.civilian_unit.is_none()
                && unit.movement.can_move_to(tile)
                && unit.movement.can_reach(tile)
        };

        let city_to_garrison = unit.civ.cities.iter()
            .map(|city| city.get_center_tile())
            .min_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
            .filter(|tile| reachable_test(tile));

        let city_to_garrison = match city_to_garrison {
            Some(city) => city,
            None => return,
        };

        if !unit.cache.has_citadel_placement_unique {
            unit.movement.head_towards(city_to_garrison);
            return;
        }

        // Try to find a good place for citadel nearby
        let tile_for_citadel = city_to_garrison.get_tiles_in_distance_range(3..=4)
            .iter()
            .find(|tile| {
                reachable_test(tile)
                    && unit.civ.get_worker_automation().evaluate_fort_placement(tile, true)
            });

        match tile_for_citadel {
            Some(tile) => {
                unit.movement.head_towards(tile);
                if unit.has_movement() && unit.current_tile == tile {
                    if let Some(action) = UnitActionsFromUniques::get_improvement_construction_actions_from_general_unique(unit, tile)
                        .first() {
                        action.action();
                    }
                }
            }
            None => unit.movement.head_towards(city_to_garrison),
        }
    }

    /// Automates settler actions
    pub fn automate_settler_actions(unit: &mut MapUnit, dangerous_tiles: &HashSet<Tile>) {
        // If we don't have any cities, we are probably at the start of the game with only one settler
        // If we are at the start of the game lets spend a maximum of 3 turns to settle our first city
        // As our turns progress lets shrink the area that we look at to make sure that we stay on target
        // If we have gone more than 3 turns without founding a city lets search a wider area
        let range_to_search = if unit.civ.cities.is_empty() && unit.civ.game_info.turns < 4 {
            Some((3 - unit.civ.game_info.turns).max(1))
        } else {
            None
        };

        // It's possible that we'll see a tile "over the sea" that's better than the tiles close by,
        // but that's not a reason to abandon the close tiles!
        let best_tiles_info = CityLocationTileRanker::get_best_tiles_to_found_city(unit, range_to_search, 50.0);
        let mut best_city_location = None;

        // Special case, we want AI to settle in place on turn 1
        if unit.civ.game_info.turns == 0 && unit.civ.cities.is_empty()
            && best_tiles_info.tile_rank_map.contains_key(&unit.get_tile()) {
            let found_city_action = UnitActionsFromUniques::get_found_city_action(unit, unit.get_tile());

            // Depending on era and difficulty we might start with more than one settler
            let all_unsettled_settlers: Vec<_> = unit.civ.units.get_civ_units().iter()
                .filter(|u| u.has_movement() && u.base_unit == unit.base_unit)
                .collect();

            // Don't settle immediately if we only have one settler, look for a better location
            let best_settler_in_range = all_unsettled_settlers.iter()
                .max_by_key(|settler| {
                    best_tiles_info.tile_rank_map.get(&settler.get_tile())
                        .map_or(-1.0, |&rank| rank)
                });

            if let Some(&best_settler) = best_settler_in_range {
                if best_settler == unit {
                    if let Some(action) = found_city_action {
                        action.action();
                        return;
                    }
                }
                // Since this settler is not in the best location, lets assume the best settler will found their city where they are
                best_tiles_info.tile_rank_map.retain(|tile, _|
                    tile.aerial_distance_to(best_settler.get_tile()) > 4
                );
            }
        }

        // If the tile we are currently on is close to the best tile, then lets just settle here instead
        if best_tiles_info.tile_rank_map.contains_key(&unit.get_tile()) {
            if best_tiles_info.best_tile.is_none()
                || best_tiles_info.tile_rank_map[&unit.get_tile()] >= best_tiles_info.best_tile_rank - 2.0 {
                best_city_location = Some(unit.get_tile());
            }
        }

        // Shortcut, if the best tile is nearby than lets just take it
        if best_city_location.is_none() && best_tiles_info.best_tile.is_some() {
            let path_size = unit.movement.get_shortest_path(best_tiles_info.best_tile.unwrap()).len();
            if (1..=3).contains(&path_size) {
                best_city_location = best_tiles_info.best_tile;
            }
        }

        if best_city_location.is_none() {
            // Find the best tile that is within range
            let is_tile_rank_ok = |entry: (&Tile, &f32)| {
                if dangerous_tiles.contains(entry.0) && entry.0 != unit.get_tile() {
                    return false;
                }
                let path_size = unit.movement.get_shortest_path(entry.0).len();
                (1..=3).contains(&path_size)
            };

            best_city_location = best_tiles_info.tile_rank_map.iter()
                .filter(|&(_, &rank)| {
                    best_tiles_info.best_tile.is_none()
                        || rank >= best_tiles_info.best_tile_rank - 5.0
                })
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .filter(|entry| is_tile_rank_ok(entry))
                .map(|(tile, _)| tile);
        }

        // We still haven't found a best city tile within 3 turns so lets just head to the best tile
        if best_city_location.is_none() && best_tiles_info.best_tile.is_some() {
            let path_size = unit.movement.get_shortest_path(best_tiles_info.best_tile.unwrap()).len();
            if (1..=8).contains(&path_size) {
                best_city_location = best_tiles_info.best_tile;
            }
        }

        if best_city_location.is_none() {
            // Try to move towards the frontier
            let get_frontier_score = |city: &City| {
                city.get_center_tile()
                    .get_tiles_at_distance(
                        city.civ.game_info.ruleset.mod_options.constants.minimal_city_distance + 1
                    )
                    .iter()
                    .filter(|tile| {
                        tile.can_be_settled()
                            && tile.get_owner().map_or(true, |owner| owner == &city.civ)
                    })
                    .count()
            };

            let frontier_city = unit.civ.cities.iter()
                .max_by_key(|city| get_frontier_score(city));

            if let Some(city) = frontier_city {
                if get_frontier_score(city) > 0 && unit.movement.can_reach(city.get_center_tile()) {
                    unit.movement.head_towards(city.get_center_tile());
                }
            }

            // Try to find new areas or wander aimlessly
            if UnitAutomation::try_explore(unit) {
                return;
            }
            UnitAutomation::wander(unit, Some(dangerous_tiles));
            return;
        }

        let best_city_location = best_city_location.unwrap();
        let found_city_action = UnitActionsFromUniques::get_found_city_action(unit, best_city_location);

        if found_city_action.is_none() {
            if unit.has_movement() && !unit.civ.is_one_city_challenger() {
                panic!("City within distance");
            }
            return;
        }

        let should_settle = unit.get_tile() == best_city_location && unit.has_movement();
        if should_settle {
            found_city_action.unwrap().action();
            return;
        }

        // Settle if we're already on the best tile, before looking if we should retreat from barbarians
        if CivilianUnitAutomation::try_run_away_if_neccessary(unit) {
            return;
        }

        unit.movement.head_towards(best_city_location);
        if should_settle {
            found_city_action.unwrap().action();
        }
    }

    /// Automates improvement placement
    /// Returns whether there was any progress in placing the improvement
    pub fn automate_improvement_placer(unit: &mut MapUnit) -> bool {
        let improvement_building_unique = unit.get_matching_uniques(UniqueType::ConstructImprovementInstantly)
            .first()
            .cloned();

        let improvement_building_unique = match improvement_building_unique {
            Some(unique) => unique,
            None => return false,
        };

        let improvement_name = &improvement_building_unique.params[0];
        let improvement = match unit.civ.game_info.ruleset.tile_improvements.get(improvement_name) {
            Some(imp) => imp,
            None => return false,
        };

        let related_stat = improvement.iter()
            .max_by_key(|(_, &value)| value)
            .map(|(stat, _)| stat)
            .unwrap_or(Stat::Culture);

        let cities_by_stat_boost: Vec<_> = unit.civ.cities.iter()
            .sorted_by(|a, b| {
                let a_bonus = a.city_stats.stat_percent_bonus_tree.total_stats[related_stat];
                let b_bonus = b.city_stats.stat_percent_bonus_tree.total_stats[related_stat];
                b_bonus.partial_cmp(&a_bonus).unwrap()
            })
            .collect();

        let average_terrain_stats_value = unit.civ.game_info.ruleset.terrains.values()
            .filter(|terrain| terrain.type_ == TerrainType::Land)
            .map(|terrain| Automation::rank_stats_value(terrain, &unit.civ))
            .sum::<f32>() / unit.civ.game_info.ruleset.terrains.len() as f32;

        let local_unique_cache = LocalUniqueCache::new();

        for city in cities_by_stat_boost {
            let applicable_tiles: Vec<_> = city.get_workable_tiles()
                .iter()
                .filter(|tile| {
                    tile.is_land
                        && tile.resource.is_none()
                        && !tile.is_city_center()
                        && (unit.current_tile == **tile || unit.movement.can_move_to(tile))
                        && tile.improvement.is_none()
                        && tile.improvement_functions.can_build_improvement(improvement, &unit.civ)
                        && Automation::rank_tile(tile, &unit.civ, &local_unique_cache) > average_terrain_stats_value
                })
                .collect();

            if applicable_tiles.is_empty() {
                continue;
            }

            let path_to_city = unit.movement.get_shortest_path(city.get_center_tile());
            if path_to_city.is_empty() {
                continue;
            }

            if path_to_city.len() > 2 && unit.get_tile().get_city() != Some(city) {
                // Radius 5 is quite arbitrary. Few units have such a high movement radius although
                // streets might modify it. Also there might be invisible units, so this is just an
                // approximation for relative safety and simplicity.
                let enemy_units_nearby = unit.get_tile()
                    .get_tiles_in_distance(5)
                    .iter()
                    .any(|tile_nearby| {
                        tile_nearby.get_units().iter().any(|unit_on_tile| {
                            unit_on_tile.is_military()
                                && unit_on_tile.civ.is_at_war_with(&unit.civ)
                        })
                    });

                // Don't move until you're accompanied by a military unit if there are enemies nearby
                if unit.get_tile().military_unit.is_none() && enemy_units_nearby {
                    return true;
                }

                unit.movement.head_towards(city.get_center_tile());
                return true;
            }

            // If we got here, we're pretty close, start looking!
            let chosen_tile = applicable_tiles.iter()
                .sorted_by(|a, b| {
                    let rank_a = Automation::rank_tile(a, &unit.civ, &local_unique_cache);
                    let rank_b = Automation::rank_tile(b, &unit.civ, &local_unique_cache);
                    rank_b.partial_cmp(&rank_a).unwrap()
                })
                .find(|tile| unit.movement.can_reach(tile));

            let chosen_tile = match chosen_tile {
                Some(tile) => tile,
                None => continue,
            };

            let unit_tile_before_movement = unit.current_tile;
            unit.movement.head_towards(chosen_tile);

            if unit.current_tile == chosen_tile {
                if unit.current_tile.is_pillaged() {
                    UnitActions::invoke_unit_action(unit, UnitActionType::Repair);
                } else {
                    UnitActions::invoke_unit_action(unit, UnitActionType::CreateImprovement);
                }
                return true;
            }

            return unit_tile_before_movement != unit.current_tile;
        }

        // No city needs this improvement
        false
    }

    /// Conducts trade mission with city-states
    /// Returns whether there was any progress in conducting the trade mission
    pub fn conduct_trade_mission(unit: &mut MapUnit) -> bool {
        let closest_city_state_tile = unit.civ.game_info.civilizations.iter()
            .filter(|civ| {
                civ != &unit.civ
                    && !unit.civ.is_at_war_with(civ)
                    && civ.is_city_state
                    && !civ.cities.is_empty()
            })
            .flat_map(|civ| civ.cities[0].get_tiles())
            .filter(|tile| unit.civ.has_explored(tile))
            .filter_map(|tile| {
                let path = unit.movement.get_shortest_path(tile);
                // 0 is unreachable, 10 is too far away
                if path.len() >= 1 && path.len() <= 10 {
                    Some((tile, path.len()))
                } else {
                    None
                }
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(tile, _)| tile);

        let closest_city_state_tile = match closest_city_state_tile {
            Some(tile) => tile,
            None => return false,
        };

        if UnitActions::invoke_unit_action(unit, UnitActionType::ConductTradeMission) {
            return true;
        }

        let unit_tile_before_movement = unit.current_tile;
        unit.movement.head_towards(closest_city_state_tile);

        unit_tile_before_movement != unit.current_tile
    }

    /// Speeds up wonder construction in nearby cities
    /// Returns whether there was any progress in speeding up a wonder construction
    pub fn speedup_wonder_construction(unit: &mut MapUnit) -> bool {
        let nearby_city_with_available_wonders = unit.civ.cities.iter()
            .filter(|city| {
                // Don't speed up construction in small cities
                city.population.population >= 3
                    && (unit.movement.can_move_to(city.get_center_tile())
                        || unit.current_tile == city.get_center_tile())
                    && Self::get_wonder_that_would_benefit_from_being_sped_up(city).is_some()
            })
            .filter_map(|city| {
                let path = unit.movement.get_shortest_path(city.get_center_tile());
                if !path.is_empty() && path.len() <= 5 {
                    Some((city, path.len()))
                } else {
                    None
                }
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(city, _)| city);

        let nearby_city = match nearby_city_with_available_wonders {
            Some(city) => city,
            None => return false,
        };

        if unit.current_tile == nearby_city.get_center_tile() {
            let wonder_to_hurry = Self::get_wonder_that_would_benefit_from_being_sped_up(nearby_city)
                .unwrap();
            nearby_city.city_constructions.construction_queue
                .insert(0, wonder_to_hurry.name.clone());

            return UnitActions::invoke_unit_action(unit, UnitActionType::HurryBuilding)
                || UnitActions::invoke_unit_action(unit, UnitActionType::HurryWonder);
        }

        // Walk towards the city
        let tile_before_moving = unit.get_tile();
        unit.movement.head_towards(nearby_city.get_center_tile());
        tile_before_moving != unit.current_tile
    }

    /// Gets a wonder that would benefit from being sped up in the given city
    fn get_wonder_that_would_benefit_from_being_sped_up(city: &City) -> Option<&Building> {
        city.city_constructions.get_buildable_buildings()
            .iter()
            .filter(|building| {
                building.is_wonder
                    && !building.has_unique(UniqueType::CannotBeHurried)
                    && city.city_constructions.turns_to_construction(&building.name) >= 5
            })
            .max_by_key(|building|
                -city.city_constructions.get_remaining_work(&building.name)
            )
    }

    /// Automates adding unit to capital
    pub fn automate_add_in_capital(unit: &mut MapUnit) {
        let capital = match unit.civ.get_capital() {
            Some(city) => city,
            None => return, // Safeguard
        };

        let capital_tile = capital.get_center_tile();
        if unit.movement.can_reach(capital_tile) {
            unit.movement.head_towards(capital_tile);
        }

        if unit.get_tile() == capital_tile {
            UnitActions::invoke_unit_action(unit, UnitActionType::AddInCapital);
        }
    }
}