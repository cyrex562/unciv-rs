use crate::models::civilization::diplomacy::{DiplomacyFlags, RelationshipLevel};
use crate::models::map::mapunit::MapUnit;
use crate::models::city::City;
use crate::models::UnitActionType;
use crate::models::ruleset::unique::UniqueType;
use crate::automation::Automation;
use crate::automation::ThreatLevel;
use crate::ui::screens::worldscreen::unit::actions::UnitActions;
use crate::constants::AI_PREFER_INQUISITOR_OVER_MISSIONARY_PRESSURE_DIFFERENCE;

/// Contains logic for automating religious unit actions
pub struct ReligiousUnitAutomation;

impl ReligiousUnitAutomation {
    /// Automates missionary movement and actions
    pub fn automate_missionary(unit: &mut MapUnit) {
        if unit.religion != unit.civ.religion_manager.religion.as_ref().map(|r| r.name.clone())
            || unit.religion.is_none() {
            return unit.disband();
        }

        let our_cities_without_religion: Vec<&City> = unit.civ.cities.iter()
            .filter(|city| city.religion.get_majority_religion() != unit.civ.religion_manager.religion)
            .collect();

        /// Returns whether a city is a valid target for spreading religion
        fn is_valid_spread_religion_target(unit: &MapUnit, city: &City) -> bool {
            let diplomacy_manager = unit.civ.get_diplomacy_manager(city.civ);
            if let Some(manager) = diplomacy_manager {
                if manager.has_flag(DiplomacyFlags::AgreedToNotSpreadReligion) {
                    // See NextTurnAutomation - these are the conditions under which AI agrees to religious demands
                    // If they still hold, keep the agreement, otherwise we can renege
                    if manager.relationship_level() == RelationshipLevel::Ally {
                        return false;
                    }
                    if Automation::threat_assessment(unit.civ, city.civ) >= ThreatLevel::High {
                        return false;
                    }
                }
            }
            true
        }

        /// Returns a rank for city priority in religion spreading (lower is better)
        fn rank_city_for_religion_spread(unit: &MapUnit, city: &City) -> i32 {
            let mut rank = city.get_center_tile().aerial_distance_to(unit.get_tile());

            let diplomacy_manager = unit.civ.get_diplomacy_manager(city.civ);
            if let Some(manager) = diplomacy_manager {
                if manager.has_flag(DiplomacyFlags::AgreedToNotSpreadReligion) {
                    rank += 10; // Greatly discourage, but if the other options are too far away we'll take it anyway
                }
            }

            rank
        }

        let city = if !our_cities_without_religion.is_empty() {
            our_cities_without_religion.into_iter()
                .min_by_key(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()))
        } else {
            unit.civ.game_info.get_cities()
                .iter()
                .filter(|city| city.religion.get_majority_religion() != unit.civ.religion_manager.religion)
                .filter(|city| city.civ.knows(unit.civ) && !city.civ.is_at_war_with(unit.civ))
                .filter(|city| !city.religion.is_protected_by_inquisitor(unit.religion.as_ref()))
                .filter(|city| is_valid_spread_religion_target(unit, city))
                .min_by_key(|city| rank_city_for_religion_spread(unit, city))
        };

        let city = match city {
            Some(city) => city,
            None => return,
        };

        let destination = city.get_tiles()
            .iter()
            .filter(|tile| unit.movement.can_move_to(tile) || *tile == unit.get_tile())
            .min_by_key(|tile| tile.aerial_distance_to(unit.get_tile()))
            .filter(|tile| unit.movement.can_reach(tile));

        let destination = match destination {
            Some(tile) => tile,
            None => return,
        };

        unit.movement.head_towards(destination);

