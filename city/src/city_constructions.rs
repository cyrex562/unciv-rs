use std::collections::{HashMap, HashSet};

use crate::city::City;
use unit::BaseUnit;

/// City constructions manager
pub struct CityConstructions<'a> {
    /// The city this manager belongs to
    pub city: Option<&'a City>,

    /// Built building objects
    built_building_objects: Vec<Building>,

    /// Built building unique map
    pub built_building_unique_map: UniqueMap,

    /// Built buildings
    pub built_buildings: HashSet<String>,

    /// In progress constructions
    pub in_progress_constructions: HashMap<String, i32>,

    /// Whether current construction is user set
    pub current_construction_is_user_set: bool,

    /// Construction queue
    pub construction_queue: Vec<String>,

    /// Production overflow
    pub production_overflow: i32,

    /// Maximum queue size
    queue_max_size: i32,

    /// Free buildings provided from this city
    pub free_buildings_provided_from_this_city: HashMap<String, HashSet<String>>,
}

impl<'a> CityConstructions<'a> {
    /// Creates a new CityConstructions
    pub fn new() -> Self {
        CityConstructions {
            city: None,
            built_building_objects: Vec::new(),
            built_building_unique_map: UniqueMap::new(),
            built_buildings: HashSet::new(),
            in_progress_constructions: HashMap::new(),
            current_construction_is_user_set: false,
            construction_queue: Vec::new(),
            production_overflow: 0,
            queue_max_size: 10,
            free_buildings_provided_from_this_city: HashMap::new(),
        }
    }

    /// Gets the current construction from queue
    pub fn get_current_construction_from_queue(&self) -> String {
        if self.construction_queue.is_empty() {
            String::new()
        } else {
            self.construction_queue[0].clone()
        }
    }

    /// Sets the current construction from queue
    pub fn set_current_construction_from_queue(&mut self, value: String) {
        if self.construction_queue.is_empty() {
            self.construction_queue.push(value);
        } else {
            self.construction_queue[0] = value;
        }
    }

    /// Clones the CityConstructions
    pub fn clone(&self) -> Self {
        let mut to_return = CityConstructions::new();
        to_return.built_buildings = self.built_buildings.clone();
        to_return.in_progress_constructions = self.in_progress_constructions.clone();
        to_return.current_construction_is_user_set = self.current_construction_is_user_set;
        to_return.construction_queue = self.construction_queue.clone();
        to_return.production_overflow = self.production_overflow;
        to_return.free_buildings_provided_from_this_city = self.free_buildings_provided_from_this_city.clone();
        to_return
    }

    /// Gets buildable buildings
    pub fn get_buildable_buildings(&self) -> Vec<Building> {
        self.city.as_ref().unwrap().get_ruleset().buildings.values()
            .filter(|building| building.is_buildable(self))
            .cloned()
            .collect()
    }

    /// Gets constructable units
    pub fn get_constructable_units(&self) -> Vec<BaseUnit> {
        self.city.as_ref().unwrap().get_ruleset().units.values()
            .filter(|unit| unit.is_buildable(self))
            .cloned()
            .collect()
    }

    /// Gets stats from buildings
    pub fn get_stats(&self, local_unique_cache: &LocalUniqueCache) -> StatTreeNode {
        let mut stats = StatTreeNode::new();
        for building in self.get_built_buildings() {
            stats.add_stats(building.get_stats(&self.city.as_ref().unwrap(), local_unique_cache), &building.name);
        }
        stats
    }

    /// Gets maintenance costs
    pub fn get_maintenance_costs(&self) -> i32 {
        let mut maintenance_cost = 0;
        let free_buildings = self.city.as_ref().unwrap().civ.civ_constructions.get_free_building_names(&self.city.as_ref().unwrap());

        for building in self.get_built_buildings() {
            if !free_buildings.contains(&building.name) {
                maintenance_cost += building.maintenance;
            }
        }

        maintenance_cost
    }

    /// Gets city production text for city button
    pub fn get_city_production_text_for_city_button(&self) -> String {
        let current_construction = self.get_current_construction_from_queue();
        let mut result = current_construction.tr(true);
        if !current_construction.is_empty() {
            let construction = PerpetualConstruction::perpetual_constructions_map.get(&current_construction);
            result += construction.map_or_else(
                || self.get_turns_to_construction_string(&current_construction, true),
                |c| c.get_production_tooltip(&self.city.as_ref().unwrap())
            );
        }
        result
    }

