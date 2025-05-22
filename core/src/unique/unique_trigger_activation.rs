use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use rand::Rng;
use crate::models::{
    civilization::{Civilization, CivFlags, DiplomacyFlags, DiplomaticModifiers, NotificationAction, NotificationCategory, NotificationIcon, PopupAlert, AlertType},
    city::City,
    map::{MapUnit, Tile, TileNormalizer},
    ruleset::{
        unique::{Unique, UniqueType, StateForConditionals, TemporaryUnique},
        tech::Technology,
        tile::{TerrainType, TileResource},
        unit::UnitPromotions,
        policy::Policy,
        belief::BeliefType,
        event::Event,
    },
    stats::{Stat, Stats},
    translations::{fill_placeholders, has_placeholder_parameters},
    constants::CONSTANTS,
};
use crate::utils::{add_to_map_of_sets, random_weighted};
use crate::game::UncivGame;
use crate::automation::NextTurnAutomation;
use crate::map_generator::{NaturalWonderGenerator, RiverGenerator};

/// Handles the activation of unique abilities in the game
pub struct UniqueTriggerActivation;

impl UniqueTriggerActivation {
    /// Triggers a unique ability for a city
    pub fn trigger_unique_for_city(
        unique: &Unique,
        city: &City,
        notification: Option<&str>,
        trigger_notification_text: Option<&str>,
    ) -> bool {
        Self::trigger_unique(
            unique,
            &city.civ,
            Some(city),
            None,
            Some(city.get_center_tile()),
            notification,
            trigger_notification_text,
        )
    }

    /// Triggers a unique ability for a unit
    pub fn trigger_unique_for_unit(
        unique: &Unique,
        unit: &MapUnit,
        notification: Option<&str>,
        trigger_notification_text: Option<&str>,
    ) -> bool {
        Self::trigger_unique(
            unique,
            &unit.civ,
            None,
            Some(unit),
            Some(unit.current_tile.clone()),
            notification,
            trigger_notification_text,
        )
    }

    /// Triggers a unique ability with the given parameters
    pub fn trigger_unique(
        unique: &Unique,
        civ_info: &Civilization,
        city: Option<&City>,
        unit: Option<&MapUnit>,
        tile: Option<&Tile>,
        notification: Option<&str>,
        trigger_notification_text: Option<&str>,
    ) -> bool {
        if let Some(function) = Self::get_trigger_function(
            unique,
            civ_info,
            city,
            unit,
            tile,
            notification,
            trigger_notification_text,
        ) {
            function()
        } else {
            false
        }
    }

