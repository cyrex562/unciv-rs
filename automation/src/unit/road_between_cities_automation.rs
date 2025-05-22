use std::collections::{HashMap, HashSet};
use crate::models::civilization::Civilization;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::{RoadStatus, Tile};
use crate::models::city::City;
use crate::models::map::{BFS, HexMath, MapPathing};
use crate::models::ruleset::unique::UniqueType;
use crate::models::Vector2;
use crate::constants::NO_ID;
use crate::unciv_game::UncivGame;
use crate::utils::{debug, log_should_log};
use std::cmp::max;

/// Constants for worker automation
struct WorkerAutomationConst;

impl WorkerAutomationConst {
    /// BFS max size is determined by the aerial distance of two cities to connect, padded with this
    /// Two tiles longer than the distance to the nearest connected city should be enough as the 'reach' of a BFS is increased by blocked tiles
    const MAX_BFS_REACH_PADDING: i32 = 2;
}

/// Responsible for the "connect cities" automation as part of worker automation
pub struct RoadBetweenCitiesAutomation {
    civ_info: Civilization,
    cached_for_turn: i32,
    /// Caches BFS by city locations (cities needing connecting)
    /// key: The city to connect from as hex position Vector2
    /// value: The BFS searching from that city, whether successful or not
    bfs_cache: HashMap<Vector2, BFS>,
    /// Caches road to build for connecting cities unless option is off or ruleset removed all roads
    best_road_available: RoadStatus,
    /// Cache of roads to build between cities each turn
    roads_to_build_by_cities_cache: HashMap<City, Vec<RoadPlan>>,
    /// Hashmap of all cached tiles in each list in roads_to_build_by_cities_cache
    tiles_of_roads_map: HashMap<Tile, RoadPlan>,
}

/// Represents a plan for road construction between cities
pub struct RoadPlan {
    tiles: Vec<Tile>,
    priority: f32,
    from_city: City,
    to_city: City,
    number_of_roads_to_build: i32,
}

impl RoadPlan {
    fn new(tiles: Vec<Tile>, priority: f32, from_city: City, to_city: City, best_road_available: RoadStatus) -> Self {
        let number_of_roads_to_build = tiles.iter()
            .filter(|tile| tile.get_unpillaged_road() != best_road_available)
            .count() as i32;

        Self {
            tiles,
            priority,
            from_city,
            to_city,
            number_of_roads_to_build,
        }
    }
}

impl RoadBetweenCitiesAutomation {
    pub fn new(civ_info: Civilization, cached_for_turn: i32, cloning_source: Option<&RoadBetweenCitiesAutomation>) -> Self {
        let best_road_available = if let Some(source) = cloning_source {
            source.best_road_available
        } else if civ_info.is_human() && !UncivGame::current().settings.auto_building_roads
            && !UncivGame::current().world_screen.auto_play.is_auto_playing_and_full_auto_play_ai() {
            RoadStatus::None
        } else {
            civ_info.tech.get_best_road_available()
        };

        Self {
            civ_info,
            cached_for_turn,
            bfs_cache: HashMap::new(),
            best_road_available,
            roads_to_build_by_cities_cache: HashMap::new(),
            tiles_of_roads_map: HashMap::new(),
        }
    }

    /// Returns a list of tiles of connected cities (unsorted)
    fn get_tiles_of_connected_cities(&self) -> Vec<Tile> {
        let result: Vec<Tile> = self.civ_info.cities.iter()
            .filter(|city| city.is_capital() || city.city_stats.is_connected_to_capital(self.best_road_available))
            .map(|city| city.get_center_tile())
            .collect();

        if log_should_log() {
            debug!("WorkerAutomation tilesOfConnectedCities for {} turn {}:", self.civ_info.civ_name, self.cached_for_turn);
            if result.is_empty() {
                debug!("\tempty");
            } else {
                for tile in &result {
                    debug!("\t{:?}", tile);
                }
            }
        }

        result
    }

    /// Gets the worst road type in a path of tiles
    fn get_worst_road_type_in_path(&self, path: &[Tile]) -> RoadStatus {
        let mut worst_road_tile = RoadStatus::Railroad;
        for tile in path {
            let road_status = tile.get_unpillaged_road();
            if road_status < worst_road_tile {
                worst_road_tile = road_status;
                if worst_road_tile == RoadStatus::None {
                    return RoadStatus::None;
                }
            }
        }
        worst_road_tile
    }

