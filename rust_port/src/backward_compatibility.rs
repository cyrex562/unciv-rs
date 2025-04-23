use std::collections::HashMap;
use regex::Regex;

use crate::constants::Constants;
use crate::city::CityConstructions;
use crate::diplomacy::{DiplomacyFlags, DiplomacyManager};
use crate::civilization::managers::TechManager;
use crate::models::ruleset::{ModOptions, PerpetualConstruction, Ruleset};
use crate::game_info::GameInfo;
use crate::tile::Tile;
use crate::unit::Unit;

/// Container for all temporarily used code managing transitions from deprecated elements to their replacements.
///
/// Please place ***all*** such code here and call it _only_ from `GameInfo::set_transients`.
/// Functions are allowed to remain once no longer used if you think they might serve as template for
/// similar usecases in the future. Please comment sufficiently.
pub struct BackwardCompatibility;

impl BackwardCompatibility {
    /// Mods can change, leading to things on the map that are no longer defined in the mod.
    /// This function removes them so the game doesn't crash when it tries to access them.
    pub fn remove_missing_mod_references(game_info: &mut GameInfo) {
        game_info.tile_map.remove_missing_terrain_mod_references(&game_info.ruleset);

        Self::remove_units_and_promotions(game_info);
        Self::remove_missing_great_person_points(game_info);

        // Mod decided you can't repair things anymore - get rid of old pillaged improvements
        Self::remove_old_pillaged_improvements(game_info);
        Self::remove_missing_last_seen_improvements(game_info);

        Self::handle_missing_references_for_each_city(game_info);

        Self::remove_tech_and_policies(game_info);
    }

    /// Migrate great general pools to the new format
    pub fn migrate_great_general_pools(game_info: &mut GameInfo) {
        for civ in &mut game_info.civilizations {
            if civ.great_people.points_for_next_great_general >=
               *civ.great_people.points_for_next_great_general_counter.get("Great General").unwrap_or(&0) {
                civ.great_people.points_for_next_great_general_counter.insert(
                    "Great General".to_string(),
                    civ.great_people.points_for_next_great_general
                );
            } else {
                civ.great_people.points_for_next_great_general =
                    *civ.great_people.points_for_next_great_general_counter.get("Great General").unwrap_or(&0);
            }
        }
    }

    /// Remove units and promotions that are no longer defined in the ruleset
    fn remove_units_and_promotions(game_info: &mut GameInfo) {
        for tile in game_info.tile_map.values_mut() {
            let units = tile.get_units().to_vec();
            for unit in units {
                if !game_info.ruleset.units.contains_key(&unit.name) {
                    tile.remove_unit(&unit);
                } else {
                    let promotions = unit.promotions.promotions.clone();
                    for promotion in promotions {
                        if !game_info.ruleset.unit_promotions.contains_key(&promotion) {
                            unit.promotions.promotions.remove(&promotion);
                        }
                    }
                }
            }
        }
    }

    /// Remove great person points for units that are no longer defined in the ruleset
    fn remove_missing_great_person_points(game_info: &mut GameInfo) {
        for civ in &mut game_info.civilizations {
            // Don't remove the 'points to next' counters, since pools do not necessarily correspond to unit names
            let great_general_keys: Vec<String> = civ.great_people.great_general_points_counter.keys().cloned().collect();
            for key in great_general_keys {
                if !game_info.ruleset.units.contains_key(&key) {
                    civ.great_people.great_general_points_counter.remove(&key);
                }
            }

            let great_person_keys: Vec<String> = civ.great_people.great_person_points_counter.keys().cloned().collect();
            for key in great_person_keys {
                if !game_info.ruleset.units.contains_key(&key) {
                    civ.great_people.great_person_points_counter.remove(&key);
                }
            }
        }
    }

    /// Remove old pillaged improvements if repair is no longer defined in the ruleset
    fn remove_old_pillaged_improvements(game_info: &mut GameInfo) {
        if !game_info.ruleset.tile_improvements.contains_key(&Constants::repair) {
            for tile in game_info.tile_map.values_mut() {
                if tile.road_is_pillaged {
                    tile.remove_road();
                }
                if tile.improvement_is_pillaged {
                    tile.improvement = None;
                    tile.improvement_is_pillaged = false;
                }
            }
        }
    }