    /// Gets the trigger function for a unique ability
    pub fn get_trigger_function(
        unique: &Unique,
        civ_info: &Civilization,
        city: Option<&City>,
        unit: Option<&MapUnit>,
        tile: Option<&Tile>,
        notification: Option<&str>,
        trigger_notification_text: Option<&str>,
    ) -> Option<Box<dyn Fn() -> bool>> {
        let relevant_city = city.or_else(|| tile.and_then(|t| t.get_city()));
        let timing_conditional = unique.get_modifiers(UniqueType::ConditionalTimedUnique).first();

        if let Some(conditional) = timing_conditional {
            return Some(Box::new(move || {
                civ_info.temporary_uniques.add(TemporaryUnique::new(unique.clone(), conditional.params[0].parse().unwrap()));
                if unique.type_ == UniqueType::ProvidesResources || unique.type_ == UniqueType::ConsumesResources {
                    civ_info.cache.update_civ_resources();
                }
                true
            }));
        }

        let state_for_conditionals = StateForConditionals::new(civ_info, city, unit, tile);
        if !unique.conditionals_apply(&state_for_conditionals) {
            return None;
        }

        let chosen_city = relevant_city.unwrap_or_else(|| {
            civ_info.cities.iter().find(|c| c.is_capital()).unwrap()
        });

        let tile_based_random = if let Some(t) = tile {
            rand::thread_rng().seed_from_u64(t.position.to_string().hash() as u64)
        } else {
            rand::thread_rng().seed_from_u64(550)
        };

        let ruleset = &civ_info.game_info.ruleset;

        match unique.type_ {
            UniqueType::TriggerEvent => {
                let event = ruleset.events.get(&unique.params[0])?;
                let choices = event.get_matching_choices(&state_for_conditionals)?;

                if civ_info.is_ai() || event.presentation == Event::Presentation::None {
                    return Some(Box::new(move || {
                        let choice = random_weighted(&choices, |c| c.get_weight_for_ai_decision(&state_for_conditionals));
                        choice.trigger_choice(civ_info, unit);
                        true
                    }));
                }

                if event.presentation == Event::Presentation::Alert {
                    return Some(Box::new(move || {
                        let mut event_text = event.name.clone();
                        if let Some(u) = unit {
                            event_text.push_str(&format!("{}unitId={}", CONSTANTS.string_split_character, u.id));
                        }
                        civ_info.popup_alerts.add(PopupAlert::new(AlertType::Event, event_text));
                        true
                    }));
                }

                panic!("Event {} has presentation type {:?} which is not implemented for use via TriggerEvent",
                    event.name, event.presentation);
            }

            UniqueType::MarkTutorialComplete => {
                return Some(Box::new(move || {
                    UncivGame::current().settings.add_completed_tutorial_task(&unique.params[0]);
                    true
                }));
            }

            UniqueType::OneTimeFreeUnit => {
                let unit_name = &unique.params[0];
                let base_unit = ruleset.units.get(unit_name)?;
                let civ_unit = civ_info.get_equivalent_unit(base_unit);

                if civ_unit.is_city_founder() && civ_info.is_one_city_challenger() {
                    return None;
                }

                let limit = civ_unit.get_matching_uniques(UniqueType::MaxNumberBuildable)
                    .iter()
                    .map(|u| u.params[0].parse::<i32>().unwrap())
                    .min();

                if let Some(l) = limit {
                    if l <= civ_info.units.get_civ_units().filter(|u| u.name == civ_unit.name).count() as i32 {
                        return None;
                    }
                }

                return Some(Box::new(move || {
                    let placed_unit = if relevant_city.is_some() || (tile.is_none() && !civ_info.cities.is_empty()) {
                        civ_info.units.add_unit(&civ_unit, chosen_city)
                    } else if let Some(t) = tile {
                        civ_info.units.place_unit_near_tile(t.position, &civ_unit)
                    } else if let Some(first_unit) = civ_info.units.get_civ_units().next() {
                        civ_info.units.place_unit_near_tile(first_unit.current_tile.position, &civ_unit)
                    } else {
                        return false;
                    }?;

                    let notification_text = Self::get_notification_text(
                        notification,
                        trigger_notification_text,
                        &format!("Gained [1] [{}] unit(s)", civ_unit.name),
                    );

                    if let Some(text) = notification_text {
                        civ_info.add_notification(
                            &text,
                            NotificationAction::MapUnit(placed_unit.clone()),
                            NotificationCategory::Units,
                            &placed_unit.name,
                        );
                    }

                    true
                }));
            }

            // ... Additional unique type handlers would go here ...
            // The pattern continues for all other unique types

            _ => None,
        }
    }

    /// Gets the notification text for a unique trigger
    fn get_notification_text(
        notification: Option<&str>,
        trigger_notification_text: Option<&str>,
        effect_notification_text: &str,
    ) -> Option<String> {
        if let Some(n) = notification {
            Some(n.to_string())
        } else if let Some(t) = trigger_notification_text {
            if UncivGame::current().translations.trigger_notification_effect_before_cause(
                UncivGame::current().settings.language,
            ) {
                Some(format!("{}{}{}", effect_notification_text, " ", t))
            } else {
                Some(format!("{}{}{}", t, " ", effect_notification_text))
            }
        } else {
            None
        }
    }

    /// Gets the trigger function for changing a river
    fn get_one_time_change_river_trigger_function(tile: &Tile) -> Option<Box<dyn Fn() -> bool>> {
        if tile.neighbors.iter().none(|n| n.is_land && !tile.is_connected_by_river(n)) {
            return None;
        }
        Some(Box::new(move || RiverGenerator::continue_river_on(tile)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_unique_for_city() {
        // Add test implementation
    }

    #[test]
    fn test_trigger_unique_for_unit() {
        // Add test implementation
    }

    #[test]
    fn test_get_notification_text() {
        // Add test implementation
    }
}