    /// Gets turns to construction string
    pub fn get_turns_to_construction_string(&self, construction_name: &str, use_stored_production: bool) -> String {
        let construction = self.get_construction(construction_name);
        if let Some(non_perpetual) = construction.as_non_perpetual() {
            let cost = non_perpetual.get_production_cost(&self.city.as_ref().unwrap().civ, &self.city.as_ref().unwrap());
            let turns_to_construction = self.turns_to_construction(construction_name, use_stored_production);
            let current_progress = if use_stored_production { self.get_work_done(construction_name) } else { 0 };
            let mut lines = Vec::new();
            let buildable = !non_perpetual.get_matching_uniques(UniqueType::Unbuildable)
                .any(|u| u.conditionals_apply(&self.city.as_ref().unwrap().state));
            if buildable {
                lines.push(format!("{}{}/{}⚒ {}⟳",
                    if current_progress == 0 { String::new() } else { format!("{}/", current_progress) },
                    cost,
                    turns_to_construction
                ));
            }
            let other_stats = Stat::iter()
                .filter(|&stat| {
                    (stat != Stat::Gold || !buildable) &&
                    non_perpetual.can_be_purchased_with_stat(&self.city.as_ref().unwrap(), *stat)
                })
                .map(|stat| format!("{}{}", non_perpetual.get_stat_buy_cost(&self.city.as_ref().unwrap(), stat), stat.character()))
                .collect::<Vec<_>>()
                .join(" / ");
            if !other_stats.is_empty() {
                lines.push(other_stats);
            }
            lines.join("\n")
        } else {
            String::new()
        }
    }

    /// Gets the current construction
    /// Gets the current construction as an enum for concrete access
    pub fn get_current_construction(&self) -> Option<ConstructionRef> {
        let construction_name = self.get_current_construction_from_queue();
        if construction_name.is_empty() {
            return None;
        }
        let ruleset = self.city.as_ref().unwrap().get_ruleset();

        if let Some(building) = ruleset.buildings.get(&construction_name) {
            return Some(ConstructionRef::Building(building));
        }
        if let Some(unit) = ruleset.units.get(&construction_name) {
            return Some(ConstructionRef::Unit(unit));
        }
        if let Some(perpetual) = PerpetualConstruction::perpetual_constructions_map.get(&construction_name) {
            return Some(ConstructionRef::Perpetual(perpetual));
        }
        None
    }
}