        if city.get_tiles().contains(unit.get_tile()) && unit.civ.religion_manager.may_spread_religion_now(unit) {
            UnitActions::invoke_unit_action(unit, UnitActionType::SpreadReligion);
        }
    }

    /// Automates inquisitor movement and actions
    pub fn automate_inquisitor(unit: &mut MapUnit) {
        let civ_religion = unit.civ.religion_manager.religion.as_ref();

        if unit.religion != civ_religion.map(|r| r.name.clone()) || unit.religion.is_none() {
            // No need to keep a unit we can't use, as it only blocks religion spreads of religions other than its own
            return unit.disband();
        }

        let holy_city = unit.civ.religion_manager.get_holy_city();
        let city_to_convert = Self::determine_best_inquisitor_city_to_convert(unit);
        let pressure_deficit = city_to_convert
            .map(|city| city.religion.get_pressure_deficit(civ_religion.map(|r| r.name.as_str())))
            .unwrap_or(0);

        let cities_to_protect = unit.civ.cities.iter()
            .filter(|city| city.religion.get_majority_religion() == civ_religion)
            // We only look at cities that are not currently protected or are protected by us
            .filter(|city| {
                !city.religion.is_protected_by_inquisitor() ||
                unit.get_tile().is_in(city.get_center_tile().get_tiles_in_distance(1))
            });

        // Cities with most populations will be prioritized by the AI
        let city_to_protect = cities_to_protect
            .max_by_key(|city| city.population.population);

        let destination_city = match (city_to_convert, holy_city, city_to_protect) {
            (Some(convert), _, _)
                if (convert == holy_city.as_ref()
                    || pressure_deficit > AI_PREFER_INQUISITOR_OVER_MISSIONARY_PRESSURE_DIFFERENCE
                    || (convert.religion.is_blocked_holy_city
                        && convert.religion.religion_this_is_the_holy_city_of == civ_religion.map(|r| r.name.clone())))
                    && unit.has_unique(UniqueType::CanRemoveHeresy) => Some(convert),
            (_, _, Some(protect)) if unit.has_unique(UniqueType::PreventSpreadingReligion) => {
                if let Some(holy) = holy_city {
                    if !holy.religion.is_protected_by_inquisitor() {
                        Some(holy)
                    } else {
                        Some(protect)
                    }
                } else {
                    Some(protect)
                }
            },
            (Some(convert), _, _) => Some(convert),
            _ => None,
        };

        let destination_city = match destination_city {
            Some(city) => city,
            None => return,
        };

        let destination_tile = destination_city.get_center_tile().neighbors()
            .iter()
            .filter(|tile| unit.movement.can_move_to(tile) || *tile == unit.get_tile())
            .min_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
            .filter(|tile| unit.movement.can_reach(tile));

        let destination_tile = match destination_tile {
            Some(tile) => tile,
            None => return,
        };

        unit.movement.head_towards(destination_tile);

        if city_to_convert.is_some() && unit.get_tile().get_city() == Some(destination_city) {
            UnitActions::invoke_unit_action(unit, UnitActionType::RemoveHeresy);
        }
    }

    /// Determines the best city for an inquisitor to convert
    fn determine_best_inquisitor_city_to_convert(unit: &MapUnit) -> Option<&City> {
        if unit.religion != unit.civ.religion_manager.religion.as_ref().map(|r| r.name.clone())
            || !unit.has_unique(UniqueType::CanRemoveHeresy) {
            return None;
        }

        let holy_city = unit.civ.religion_manager.get_holy_city();
        if let Some(holy_city) = holy_city {
            if holy_city.religion.get_majority_religion() != unit.civ.religion_manager.religion {
                return Some(holy_city);
            }
        }

        let blocked_holy_city = unit.civ.cities.iter()
            .find(|city| {
                city.religion.is_blocked_holy_city &&
                city.religion.religion_this_is_the_holy_city_of == unit.religion
            });

        if let Some(city) = blocked_holy_city {
            return Some(city);
        }

        unit.civ.cities.iter()
            .filter(|city| city.religion.get_majority_religion().is_some())
            .filter(|city| city.religion.get_majority_religion() != unit.civ.religion_manager.religion)
            // Don't go if it takes too long
            .filter(|city| city.get_center_tile().aerial_distance_to(unit.current_tile) <= 20)
            .max_by_key(|city| city.religion.get_pressure_deficit(
                unit.civ.religion_manager.religion.as_ref().map(|r| r.name.as_str())
            ))
    }

    /// Handles founding a religion with a great prophet
    pub fn found_religion(unit: &mut MapUnit) {
        let city_to_found_religion_at = if unit.get_tile().is_city_center()
            && !unit.get_tile().owning_city().unwrap().is_holy_city() {
            unit.get_tile().owning_city()
        } else {
            unit.civ.cities.iter()
                .find(|city| {
                    !city.is_holy_city()
                        && unit.movement.can_move_to(city.get_center_tile())
                        && unit.movement.can_reach(city.get_center_tile())
                })
        };

        let city_to_found_religion_at = match city_to_found_religion_at {
            Some(city) => city,
            None => return,
        };

        if unit.get_tile() != city_to_found_religion_at.get_center_tile() {
            unit.movement.head_towards(city_to_found_religion_at.get_center_tile());
            return;
        }

        UnitActions::invoke_unit_action(unit, UnitActionType::FoundReligion);
    }

    /// Handles enhancing a religion with a great prophet
    pub fn enhance_religion(unit: &mut MapUnit) {
        // Try go to a nearby city
        if !unit.get_tile().is_city_center() {
            UnitAutomation::try_enter_own_closest_city(unit);
        }

        // If we were unable to go there this turn, unable to do anything else
        if !unit.get_tile().is_city_center() {
            return;
        }

        UnitActions::invoke_unit_action(unit, UnitActionType::EnhanceReligion);
    }
}