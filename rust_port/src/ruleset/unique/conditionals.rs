use std::collections::HashSet;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::models::ruleset::{
    Belief, Ruleset, RulesetStatsObject, StateForConditionals, UniqueTarget, UniqueType,
};
use crate::models::ruleset::tile::Terrain;
use crate::models::civilization::Civilization;
use crate::models::map::tile::Tile;
use crate::models::stats::{GameResource, Stats};
use crate::models::ui::FormattedLine;
use crate::utils::{MultiFilter, uniques_to_civilopedia_text_lines};
use crate::models::game_info::GameInfo;
use crate::models::battle::CombatAction;
use crate::models::city::City;
use crate::models::civilization::managers::ReligionState;
use crate::models::ruleset::validation::ModCompatibility;
use crate::models::stats::Stat;
use crate::models::unique::Unique;
use crate::models::unique::Countables;

/// A module for handling conditional logic in the game
pub struct Conditionals;

impl Conditionals {
    /// Gets a random number based on the state and unique
    fn get_state_based_random(state: &StateForConditionals, unique: Option<&Unique>) -> f32 {
        let mut seed = state.game_info.as_ref().map_or(0, |gi| gi.turns as i32);
        seed = seed.wrapping_mul(31).wrapping_add(unique.map_or(0, |u| u.hash_code()));
        seed = seed.wrapping_mul(31).wrapping_add(state.hash_code());

        let mut rng = StdRng::seed_from_u64(seed as u64);
        rng.gen::<f32>()
    }