    /// Returns a road that can connect this city to the capital
    fn get_road_to_connect_city_to_capital(&mut self, unit: &MapUnit, city: &City) -> Option<(City, Vec<Tile>)> {
        let tiles_of_connected_cities = self.get_tiles_of_connected_cities();
        if tiles_of_connected_cities.is_empty() {
            return None; // In mods with no capital city indicator, there are no connected cities
        }

        let is_candidate_tile = |tile: &Tile| tile.is_land && unit.movement.can_pass_through(tile);
        let to_connect_tile = city.get_center_tile();

        let bfs = if let Some(cached_bfs) = self.bfs_cache.get(&to_connect_tile.position) {
            cached_bfs
        } else {
            let min_distance = tiles_of_connected_cities.iter()
                .map(|tile| tile.aerial_distance_to(to_connect_tile))
                .min()
                .unwrap_or(0);

            let max_size = HexMath::get_number_of_tiles_in_hexagon(
                WorkerAutomationConst::MAX_BFS_REACH_PADDING + min_distance
            );

            let new_bfs = BFS::new(to_connect_tile, is_candidate_tile, max_size);
            self.bfs_cache.insert(to_connect_tile.position.clone(), new_bfs);
            self.bfs_cache.get(&to_connect_tile.position).unwrap()
        };

        let city_tiles_to_seek: HashSet<_> = tiles_of_connected_cities.iter().collect();

        let mut next_tile = bfs.next_step();
        while let Some(tile) = next_tile {
            if city_tiles_to_seek.contains(tile) {
                // We have a winner!
                let city_tile = tile;
                let path_to_city = bfs.get_path_to(city_tile);
                return Some((city_tile.get_city().unwrap(), path_to_city));
            }
            next_tile = bfs.next_step();
        }
        None
    }

    /// Gets roads to build from a specific city
    pub fn get_roads_to_build_from_city(&mut self, city: &City) -> Vec<RoadPlan> {
        if let Some(cached_roads) = self.roads_to_build_by_cities_cache.get(city) {
            return cached_roads.clone();
        }

        // TODO: some better worker representative needs to be used here
        let worker_unit = self.civ_info.game_info.ruleset.units.values()
            .find(|unit| unit.has_unique(UniqueType::BuildImprovements))
            .map(|unit| unit.get_map_unit(&self.civ_info, NO_ID));

        let worker_unit = match worker_unit {
            Some(unit) => unit,
            None => return Vec::new(),
        };

        let road_to_capital_status = city.city_stats.get_road_type_of_connection_to_capital();

        let rank_road_capital_priority = |road_status: RoadStatus| -> f32 {
            match road_status {
                RoadStatus::None => if self.best_road_available != RoadStatus::None { 2.0 } else { 0.0 },
                RoadStatus::Road => if self.best_road_available != RoadStatus::Road { 1.0 } else { 0.0 },
                _ => 0.0,
            }
        };

        let base_priority = rank_road_capital_priority(road_to_capital_status);
        let mut roads_to_build = Vec::new();

        // Handle nearby cities
        for close_city in city.neighboring_cities.iter()
            .filter(|c| c.civ == self.civ_info && c.get_center_tile().aerial_distance_to(city.get_center_tile()) <= 8)
        {
            // Check if other city has planned road to this city
            if let Some(cached_roads) = self.roads_to_build_by_cities_cache.get(close_city) {
                if let Some(road) = cached_roads.iter()
                    .find(|r| r.from_city == *city || r.to_city == *city)
                {
                    roads_to_build.push(road.clone());
                    continue;
                }
            }

            // Try to build a plan for the road to the city
            let road_path = if self.civ_info.cities.iter().position(|c| c == city)
                < self.civ_info.cities.iter().position(|c| c == close_city)
            {
                MapPathing::get_road_path(&worker_unit, city.get_center_tile(), close_city.get_center_tile())
            } else {
                MapPathing::get_road_path(&worker_unit, close_city.get_center_tile(), city.get_center_tile())
            };

            let road_path = match road_path {
                Some(path) => path,
                None => continue,
            };

            let worst_road_status = self.get_worst_road_type_in_path(&road_path);
            if worst_road_status == self.best_road_available {
                continue;
            }

            let mut road_priority = max(
                base_priority,
                rank_road_capital_priority(close_city.city_stats.get_road_type_of_connection_to_capital())
            );

            if worst_road_status == RoadStatus::None {
                road_priority += 2.0;
            } else if worst_road_status == RoadStatus::Road && self.best_road_available == RoadStatus::Railroad {
                road_priority += 1.0;
            }

            if close_city.city_stats.get_road_type_of_connection_to_capital() > road_to_capital_status {
                road_priority += 1.0;
            }

            let new_road_plan = RoadPlan::new(
                road_path,
                road_priority + (city.population.population + close_city.population.population) as f32 / 4.0,
                city.clone(),
                close_city.clone(),
                self.best_road_available
            );

            for tile in &new_road_plan.tiles {
                if !self.tiles_of_roads_map.contains_key(tile)
                    || self.tiles_of_roads_map[tile].priority < new_road_plan.priority {
                    self.tiles_of_roads_map.insert(tile.clone(), new_road_plan.clone());
                }
            }

            roads_to_build.push(new_road_plan);
        }

        // Handle road to capital if needed
        if roads_to_build.is_empty() && road_to_capital_status < self.best_road_available {
            if let Some((connect_city, road_path)) = self.get_road_to_connect_city_to_capital(&worker_unit, city) {
                let worst_road_status = self.get_worst_road_type_in_path(&road_path);
                let mut road_priority = base_priority;
                road_priority += if worst_road_status == RoadStatus::None { 2.0 } else { 1.0 };

                let new_road_plan = RoadPlan::new(
                    road_path,
                    road_priority + city.population.population as f32 / 2.0,
                    city.clone(),
                    connect_city,
                    self.best_road_available
                );

                for tile in &new_road_plan.tiles {
                    if !self.tiles_of_roads_map.contains_key(tile)
                        || self.tiles_of_roads_map[tile].priority < new_road_plan.priority {
                        self.tiles_of_roads_map.insert(tile.clone(), new_road_plan.clone());
                    }
                }

                roads_to_build.push(new_road_plan);
            }
        }

        self.roads_to_build_by_cities_cache.insert(city.clone(), roads_to_build.clone());
        roads_to_build
    }

