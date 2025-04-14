use std::str::FromStr;

use crate::models::ruleset::StateForConditionals;
use crate::models::stats::Stat;
use crate::models::translations::{equals_placeholder_text, get_placeholder_parameters};

/// A module for handling countable values in the game
pub struct Countables;

impl Countables {
    /// Gets the amount of a countable value
    pub fn get_countable_amount(countable: &str, state_for_conditionals: &StateForConditionals) -> Option<i32> {
        // Check if the countable is a direct integer
        if let Ok(value) = i32::from_str(countable) {
            return Some(value);
        }

        // Check if the countable is a stat
        if let Some(relevant_stat) = Stat::safe_value_of(countable) {
            return Some(state_for_conditionals.get_stat_amount(relevant_stat));
        }

        // Get game info or return None
        let game_info = state_for_conditionals.game_info.as_ref()?;

        // Check for special countables
        if countable == "turns" {
            return Some(game_info.turns);
        }
        if countable == "year" {
            return Some(game_info.get_year(game_info.turns));
        }

        // Get civ info or return None
        let civ_info = state_for_conditionals.relevant_civ.as_ref()?;

        // Check for city-related countables
        if countable == "Cities" {
            return Some(civ_info.cities.len() as i32);
        }

        // Check for filtered city countables
        let placeholder_parameters = get_placeholder_parameters(countable);
        if equals_placeholder_text(countable, "[] Cities") {
            return Some(civ_info.cities.iter()
                .filter(|city| city.matches_filter(&placeholder_parameters[0]))
                .count() as i32);
        }

        // Check for unit-related countables
        if countable == "Units" {
            return Some(civ_info.units.get_civ_units_size() as i32);
        }
        if equals_placeholder_text(countable, "[] Units") {
            return Some(civ_info.units.get_civ_units().iter()
                .filter(|unit| unit.matches_filter(&placeholder_parameters[0]))
                .count() as i32);
        }

        // Check for building-related countables
        if equals_placeholder_text(countable, "[] Buildings") {
            return Some(civ_info.cities.iter()
                .map(|city| city.city_constructions.get_built_buildings().iter()
                    .filter(|building| building.matches_filter(&placeholder_parameters[0]))
                    .count())
                .sum::<usize>() as i32);
        }

        // Check for civilization-related countables
        if equals_placeholder_text(countable, "Remaining [] Civilizations") {
            return Some(game_info.civilizations.iter()
                .filter(|civ| !civ.is_defeated())
                .filter(|civ| civ.matches_filter(&placeholder_parameters[0]))
                .count() as i32);
        }

        // Check for tile-related countables
        if equals_placeholder_text(countable, "Owned [] Tiles") {
            return Some(civ_info.cities.iter()
                .map(|city| city.get_tiles().iter()
                    .filter(|tile| tile.matches_filter(&placeholder_parameters[0]))
                    .count())
                .sum::<usize>() as i32);
        }

        // Check if the countable is a resource
        if game_info.ruleset.tile_resources.contains_key(countable) {
            return Some(state_for_conditionals.get_resource_amount(countable));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_countable_amount_direct_integer() {
        let state = StateForConditionals::new(None, None, None);
        assert_eq!(Countables::get_countable_amount("42", &state), Some(42));
    }

    #[test]
    fn test_get_countable_amount_invalid() {
        let state = StateForConditionals::new(None, None, None);
        assert_eq!(Countables::get_countable_amount("invalid", &state), None);
    }
}