    /// Remove last seen improvements that are no longer defined in the ruleset
    fn remove_missing_last_seen_improvements(game_info: &mut GameInfo) {
        for civ in &mut game_info.civilizations {
            let improvement_entries: Vec<(String, String)> = civ.last_seen_improvement.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            for (vector, improvement_name) in improvement_entries {
                if !game_info.ruleset.tile_improvements.contains_key(&improvement_name) {
                    civ.last_seen_improvement.remove(&vector);
                }
            }
        }
    }

    /// Handle missing references for each city
    fn handle_missing_references_for_each_city(game_info: &mut GameInfo) {
        for civ in &mut game_info.civilizations {
            for city in &mut civ.cities {
                // Remove built buildings that are no longer defined in the ruleset
                let built_buildings: Vec<String> = city.city_constructions.built_buildings.clone();
                for building in built_buildings {
                    if !game_info.ruleset.buildings.contains_key(&building) {
                        city.city_constructions.built_buildings.remove(&building);
                    }
                }

                // Check if a construction is invalid (not in buildings, units, or perpetual constructions)
                let is_invalid_construction = |construction: &str| -> bool {
                    !game_info.ruleset.buildings.contains_key(construction)
                        && !game_info.ruleset.units.contains_key(construction)
                        && !PerpetualConstruction::perpetual_constructions_map().contains_key(construction)
                };

                // Remove invalid buildings or units from the queue
                let construction_queue: Vec<String> = city.city_constructions.construction_queue.clone();
                for construction in construction_queue {
                    if is_invalid_construction(&construction) {
                        city.city_constructions.construction_queue.retain(|c| c != &construction);
                    }
                }

                // Remove invalid buildings or units from in-progress constructions
                let in_progress_keys: Vec<String> = city.city_constructions.in_progress_constructions.keys().cloned().collect();
                for construction in in_progress_keys {
                    if is_invalid_construction(&construction) {
                        city.city_constructions.in_progress_constructions.remove(&construction);
                    }
                }
            }
        }
    }

    /// Remove technologies and policies that are no longer defined in the ruleset
    fn remove_tech_and_policies(game_info: &mut GameInfo) {
        for civ in &mut game_info.civilizations {
            // Remove technologies that are no longer defined in the ruleset
            let techs_researched: Vec<String> = civ.tech.techs_researched.clone();
            for tech in techs_researched {
                if !game_info.ruleset.technologies.contains_key(&tech) {
                    civ.tech.techs_researched.remove(&tech);
                }
            }

            // Remove policies that are no longer defined in the ruleset
            let adopted_policies: Vec<String> = civ.policies.adopted_policies.clone();
            for policy in adopted_policies {
                if !game_info.ruleset.policies.contains_key(&policy) {
                    civ.policies.adopted_policies.remove(&policy);
                }
            }
        }
    }

    /// Replaces all occurrences of `old_building_name` in `city_constructions` with `new_building_name`
    /// if the former is not contained in the ruleset.
    pub fn change_building_name_if_not_in_ruleset(
        rule_set: &Ruleset,
        city_constructions: &mut CityConstructions,
        old_building_name: &str,
        new_building_name: &str
    ) {
        if rule_set.buildings.contains_key(old_building_name) {
            return;
        }

        // Replace in built buildings
        if city_constructions.is_built(old_building_name) {
            city_constructions.remove_building(old_building_name);
            city_constructions.add_building(new_building_name);
        }

        // Replace in construction queue
        if !city_constructions.is_built(new_building_name) && !city_constructions.construction_queue.contains(&new_building_name.to_string()) {
            city_constructions.construction_queue = city_constructions.construction_queue
                .iter()
                .map(|it| if it == old_building_name { new_building_name.to_string() } else { it.clone() })
                .collect();
        } else {
            city_constructions.construction_queue.retain(|it| it != old_building_name);
        }

        // Replace in in-progress constructions
        if city_constructions.in_progress_constructions.contains_key(old_building_name) {
            if !city_constructions.is_built(new_building_name) && !city_constructions.in_progress_constructions.contains_key(new_building_name) {
                let value = city_constructions.in_progress_constructions.get(old_building_name).unwrap().clone();
                city_constructions.in_progress_constructions.insert(new_building_name.to_string(), value);
            }
            city_constructions.in_progress_constructions.remove(old_building_name);
        }
    }

