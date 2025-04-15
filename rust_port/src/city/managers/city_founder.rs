use crate::city::City;
use crate::civilization::Civilization;
use crate::civilization::proximity::Proximity;
use crate::civilization::diplomacy::diplomacy_flags::DiplomacyFlags;
use crate::civilization::managers::religion_state::ReligionState;
use crate::map::mapunit::MapUnit;
use crate::models::ruleset::nation::Nation;
use crate::models::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::models::ruleset::unique::unique_trigger_activation::UniqueTriggerActivation;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::constants::Constants;
use std::sync::Arc;
use nalgebra::Vector2;
use rand::Rng;
use std::collections::{HashSet, HashMap};

/// Manages the founding of new cities
pub struct CityFounder;

impl CityFounder {
    /// Found a new city at the specified location
    pub fn found_city(civ_info: &Arc<Civilization>, city_location: Vector2<i32>, unit: Option<&Arc<MapUnit>>) -> Arc<City> {
        let city = Arc::new(City::new());

        city.founding_civ = civ_info.civ_name.clone();
        city.turn_acquired = civ_info.game_info.turns;
        city.location = city_location;
        city.set_transients(civ_info);

        // Generate a new city name
        let alive_civs: HashSet<Arc<Civilization>> = civ_info.game_info.civilizations.iter()
            .filter(|civ| civ.is_alive())
            .cloned()
            .collect();

        let city_name = Self::generate_new_city_name(civ_info, &alive_civs)
            .unwrap_or_else(|| NamingConstants::fallback.to_string());

        city.name = city_name;

        city.is_original_capital = civ_info.cities_created == 0;
        if city.is_original_capital {
            civ_info.has_ever_owned_original_capital = true;
            // if you have some culture before the 1st city is founded, you may want to adopt the 1st policy
            civ_info.policies.should_open_policy_picker = true;
        }
        civ_info.cities_created += 1;

        // Add the city to the civilization's cities
        let mut cities = civ_info.cities.clone();
        cities.push(city.clone());
        civ_info.cities = cities;

        let starting_era = civ_info.game_info.game_parameters.starting_era.clone();

        city.expansion.reset();

        city.try_update_road_status();

        let tile = city.get_center_tile();
        for terrain_feature in tile.terrain_features.iter()
            .filter(|feature| city.get_ruleset().tile_improvements.contains_key(&format!("Remove {}", feature))) {
            tile.remove_terrain_feature(terrain_feature);
        }

        if civ_info.game_info.ruleset.tile_improvements.contains_key(&Constants::city_center) {
            tile.set_improvement(&Constants::city_center, civ_info);
        }
        tile.stop_working_on_improvement();

        let ruleset = &civ_info.game_info.ruleset;
        city.worked_tiles = HashSet::new(); // reassign 1st working tile

        city.population.set_population(ruleset.eras.get(&starting_era).unwrap().settler_population);

        if civ_info.religion_manager.religion_state == ReligionState::Pantheon {
            if let Some(religion) = &civ_info.religion_manager.religion {
                city.religion.add_pressure(
                    &religion.name,
                    200 * city.population.population
                );
            }
        }

        city.population.auto_assign_population();

        // Update proximity rankings for all civs
        for other_civ in civ_info.game_info.get_alive_major_civs() {
            if civ_info.get_proximity(&other_civ) != Proximity::Neighbors { // unless already neighbors
                let other_proximity = other_civ.cache.update_proximity(civ_info);
                civ_info.cache.update_proximity(&other_civ, other_proximity);
            }
        }
        for other_civ in civ_info.game_info.get_alive_city_states() {
            if civ_info.get_proximity(&other_civ) != Proximity::Neighbors { // unless already neighbors
                let other_proximity = other_civ.cache.update_proximity(civ_info);
                civ_info.cache.update_proximity(&other_civ, other_proximity);
            }
        }

        Self::trigger_cities_settled_near_other_civ(&city);
        civ_info.game_info.city_distances.set_dirty();

        Self::add_starting_buildings(&city, civ_info, &starting_era);

        // Trigger uniques for the civilization
        for unique in civ_info.get_triggered_uniques(
            UniqueType::TriggerUponFoundingCity,
            StateForConditionals::new(civ_info, &city, unit)
        ) {
            UniqueTriggerActivation::trigger_unique(
                unique,
                civ_info,
                &city,
                unit,
                "due to founding a city"
            );
        }

        // Trigger uniques for the unit if it exists
        if let Some(unit) = unit {
            for unique in unit.get_triggered_uniques(
                UniqueType::TriggerUponFoundingCity,
                StateForConditionals::new(civ_info, &city, Some(unit))
            ) {
                UniqueTriggerActivation::trigger_unique(
                    unique,
                    civ_info,
                    &city,
                    Some(unit),
                    "due to founding a city"
                );
            }
        }

        city
    }

