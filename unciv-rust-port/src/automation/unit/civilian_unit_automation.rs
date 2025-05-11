use crate::automation::unit::SpecificUnitAutomation;
use crate::models::civilization::Civilization;
use crate::models::civilization::managers::ReligionState;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::UnitActionType;
use crate::models::ruleset::unique::UniqueTriggerActivation;
use crate::models::ruleset::unique::UniqueType;
use crate::ui::screens::worldscreen::unit::actions::UnitActionModifiers;
use crate::ui::screens::worldscreen::unit::actions::UnitActions;
use std::collections::HashSet;

/// Contains logic for automating civilian units
pub struct CivilianUnitAutomation;

impl CivilianUnitAutomation {
    /// Checks if a unit should clear a tile for units with AddInCapital unique
    pub fn should_clear_tile_for_add_in_capital_units(unit: &MapUnit, tile: &Tile) -> bool {
        tile.is_city_center() &&
        tile.get_city().map_or(false, |city| city.is_capital()) &&
        !unit.has_unique(UniqueType::AddInCapital) &&
        unit.civ.units.get_civ_units().iter().any(|u| u.has_unique(UniqueType::AddInCapital))
    }

    /// Automates a civilian unit's actions
    pub fn automate_civilian_unit(unit: &mut MapUnit, dangerous_tiles: &HashSet<Tile>) {
        // To allow "found city" actions that can only trigger a limited number of times
        let settler_unique =
            UnitActionModifiers::get_usable_unit_action_uniques(unit, UniqueType::FoundCity).first()
                .or_else(|| UnitActionModifiers::get_usable_unit_action_uniques(unit, UniqueType::FoundPuppetCity).first());

        if let Some(_) = settler_unique {
            return SpecificUnitAutomation::automate_settler_actions(unit, dangerous_tiles);
        }

        if Self::try_run_away_if_necessary(unit) {
            return;
        }

        if Self::should_clear_tile_for_add_in_capital_units(unit, &unit.current_tile) {
            // First off get out of the way, then decide if you actually want to do something else
            let tiles_can_move_to: Vec<_> = unit.movement.get_distance_to_tiles()
                .iter()
                .filter(|(tile, _)| unit.movement.can_move_to(tile))
                .collect();

            if !tiles_can_move_to.is_empty() {
                let min_movement_tile = tiles_can_move_to.iter()
                    .min_by(|(_, a), (_, b)| a.total_movement.partial_cmp(&b.total_movement).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(tile, _)| tile);

                if let Some(tile) = min_movement_tile {
                    unit.movement.move_to_tile(tile);
                }
            }
        }

        if unit.is_automating_road_connection() {
            return unit.civ.get_worker_automation().road_to_automation.automate_connect_road(unit, dangerous_tiles);
        }

        if unit.cache.has_unique_to_build_improvements {
            return unit.civ.get_worker_automation().automate_worker_action(unit, dangerous_tiles);
        }

        if unit.cache.has_unique_to_create_water_improvements {
            if !unit.civ.get_worker_automation().automate_work_boats(unit) {
                UnitAutomation::try_explore(unit);
            }
            return;
        }

        if unit.has_unique(UniqueType::MayFoundReligion)
            && unit.civ.religion_manager.religion_state < ReligionState::Religion
            && unit.civ.religion_manager.may_found_religion_at_all()
        {
            return ReligiousUnitAutomation::found_religion(unit);
        }

        if unit.has_unique(UniqueType::MayEnhanceReligion)
            && unit.civ.religion_manager.religion_state < ReligionState::EnhancedReligion
            && unit.civ.religion_manager.may_enhance_religion_at_all()
        {
            return ReligiousUnitAutomation::enhance_religion(unit);
        }

        // We try to add any unit in the capital we can, though that might not always be desirable
        // For now its a simple option to allow AI to win a science victory again
        if unit.has_unique(UniqueType::AddInCapital) {
            return SpecificUnitAutomation::automate_add_in_capital(unit);
        }

        // This now supports "Great General"-like mod units not combining 'aura' and citadel
        // abilities, but not additional capabilities if automation finds no use for those two
        if unit.cache.has_strength_bonus_in_radius_unique
            && SpecificUnitAutomation::automate_great_general(unit)
        {
            return;
        }

        if unit.cache.has_citadel_placement_unique && SpecificUnitAutomation::automate_citadel_placer(unit) {
            return;
        }

        if unit.cache.has_citadel_placement_unique || unit.cache.has_strength_bonus_in_radius_unique {
            return SpecificUnitAutomation::automate_great_general_fallback(unit);
        }

        if unit.civ.religion_manager.may_spread_religion_at_all(unit) {
            return ReligiousUnitAutomation::automate_missionary(unit);
        }

        if unit.has_unique(UniqueType::PreventSpreadingReligion) || unit.has_unique(UniqueType::CanRemoveHeresy) {
            return ReligiousUnitAutomation::automate_inquisitor(unit);
        }

        let is_late_game = Self::is_late_game(&unit.civ);
        // Great scientist -> Hurry research if late game
        // Great writer -> Hurry policy if late game
        if is_late_game {
            let hurried_research = UnitActions::invoke_unit_action(unit, UnitActionType::HurryResearch);
            if hurried_research {
                return;
            }

            let hurried_policy = UnitActions::invoke_unit_action(unit, UnitActionType::HurryPolicy);
            if hurried_policy {
                return;
            }
            // TODO: save up great scientists/writers for late game (8 turns after research labs/broadcast towers resp.)
        }

        // Great merchant -> Conduct trade mission if late game and if not at war.
        // TODO: This could be more complex to walk to the city state that is most beneficial to
        //  also have more influence.
        if unit.has_unique(UniqueType::CanTradeWithCityStateForGoldAndInfluence)
            // Don't wander around with the great merchant when at war. Barbs might also be a
            // problem, but hopefully by the time we have a great merchant, they're under control.
            && !unit.civ.is_at_war()
            && is_late_game
        {
            let trade_mission_can_be_conducted_eventually =
                SpecificUnitAutomation::conduct_trade_mission(unit);
            if trade_mission_can_be_conducted_eventually {
                return;
            }
        }

        // Great engineer -> Try to speed up wonder construction
        if unit.has_unique(UniqueType::CanSpeedupConstruction)
                || unit.has_unique(UniqueType::CanSpeedupWonderConstruction)
        {
            let wonder_can_be_sped_up_eventually = SpecificUnitAutomation::speedup_wonder_construction(unit);
            if wonder_can_be_sped_up_eventually {
                return;
            }
        }

        if unit.has_unique(UniqueType::GainFreeBuildings) {
            if let Some(unique) = unit.get_matching_uniques(UniqueType::GainFreeBuildings).first() {
                let building_name = &unique.params[0];
                // Choose the city that is closest in distance and does not have the building constructed.
                let city_to_gain_building = unit.civ.cities.iter()
                    .filter(|city| {
                        !city.city_constructions.contains_building_or_equivalent(building_name)
                            && (unit.movement.can_move_to(city.get_center_tile()) || unit.current_tile == city.get_center_tile())
                    })
                    .min_by(|city1, city2| {
                        let path1 = unit.movement.get_shortest_path(city1.get_center_tile());
                        let path2 = unit.movement.get_shortest_path(city2.get_center_tile());
                        path1.len().cmp(&path2.len())
                    });

                if let Some(city) = city_to_gain_building {
                    if unit.current_tile == city.get_center_tile() {
                        UniqueTriggerActivation::trigger_unique(unique, &unit.civ, Some(unit), Some(&unit.current_tile));
                        UnitActionModifiers::activate_side_effects(unit, unique);
                        return;
                    } else {
                        unit.movement.head_towards(city.get_center_tile());
                    }
                }
            }
            return;
        }

        // TODO: The AI tends to have a lot of great generals. Maybe there should be a cutoff
        //  (depending on number of cities) and after that they should just be used to start golden
        //  ages?

        if SpecificUnitAutomation::automate_improvement_placer(unit) {
            return;
        }

        let golden_age_action = UnitActions::get_unit_actions(unit, UnitActionType::TriggerUnique)
            .iter()
            .filter(|action| {
                action.action.is_some() &&
                action.associated_unique.as_ref().map_or(false, |unique| {
                    matches!(unique.type_,
                        UniqueType::OneTimeEnterGoldenAge |
                        UniqueType::OneTimeEnterGoldenAgeTurns
                    )
                })
            })
            .next();

        if let Some(action) = golden_age_action {
            if let Some(action_fn) = &action.action {
                action_fn();
                return;
            }
        }

        // The AI doesn't know how to handle unknown civilian units
    }

