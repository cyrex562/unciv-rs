use std::collections::{HashMap, HashSet, VecDeque};
use std::f64::consts::PI;
use rand::Rng;
use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::{Terrain, TerrainType};
use crate::models::ruleset::unique::{StateForConditionals, Unique, UniqueType};
use crate::utils::debug;
use crate::constants::Constants;

/// Generator for natural wonders on the map
pub struct NaturalWonderGenerator {
    ruleset: Ruleset,
    randomness: MapGenerationRandomness,
    blocked_tiles: HashSet<Tile>,
}

impl NaturalWonderGenerator {
    /// Creates a new NaturalWonderGenerator
    pub fn new(ruleset: Ruleset, randomness: MapGenerationRandomness) -> Self {
        Self {
            ruleset,
            randomness,
            blocked_tiles: HashSet::new(),
        }
    }

    /// Spawns natural wonders on the map
    ///
    /// This method places natural wonders on the map based on the map parameters and ruleset.
    /// The number of wonders scales linearly with the map radius.
    /// Wonders are placed in order of least candidate tiles to most.
    /// There is a minimum distance between wonders equal to the map height / 5.
    pub fn spawn_natural_wonders(&mut self, tile_map: &mut TileMap) {
        if tile_map.map_parameters.no_natural_wonders {
            return;
        }

        let map_radius = tile_map.map_parameters.map_size.radius;
        // number of Natural Wonders scales linearly with mapRadius
        let number_to_spawn = (map_radius * self.ruleset.mod_options.constants.natural_wonder_count_multiplier +
                              self.ruleset.mod_options.constants.natural_wonder_count_added_constant) as i32;

        let mut chosen_wonders = Vec::new();
        let mut wonder_candidate_tiles = HashMap::new();
        let mut all_natural_wonders: Vec<Terrain> = self.ruleset.terrains.values()
            .filter(|t| t.terrain_type == TerrainType::NaturalWonder)
            .cloned()
            .collect();
        let mut spawned = Vec::new();

        // Choose wonders based on weight
        while !all_natural_wonders.is_empty() && chosen_wonders.len() < number_to_spawn as usize {
            let total_weight: f64 = all_natural_wonders.iter().map(|w| w.weight as f64).sum();
            let random = self.randomness.rng.gen::<f64>();
            let mut sum = 0.0;

            for (i, wonder) in all_natural_wonders.iter().enumerate() {
                sum += wonder.weight as f64 / total_weight;
                if random <= sum {
                    chosen_wonders.push(wonder.clone());
                    all_natural_wonders.remove(i);
                    break;
                }
            }
        }

        // Get tiles too close to spawn locations
        let tiles_too_close_to_spawn_locations: HashSet<Tile> = tile_map.starting_locations_by_nation.values()
            .flatten()
            .flat_map(|loc| loc.get_tiles_in_distance(5))
            .collect();

        // First attempt to spawn the chosen wonders in order of least candidate tiles
        for wonder in &chosen_wonders {
            wonder_candidate_tiles.insert(wonder.clone(),
                self.get_candidate_tiles_for_wonder(tile_map, wonder, &tiles_too_close_to_spawn_locations));
        }

        // Sort wonders by number of candidate tiles (ascending)
        chosen_wonders.sort_by(|a, b| {
            wonder_candidate_tiles.get(a).unwrap().len().cmp(&wonder_candidate_tiles.get(b).unwrap().len())
        });

        for wonder in &chosen_wonders {
            let candidate_tiles: Vec<Tile> = wonder_candidate_tiles.get(wonder).unwrap()
                .iter()
                .filter(|t| !self.blocked_tiles.contains(*t))
                .cloned()
                .collect();

            if self.try_spawn_on_suitable_location(candidate_tiles, wonder) {
                spawned.push(wonder.clone());
            }
        }

        // If some wonders were not able to be spawned we will pull a wonder from the fallback list
        if spawned.len() < number_to_spawn as usize {
            // Now we have to do some more calculations. Unfortunately we have to calculate candidate tiles for everyone.
            for wonder in &all_natural_wonders {
                wonder_candidate_tiles.insert(wonder.clone(),
                    self.get_candidate_tiles_for_wonder(tile_map, wonder, &tiles_too_close_to_spawn_locations));
            }

            // Sort wonders by number of candidate tiles (ascending)
            all_natural_wonders.sort_by(|a, b| {
                wonder_candidate_tiles.get(a).unwrap().len().cmp(&wonder_candidate_tiles.get(b).unwrap().len())
            });

            for wonder in &all_natural_wonders {
                let candidate_tiles: Vec<Tile> = wonder_candidate_tiles.get(wonder).unwrap()
                    .iter()
                    .filter(|t| !self.blocked_tiles.contains(*t))
                    .cloned()
                    .collect();

                if self.try_spawn_on_suitable_location(candidate_tiles, wonder) {
                    spawned.push(wonder.clone());
                }

                if spawned.len() >= number_to_spawn as usize {
                    break;
                }
            }
        }

        debug!("Natural Wonders for this game: {:?}", spawned);
    }