    /// Replace a changed tech name
    pub fn replace_updated_tech_name(tech_manager: &mut TechManager, old_tech_name: &str, new_tech_name: &str) {
        if tech_manager.techs_researched.contains(old_tech_name) {
            tech_manager.techs_researched.remove(old_tech_name);
            tech_manager.techs_researched.insert(new_tech_name.to_string());
        }

        if let Some(index) = tech_manager.techs_to_research.iter().position(|t| t == old_tech_name) {
            tech_manager.techs_to_research[index] = new_tech_name.to_string();
        }

        if tech_manager.techs_in_progress.contains_key(old_tech_name) {
            let research = tech_manager.research_of_tech(old_tech_name);
            tech_manager.techs_in_progress.insert(new_tech_name.to_string(), research);
            tech_manager.techs_in_progress.remove(old_tech_name);
        }
    }

    /// Replace a deprecated DiplomacyFlags instance
    pub fn replace_diplomacy_flag(game_info: &mut GameInfo, old_flag: DiplomacyFlags, new_flag: DiplomacyFlags) {
        for civ in &mut game_info.civilizations {
            for diplomacy_manager in civ.diplomacy.values_mut() {
                if diplomacy_manager.has_flag(old_flag) {
                    let value = diplomacy_manager.get_flag(old_flag);
                    diplomacy_manager.remove_flag(old_flag);
                    diplomacy_manager.set_flag(new_flag, value);
                }
            }
        }
    }

    /// Make sure all MapUnits have the starting promotions that they're supposed to.
    pub fn guarantee_unit_promotions(game_info: &mut GameInfo) {
        for tile in game_info.tile_map.values() {
            for unit in tile.get_units() {
                for starting_promo in &unit.base_unit.promotions {
                    unit.promotions.add_promotion(starting_promo, true);
                }
            }
        }
    }

    /// Update deprecations in mod options
    pub fn update_deprecations(_mod_options: &mut ModOptions) {
        // Empty function block as in the original
    }

    /// Convert from Fortify X to Fortify and save off X
    pub fn convert_fortify(game_info: &mut GameInfo) {
        let reg = Regex::new(r"^Fortify\s+(\d+)([\w\s]*)").unwrap();

        for civ in &mut game_info.civilizations {
            for unit in civ.units.get_civ_units_mut() {
                if let Some(action) = &unit.action {
                    if let Some(caps) = reg.captures(action) {
                        let turns = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
                        let heal = caps.get(2).unwrap().as_str();

                        unit.turns_fortified = turns;
                        unit.action = Some(format!("Fortify{}", heal));
                    }
                }
            }
        }
    }

    /// Migrate to tile history
    pub fn migrate_to_tile_history(game_info: &mut GameInfo) {
        if game_info.history_start_turn >= 0 {
            return;
        }

        for city in game_info.get_cities() {
            for tile in city.get_tiles() {
                tile.history.record_take_ownership(tile);
            }
        }

        game_info.history_start_turn = game_info.turns;
    }

    /// Ensure all units have valid IDs
    pub fn ensure_unit_ids(game_info: &mut GameInfo) {
        if game_info.last_unit_id == 0 {
            let max_id = game_info.tile_map.values()
                .flat_map(|tile| tile.get_units())
                .map(|unit| unit.id)
                .max()
                .unwrap_or(0)
                .max(0);

            game_info.last_unit_id = max_id;
        }

        for tile in game_info.tile_map.values_mut() {
            for unit in tile.get_units_mut() {
                if unit.id == Constants::NO_ID {
                    game_info.last_unit_id += 1;
                    unit.id = game_info.last_unit_id;
                }
            }
        }
    }
}