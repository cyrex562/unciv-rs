use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};
use crate::civilization::Civilization;
use crate::city::City;
use crate::map::tile::Tile;
use crate::map::mapunit::MapUnit;
use crate::ui::screens::victoryscreen::RankingType;

/// Handles optimized operations related to finding threats or allies in an area.
#[derive(Clone, Serialize, Deserialize)]
pub struct ThreatManager {
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// Stores the distance to closest enemy tiles for each tile
    #[serde(skip)]
    distance_to_closest_enemy_tiles: HashMap<Tile, ClosestEnemyTileData>,
}

/// Data structure for tracking closest enemy tiles
#[derive(Clone, Serialize, Deserialize)]
pub struct ClosestEnemyTileData {
    /// The farthest radius in which we have checked tiles for enemies.
    /// A value of 2 means all enemies at a radius of 2 are in tiles_with_enemies.
    pub distance_searched: i32,

    /// Stores the location of the enemy tiles that we saw with the distance at which we saw them.
    /// Tiles are sorted by distance in increasing order.
    /// This allows us to quickly check if they are still alive and if we should search farther.
    /// It is not guaranteed that each tile in this list has an enemy (since they may have died).
    pub tiles_with_enemies: VecDeque<(Tile, i32)>,
}

impl ThreatManager {
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        Self {
            civ_info: Some(civ_info),
            distance_to_closest_enemy_tiles: HashMap::new(),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            civ_info: self.civ_info.clone(),
            distance_to_closest_enemy_tiles: self.distance_to_closest_enemy_tiles.clone(),
        }
    }

    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info);
    }

    /// Gets the distance to the closest visible enemy unit or city.
    /// The result value is cached and since it is called each turn in NextTurnAutomation.getUnitPriority
    /// each subsequent calls are likely to be free.
    pub fn get_distance_to_closest_enemy_unit(&self, tile: &Tile, max_dist: i32, take_larger_values: bool) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let tile_data = self.distance_to_closest_enemy_tiles.get(tile);

        // Needs to be a high value, but not the max value so we can still add to it. Example: nextTurnAutomation sorting
        let not_found_distance = if take_larger_values { 500000 } else { max_dist };
        let mut min_distance_to_search = 1;

        // Look if we can return the cache or if we can reduce our search
        if let Some(tile_data) = tile_data {
            let mut tiles_with_enemies = tile_data.tiles_with_enemies.clone();

            // Check the tiles where we have previously found an enemy, if so it must be the closest
            while !tiles_with_enemies.is_empty() {
                let enemy_tile = tiles_with_enemies.front().unwrap();
                if self.does_tile_have_military_enemy(&enemy_tile.0) {
                    return if take_larger_values { enemy_tile.1 } else { enemy_tile.1.min(max_dist) };
                } else {
                    // This tile is no longer valid
                    tiles_with_enemies.pop_front();
                }
            }

            if tile_data.distance_searched > max_dist {
                // We have already searched past the range we want to search and haven't found any enemies
                return if take_larger_values { not_found_distance } else { max_dist };
            }

            // Only search the tiles that we haven't searched yet
            min_distance_to_search = (tile_data.distance_searched + 1).max(1);
        }

        if let Some(tile_data) = tile_data {
            assert!(tile_data.tiles_with_enemies.is_empty(), "There must be no elements in tile.data.tiles_with_enemies at this point");
        }

        let mut tiles_with_enemy_at_distance = VecDeque::new();

        // Search for nearby enemies and store the results
        for i in min_distance_to_search..=max_dist {
            for search_tile in tile.get_tiles_at_distance(i) {
                if self.does_tile_have_military_enemy(&search_tile) {
                    tiles_with_enemy_at_distance.push_back((search_tile, i));
                }
            }
            if !tiles_with_enemy_at_distance.is_empty() {
                self.distance_to_closest_enemy_tiles.insert(
                    tile.clone(),
                    ClosestEnemyTileData {
                        distance_searched: i,
                        tiles_with_enemies: tiles_with_enemy_at_distance,
                    },
                );
                return i;
            }
        }

        self.distance_to_closest_enemy_tiles.insert(
            tile.clone(),
            ClosestEnemyTileData {
                distance_searched: max_dist,
                tiles_with_enemies: VecDeque::new(),
            },
        );
        not_found_distance
    }

    /// Returns all tiles with enemy units on them in distance.
    /// Every tile is guaranteed to have an enemy.
    /// May be quicker than a manual search because of caching.
    /// Also ends up calculating and caching get_distance_to_closest_enemy_unit.
    pub fn get_tiles_with_enemy_units_in_distance(&self, tile: &Tile, max_dist: i32) -> Vec<Tile> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let tile_data = self.distance_to_closest_enemy_tiles.get(tile);

        // The list of tiles that we will return
        let mut tiles_with_enemies = Vec::new();
        // The list of tiles with distance that will be stored in distance_to_closest_enemy_tiles
        let mut tile_data_tiles_with_enemies = if let Some(data) = tile_data {
            data.tiles_with_enemies.clone()
        } else {
            VecDeque::new()
        };

        if let Some(tile_data) = tile_data {
            if tile_data.distance_searched >= max_dist {
                // Add all tiles that we have previously found
                let mut tiles_with_enemies_iterator = tile_data_tiles_with_enemies.iter();
                while let Some(tile_with_distance) = tiles_with_enemies_iterator.next() {
                    // Check if the next tile is out of our search range, if so lets stop here
                    if tile_with_distance.1 > max_dist {
                        return tiles_with_enemies;
                    }
                    // Check if the threat on the tile is still present
                    if self.does_tile_have_military_enemy(&tile_with_distance.0) {
                        tiles_with_enemies.push(tile_with_distance.0.clone());
                    }
                }
            }
        }

        // We don't need to search for anything more if we have previously searched past max_dist
        if let Some(tile_data) = tile_data {
            if max_dist <= tile_data.distance_searched {
                return tiles_with_enemies;
            }
        }

        // Search all tiles that haven't been searched yet up until max_dist
        let min_distance_to_search = (tile_data.map(|d| d.distance_searched).unwrap_or(0) + 1).max(1);

        for i in min_distance_to_search..=max_dist {
            for search_tile in tile.get_tiles_at_distance(i) {
                if self.does_tile_have_military_enemy(&search_tile) {
                    tiles_with_enemies.push(search_tile.clone());
                    tile_data_tiles_with_enemies.push_back((search_tile, i));
                }
            }
        }

        if let Some(tile_data) = tile_data {
            tile_data.distance_searched = tile_data.distance_searched.max(max_dist);
        } else {
            // Cache our results for later
            self.distance_to_closest_enemy_tiles.insert(
                tile.clone(),
                ClosestEnemyTileData {
                    distance_searched: max_dist,
                    tiles_with_enemies: tile_data_tiles_with_enemies,
                },
            );
        }
        tiles_with_enemies
    }

    /// Returns all enemy military units within max_distance of the tile.
    pub fn get_enemy_military_units_in_distance(&self, tile: &Tile, max_dist: i32) -> Vec<MapUnit> {
        self.get_enemy_units_on_tiles(&self.get_tiles_with_enemy_units_in_distance(tile, max_dist))
    }

    /// Returns all enemy military units on the given tiles.
    pub fn get_enemy_units_on_tiles(&self, tiles_with_enemy_units_in_distance: &[Tile]) -> Vec<MapUnit> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        tiles_with_enemy_units_in_distance
            .iter()
            .flat_map(|enemy_tile| enemy_tile.get_units())
            .filter(|unit| unit.is_military() && civ_info.is_at_war_with(&unit.civ))
            .collect()
    }

    /// Returns tiles that are dangerous for the given unit.
    pub fn get_dangerous_tiles(&self, unit: &MapUnit, distance: i32) -> HashSet<Tile> {
        let tiles_with_enemy_units = self.get_tiles_with_enemy_units_in_distance(&unit.get_tile(), distance);
        let nearby_ranged_enemy_units = self.get_enemy_units_on_tiles(&tiles_with_enemy_units);

        let tiles_in_range_of_attack = nearby_ranged_enemy_units
            .iter()
            .flat_map(|unit| unit.get_tile().get_tiles_in_distance(unit.get_range()));

        let tiles_within_bombardment_range = tiles_with_enemy_units
            .iter()
            .filter(|tile| tile.is_city_center() && tile.get_city().unwrap().civ.is_at_war_with(&unit.civ))
            .flat_map(|tile| tile.get_tiles_in_distance(tile.get_city().unwrap().get_bombard_range()));

        let tiles_with_terrain_damage = unit.current_tile
            .get_tiles_in_distance(distance)
            .into_iter()
            .filter(|tile| unit.get_damage_from_terrain(tile) > 0);

        tiles_in_range_of_attack
            .chain(tiles_within_bombardment_range)
            .chain(tiles_with_terrain_damage)
            .collect()
    }

    /// Returns true if the tile has a visible enemy, otherwise returns false.
    pub fn does_tile_have_military_enemy(&self, tile: &Tile) -> bool {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        if !tile.is_explored(civ_info) {
            return false;
        }
        if tile.is_city_center() && tile.get_city().unwrap().civ.is_at_war_with(civ_info) {
            return true;
        }
        if !tile.is_visible(civ_info) {
            return false;
        }
        tile.get_units().iter().any(|unit| {
            unit.is_military() && unit.civ.is_at_war_with(civ_info) && !unit.is_invisible(civ_info)
        })
    }

    /// Returns a sequence of pairs of cities, the first city is our city and the second city is a nearby city that is not from our civ.
    pub fn get_neighboring_cities_of_other_civs(&self) -> Vec<(City, City)> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        civ_info.cities
            .iter()
            .flat_map(|our_city| {
                our_city.neighboring_cities
                    .iter()
                    .filter(|city| city.civ != *civ_info)
                    .map(|city| (our_city.clone(), city.clone()))
            })
            .collect()
    }

    /// Returns all neighboring civilizations.
    pub fn get_neighboring_civilizations(&self) -> HashSet<Arc<Civilization>> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        civ_info.cities
            .iter()
            .flat_map(|city| city.neighboring_cities.iter())
            .filter(|city| city.civ != *civ_info && civ_info.knows(&city.civ))
            .map(|city| city.civ.clone())
            .collect()
    }

    /// Returns the combined force of all warring civilizations.
    pub fn get_combined_force_of_warring_civs(&self) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        civ_info.get_civs_at_war_with()
            .iter()
            .map(|civ| civ.get_stat_for_ranking(RankingType::Force))
            .sum()
    }

    /// Clears all cached data.
    pub fn clear(&mut self) {
        self.distance_to_closest_enemy_tiles.clear();
    }
}