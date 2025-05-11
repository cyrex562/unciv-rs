use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use cgmath::Vector2;
use crate::civilization::{Civilization, Notification, NotificationCategory, NotificationIcon, PlayerType};
use crate::civilization::Proximity;
use crate::city::City;
use crate::constants::Constants;
use crate::map::{MapShape, Tile};
use crate::models::ruleset::{Building, ResourceSupplyList, ResourceType, TileImprovement};
use crate::models::ruleset::unique::{StateForConditionals, UniqueMap, UniqueTarget, UniqueType, UniqueTriggerActivation};
use crate::models::ruleset::unit::BaseUnit;
use crate::models::stats::Stats;
use crate::utils::DebugUtils;
use crate::civilization::transients::capital_connections_finder::CapitalConnectionsFinder;

/// CivInfo class was getting too crowded
pub struct CivInfoTransientCache {
    civ_info: Arc<Civilization>,

    /// Last era resource used for building
    pub last_era_resource_used_for_building: HashMap<String, i32>,

    /// Last era resource used for unit
    pub last_era_resource_used_for_unit: HashMap<String, i32>,

    /// Easy way to look up a Civilization's unique units and buildings
    pub unique_units: HashSet<Arc<BaseUnit>>,

    pub unique_improvements: HashSet<Arc<TileImprovement>>,

    pub unique_buildings: HashSet<Arc<Building>>,

    /// Contains mapping of cities to travel mediums from ALL civilizations connected by trade routes to the capital
    pub cities_connected_to_capital_to_mediums: HashMap<Arc<City>, HashSet<String>>,

    /// Our tiles and neighboring tiles
    pub our_tiles_and_neighboring_tiles: HashSet<Arc<Tile>>,
}