    /// Checks if the game is in the late game phase
    fn is_late_game(civ: &Civilization) -> bool {
        let research_complete_percent =
            (civ.tech.researched_technologies.len() as f32) / civ.game_info.ruleset.technologies.len() as f32;
        research_complete_percent >= 0.6
    }

    /// Returns whether the civilian spends its turn hiding and not moving
    pub fn try_run_away_if_necessary(unit: &mut MapUnit) -> bool {
        // This is a little 'Bugblatter Beast of Traal': Run if we can attack an enemy
        // Cheaper than determining which enemies could attack us next turn
        let enemy_units_in_walking_distance: Vec<_> = unit.movement.get_distance_to_tiles().keys()
            .filter(|tile| unit.civ.threat_manager.does_tile_have_military_enemy(tile))
            .collect();

        if !enemy_units_in_walking_distance.is_empty() && !unit.base_unit.is_military
            && unit.get_tile().military_unit.is_none() && !unit.get_tile().is_city_center()
        {
            Self::run_away(unit);
            return true;
        }

        false
    }

    /// Makes the unit run away from danger
    fn run_away(unit: &mut MapUnit) {
        let reachable_tiles = unit.movement.get_distance_to_tiles();

        // Try to find an enterable city
        let enterable_city = reachable_tiles.keys()
            .find(|tile| tile.is_city_center() && unit.movement.can_move_to(tile));

        if let Some(city) = enterable_city {
            unit.movement.move_to_tile(city);
            return;
        }

        // Try to find a defensive unit
        let defensive_unit = reachable_tiles.keys()
            .find(|tile| {
                tile.military_unit.is_some() &&
                tile.military_unit.as_ref().map_or(false, |u| u.civ == unit.civ) &&
                tile.civilian_unit.is_none()
            });

        if let Some(defensive_tile) = defensive_unit {
            unit.movement.move_to_tile(defensive_tile);
            return;
        }

        // Find the tile furthest from enemy
        let tile_furthest_from_enemy = reachable_tiles.keys()
            .filter(|tile| unit.movement.can_move_to(tile) && unit.get_damage_from_terrain(tile) < unit.health)
            .max_by(|tile1, tile2| {
                let dist1 = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(&unit.get_tile(), 4, false);
                let dist2 = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(&unit.get_tile(), 4, false);
                dist1.partial_cmp(&dist2).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(tile) = tile_furthest_from_enemy {
            unit.movement.move_to_tile(tile);
        }
        // can't move anywhere!
    }
}