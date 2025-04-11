use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::civilization::Civilization;
use crate::city::City;
use crate::models::Counter;
use crate::models::ruleset::{Building, INonPerpetualConstruction};
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::models::ruleset::unit::BaseUnit;
use crate::models::stats::Stat;
use crate::utils::{add_to_map_of_sets, contains};

/// Manages construction-related data for a civilization
pub struct CivConstructions {
    /// Reference to the civilization this cache belongs to
    pub civ_info: Arc<Civilization>,

    /// Maps construction names to the amount of times bought
    pub bought_items_with_increasing_price: Counter<String>,

    /// Maps construction names to the amount of times built
    pub built_items_with_increasing_cost: Counter<String>,

    /// Maps cities by id to a set of all free buildings by name they contain.
    /// The building name is the Nation-specific equivalent if available.
    /// Sources: [UniqueType::FreeStatBuildings] **and** [UniqueType::FreeSpecificBuildings]
    /// This is persisted and _never_ cleared or elements removed (per civ and game).
    free_buildings: HashMap<String, HashSet<String>>,

    /// Maps stat names to a set of cities by id that have received a building of that stat.
    /// Source: [UniqueType::FreeStatBuildings]
    /// This is persisted and _never_ cleared or elements removed (per civ and game).
    /// We can't use the Stat enum instead of a string, due to the inability of the JSON-parser
    /// to function properly and forcing this to be an `HashMap<String, HashSet<String>>`
    /// when loading, even if this wasn't the original type, leading to run-time errors.
    free_stat_buildings_provided: HashMap<String, HashSet<String>>,

    /// Maps buildings by name to a set of cities by id that have received that building.
    /// The building name is the Nation-specific equivalent if available.
    /// Source: [UniqueType::FreeSpecificBuildings]
    /// This is persisted and _never_ cleared or elements removed (per civ and game).
    free_specific_buildings_provided: HashMap<String, HashSet<String>>,
}

impl CivConstructions {
    /// Creates a new CivConstructions instance
    pub fn new() -> Self {
        Self {
            civ_info: Arc::new(Civilization::new()),
            bought_items_with_increasing_price: Counter::new(),
            built_items_with_increasing_cost: Counter::new(),
            free_buildings: HashMap::new(),
            free_stat_buildings_provided: HashMap::new(),
            free_specific_buildings_provided: HashMap::new(),
        }
    }