    /// Checks if a conditional applies to a unique
    pub fn conditional_applies(
        unique: Option<&Unique>,
        conditional: &Unique,
        state: &StateForConditionals
    ) -> bool {
        // Check if the conditional is a non-filtering type
        if let Some(target_types) = &conditional.unique_type.target_types {
            if target_types.iter().any(|t| t.modifier_type == UniqueTarget::ModifierType::Other) {
                return true;
            }
        }

        // Helper to simplify conditional tests requiring gameInfo
        let check_on_game_info = |predicate: impl FnOnce(&GameInfo) -> bool| -> bool {
            state.game_info.as_ref().map_or(false, predicate)
        };

        // Helper to simplify conditional tests requiring a Civilization
        let check_on_civ = |predicate: impl FnOnce(&Civilization) -> bool| -> bool {
            state.relevant_civ.as_ref().map_or(false, predicate)
        };

        // Helper to simplify conditional tests requiring a City
        let check_on_city = |predicate: impl FnOnce(&City) -> bool| -> bool {
            state.relevant_city.as_ref().map_or(false, predicate)
        };

        // Helper to simplify the "compare civ's current era with named era" conditions
        let compare_era = |era_param: &str, compare: impl FnOnce(i32, i32) -> bool| -> bool {
            if let Some(game_info) = &state.game_info {
                if let Some(era) = game_info.ruleset.eras.get(era_param) {
                    if let Some(civ) = &state.relevant_civ {
                        return compare(civ.get_era_number(), era.era_number);
                    }
                }
            }
            false
        };

        // Helper for ConditionalWhenAboveAmountStatResource and its below counterpart
        let check_resource_or_stat_amount = |resource_or_stat_name: &str, lower_limit: f32, upper_limit: f32, modify_by_game_speed: bool, compare: impl FnOnce(i32, f32, f32) -> bool| -> bool {
            if let Some(game_info) = &state.game_info {
                let game_speed_modifier = if modify_by_game_speed { game_info.speed.modifier } else { 1.0 };

                if game_info.ruleset.tile_resources.contains_key(resource_or_stat_name) {
                    return compare(state.get_resource_amount(resource_or_stat_name), lower_limit * game_speed_modifier, upper_limit * game_speed_modifier);
                }

                if let Some(stat) = Stat::safe_value_of(resource_or_stat_name) {
                    let stat_reserve = state.get_stat_amount(stat);
                    let game_speed_modifier = if modify_by_game_speed { game_info.speed.stat_cost_modifiers.get(&stat).copied().unwrap_or(1.0) } else { 1.0 };
                    return compare(stat_reserve, lower_limit * game_speed_modifier, upper_limit * game_speed_modifier);
                }
            }
            false
        };

        // Helper for comparing countables
        let compare_countables = |first: &str, second: &str, compare: impl FnOnce(i32, i32) -> bool| -> bool {
            let first_number = Countables::get_countable_amount(first, state);
            let second_number = Countables::get_countable_amount(second, state);

            if let (Some(first), Some(second)) = (first_number, second_number) {
                compare(first, second)
            } else {
                false
            }
        };

        // Helper for comparing three countables
        let compare_countables_three = |first: &str, second: &str, third: &str, compare: impl FnOnce(i32, i32, i32) -> bool| -> bool {
            let first_number = Countables::get_countable_amount(first, state);
            let second_number = Countables::get_countable_amount(second, state);
            let third_number = Countables::get_countable_amount(third, state);

            if let (Some(first), Some(second), Some(third)) = (first_number, second_number, third_number) {
                compare(first, second, third)
            } else {
                false
            }
        };

        match conditional.unique_type {
            UniqueType::ConditionalChance => {
                let chance = conditional.params[0].parse::<f32>().unwrap_or(0.0) / 100.0;
                Self::get_state_based_random(state, unique) < chance
            },
            UniqueType::ConditionalEveryTurns => {
                check_on_game_info(|gi| {
                    let turns = conditional.params[0].parse::<i32>().unwrap_or(0);
                    gi.turns % turns == 0
                })
            },
            UniqueType::ConditionalBeforeTurns => {
                check_on_game_info(|gi| {
                    let turns = conditional.params[0].parse::<i32>().unwrap_or(0);
                    gi.turns < turns
                })
            },
            UniqueType::ConditionalAfterTurns => {
                check_on_game_info(|gi| {
                    let turns = conditional.params[0].parse::<i32>().unwrap_or(0);
                    gi.turns >= turns
                })
            },
            UniqueType::ConditionalTutorialsEnabled => {
                crate::models::game::UncivGame::current().settings.show_tutorials
            },
            UniqueType::ConditionalTutorialCompleted => {
                let tutorial_task = &conditional.params[0];
                crate::models::game::UncivGame::current().settings.tutorial_tasks_completed.contains(tutorial_task)
            },
            UniqueType::ConditionalCivFilter => {
                check_on_civ(|civ| civ.matches_filter(&conditional.params[0], state))
            },
            UniqueType::ConditionalWar => {
                check_on_civ(|civ| civ.is_at_war())
            },
            UniqueType::ConditionalNotWar => {
                check_on_civ(|civ| !civ.is_at_war())
            },
            UniqueType::ConditionalWithResource => {
                state.get_resource_amount(&conditional.params[0]) > 0
            },
            UniqueType::ConditionalWithoutResource => {
                state.get_resource_amount(&conditional.params[0]) <= 0
            },
            UniqueType::ConditionalWhenAboveAmountStatResource => {
                let modify_by_game_speed = unique.map_or(false, |u| u.is_modified_by_game_speed());
                check_resource_or_stat_amount(
                    &conditional.params[1],
                    conditional.params[0].parse::<f32>().unwrap_or(0.0),
                    f32::MAX,
                    modify_by_game_speed,
                    |current, lower_limit, _| current > lower_limit as i32
                )
            },
            UniqueType::ConditionalWhenBelowAmountStatResource => {
                let modify_by_game_speed = unique.map_or(false, |u| u.is_modified_by_game_speed());
                check_resource_or_stat_amount(
                    &conditional.params[1],
                    f32::MIN,
                    conditional.params[0].parse::<f32>().unwrap_or(0.0),
                    modify_by_game_speed,
                    |current, _, upper_limit| current < upper_limit as i32
                )
            },
            UniqueType::ConditionalWhenBetweenStatResource => {
                let modify_by_game_speed = unique.map_or(false, |u| u.is_modified_by_game_speed());
                check_resource_or_stat_amount(
                    &conditional.params[2],
                    conditional.params[0].parse::<f32>().unwrap_or(0.0),
                    conditional.params[1].parse::<f32>().unwrap_or(0.0),
                    modify_by_game_speed,
                    |current, lower_limit, upper_limit| current >= lower_limit as i32 && current <= upper_limit as i32
                )
            },
            UniqueType::ConditionalHappy => {
                check_on_civ(|civ| civ.stats.happiness >= 0)
            },
            UniqueType::ConditionalBetweenHappiness => {
                check_on_civ(|civ| {
                    let min_happiness = conditional.params[0].parse::<i32>().unwrap_or(0);
                    let max_happiness = conditional.params[1].parse::<i32>().unwrap_or(0);
                    (min_happiness..=max_happiness).contains(&civ.stats.happiness)
                })
            },
            UniqueType::ConditionalAboveHappiness => {
                check_on_civ(|civ| {
                    let happiness = conditional.params[0].parse::<i32>().unwrap_or(0);
                    civ.stats.happiness > happiness
                })
            },
            UniqueType::ConditionalBelowHappiness => {
                check_on_civ(|civ| {
                    let happiness = conditional.params[0].parse::<i32>().unwrap_or(0);
                    civ.stats.happiness < happiness
                })
            },
            UniqueType::ConditionalGoldenAge => {
                check_on_civ(|civ| civ.golden_ages.is_golden_age())
            },
            UniqueType::ConditionalNotGoldenAge => {
                check_on_civ(|civ| !civ.golden_ages.is_golden_age())
            },
            UniqueType::ConditionalBeforeEra => {
                compare_era(&conditional.params[0], |current, param| current < param)
            },
            UniqueType::ConditionalStartingFromEra => {
                compare_era(&conditional.params[0], |current, param| current >= param)
            },
            UniqueType::ConditionalDuringEra => {
                compare_era(&conditional.params[0], |current, param| current == param)
            },
            UniqueType::ConditionalIfStartingInEra => {
                check_on_game_info(|gi| gi.game_parameters.starting_era == conditional.params[0])
            },
            UniqueType::ConditionalSpeed => {
                check_on_game_info(|gi| gi.game_parameters.speed == conditional.params[0])
            },
            UniqueType::ConditionalDifficulty => {
                check_on_game_info(|gi| gi.game_parameters.difficulty == conditional.params[0])
            },
            UniqueType::ConditionalVictoryEnabled => {
                check_on_game_info(|gi| gi.game_parameters.victory_types.contains(&conditional.params[0]))
            },
            UniqueType::ConditionalVictoryDisabled => {
                check_on_game_info(|gi| !gi.game_parameters.victory_types.contains(&conditional.params[0]))
            },
            UniqueType::ConditionalReligionEnabled => {
                check_on_game_info(|gi| gi.is_religion_enabled())
            },
            UniqueType::ConditionalReligionDisabled => {
                check_on_game_info(|gi| !gi.is_religion_enabled())
            },
            UniqueType::ConditionalEspionageEnabled => {
                check_on_game_info(|gi| gi.is_espionage_enabled())
            },
            UniqueType::ConditionalEspionageDisabled => {
                check_on_game_info(|gi| !gi.is_espionage_enabled())
            },
            UniqueType::ConditionalNuclearWeaponsEnabled => {
                check_on_game_info(|gi| gi.game_parameters.nuclear_weapons_enabled)
            },
            UniqueType::ConditionalTech => {
                check_on_civ(|civ| {
                    let filter = &conditional.params[0];
                    if civ.game_info.ruleset.technologies.contains_key(filter) {
                        civ.tech.is_researched(filter)
                    } else {
                        civ.tech.researched_technologies.iter().any(|t| t.matches_filter(filter))
                    }
                })
            },
            UniqueType::ConditionalNoTech => {
                check_on_civ(|civ| {
                    let filter = &conditional.params[0];
                    if civ.game_info.ruleset.technologies.contains_key(filter) {
                        !civ.tech.is_researched(filter)
                    } else {
                        !civ.tech.researched_technologies.iter().any(|t| t.matches_filter(filter))
                    }
                })
            },
            UniqueType::ConditionalWhileResearching => {
                check_on_civ(|civ| {
                    civ.tech.current_technology().map_or(false, |t| t.matches_filter(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalAfterPolicyOrBelief => {
                check_on_civ(|civ| {
                    civ.policies.is_adopted(&conditional.params[0]) ||
                    civ.religion_manager.religion.as_ref().map_or(false, |r| r.has_belief(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalBeforePolicyOrBelief => {
                check_on_civ(|civ| {
                    !civ.policies.is_adopted(&conditional.params[0]) &&
                    !civ.religion_manager.religion.as_ref().map_or(false, |r| r.has_belief(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalBeforePantheon => {
                check_on_civ(|civ| civ.religion_manager.religion_state == ReligionState::None)
            },
            UniqueType::ConditionalAfterPantheon => {
                check_on_civ(|civ| civ.religion_manager.religion_state != ReligionState::None)
            },
            UniqueType::ConditionalBeforeReligion => {
                check_on_civ(|civ| civ.religion_manager.religion_state < ReligionState::Religion)
            },
            UniqueType::ConditionalAfterReligion => {
                check_on_civ(|civ| civ.religion_manager.religion_state >= ReligionState::Religion)
            },
            UniqueType::ConditionalBeforeEnhancingReligion => {
                check_on_civ(|civ| civ.religion_manager.religion_state < ReligionState::EnhancedReligion)
            },
            UniqueType::ConditionalAfterEnhancingReligion => {
                check_on_civ(|civ| civ.religion_manager.religion_state >= ReligionState::EnhancedReligion)
            },
            UniqueType::ConditionalAfterGeneratingGreatProphet => {
                check_on_civ(|civ| civ.religion_manager.great_prophets_earned() > 0)
            },
            UniqueType::ConditionalBuildingBuilt => {
                check_on_civ(|civ| {
                    civ.cities.iter().any(|city| city.city_constructions.contains_building_or_equivalent(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalBuildingNotBuilt => {
                check_on_civ(|civ| {
                    !civ.cities.iter().any(|city| city.city_constructions.contains_building_or_equivalent(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalBuildingBuiltAll => {
                check_on_civ(|civ| {
                    let filtered_cities: Vec<_> = civ.cities.iter()
                        .filter(|city| city.matches_filter(&conditional.params[1], state))
                        .collect();

                    filtered_cities.iter().all(|city|
                        city.city_constructions.contains_building_or_equivalent(&conditional.params[0])
                    )
                })
            },
            UniqueType::ConditionalBuildingBuiltAmount => {
                check_on_civ(|civ| {
                    let required_amount = conditional.params[1].parse::<i32>().unwrap_or(0);
                    let count = civ.cities.iter()
                        .filter(|city|
                            city.city_constructions.contains_building_or_equivalent(&conditional.params[0]) &&
                            city.matches_filter(&conditional.params[2], state)
                        )
                        .count();

                    count >= required_amount as usize
                })
            },
            UniqueType::ConditionalBuildingBuiltByAnybody => {
                check_on_game_info(|gi| {
                    gi.get_cities().iter().any(|city|
                        city.city_constructions.contains_building_or_equivalent(&conditional.params[0])
                    )
                })
            },
            UniqueType::ConditionalInThisCity => {
                state.relevant_city.is_some()
            },
            UniqueType::ConditionalCityFilter => {
                check_on_city(|city| city.matches_filter(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalCityConnected => {
                check_on_city(|city| city.is_connected_to_capital())
            },
            UniqueType::ConditionalCityReligion => {
                check_on_city(|city| {
                    city.religion.get_majority_religion()
                        .map_or(false, |r| r.matches_filter(&conditional.params[0], state, state.relevant_civ.as_ref()))
                })
            },
            UniqueType::ConditionalCityNotReligion => {
                check_on_city(|city| {
                    !city.religion.get_majority_religion()
                        .map_or(false, |r| r.matches_filter(&conditional.params[0], state, state.relevant_civ.as_ref()))
                })
            },
            UniqueType::ConditionalCityMajorReligion => {
                check_on_city(|city| {
                    city.religion.get_majority_religion()
                        .map_or(false, |r| r.is_major_religion())
                })
            },
            UniqueType::ConditionalCityEnhancedReligion => {
                check_on_city(|city| {
                    city.religion.get_majority_religion()
                        .map_or(false, |r| r.is_enhanced_religion())
                })
            },
            UniqueType::ConditionalCityThisReligion => {
                check_on_city(|city| {
                    city.religion.get_majority_religion()
                        .map_or(false, |r| Some(r) == state.relevant_civ.as_ref().and_then(|civ| civ.religion_manager.religion.as_ref()))
                })
            },
            UniqueType::ConditionalWLTKD => {
                check_on_city(|city| city.is_we_love_the_king_day_active())
            },
            UniqueType::ConditionalCityWithBuilding => {
                check_on_city(|city| city.city_constructions.contains_building_or_equivalent(&conditional.params[0]))
            },
            UniqueType::ConditionalCityWithoutBuilding => {
                check_on_city(|city| !city.city_constructions.contains_building_or_equivalent(&conditional.params[0]))
            },
            UniqueType::ConditionalPopulationFilter => {
                check_on_city(|city| {
                    let required_amount = conditional.params[0].parse::<i32>().unwrap_or(0);
                    city.population.get_population_filter_amount(&conditional.params[1]) >= required_amount
                })
            },
            UniqueType::ConditionalExactPopulationFilter => {
                check_on_city(|city| {
                    let required_amount = conditional.params[0].parse::<i32>().unwrap_or(0);
                    city.population.get_population_filter_amount(&conditional.params[1]) == required_amount
                })
            },
            UniqueType::ConditionalBetweenPopulationFilter => {
                check_on_city(|city| {
                    let min_amount = conditional.params[0].parse::<i32>().unwrap_or(0);
                    let max_amount = conditional.params[1].parse::<i32>().unwrap_or(0);
                    let amount = city.population.get_population_filter_amount(&conditional.params[2]);
                    (min_amount..=max_amount).contains(&amount)
                })
            },
            UniqueType::ConditionalBelowPopulationFilter => {
                check_on_city(|city| {
                    let required_amount = conditional.params[0].parse::<i32>().unwrap_or(0);
                    city.population.get_population_filter_amount(&conditional.params[1]) < required_amount
                })
            },
            UniqueType::ConditionalWhenGarrisoned => {
                check_on_city(|city| {
                    city.get_center_tile().military_unit.as_ref()
                        .map_or(false, |u| u.can_garrison())
                })
            },
            UniqueType::ConditionalVsCity => {
                state.their_combatant.as_ref()
                    .map_or(false, |c| c.matches_filter("City", false))
            },
            UniqueType::ConditionalVsUnits | UniqueType::ConditionalVsCombatant => {
                state.their_combatant.as_ref()
                    .map_or(false, |c| c.matches_filter(&conditional.params[0]))
            },
            UniqueType::ConditionalOurUnit | UniqueType::ConditionalOurUnitOnUnit => {
                state.relevant_unit.as_ref()
                    .map_or(false, |u| u.matches_filter(&conditional.params[0]))
            },
            UniqueType::ConditionalUnitWithPromotion => {
                state.relevant_unit.as_ref().map_or(false, |u| {
                    u.promotions.promotions.contains(&conditional.params[0]) || u.has_status(&conditional.params[0])
                })
            },
            UniqueType::ConditionalUnitWithoutPromotion => {
                state.relevant_unit.as_ref().map_or(false, |u| {
                    !(u.promotions.promotions.contains(&conditional.params[0]) || u.has_status(&conditional.params[0]))
                })
            },
            UniqueType::ConditionalAttacking => {
                state.combat_action == Some(CombatAction::Attack)
            },
            UniqueType::ConditionalDefending => {
                state.combat_action == Some(CombatAction::Defend)
            },
            UniqueType::ConditionalAboveHP => {
                if let Some(unit) = &state.relevant_unit {
                    let hp = conditional.params[0].parse::<i32>().unwrap_or(0);
                    unit.health > hp
                } else if let Some(combatant) = &state.our_combatant {
                    let hp = conditional.params[0].parse::<i32>().unwrap_or(0);
                    combatant.get_health() > hp
                } else {
                    false
                }
            },
            UniqueType::ConditionalBelowHP => {
                if let Some(unit) = &state.relevant_unit {
                    let hp = conditional.params[0].parse::<i32>().unwrap_or(0);
                    unit.health < hp
                } else if let Some(combatant) = &state.our_combatant {
                    let hp = conditional.params[0].parse::<i32>().unwrap_or(0);
                    combatant.get_health() < hp
                } else {
                    false
                }
            },
            UniqueType::ConditionalHasNotUsedOtherActions => {
                state.unit.as_ref().map_or(true, |u| u.ability_to_times_used.is_empty())
            },
            UniqueType::ConditionalInTiles => {
                state.relevant_tile.as_ref()
                    .map_or(false, |t| t.matches_filter(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalInTilesNot => {
                state.relevant_tile.as_ref()
                    .map_or(false, |t| !t.matches_filter(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalAdjacentTo => {
                state.relevant_tile.as_ref()
                    .map_or(false, |t| t.is_adjacent_to(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalNotAdjacentTo => {
                state.relevant_tile.as_ref()
                    .map_or(false, |t| !t.is_adjacent_to(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalFightingInTiles => {
                state.attacked_tile.as_ref()
                    .map_or(false, |t| t.matches_filter(&conditional.params[0], state.relevant_civ.as_ref()))
            },
            UniqueType::ConditionalNearTiles => {
                if let Some(tile) = &state.relevant_tile {
                    let distance = conditional.params[0].parse::<i32>().unwrap_or(0);
                    tile.get_tiles_in_distance(distance).iter().any(|t| t.matches_filter(&conditional.params[1]))
                } else {
                    false
                }
            },
            UniqueType::ConditionalVsLargerCiv => {
                let your_cities = state.relevant_civ.as_ref().map_or(1, |civ| civ.cities.len());
                let their_cities = state.their_combatant.as_ref()
                    .and_then(|c| c.get_civ_info())
                    .map_or(0, |c| c.cities.len());

                your_cities < their_cities
            },
            UniqueType::ConditionalForeignContinent => {
                check_on_civ(|civ| {
                    if let Some(tile) = &state.relevant_tile {
                        civ.cities.is_empty() || civ.get_capital().is_none() ||
                        civ.get_capital().unwrap().get_center_tile().get_continent() != tile.get_continent()
                    } else {
                        false
                    }
                })
            },
            UniqueType::ConditionalAdjacentUnit => {
                if let (Some(civ), Some(unit), Some(tile)) = (&state.relevant_civ, &state.relevant_unit, &state.relevant_tile) {
                    tile.neighbors.iter().any(|neighbor| {
                        neighbor.get_units().iter().any(|neighbor_unit| {
                            neighbor_unit != unit &&
                            neighbor_unit.civ == *civ &&
                            neighbor_unit.matches_filter(&conditional.params[0])
                        })
                    })
                } else {
                    false
                }
            },
            UniqueType::ConditionalNeighborTiles => {
                if let Some(tile) = &state.relevant_tile {
                    let min_count = conditional.params[0].parse::<i32>().unwrap_or(0);
                    let max_count = conditional.params[1].parse::<i32>().unwrap_or(0);
                    let count = tile.neighbors.iter()
                        .filter(|n| n.matches_filter(&conditional.params[2], state.relevant_civ.as_ref()))
                        .count();

                    (min_count as usize..=max_count as usize).contains(&count)
                } else {
                    false
                }
            },
            UniqueType::ConditionalOnWaterMaps => {
                state.region.as_ref().map_or(false, |r| r.continent_id == -1)
            },
            UniqueType::ConditionalInRegionOfType => {
                state.region.as_ref().map_or(false, |r| r.type_ == conditional.params[0])
            },
            UniqueType::ConditionalInRegionExceptOfType => {
                state.region.as_ref().map_or(false, |r| r.type_ != conditional.params[0])
            },
            UniqueType::ConditionalFirstCivToResearch => {
                if let Some(unique) = unique {
                    if unique.source_object_type == UniqueTarget::Tech {
                        if let Some(source_name) = &unique.source_object_name {
                            check_on_game_info(|gi| {
                                !gi.civilizations.iter().any(|civ| {
                                    civ != state.relevant_civ.as_ref().unwrap() &&
                                    civ.is_major_civ() &&
                                    civ.tech.is_researched(source_name)
                                })
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            UniqueType::ConditionalFirstCivToAdopt => {
                if let Some(unique) = unique {
                    if unique.source_object_type == UniqueTarget::Policy {
                        if let Some(source_name) = &unique.source_object_name {
                            check_on_game_info(|gi| {
                                !gi.civilizations.iter().any(|civ| {
                                    civ != state.relevant_civ.as_ref().unwrap() &&
                                    civ.is_major_civ() &&
                                    civ.policies.is_adopted(source_name)
                                })
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            UniqueType::ConditionalCountableEqualTo => {
                compare_countables(&conditional.params[0], &conditional.params[1], |first, second| first == second)
            },
            UniqueType::ConditionalCountableDifferentThan => {
                compare_countables(&conditional.params[0], &conditional.params[1], |first, second| first != second)
            },
            UniqueType::ConditionalCountableMoreThan => {
                compare_countables(&conditional.params[0], &conditional.params[1], |first, second| first > second)
            },
            UniqueType::ConditionalCountableLessThan => {
                compare_countables(&conditional.params[0], &conditional.params[1], |first, second| first < second)
            },
            UniqueType::ConditionalCountableBetween => {
                compare_countables_three(&conditional.params[0], &conditional.params[1], &conditional.params[2], |first, second, third| (second..=third).contains(&first))
            },
            UniqueType::ConditionalModEnabled => {
                check_on_game_info(|gi| {
                    let filter = &conditional.params[0];
                    let all_mods: Vec<_> = gi.game_parameters.mods.iter()
                        .chain(std::iter::once(&gi.game_parameters.base_ruleset))
                        .collect();

                    all_mods.iter().any(|mod_name| ModCompatibility::mod_name_filter(mod_name, filter))
                })
            },
            UniqueType::ConditionalModNotEnabled => {
                check_on_game_info(|gi| {
                    let filter = &conditional.params[0];
                    let all_mods: Vec<_> = gi.game_parameters.mods.iter()
                        .chain(std::iter::once(&gi.game_parameters.base_ruleset))
                        .collect();

                    !all_mods.iter().any(|mod_name| ModCompatibility::mod_name_filter(mod_name, filter))
                })
            },
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_state_based_random() {
        let state = StateForConditionals::new(None, None, None);
        let unique = None;

        let random1 = Conditionals::get_state_based_random(&state, unique);
        let random2 = Conditionals::get_state_based_random(&state, unique);

        assert!(random1 >= 0.0 && random1 <= 1.0);
        assert!(random2 >= 0.0 && random2 <= 1.0);
        assert_eq!(random1, random2); // Same seed should produce same result
    }
}