/// Enum for concrete construction reference
pub enum ConstructionRef<'a> {
    Building(&'a crate::city_constructions::ConstructionRef),
    Unit(&'a BaseUnit),
    Perpetual(&'a PerpetualConstruction),
}

// Move the following methods inside the impl block above (before its closing brace)
impl<'a> CityConstructions<'a> {
    /// Checks if a building is built
    pub fn is_built(&self, building_name: &str) -> bool {
        self.built_buildings.contains(building_name)
    }

    /// Checks if a construction is being constructed
    pub fn is_being_constructed(&self, construction_name: &str) -> bool {
        self.get_current_construction_from_queue() == construction_name
    }

    /// Checks if a construction is enqueued for later
    pub fn is_enqueued_for_later(&self, construction_name: &str) -> bool {
        self.construction_queue.iter().skip(1).any(|name| name == construction_name)
    }

    /// Checks if a construction is being constructed or enqueued
    pub fn is_being_constructed_or_enqueued(&self, construction_name: &str) -> bool {
        self.construction_queue.contains(&construction_name.to_string())
    }

    /// Checks if the queue is full
    pub fn is_queue_full(&self) -> bool {
        self.construction_queue.len() >= self.queue_max_size as usize
    }

    /// Checks if building a wonder
    pub fn is_building_wonder(&self) -> bool {
        if let Some(building) = self.get_current_construction().as_building() {
            building.is_wonder
        } else {
            false
        }
    }

    /// Checks if can be hurried
    pub fn can_be_hurried(&self) -> bool {
        if let Some(non_perpetual) = self.get_current_construction().as_non_perpetual() {
            !non_perpetual.has_unique(UniqueType::CannotBeHurried)
        } else {
            false
        }
    }

    /// Gets work done for a construction
    pub fn get_work_done(&self, construction_name: &str) -> i32 {
        *self.in_progress_constructions.get(construction_name).unwrap_or(&0)
    }

    /// Gets remaining work for a construction
    pub fn get_remaining_work(&self, construction_name: &str, use_stored_production: bool) -> i32 {
        let construction = self.get_construction(construction_name);
        if let Some(non_perpetual) = construction.as_non_perpetual() {
            let cost = non_perpetual.get_production_cost(&self.city.as_ref().unwrap().civ, &self.city.as_ref().unwrap());
            if use_stored_production {
                cost - self.get_work_done(construction_name)
            } else {
                cost
            }
        } else {
            0
        }
    }

    /// Gets turns to construction
    pub fn turns_to_construction(&self, construction_name: &str, use_stored_production: bool) -> i32 {
        let work_left = self.get_remaining_work(construction_name, use_stored_production);
        if work_left <= 0 {
            0
        } else if work_left <= self.production_overflow {
            1
        } else {
            ((work_left - self.production_overflow) as f32 / self.production_for_construction(construction_name) as f32).ceil() as i32
        }
    }

    /// Gets production for construction
    pub fn production_for_construction(&self, construction_name: &str) -> i32 {
        let city_stats_for_construction = if self.get_current_construction_from_queue() == construction_name {
            self.city.as_ref().unwrap().city_stats.current_city_stats.clone()
        } else {
            let mut city_stats = CityStats::new(&self.city.as_ref().unwrap());
            city_stats.stats_from_tiles = self.city.as_ref().unwrap().city_stats.stats_from_tiles.clone();
            let construction = self.get_construction(construction_name);
            city_stats.update(&construction, false, false);
            city_stats.current_city_stats
        };

        city_stats_for_construction.production.round() as i32
    }

    /// Gets cheapest stat building
    pub fn cheapest_stat_building(&self, stat: Stat) -> Option<Building> {
        self.city.as_ref().unwrap().get_ruleset().buildings.values()
            .filter(|building| {
                !building.is_any_wonder() &&
                building.is_stat_related(stat, &self.city.as_ref().unwrap()) &&
                (building.is_buildable(self) || self.is_being_constructed_or_enqueued(&building.name))
            })
            .min_by_key(|building| building.cost)
            .cloned()
    }

    /// Sets transients
    pub fn set_transients(&mut self) {
        self.built_building_objects = self.built_buildings.iter()
            .map(|name| {
                self.city.as_ref().unwrap().get_ruleset().buildings.get(name)
                    .unwrap_or_else(|| panic!("Building {} is not found!", name))
                    .clone()
            })
            .collect();
        self.update_uniques(true);
    }

    /// Adds production points
    pub fn add_production_points(&mut self, production_to_add: i32) {
        let construction = self.get_construction(&self.get_current_construction_from_queue());
        if construction.is_perpetual() {
            self.production_overflow += production_to_add;
            return;
        }
        *self.in_progress_constructions.entry(self.get_current_construction_from_queue())
            .or_insert(0) += production_to_add;
    }

    /// Gets built buildings
    pub fn get_built_buildings(&self) -> &[Building] {
        &self.built_building_objects
    }

    /// Updates uniques
    pub fn update_uniques(&mut self, on_load_game: bool) {
        self.built_building_unique_map.clear();
        for building in self.get_built_buildings() {
            self.built_building_unique_map.add_uniques(&building.unique_objects);
        }
        if !on_load_game {
            self.city.as_ref().unwrap().civ.cache.update_cities_connected_to_capital(false);
            self.city.as_ref().unwrap().city_stats.update();
            self.city.as_ref().unwrap().civ.cache.update_civ_resources();
        }
    }

    /// Adds a construction to queue
    pub fn add_to_queue(&mut self, construction_name: String) {
        if self.is_queue_full() {
            return;
        }
        if self.construction_queue.is_empty() {
            self.construction_queue.push(construction_name);
            self.current_construction_is_user_set = true;
        } else {
            self.construction_queue.push(construction_name);
        }
    }

    /// Removes a construction from queue
    pub fn remove_from_queue(&mut self, index: usize) {
        if index < self.construction_queue.len() {
            self.construction_queue.remove(index);
            if index == 0 {
                self.current_construction_is_user_set = false;
            }
        }
    }

    /// Moves a construction in queue
    pub fn move_in_queue(&mut self, from: usize, to: usize) {
        if from >= self.construction_queue.len() || to >= self.construction_queue.len() {
            return;
        }
        let construction = self.construction_queue.remove(from);
        self.construction_queue.insert(to, construction);
    }

    /// Clears the construction queue
    pub fn clear_queue(&mut self) {
        self.construction_queue.clear();
        self.current_construction_is_user_set = false;
    }

    /// Gets a construction by name
    pub fn get_construction(&self, construction_name: &str) -> Box<dyn Construction> {
        if construction_name.is_empty() {
            return Box::new(PerpetualConstruction::nothing());
        }

        let ruleset = self.city.as_ref().unwrap().get_ruleset();

        if let Some(building) = ruleset.buildings.get(construction_name) {
            return Box::new(building.clone());
        }
        if let Some(unit) = ruleset.units.get(construction_name) {
            return Box::new(unit.clone());
        }
        if let Some(perpetual) = PerpetualConstruction::perpetual_constructions_map.get(construction_name) {
            return Box::new(perpetual.clone());
        }

        panic!("No construction found for name: {}", construction_name);
    }

    /// Completes a construction
    pub fn complete_construction(&mut self, construction_name: &str, city_info: &CityInfo) {
        let construction = self.get_construction(construction_name);

        if let Some(building) = construction.as_building() {
            self.built_buildings.insert(building.name.clone());
            self.built_building_objects.push(building.clone());
            self.update_uniques(false);
        } else if let Some(unit) = construction.as_unit() {
            let unit_name = unit.name.clone();
            self.city.as_ref().unwrap().civ.add_unit(
                &unit_name,
                &self.city.as_ref().unwrap().location,
                city_info
            );
        }

        self.in_progress_constructions.remove(construction_name);

        if !construction.is_perpetual() {
            self.remove_from_queue(0);
            self.current_construction_is_user_set = false;
        }
    }

    /// Handles construction completion
    pub fn construction_complete(&mut self, construction_name: &str) -> bool {
        let construction = self.get_construction(construction_name);
        if let Some(non_perpetual) = construction.as_non_perpetual() {
            let cost = non_perpetual.get_production_cost(&self.city.as_ref().unwrap().civ, &self.city.as_ref().unwrap());
            let work_done = self.get_work_done(construction_name);
            work_done >= cost
        } else {
            false
        }
    }

    /// Processes construction queue
    pub fn process_construction(&mut self, city_info: &CityInfo) {
        if self.construction_queue.is_empty() {
            return;
        }

        let current_construction = self.get_current_construction_from_queue();
        if current_construction.is_empty() {
            return;
        }

        let construction = self.get_construction(&current_construction);
        if construction.is_perpetual() {
            let perpetual = construction.as_perpetual().unwrap();
            perpetual.process_production(&self.city.as_ref().unwrap(), self.production_overflow);
            self.production_overflow = 0;
            return;
        }

        let production = self.production_for_construction(&current_construction);
        if production <= 0 {
            return;
        }

        self.add_production_points(production + self.production_overflow);
        self.production_overflow = 0;

        if self.construction_complete(&current_construction) {
            let construction = self.get_construction(&current_construction);
            if let Some(non_perpetual) = construction.as_non_perpetual() {
                let cost = non_perpetual.get_production_cost(&self.city.as_ref().unwrap().civ, &self.city.as_ref().unwrap());
                let work_done = self.get_work_done(&current_construction);
                self.production_overflow = work_done - cost;
            }
            self.complete_construction(&current_construction, city_info);
        }
    }

    /// Buys a construction with gold
    pub fn buy_construction_with_gold(&mut self, construction_name: &str, city_info: &CityInfo) {
        let construction = self.get_construction(construction_name);
        if let Some(non_perpetual) = construction.as_non_perpetual() {
            let cost = non_perpetual.get_gold_cost(&self.city.as_ref().unwrap());
            self.city.as_ref().unwrap().civ.gold -= cost;
            self.complete_construction(construction_name, city_info);
        }
    }

    /// Gets construction status text
    pub fn get_construction_status_text(&self) -> String {
        let current_construction = self.get_current_construction_from_queue();
        if current_construction.is_empty() {
            return "Pick a construction".to_string();
        }

        let construction = self.get_construction(&current_construction);
        if construction.is_perpetual() {
            return format!("{}{}",
                current_construction,
                construction.as_perpetual().unwrap().get_production_tooltip(&self.city.as_ref().unwrap())
            );
        }

        let work_done = self.get_work_done(&current_construction);
        let cost = construction.as_non_perpetual().unwrap()
            .get_production_cost(&self.city.as_ref().unwrap().civ, &self.city.as_ref().unwrap());

        format!("{} - {} turns left",
            current_construction,
            self.turns_to_construction(&current_construction, true)
        )
    }
}