    /// Clones this CivConstructions instance
    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.civ_info = self.civ_info.clone();
        to_return.free_buildings = self.free_buildings.clone();
        to_return.free_stat_buildings_provided = self.free_stat_buildings_provided.clone();
        to_return.free_specific_buildings_provided = self.free_specific_buildings_provided.clone();
        to_return.bought_items_with_increasing_price.add(&self.bought_items_with_increasing_price);  // add copies
        to_return.built_items_with_increasing_cost.add(&self.built_items_with_increasing_cost);
        to_return
    }

    /// Sets transients
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = civ_info;
    }

    /// Starts the turn
    pub fn start_turn(&mut self) {
        self.try_add_free_buildings();
    }

    /// Tries to add free buildings
    pub fn try_add_free_buildings(&mut self) {
        self.add_free_stats_buildings();
        self.add_free_specific_buildings();
        self.add_free_buildings();
    }

    /// Common to [has_free_building] and [get_free_building_names] - 'has' doesn't need the whole set, one enumeration is enough.
    /// Note: Operates on String city.id and String building name, close to the serialized and stored form.
    /// When/if we do a transient cache for these using our objects, please rewrite this.
    fn get_free_building_names_sequence(&self, city_id: &str) -> Vec<String> {
        let mut result = Vec::new();

        if let Some(buildings) = self.free_buildings.get(city_id) {
            result.extend(buildings.iter().cloned());
        }

        for city in &self.civ_info.cities {
            if let Some(buildings) = city.city_constructions.free_buildings_provided_from_this_city.get(city_id) {
                result.extend(buildings.iter().cloned());
            }
        }

        result
    }

    /// Gets a Set of all building names the [city] has for free, from nationwide sources or buildings in other cities
    pub fn get_free_building_names(&self, city: &City) -> HashSet<String> {
        self.get_free_building_names_sequence(&city.id).into_iter().collect()
    }

    /// Tests whether the [city] has [building] for free, from nationwide sources or buildings in other cities
    pub fn has_free_building(&self, city: &City, building: &Building) -> bool {
        self.has_free_building_by_name(&city.id, &building.name)
    }

    /// Tests whether a city by [city_id] has a building named [building_name] for free, from nationwide sources or buildings in other cities
    fn has_free_building_by_name(&self, city_id: &str, building_name: &str) -> bool {
        self.get_free_building_names_sequence(city_id).contains(&building_name.to_string())
    }

    /// Adds a free building to a city
    fn add_free_building(&mut self, city_id: &str, building: &str) {
        add_to_map_of_sets(&mut self.free_buildings, city_id.to_string(), self.civ_info.get_equivalent_building(building).name);
    }

    /// Adds free stat buildings
    fn add_free_stats_buildings(&mut self) {
        let stat_uniques_data: HashMap<Stat, i32> = self.civ_info.get_matching_uniques(UniqueType::FreeStatBuildings)
            .iter()
            .filter(|unique| !unique.has_trigger_conditional())
            .fold(HashMap::new(), |mut acc, unique| {
                let stat = Stat::from_str(&unique.params[0]).unwrap();
                let amount = unique.params[1].parse::<i32>().unwrap();
                *acc.entry(stat).or_insert(0) += amount;
                acc
            });

        for (stat, amount) in stat_uniques_data {
            self.add_free_stat_buildings(stat, amount);
        }
    }

    /// Adds free stat buildings for a specific stat
    pub fn add_free_stat_buildings(&mut self, stat: Stat, amount: i32) {
        for city in self.civ_info.cities.iter().take(amount as usize) {
            if contains(&self.free_stat_buildings_provided, &stat.name, &city.id) {
                continue;
            }

            let building = match city.city_constructions.cheapest_stat_building(stat) {
                Some(building) => building,
                None => continue,
            };

            add_to_map_of_sets(&mut self.free_stat_buildings_provided, stat.name.clone(), city.id.clone());
            self.add_free_building(&city.id, &building.name);
            city.city_constructions.complete_construction(building);
        }
    }

    /// Adds free specific buildings
    fn add_free_specific_buildings(&mut self) {
        let buildings_uniques_data: HashMap<String, i32> = self.civ_info.get_matching_uniques(UniqueType::FreeSpecificBuildings)
            .iter()
            .filter(|unique| !unique.has_trigger_conditional())
            .fold(HashMap::new(), |mut acc, unique| {
                let building_name = unique.params[0].clone();
                let amount = unique.params[1].parse::<i32>().unwrap();
                *acc.entry(building_name).or_insert(0) += amount;
                acc
            });

        for (building_name, amount) in buildings_uniques_data {
            let civ_building_equivalent = self.civ_info.get_equivalent_building(&building_name);
            self.add_free_buildings(civ_building_equivalent, amount);
        }
    }

    /// Adds free buildings
    pub fn add_free_buildings(&mut self, building: &Building, amount: i32) {
        let equivalent_building = self.civ_info.get_equivalent_building(building);
        for city in self.civ_info.cities.iter().take(amount as usize) {
            if contains(&self.free_specific_buildings_provided, &equivalent_building.name, &city.id)
                || city.city_constructions.contains_building_or_equivalent(&building.name) {
                continue;
            }

            add_to_map_of_sets(&mut self.free_specific_buildings_provided, equivalent_building.name.clone(), city.id.clone());
            self.add_free_building(&city.id, &equivalent_building.name);
            city.city_constructions.complete_construction(equivalent_building.clone());
        }
    }

    /// Adds free buildings
    pub fn add_free_buildings_general(&mut self) {
        let auto_granted_buildings: Vec<_> = self.civ_info.game_info.ruleset.buildings.values()
            .iter()
            .filter(|building| building.has_unique(UniqueType::GainBuildingWhereBuildable))
            .cloned()
            .collect();

        // "Gain a free [buildingName] [cityFilter]"
        let free_buildings_from_civ = self.civ_info.get_matching_uniques(UniqueType::GainFreeBuildings, StateForConditionals::IgnoreConditionals);

        for city in &self.civ_info.cities {
            let free_buildings_from_city = city.get_local_matching_uniques(UniqueType::GainFreeBuildings, StateForConditionals::IgnoreConditionals);
            let free_building_uniques: Vec<_> = free_buildings_from_civ.iter()
                .chain(free_buildings_from_city.iter())
                .filter(|unique| city.matches_filter(&unique.params[1]) && unique.conditionals_apply(&city.state)
                    && !unique.has_trigger_conditional())
                .collect();

            for unique in free_building_uniques {
                let free_building = city.civ.get_equivalent_building(&unique.params[0]);
                add_to_map_of_sets(&mut city.city_constructions.free_buildings_provided_from_this_city, city.id.clone(), free_building.name.clone());

                if city.city_constructions.contains_building_or_equivalent(&free_building.name) {
                    continue;
                }
                city.city_constructions.complete_construction(free_building);
            }

            for building in &auto_granted_buildings {
                if building.is_buildable(&city.city_constructions) {
                    city.city_constructions.complete_construction(building.clone());
                }
            }
        }
    }

    /// Calculates a civ-wide total for [object_to_count].
    ///
    /// It counts:
    /// * "Spaceship part" units added to "spaceship" in capital
    /// * Built buildings or those in a construction queue
    /// * Units on the map or being constructed
    pub fn count_constructed_objects(&self, object_to_count: &dyn INonPerpetualConstruction) -> i32 {
        let amount_in_space_ship = self.civ_info.victory_manager.currents_spaceship_parts.get(&object_to_count.name).unwrap_or(&0);

        let mut count = *amount_in_space_ship;

        match object_to_count {
            building if building.is::<Building>() => {
                let building = building.downcast_ref::<Building>().unwrap();
                count += self.civ_info.cities.iter()
                    .filter(|city| {
                        city.city_constructions.contains_building_or_equivalent(&building.name)
                        || city.city_constructions.is_being_constructed_or_enqueued(&building.name)
                    })
                    .count() as i32;
            },
            unit if unit.is::<BaseUnit>() => {
                let unit = unit.downcast_ref::<BaseUnit>().unwrap();
                count += self.civ_info.units.get_civ_units().iter()
                    .filter(|u| u.name == unit.name)
                    .count() as i32;
                count += self.civ_info.cities.iter()
                    .filter(|city| city.city_constructions.is_being_constructed_or_enqueued(&unit.name))
                    .count() as i32;
            },
            _ => {}
        }

        count
    }
}