impl CivInfoTransientCache {
    /// Creates a new CivInfoTransientCache
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        Self {
            civ_info,
            last_era_resource_used_for_building: HashMap::new(),
            last_era_resource_used_for_unit: HashMap::new(),
            unique_units: HashSet::new(),
            unique_improvements: HashSet::new(),
            unique_buildings: HashSet::new(),
            cities_connected_to_capital_to_mediums: HashMap::new(),
            our_tiles_and_neighboring_tiles: HashSet::new(),
        }
    }

    /// Updates the state
    pub fn update_state(&mut self) {
        self.civ_info.state = StateForConditionals::new(&self.civ_info, None);
    }

    /// Sets transients
    pub fn set_transients(&mut self) {
        let ruleset = &self.civ_info.game_info.ruleset;

        let state = &self.civ_info.state;

        let buildings_to_required_resources: HashMap<_, _> = ruleset.buildings.values()
            .iter()
            .filter(|building| self.civ_info.get_equivalent_building(building) == *building)
            .map(|building| (building, building.required_resources(state)))
            .collect();

        let units_to_required_resources: HashMap<_, _> = ruleset.units.values()
            .iter()
            .filter(|unit| self.civ_info.get_equivalent_unit(unit) == *unit)
            .map(|unit| (unit, unit.required_resources(state)))
            .collect();

        for resource_name in ruleset.tile_resources.values()
            .iter()
            .filter(|resource| resource.resource_type == ResourceType::Strategic)
            .map(|resource| resource.name.clone()) {

            let applicable_buildings: Vec<_> = buildings_to_required_resources.iter()
                .filter(|(_, resources)| resources.contains(&resource_name))
                .map(|(building, _)| building)
                .collect();

            let applicable_units: Vec<_> = units_to_required_resources.iter()
                .filter(|(_, resources)| resources.contains(&resource_name))
                .map(|(unit, _)| unit)
                .collect();

            let last_era_for_building = applicable_buildings.iter()
                .map(|building| building.era(ruleset).map(|era| era.era_number).unwrap_or(0))
                .max();

            let last_era_for_unit = applicable_units.iter()
                .map(|unit| unit.era(ruleset).map(|era| era.era_number).unwrap_or(0))
                .max();

            if let Some(era) = last_era_for_building {
                self.last_era_resource_used_for_building.insert(resource_name.clone(), era);
            }

            if let Some(era) = last_era_for_unit {
                self.last_era_resource_used_for_unit.insert(resource_name.clone(), era);
            }
        }

        for building in ruleset.buildings.values() {
            if let Some(unique_to) = &building.unique_to {
                if self.civ_info.matches_filter(unique_to) {
                    self.unique_buildings.insert(building.clone());
                }
            }
        }

        for improvement in ruleset.tile_improvements.values() {
            if let Some(unique_to) = &improvement.unique_to {
                if self.civ_info.matches_filter(unique_to) {
                    self.unique_improvements.insert(improvement.clone());
                }
            }
        }

        for unit in ruleset.units.values() {
            if let Some(unique_to) = &unit.unique_to {
                if self.civ_info.matches_filter(unique_to) {
                    self.unique_units.insert(unit.clone());
                }
            }
        }
    }

    /// Updates sight and resources
    pub fn update_sight_and_resources(&mut self) {
        self.update_viewable_tiles(None);
        self.update_has_active_enemy_movement_penalty();
        self.update_civ_resources();
    }

    /// Updates viewable tiles
    pub fn update_viewable_tiles(&mut self, explorer_position: Option<Vector2<i32>>) {
        self.set_new_viewable_tiles();

        self.update_viewable_invisible_tiles();

        self.update_last_seen_improvements();

        // updating the viewable tiles also affects the explored tiles, obviously.
        // So why don't we play switcharoo with the explored tiles as well?
        // Well, because it gets REALLY LARGE so it's a lot of memory space,
        // and we never actually iterate on the explored tiles (only check contains()),
        // so there's no fear of concurrency problems.
        for tile in &self.civ_info.viewable_tiles {
            tile.set_explored(&self.civ_info, true, explorer_position);
        }

        let mut viewed_civs = HashMap::new();
        for tile in &self.civ_info.viewable_tiles {
            if let Some(tile_owner) = tile.get_owner() {
                viewed_civs.insert(tile_owner, tile.clone());
            }

            if let Some(unit) = tile.get_first_unit() {
                if let Some(unit_owner) = &unit.civ {
                    viewed_civs.insert(unit_owner.clone(), tile.clone());
                }
            }
        }

        if !self.civ_info.is_barbarian {
            for (met_civ, tile) in viewed_civs {
                if met_civ == self.civ_info || met_civ.is_barbarian ||
                   self.civ_info.diplomacy.contains_key(&met_civ.civ_name) {
                    continue;
                }

                self.civ_info.diplomacy_functions.make_civilizations_meet(&met_civ);

                if !self.civ_info.is_spectator() {
                    self.civ_info.add_notification(
                        format!("We have encountered [{}]!", met_civ.civ_name),
                        tile.position,
                        NotificationCategory::Diplomacy,
                        &met_civ.civ_name,
                        NotificationIcon::Diplomacy
                    );
                }

                met_civ.add_notification(
                    format!("We have encountered [{}]!", self.civ_info.civ_name),
                    tile.position,
                    NotificationCategory::Diplomacy,
                    &self.civ_info.civ_name,
                    NotificationIcon::Diplomacy
                );
            }

            self.discover_natural_wonders();
        }
    }

    /// Updates viewable invisible tiles
    fn update_viewable_invisible_tiles(&mut self) {
        let mut new_viewable_invisible_tiles = HashSet::new();

        for unit in self.civ_info.units.get_civ_units() {
            let invisible_unit_uniques = unit.get_matching_uniques(UniqueType::CanSeeInvisibleUnits);
            if invisible_unit_uniques.is_empty() {
                continue;
            }

            let visible_unit_types: Vec<_> = invisible_unit_uniques.iter()
                .map(|unique| unique.params[0].clone())
                .collect(); // save this, it'll be seeing a lot of use

            for tile in &unit.viewable_tiles {
                if tile.military_unit.is_none() {
                    continue;
                }

                if new_viewable_invisible_tiles.contains(tile) {
                    continue;
                }

                if let Some(military_unit) = &tile.military_unit {
                    if visible_unit_types.iter().any(|unit_type| military_unit.matches_filter(unit_type)) {
                        new_viewable_invisible_tiles.insert(tile.clone());
                    }
                }
            }
        }

        self.civ_info.viewable_invisible_units_tiles = new_viewable_invisible_tiles;
    }

    /// Updates our tiles
    pub fn update_our_tiles(&mut self) {
        self.our_tiles_and_neighboring_tiles = self.civ_info.cities.iter()
            .flat_map(|city| city.get_tiles()) // our owned tiles, still distinct
            .flat_map(|tile| {
                let mut tiles = vec![tile.clone()];
                tiles.extend(tile.neighbors.iter().cloned());
                tiles
            })
            .collect(); // now we got a mix of owned, unowned and competitor-owned tiles, and **duplicates**
            // but HashSet is just as good at making them distinct as any other operation

        self.update_viewable_tiles(None);
        self.update_civ_resources();
    }

    /// Sets new viewable tiles
    fn set_new_viewable_tiles(&mut self) {
        if self.civ_info.is_defeated() {
            // Avoid meeting dead city states when entering a tile owned by their former ally (#9245)
            // In that case ourTilesAndNeighboringTiles and getCivUnits will be empty, but the for
            // loop getKnownCivs/getAllyCiv would add tiles.
            self.civ_info.viewable_tiles = HashSet::new();
            return;
        }

        // while spectating all map is visible
        if self.civ_info.is_spectator() || DebugUtils::VISIBLE_MAP {
            let all_tiles: HashSet<_> = self.civ_info.game_info.tile_map.values().cloned().collect();
            self.civ_info.viewable_tiles = all_tiles.clone();
            self.civ_info.viewable_invisible_units_tiles = all_tiles;
            return;
        }

        let mut new_viewable_tiles = self.our_tiles_and_neighboring_tiles.clone();

        for unit in self.civ_info.units.get_civ_units() {
            for tile in &unit.viewable_tiles {
                if let Some(owner) = tile.get_owner() {
                    if owner != self.civ_info {
                        new_viewable_tiles.insert(tile.clone());
                    }
                }
            }
        }

        for other_civ in self.civ_info.get_known_civs() {
            if other_civ.get_ally_civ() == Some(self.civ_info.civ_name.clone()) ||
               other_civ.civ_name == self.civ_info.get_ally_civ() {
                for city in &other_civ.cities {
                    for tile in city.get_tiles() {
                        new_viewable_tiles.insert(tile.clone());
                    }
                }
            }
        }

        for tile in self.civ_info.espionage_manager.get_tiles_visible_via_spies() {
            new_viewable_tiles.insert(tile.clone());
        }

        self.civ_info.viewable_tiles = new_viewable_tiles; // to avoid concurrent modification problems
    }

    /// Updates last seen improvements
    fn update_last_seen_improvements(&mut self) {
        if self.civ_info.player_type == PlayerType::AI {
            return; // don't bother for AI, they don't really use the info anyway
        }

        for tile in &self.civ_info.viewable_tiles {
            self.civ_info.set_last_seen_improvement(tile.position, tile.improvement.clone());
        }
    }

    /// Discovers natural wonders
    pub fn discover_natural_wonders(&mut self) {
        let mut newly_viewed_natural_wonders = HashSet::new();

        for tile in &self.civ_info.viewable_tiles {
            if let Some(natural_wonder) = &tile.natural_wonder {
                if !self.civ_info.natural_wonders.contains(natural_wonder) {
                    newly_viewed_natural_wonders.insert(tile.clone());
                }
            }
        }

        for tile in newly_viewed_natural_wonders {
            if let Some(natural_wonder) = &tile.natural_wonder {
                // GBR could be discovered twice otherwise!
                if self.civ_info.natural_wonders.contains(natural_wonder) {
                    continue;
                }

                self.civ_info.natural_wonders.insert(natural_wonder.clone());

                if self.civ_info.is_spectator() {
                    continue; // don't trigger anything
                }

                self.civ_info.add_notification(
                    format!("We have discovered [{}]!", natural_wonder),
                    tile.position,
                    NotificationCategory::General,
                    "StatIcons/Happiness"
                );

                let mut stats_gained = Stats::new();

                let discovered_natural_wonders: HashSet<_> = self.civ_info.game_info.civilizations.iter()
                    .filter(|civ| civ != &self.civ_info && civ.is_major_civ())
                    .flat_map(|civ| civ.natural_wonders.iter().cloned())
                    .collect();

                if tile.terrain_has_unique(UniqueType::GrantsStatsToFirstToDiscover) &&
                   !discovered_natural_wonders.contains(natural_wonder) {

                    for unique in tile.get_terrain_matching_uniques(UniqueType::GrantsStatsToFirstToDiscover) {
                        stats_gained.add_stats(&unique.stats);
                    }
                }

                for unique in self.civ_info.get_matching_uniques(UniqueType::StatBonusWhenDiscoveringNaturalWonder) {
                    let normal_bonus = Stats::parse(&unique.params[0]);
                    let first_discovered_bonus = Stats::parse(&unique.params[1]);

                    if discovered_natural_wonders.contains(natural_wonder) {
                        stats_gained.add_stats(&normal_bonus);
                    } else {
                        stats_gained.add_stats(&first_discovered_bonus);
                    }
                }

                let mut natural_wonder_name = None;

                if !stats_gained.is_empty() {
                    natural_wonder_name = Some(natural_wonder.clone());
                }

                if !stats_gained.is_empty() && natural_wonder_name.is_some() {
                    self.civ_info.add_stats(&stats_gained);
                    self.civ_info.add_notification(
                        format!("We have received [{}] for discovering [{}]",
                            stats_gained, natural_wonder_name.unwrap()),
                        NotificationCategory::General,
                        &stats_gained.to_string()
                    );
                }

                for unique in self.civ_info.get_triggered_uniques(
                    UniqueType::TriggerUponDiscoveringNaturalWonder,
                    StateForConditionals::new(&self.civ_info, Some(tile.clone()))
                ) {
                    UniqueTriggerActivation::trigger_unique(
                        unique,
                        &self.civ_info,
                        Some(tile.clone()),
                        "due to discovering a Natural Wonder"
                    );
                }
            }
        }
    }

    /// Updates has active enemy movement penalty
    pub fn update_has_active_enemy_movement_penalty(&mut self) {
        self.civ_info.has_active_enemy_movement_penalty = self.civ_info.has_unique(UniqueType::EnemyUnitsSpendExtraMovement);
        self.civ_info.enemy_movement_penalty_uniques = self.civ_info.get_matching_uniques(UniqueType::EnemyUnitsSpendExtraMovement);
    }

    /// Updates cities connected to capital
    pub fn update_cities_connected_to_capital(&mut self, initial_setup: bool) {
        if self.civ_info.cities.is_empty() {
            return; // No cities to connect
        }

        let old_connected_cities: Vec<_> = if initial_setup {
            self.civ_info.cities.iter()
                .filter(|city| city.connected_to_capital_status == City::ConnectedToCapitalStatus::True)
                .cloned()
                .collect()
        } else {
            self.cities_connected_to_capital_to_mediums.keys().cloned().collect()
        };

        let old_maybe_connected_cities: Vec<_> = if initial_setup {
            self.civ_info.cities.iter()
                .filter(|city| city.connected_to_capital_status != City::ConnectedToCapitalStatus::False)
                .cloned()
                .collect()
        } else {
            self.cities_connected_to_capital_to_mediums.keys().cloned().collect()
        };

        self.cities_connected_to_capital_to_mediums = if self.civ_info.get_capital().is_none() {
            HashMap::new()
        } else {
            CapitalConnectionsFinder::new(&self.civ_info).find()
        };

        let new_connected_cities: Vec<_> = self.cities_connected_to_capital_to_mediums.keys().cloned().collect();

        for city in &new_connected_cities {
            if !old_maybe_connected_cities.contains(city) &&
               city.civ == self.civ_info &&
               city != self.civ_info.get_capital().unwrap() {
                self.civ_info.add_notification(
                    format!("[{}] has been connected to your capital!", city.name),
                    city.location,
                    NotificationCategory::Cities,
                    NotificationIcon::Gold
                );
            }
        }

        // This may still contain cities that have just been destroyed by razing - thus the population test
        for city in &old_connected_cities {
            if !new_connected_cities.contains(city) &&
               city.civ == self.civ_info &&
               city.population.population > 0 {
                self.civ_info.add_notification(
                    format!("[{}] has been disconnected from your capital!", city.name),
                    city.location,
                    NotificationCategory::Cities,
                    NotificationIcon::Gold
                );
            }
        }

        for city in &self.civ_info.cities {
            city.connected_to_capital_status = if new_connected_cities.contains(city) {
                City::ConnectedToCapitalStatus::True
            } else {
                City::ConnectedToCapitalStatus::False
            };
        }
    }

    /// Updates civ resources
    pub fn update_civ_resources(&mut self) {
        let mut new_detailed_civ_resources = ResourceSupplyList::new();
        let resource_modifiers = self.civ_info.get_resource_modifiers();

        for city in &self.civ_info.cities {
            new_detailed_civ_resources.add(city.get_resources_generated_by_city(&resource_modifiers));
        }

        if !self.civ_info.is_city_state {
            // First we get all these resources of each city state separately
            let mut city_state_provided_resources = ResourceSupplyList::new();
            let mut resource_bonus_percentage = 1.0;

            for unique in self.civ_info.get_matching_uniques(UniqueType::CityStateResources) {
                resource_bonus_percentage += unique.params[0].parse::<f32>().unwrap() / 100.0;
            }

            for city_state_ally in self.civ_info.get_known_civs()
                .iter()
                .filter(|civ| civ.get_ally_civ() == Some(self.civ_info.civ_name.clone())) {
                for resource_supply in city_state_ally.city_state_functions.get_city_state_resources_for_ally() {
                    if resource_supply.resource.has_unique(UniqueType::CannotBeTraded, &city_state_ally.state) {
                        continue;
                    }

                    let new_amount = (resource_supply.amount as f32 * resource_bonus_percentage) as i32;
                    city_state_provided_resources.add(resource_supply.copy_with_amount(new_amount));
                }
            }

            // Then we combine these into one
            new_detailed_civ_resources.add_by_resource(&city_state_provided_resources, Constants::CITY_STATES);
        }

        for unique in self.civ_info.get_matching_uniques(UniqueType::ProvidesResources) {
            if unique.source_object_type == UniqueTarget::Building || unique.source_object_type == UniqueTarget::Wonder {
                continue; // already calculated in city
            }

            let resource = &self.civ_info.game_info.ruleset.tile_resources[&unique.params[1]];
            new_detailed_civ_resources.add(
                resource,
                unique.get_source_name_for_user(),
                (unique.params[0].parse::<f32>().unwrap() * self.civ_info.get_resource_modifier(resource)) as i32
            );
        }

        for diplomacy_manager in self.civ_info.diplomacy.values() {
            new_detailed_civ_resources.add(diplomacy_manager.resources_from_trade());
        }

        for unit in self.civ_info.units.get_civ_units() {
            new_detailed_civ_resources.subtract_resource_requirements(
                unit.get_resource_requirements_per_turn(),
                &self.civ_info.game_info.ruleset,
                "Units"
            );
        }

        new_detailed_civ_resources.remove_all(|res| res.resource.is_city_wide);

        // Check if anything has actually changed so we don't update stats for no reason - this uses List equality which means it checks the elements
        if self.civ_info.detailed_civ_resources == new_detailed_civ_resources {
            return;
        }

        let summarized_resource_supply = new_detailed_civ_resources.sum_by_resource("All");

        let mut new_resource_unique_map = UniqueMap::new();
        for resource in &summarized_resource_supply {
            if resource.amount > 0 {
                new_resource_unique_map.add_uniques(&resource.resource.unique_objects);
            }
        }

        self.civ_info.detailed_civ_resources = new_detailed_civ_resources;
        self.civ_info.summarized_civ_resource_supply = summarized_resource_supply;
        self.civ_info.civ_resources_unique_map = new_resource_unique_map;

        self.civ_info.update_stats_for_next_turn(); // More or less resources = more or less happiness, with potential domino effects
    }

    /// Updates proximity
    pub fn update_proximity(&mut self, other_civ: &Arc<Civilization>, pre_calculated: Option<Proximity>) -> Proximity {
        if other_civ == &self.civ_info {
            return Proximity::None;
        }

        if let Some(proximity) = pre_calculated {
            // We usually want to update this for a pair of civs at the same time
            // Since this function *should* be symmetrical for both civs, we can just do it once
            self.civ_info.proximity.insert(other_civ.civ_name.clone(), proximity);
            return proximity;
        }

        if self.civ_info.cities.is_empty() || other_civ.cities.is_empty() {
            self.civ_info.proximity.insert(other_civ.civ_name.clone(), Proximity::None);
            return Proximity::None;
        }

        let map_params = &self.civ_info.game_info.tile_map.map_parameters;
        let mut min_distance = 100000; // a long distance
        let mut total_distance = 0;
        let mut connections = 0;

        let mut proximity = Proximity::None;

        for our_city in &self.civ_info.cities {
            for their_city in &other_civ.cities {
                let distance = our_city.get_center_tile().aerial_distance_to(their_city.get_center_tile());
                total_distance += distance;
                connections += 1;
                if min_distance > distance {
                    min_distance = distance;
                }
            }
        }

        if min_distance <= 7 {
            proximity = Proximity::Neighbors;
        } else if connections > 0 {
            let average_distance = total_distance / connections;
            let map_factor = if map_params.shape == MapShape::Rectangular {
                (map_params.map_size.height + map_params.map_size.width) / 2
            } else {
                (map_params.map_size.radius * 3) / 2 // slightly less area than equal size rect
            };

            let close_distance = ((map_factor * 25) / 100).clamp(10, 20);
            let far_distance = ((map_factor * 45) / 100).clamp(20, 50);

            proximity = if min_distance <= 11 && average_distance <= close_distance {
                Proximity::Close
            } else if average_distance <= far_distance {
                Proximity::Far
            } else {
                Proximity::Distant
            };
        }

        // Check if different continents (unless already max distance, or water map)
        if connections > 0 && proximity != Proximity::Distant && !self.civ_info.game_info.tile_map.is_water_map() {
            if let (Some(our_capital), Some(their_capital)) = (self.civ_info.get_capital(), other_civ.get_capital()) {
                if our_capital.get_center_tile().get_continent() != their_capital.get_center_tile().get_continent() {
                    // Different continents - increase separation by one step
                    proximity = match proximity {
                        Proximity::Far => Proximity::Distant,
                        Proximity::Close => Proximity::Far,
                        Proximity::Neighbors => Proximity::Close,
                        _ => proximity,
                    };
                }
            }
        }

        // If there aren't many players (left) we can't be that far
        let num_majors = self.civ_info.game_info.get_alive_major_civs().len();
        if num_majors <= 2 && proximity > Proximity::Close {
            proximity = Proximity::Close;
        }
        if num_majors <= 4 && proximity > Proximity::Far {
            proximity = Proximity::Far;
        }

        self.civ_info.proximity.insert(other_civ.civ_name.clone(), proximity);

        proximity
    }
}