    /// Gets candidate tiles for a natural wonder
    fn get_candidate_tiles_for_wonder(&self, tile_map: &TileMap, natural_wonder: &Terrain,
                                     tiles_too_close_to_spawn_locations: &HashSet<Tile>) -> HashSet<Tile> {
        let suitable_locations: HashSet<Tile> = tile_map.values()
            .filter(|tile| {
                tile.resource.is_none() &&
                !tiles_too_close_to_spawn_locations.contains(tile) &&
                natural_wonder.occurs_on.contains(&tile.last_terrain.name) &&
                Self::fits_terrain_uniques(natural_wonder, tile)
            })
            .cloned()
            .collect();

        suitable_locations
    }

    /// Tries to spawn a natural wonder on a suitable location
    fn try_spawn_on_suitable_location(&mut self, suitable_locations: Vec<Tile>, wonder: &Terrain) -> bool {
        let (min_group_size, max_group_size) = if let Some(group_unique) = wonder.get_matching_uniques(UniqueType::NaturalWonderGroups).first() {
            (group_unique.get_int_param(0), group_unique.get_int_param(1))
        } else {
            (1, 1)
        };

        let target_group_size = if min_group_size == max_group_size {
            max_group_size
        } else {
            self.randomness.rng.gen_range(min_group_size..=max_group_size)
        };

        if suitable_locations.len() >= min_group_size as usize {
            let location = suitable_locations[self.randomness.rng.gen_range(0..suitable_locations.len())].clone();
            let mut list = vec![location.clone()];

            while list.len() < target_group_size as usize {
                let all_neighbors: HashSet<Tile> = list.iter()
                    .flat_map(|t| t.neighbors.iter().cloned())
                    .filter(|t| !list.contains(t))
                    .collect();

                let candidates: Vec<Tile> = suitable_locations.iter()
                    .filter(|t| all_neighbors.contains(t))
                    .cloned()
                    .collect();

                if candidates.is_empty() {
                    break;
                }

                list.push(candidates[self.randomness.rng.gen_range(0..candidates.len())].clone());
            }

            if list.len() >= min_group_size as usize {
                for tile_to_convert in &list {
                    Self::place_natural_wonder(wonder, tile_to_convert);
                    // Add all tiles within a certain distance to a blacklist so NW:s don't cluster
                    let blocked_distance = tile_to_convert.tile_map.map_parameters.map_size.height / 5;
                    self.blocked_tiles.extend(tile_to_convert.get_tiles_in_distance(blocked_distance));
                }

                debug!("Natural Wonder {} @{:?}", wonder.name, location.position);
                return true;
            }
        }

        debug!("No suitable location for {}", wonder.name);
        false
    }

    /// Places a natural wonder on a tile
    pub fn place_natural_wonder(wonder: &Terrain, location: &Tile) {
        location.natural_wonder = Some(wonder.name.clone());

        if let Some(turns_into_object) = location.ruleset.terrains.get(&wonder.turns_into) {
            Self::clear_tile(location);
            location.set_base_terrain(turns_into_object);
        } else {
            Self::clear_tile(location, &wonder.occurs_on);
        }

        let conversion_uniques = wonder.get_matching_uniques(UniqueType::NaturalWonderConvertNeighbors, StateForConditionals::IgnoreConditionals);
        if conversion_uniques.is_empty() {
            return;
        }

        for tile in &location.neighbors {
            let state = StateForConditionals::new(tile.clone());
            for unique in &conversion_uniques {
                if !unique.conditionals_apply(&state) {
                    continue;
                }

                let convert_to = &unique.params[0];
                if tile.base_terrain.name == *convert_to || tile.terrain_features.contains(convert_to) {
                    continue;
                }

                if *convert_to == Constants::LAKES && tile.is_coastal_tile() {
                    continue;
                }

                let terrain_object = match location.ruleset.terrains.get(convert_to) {
                    Some(obj) => obj,
                    None => continue,
                };

                if terrain_object.terrain_type == TerrainType::TerrainFeature &&
                   !terrain_object.occurs_on.contains(&tile.base_terrain.name) {
                    continue;
                }

                if *convert_to == Constants::COAST {
                    Self::remove_lakes_next_to_future_coast(location, tile);
                }

                if terrain_object.terrain_type.is_base_terrain() {
                    Self::clear_tile(tile);
                    tile.set_base_terrain(terrain_object);
                }

                if terrain_object.terrain_type == TerrainType::TerrainFeature {
                    Self::clear_tile(tile, &tile.terrain_features);
                    tile.add_terrain_feature(convert_to);
                }
            }
        }
    }