    /// Constants for city naming
    struct NamingConstants {
        /// Prefixes to add when every base name is taken, ordered
        prefixes: Vec<String>,
        /// Fallback name if no other name can be generated
        fallback: String,
    }

    impl NamingConstants {
        /// Get the prefixes for city naming
        fn prefixes() -> Vec<String> {
            vec!["New".to_string(), "Neo".to_string(), "Nova".to_string(), "Altera".to_string()]
        }

        /// Get the fallback city name
        fn fallback() -> String {
            "City Without A Name".to_string()
        }
    }

    /// Generates and returns a new city name for the founding civilization
    ///
    /// This method attempts to return the first unused city name of the founding civilization,
    /// taking used city names into consideration (including foreign cities). If that fails, it then checks
    /// whether the civilization has BorrowsCityNames unique and, if true, returns a borrowed name.
    /// Else, it repeatedly attaches one of the given prefixes to the list of names
    /// up to ten times until an unused name is successfully generated. If all else fails, None is returned.
    fn generate_new_city_name(
        founding_civ: &Arc<Civilization>,
        alive_civs: &HashSet<Arc<Civilization>>
    ) -> Option<String> {
        // Collect all used city names
        let used_city_names: HashSet<String> = alive_civs.iter()
            .flat_map(|civilization| civilization.cities.iter().map(|city| city.name.clone()))
            .collect();

        // Attempt to return the first missing name from the list of city names
        for city_name in founding_civ.nation.cities.iter() {
            if !used_city_names.contains(city_name) {
                return Some(city_name.clone());
            }
        }

        // If all names are taken and this nation borrows city names,
        // return a random borrowed city name
        if founding_civ.has_unique(UniqueType::BorrowsCityNames) {
            return Self::borrow_city_name(founding_civ, alive_civs, &used_city_names);
        }

        // If the nation doesn't have the unique above,
        // return the first missing name with an increasing number of prefixes attached
        for number in 1..=10 {
            for prefix in NamingConstants::prefixes() {
                let repeated_prefix = format!("{} [", prefix).repeat(number);
                let suffix = "]".repeat(number);

                for base_name in founding_civ.nation.cities.iter() {
                    let candidate = format!("{}{}{}", repeated_prefix, base_name, suffix);
                    if !used_city_names.contains(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }

        // If all else fails (by using some sort of rule set mod without city names),
        None
    }

    /// Borrows a city name from another major civilization
    fn borrow_city_name(
        founding_civ: &Arc<Civilization>,
        alive_civs: &HashSet<Arc<Civilization>>,
        used_city_names: &HashSet<String>
    ) -> Option<String> {
        // Get alive major nations
        let alive_major_nations: Vec<&Nation> = alive_civs.iter()
            .filter(|civ| civ.is_major_civ())
            .map(|civ| &civ.nation)
            .collect();

        // Get other major nations (excluding the founding civ's nation)
        let other_major_nations: Vec<&Nation> = alive_major_nations.iter()
            .filter(|nation| **nation != founding_civ.nation)
            .cloned()
            .collect();

        // Get the last unused city name for each other major nation
        let mut new_city_names: HashSet<String> = other_major_nations.iter()
            .filter_map(|nation| {
                nation.cities.iter()
                    .rev()
                    .find(|city| !used_city_names.contains(*city))
                    .cloned()
            })
            .collect();

        // If we found some names, return a random one
        if !new_city_names.is_empty() {
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..new_city_names.len());
            return Some(new_city_names.into_iter().nth(index).unwrap());
        }

        // As per fandom wiki, once the names from the other nations in the game are exhausted,
        // names are taken from the rest of the major nations in the rule set
        let absent_major_nations: Vec<&Nation> = founding_civ.game_info.ruleset.nations.values()
            .filter(|nation| nation.is_major_civ && !alive_major_nations.contains(&nation))
            .collect();

        new_city_names = absent_major_nations.iter()
            .flat_map(|nation| {
                nation.cities.iter()
                    .filter(|city| !used_city_names.contains(*city))
                    .cloned()
            })
            .collect();

        // If we found some names, return a random one
        if !new_city_names.is_empty() {
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..new_city_names.len());
            return Some(new_city_names.into_iter().nth(index).unwrap());
        }

        // If for some reason we have used every single city name in the game,
        // (are we using some sort of rule set mod without city names?)
        None
    }

    /// Adds starting buildings to a newly founded city
    fn add_starting_buildings(city: &Arc<City>, civ_info: &Arc<Civilization>, starting_era: &str) {
        let ruleset = &civ_info.game_info.ruleset;

        // Add capital city indicator if this is the first city
        if civ_info.cities.len() == 1 {
            if let Some(capital_city_indicator) = civ_info.capital_city_indicator(city) {
                city.city_constructions.add_building(capital_city_indicator, false);
            }
        }

        // Add buildings and pop we get from starting in this era
        if let Some(era) = ruleset.eras.get(starting_era) {
            for building_name in era.settler_buildings.iter() {
                if let Some(building) = ruleset.buildings.get(building_name) {
                    let unique_building = civ_info.get_equivalent_building(building);
                    if unique_building.is_buildable(&city.city_constructions) {
                        city.city_constructions.add_building(unique_building, false);
                    }
                }
            }
        }

        civ_info.civ_constructions.try_add_free_buildings();
    }

    /// Triggers events when a city is settled near another civilization
    ///
    /// When someone settles a city within 6 tiles of another civ, this makes the AI unhappy and it starts a rolling event.
    /// The SettledCitiesNearUs flag gets added to the AI so it knows this happened,
    /// and on its turn it asks the player to stop (with a DemandToStopSettlingCitiesNear alert type)
    /// If the player says "whatever, I'm not promising to stop", they get a -10 modifier which gradually disappears in 40 turns
    /// If they DO agree, then if they keep their promise for ~100 turns they get a +10 modifier for keeping the promise,
    /// But if they don't keep their promise they get a -20 that will only fully disappear in 160 turns.
    fn trigger_cities_settled_near_other_civ(city: &Arc<City>) {
        // Find cities within 6 tiles
        let cities_within_6_tiles: Vec<Arc<City>> = city.civ.game_info.civilizations.iter()
            .filter(|it| it.is_major_civ() && **it != city.civ)
            .flat_map(|it| it.cities.iter().cloned())
            .filter(|it| it.get_center_tile().aerial_distance_to(&city.get_center_tile()) <= 6)
            .collect();

        // Get the civilizations that own these cities
        let civs_with_close_cities: Vec<Arc<Civilization>> = cities_within_6_tiles.iter()
            .map(|it| it.civ.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|it| it.knows(&city.civ) && it.has_explored(&city.get_center_tile()))
            .collect();

        // Set the flag for each civilization
        for other_civ in civs_with_close_cities {
            if let Some(diplomacy_manager) = other_civ.get_diplomacy_manager(&city.civ) {
                diplomacy_manager.set_flag(DiplomacyFlags::SettledCitiesNearUs, 30);
            }
        }
    }
}