    /// Gets nearby cities that need to be connected
    pub fn get_nearby_cities_to_connect(&mut self, unit: &MapUnit) -> Vec<City> {
        if self.best_road_available == RoadStatus::None {
            return Vec::new();
        }

        self.civ_info.cities.iter()
            .filter(|city| {
                city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20
                    && !self.get_roads_to_build_from_city(city).is_empty()
            })
            .cloned()
            .collect()
    }

    /// Tries to connect cities with roads
    /// Returns whether we actually did anything
    pub fn try_connecting_cities(&mut self, unit: &mut MapUnit, candidate_cities: &[City]) -> bool {
        if self.best_road_available == RoadStatus::None || candidate_cities.is_empty() {
            return false;
        }

        let current_tile = unit.get_tile();

        // Search through ALL candidate cities for the closest tile to build a road on
        for to_connect_city in candidate_cities.iter()
            .sorted_by_key(|city| -city.get_center_tile().aerial_distance_to(unit.get_tile()))
        {
            let tiles_by_priority: Vec<_> = self.get_roads_to_build_from_city(to_connect_city)
                .iter()
                .flat_map(|road_plan| road_plan.tiles.iter().map(|tile| (tile, road_plan.priority)))
                .collect();

            let tiles_sorted: Vec<_> = tiles_by_priority.iter()
                .filter(|(tile, _)| tile.get_unpillaged_road() < self.best_road_available)
                .sorted_by(|(tile_a, priority_a), (tile_b, priority_b)| {
                    let dist_a = tile_a.aerial_distance_to(unit.get_tile()) as f32 + (priority_a / 10.0);
                    let dist_b = tile_b.aerial_distance_to(unit.get_tile()) as f32 + (priority_b / 10.0);
                    dist_a.partial_cmp(&dist_b).unwrap()
                })
                .collect();

            let best_tile = tiles_sorted.iter()
                .find(|(tile, _)| unit.movement.can_move_to(tile) && unit.movement.can_reach(tile))
                .map(|(tile, _)| *tile);

            let best_tile = match best_tile {
                Some(tile) => tile,
                None => continue,
            };

            if best_tile != current_tile && unit.has_movement() {
                unit.movement.head_towards(best_tile);
            }

            if unit.has_movement() && best_tile == current_tile
                && current_tile.improvement_in_progress != Some(self.best_road_available.name()) {
                let improvement = self.best_road_available.improvement(&self.civ_info.game_info.ruleset).unwrap();
                best_tile.start_working_on_improvement(improvement, &self.civ_info, unit);
            }

            return true;
        }

        false
    }
}