    /// Checks if a tile fits the terrain uniques for a natural wonder
    pub fn fits_terrain_uniques(natural_wonder_or_terrain_feature: &Terrain, tile: &Tile) -> bool {
        let continents_relevant = natural_wonder_or_terrain_feature.has_unique(UniqueType::NaturalWonderLargerLandmass) ||
                natural_wonder_or_terrain_feature.has_unique(UniqueType::NaturalWonderSmallerLandmass);

        let sorted_continents = if continents_relevant {
            let mut continents: Vec<_> = tile.tile_map.continent_sizes.iter()
                .collect();
            continents.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            continents.into_iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        natural_wonder_or_terrain_feature.unique_objects.iter().all(|unique| {
            match unique.unique_type {
                UniqueType::NaturalWonderNeighborCount => {
                    let count = tile.neighbors.iter()
                        .filter(|t| Self::matches_wonder_filter(t, &unique.params[1]))
                        .count();
                    count == unique.get_int_param(0) as usize
                }

                UniqueType::NaturalWonderNeighborsRange => {
                    let count = tile.neighbors.iter()
                        .filter(|t| Self::matches_wonder_filter(t, &unique.params[2]))
                        .count();
                    count >= unique.get_int_param(0) as usize && count <= unique.get_int_param(1) as usize
                }

                UniqueType::NaturalWonderSmallerLandmass => {
                    !sorted_continents.iter().take(unique.get_int_param(0) as usize)
                        .any(|c| c == &tile.get_continent())
                }

                UniqueType::NaturalWonderLargerLandmass => {
                    sorted_continents.iter().take(unique.get_int_param(0) as usize)
                        .any(|c| c == &tile.get_continent())
                }

                UniqueType::NaturalWonderLatitude => {
                    let lower = tile.tile_map.max_latitude * unique.get_int_param(0) as f32 * 0.01;
                    let upper = tile.tile_map.max_latitude * unique.get_int_param(1) as f32 * 0.01;
                    let abs_lat = tile.latitude.abs();
                    abs_lat >= lower && abs_lat <= upper
                }

                _ => true
            }
        })
    }

    /// Gets an integer parameter from a unique
    fn get_int_param(unique: &Unique, index: usize) -> i32 {
        unique.params[index].parse().unwrap_or(0)
    }

    /// Removes lakes next to future coast
    fn remove_lakes_next_to_future_coast(location: &Tile, tile: &Tile) {
        for neighbor in &tile.neighbors {
            // This is so we don't have this tile turn into Coast, and then it's touching a Lake tile.
            // We just turn the lake tiles into this kind of tile.
            if neighbor.base_terrain.name == Constants::LAKES {
                Self::clear_tile(neighbor);
                neighbor.base_terrain = tile.base_terrain.clone();
                neighbor.set_terrain_transients();
            }
        }
        location.set_connected_by_river(tile, false);
    }

    /// Checks if a tile matches a wonder filter
    fn matches_wonder_filter(tile: &Tile, filter: &str) -> bool {
        match filter {
            "Elevated" => tile.base_terrain.name == Constants::MOUNTAIN || tile.is_hill(),
            "Water" => tile.is_water(),
            "Land" => tile.is_land(),
            name if name == Constants::HILL => tile.is_hill(),
            name if name == tile.natural_wonder.as_ref().unwrap_or(&String::new()) => true,
            name if name == tile.last_terrain.name => true,
            _ => tile.base_terrain.name == filter
        }
    }

    /// Clears a tile
    fn clear_tile(tile: &Tile, except_features: &[String]) {
        if !tile.terrain_features.is_empty() && except_features != &tile.terrain_features {
            tile.set_terrain_features(tile.terrain_features.iter()
                .filter(|f| except_features.contains(f))
                .cloned()
                .collect());
        }
        tile.resource = None;
        tile.remove_improvement();
        tile.set_terrain_transients();
    }
}

/// Randomness generator for map generation
pub struct MapGenerationRandomness {
    pub rng: rand::rngs::StdRng,
}

impl MapGenerationRandomness {
    /// Creates a new MapGenerationRandomness with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            rng: rand::SeedableRng::seed_from_u64(seed),
        }
    }
}

/// Extension trait for Unique to get integer parameters
pub trait UniqueExt {
    /// Gets an integer parameter from a unique
    fn get_int_param(&self, index: usize) -> i32;
}

impl UniqueExt for Unique {
    fn get_int_param(&self, index: usize) -> i32 {
        self.params.get(index)
            .and_then(|p| p.parse().ok())
            .unwrap_or(0)
    }
}