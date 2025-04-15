use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use rand::Rng;

use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::UniqueType;
use crate::utils::debug;
use crate::constants::Constants;

/// Handles River generation for MapGenerator, UniqueType.OneTimeChangeTerrain and console.
///
/// Map generation follows the vertices of map hexes (RiverCoordinate): spawn_rivers.
/// In-game new rivers work on edges: continue_river_on.
pub struct RiverGenerator<'a> {
    tile_map: &'a TileMap,
    randomness: &'a MapGenerationRandomness,
    river_count_multiplier: f64,
    min_river_length: i32,
    max_river_length: i32,
}

impl<'a> RiverGenerator<'a> {
    /// Creates a new RiverGenerator
    pub fn new(
        tile_map: &'a TileMap,
        randomness: &'a MapGenerationRandomness,
        ruleset: &Ruleset
    ) -> Self {
        let river_count_multiplier = ruleset.mod_options.constants.river_count_multiplier;
        let min_river_length = ruleset.mod_options.constants.min_river_length;
        let max_river_length = ruleset.mod_options.constants.max_river_length;

        Self {
            tile_map,
            randomness,
            river_count_multiplier,
            min_river_length,
            max_river_length,
        }
    }

    /// Spawns rivers on the map
    ///
    /// # Arguments
    ///
    /// * `resulting_tiles` - Optional set to store affected tiles for map editor
    pub fn spawn_rivers(&self, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        if !self.tile_map.values().any(|t| t.is_water()) {
            return;
        }

        let land_tile_count = self.tile_map.values().filter(|t| t.is_land()).count();
        let number_of_rivers = (land_tile_count as f64 * self.river_count_multiplier).round() as i32;

        // Find potential starting points for rivers
        let mut optional_tiles: Vec<&Tile> = self.tile_map.values()
            .filter(|t| t.base_terrain() == Constants::MOUNTAIN && self.is_far_enough_from_water(t))
            .collect();

        if optional_tiles.len() < number_of_rivers as usize {
            let hill_tiles: Vec<&Tile> = self.tile_map.values()
                .filter(|t| t.is_hill() && self.is_far_enough_from_water(t))
                .collect();
            optional_tiles.extend(hill_tiles);
        }

        if optional_tiles.len() < number_of_rivers as usize {
            optional_tiles = self.tile_map.values()
                .filter(|t| t.is_land() && self.is_far_enough_from_water(t))
                .collect();
        }

        let map_radius = self.tile_map.map_parameters.map_size.radius;
        let river_starts = self.randomness.choose_spread_out_locations(
            number_of_rivers,
            &optional_tiles,
            map_radius
        );

        for tile in river_starts {
            self.spawn_river(tile, resulting_tiles);
        }
    }

    /// Checks if a tile is far enough from water to be a river source
    fn is_far_enough_from_water(&self, tile: &Tile) -> bool {
        for distance in 1..self.min_river_length {
            if tile.get_tiles_at_distance(distance).iter().any(|t| t.is_water()) {
                return false;
            }
        }
        true
    }

    /// Gets the closest water tile to the given tile
    pub fn get_closest_water_tile(&self, tile: &Tile) -> Option<&Tile> {
        for distance in 1..=self.max_river_length {
            let water_tiles: Vec<&Tile> = tile.get_tiles_at_distance(distance)
                .iter()
                .filter(|t| t.is_water())
                .collect();

            if !water_tiles.is_empty() {
                let mut rng = rand::thread_rng();
                return Some(water_tiles[rng.gen_range(0..water_tiles.len())]);
            }
        }
        None
    }

    /// Spawns a river from an initial position
    fn spawn_river(&self, initial_position: &Tile, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        let end_position = self.get_closest_water_tile(initial_position)
            .expect("No water found for river destination");

        self.spawn_river_between(initial_position, end_position, resulting_tiles);
    }

    /// Spawns a river between two positions
    ///
    /// # Arguments
    ///
    /// * `initial_position` - Starting position of the river
    /// * `end_position` - Ending position of the river
    /// * `resulting_tiles` - Optional set to store affected tiles for map editor
    pub fn spawn_river_between(
        &self,
        initial_position: &Tile,
        end_position: &Tile,
        resulting_tiles: Option<&mut HashSet<&Tile>>
    ) {
        // Recommendation: Draw a bunch of hexagons on paper before trying to understand this, it's super helpful!
        let mut rng = rand::thread_rng();
        let bottom_right_or_left = if rng.gen_bool(0.5) {
            RiverCoordinate::BottomRightOrLeft::BottomLeft
        } else {
            RiverCoordinate::BottomRightOrLeft::BottomRight
        };

        let mut river_coordinate = RiverCoordinate::new(
            self.tile_map,
            initial_position.position,
            bottom_right_or_left
        );

        for _ in 0..self.max_river_length {
            if river_coordinate.get_adjacent_tiles().iter().any(|t| t.is_water()) {
                return;
            }

            let possible_coordinates = river_coordinate.get_adjacent_positions();
            if possible_coordinates.is_empty() {
                return; // end of the line
            }

            // Group coordinates by their minimum distance to the end position
            let mut distance_groups: HashMap<i32, Vec<RiverCoordinate>> = HashMap::new();

            for new_coordinate in possible_coordinates {
                let min_distance = new_coordinate.get_adjacent_tiles()
                    .iter()
                    .map(|t| t.aerial_distance_to(end_position))
                    .min()
                    .unwrap_or(i32::MAX);

                distance_groups.entry(min_distance)
                    .or_insert_with(Vec::new)
                    .push(new_coordinate);
            }

            // Find the group with the minimum distance
            let min_distance = distance_groups.keys().min().unwrap();
            let min_distance_group = &distance_groups[min_distance];

            // Choose a random coordinate from the group
            let new_coordinate = &min_distance_group[rng.gen_range(0..min_distance_group.len())];

            // Set one new river edge in place
            river_coordinate.paint_to(new_coordinate, resulting_tiles);

            // Move on
            river_coordinate = new_coordinate.clone();
        }

        debug!("River reached max length!");
    }
}

/// Describes a Vertex on our hexagonal grid via a neighboring hex and clock direction, normalized
/// such that always the north-most hex and one of the two clock directions 5 / 7 o'clock are used.
#[derive(Clone)]
struct RiverCoordinate<'a> {
    tile_map: &'a TileMap,
    position: Vector2,
    bottom_right_or_left: BottomRightOrLeft,
    x: i32,
    y: i32,
    my_tile: &'a Tile,
    my_top_left: Option<&'a Tile>,
    my_bottom_left: Option<&'a Tile>,
    my_top_right: Option<&'a Tile>,
    my_bottom_right: Option<&'a Tile>,
    my_bottom_center: Option<&'a Tile>,
}

impl<'a> RiverCoordinate<'a> {
    /// Bottom right or left direction for river coordinates
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BottomRightOrLeft {
        /// 7 O'Clock of the tile
        BottomLeft,
        /// 5 O'Clock of the tile
        BottomRight
    }

    /// Creates a new RiverCoordinate
    fn new(
        tile_map: &'a TileMap,
        position: Vector2,
        bottom_right_or_left: BottomRightOrLeft
    ) -> Self {
        let x = position.x as i32;
        let y = position.y as i32;

        let my_tile = tile_map.get_tile_at_position(&position).unwrap();
        let my_top_left = tile_map.get_if_tile_exists_or_null(x + 1, y);
        let my_bottom_left = tile_map.get_if_tile_exists_or_null(x, y - 1);
        let my_top_right = tile_map.get_if_tile_exists_or_null(x, y + 1);
        let my_bottom_right = tile_map.get_if_tile_exists_or_null(x - 1, y);
        let my_bottom_center = tile_map.get_if_tile_exists_or_null(x - 1, y - 1);

        Self {
            tile_map,
            position,
            bottom_right_or_left,
            x,
            y,
            my_tile,
            my_top_left,
            my_bottom_left,
            my_top_right,
            my_bottom_right,
            my_bottom_center,
        }
    }

    /// Lists the three neighboring vertices which have their anchor hex on the map
    fn get_adjacent_positions(&self) -> Vec<RiverCoordinate<'a>> {
        let mut positions = Vec::new();

        // What's nice is that adjacents are always the OPPOSITE in terms of right-left - rights are adjacent to only lefts, and vice-versa
        if self.bottom_right_or_left == BottomRightOrLeft::BottomLeft {
            // Same tile, other side
            positions.push(RiverCoordinate::new(
                self.tile_map,
                self.position,
                BottomRightOrLeft::BottomRight
            ));

            // Tile to MY top-left, take its bottom right corner
            if let Some(top_left) = self.my_top_left {
                positions.push(RiverCoordinate::new(
                    self.tile_map,
                    top_left.position,
                    BottomRightOrLeft::BottomRight
                ));
            }

            // Tile to MY bottom-left, take its bottom right
            if let Some(bottom_left) = self.my_bottom_left {
                positions.push(RiverCoordinate::new(
                    self.tile_map,
                    bottom_left.position,
                    BottomRightOrLeft::BottomRight
                ));
            }
        } else {
            // Same tile, other side
            positions.push(RiverCoordinate::new(
                self.tile_map,
                self.position,
                BottomRightOrLeft::BottomLeft
            ));

            // Tile to MY top-right, take its bottom left
            if let Some(top_right) = self.my_top_right {
                positions.push(RiverCoordinate::new(
                    self.tile_map,
                    top_right.position,
                    BottomRightOrLeft::BottomLeft
                ));
            }

            // Tile to MY bottom-right, take its bottom left
            if let Some(bottom_right) = self.my_bottom_right {
                positions.push(RiverCoordinate::new(
                    self.tile_map,
                    bottom_right.position,
                    BottomRightOrLeft::BottomLeft
                ));
            }
        }

        positions
    }

    /// Lists the three neighboring hexes to this vertex which are on the map
    fn get_adjacent_tiles(&self) -> Vec<&'a Tile> {
        let mut tiles = Vec::new();

        tiles.push(self.my_tile);

        if let Some(bottom_center) = self.my_bottom_center {
            tiles.push(bottom_center);
        }

        if self.bottom_right_or_left == BottomRightOrLeft::BottomLeft {
            if let Some(bottom_left) = self.my_bottom_left {
                tiles.push(bottom_left);
            }
        } else {
            if let Some(bottom_right) = self.my_bottom_right {
                tiles.push(bottom_right);
            }
        }

        tiles
    }

    /// Paints a river from this coordinate to the new coordinate
    fn paint_to(&self, new_coordinate: &RiverCoordinate<'a>, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        if new_coordinate.position == self.position {
            // Same tile, switched right-to-left
            self.paint_bottom(resulting_tiles);
        } else if self.bottom_right_or_left == BottomRightOrLeft::BottomRight {
            if new_coordinate.get_adjacent_tiles().contains(&self.my_tile) {
                // Moved from our 5 O'Clock to our 3 O'Clock
                self.paint_bottom_right(resulting_tiles);
            } else {
                // Moved from our 5 O'Clock down in the 5 O'Clock direction - this is the 8 O'Clock river of the tile to our 4 O'Clock!
                new_coordinate.paint_bottom_left(resulting_tiles);
            }
        } else {
            // bottom_right_or_left == BottomRightOrLeft::BottomLeft
            if new_coordinate.get_adjacent_tiles().contains(&self.my_tile) {
                // Moved from our 7 O'Clock to our 9 O'Clock
                self.paint_bottom_left(resulting_tiles);
            } else {
                // Moved from our 7 O'Clock down in the 7 O'Clock direction
                new_coordinate.paint_bottom_right(resulting_tiles);
            }
        }
    }

    /// Paints a river at the bottom of the tile
    fn paint_bottom(&self, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        self.my_tile.set_has_bottom_river(true);

        if let Some(resulting_tiles) = resulting_tiles {
            resulting_tiles.insert(self.my_tile);

            if let Some(bottom_center) = self.my_bottom_center {
                resulting_tiles.insert(bottom_center);
            }
        }
    }

    /// Paints a river at the bottom left of the tile
    fn paint_bottom_left(&self, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        self.my_tile.set_has_bottom_left_river(true);

        if let Some(resulting_tiles) = resulting_tiles {
            resulting_tiles.insert(self.my_tile);

            if let Some(bottom_left) = self.my_bottom_left {
                resulting_tiles.insert(bottom_left);
            }
        }
    }

    /// Paints a river at the bottom right of the tile
    fn paint_bottom_right(&self, resulting_tiles: Option<&mut HashSet<&Tile>>) {
        self.my_tile.set_has_bottom_right_river(true);

        if let Some(resulting_tiles) = resulting_tiles {
            resulting_tiles.insert(self.my_tile);

            if let Some(bottom_right) = self.my_bottom_right {
                resulting_tiles.insert(bottom_right);
            }
        }
    }

    /// Count edges with a river from this vertex
    #[allow(dead_code)]
    fn number_of_connected_rivers(&self) -> i32 {
        let mut count = 0;

        if self.my_tile.has_bottom_river() {
            count += 1;
        }

        if self.bottom_right_or_left == BottomRightOrLeft::BottomLeft {
            if self.my_tile.has_bottom_left_river() {
                count += 1;
            }

            if let Some(bottom_left) = self.my_bottom_left {
                if bottom_left.has_bottom_right_river() {
                    count += 1;
                }
            }
        } else {
            if self.my_tile.has_bottom_right_river() {
                count += 1;
            }

            if let Some(bottom_right) = self.my_bottom_right {
                if bottom_right.has_bottom_left_river() {
                    count += 1;
                }
            }
        }

        count
    }
}

/// River directions in clock positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiverDirections {
    North = 12,
    NorthEast = 2,
    SouthEast = 4,
    South = 6,
    SouthWest = 8,
    NorthWest = 10,
}

impl RiverDirections {
    /// Gets the neighbor tile in the specified direction
    pub fn get_neighbor_tile(&self, selected_tile: &Tile) -> Option<&Tile> {
        let clock_position = *self as i32;
        selected_tile.tile_map.get_clock_position_neighbor_tile(selected_tile, clock_position)
    }

    /// Gets the names of all river directions
    pub fn names() -> Vec<String> {
        vec![
            "North".to_string(),
            "NorthEast".to_string(),
            "SouthEast".to_string(),
            "South".to_string(),
            "SouthWest".to_string(),
            "NorthWest".to_string(),
        ]
    }
}

impl RiverGenerator<'_> {
    /// UniqueType.OneTimeChangeTerrain tries to place a "River" feature.
    ///
    /// Operates on edges - while spawn_river hops from vertex (RiverCoordinate) to vertex!
    /// Placed here to make comparison easier, even though the implementation has nothing else in common.
    ///
    /// # Arguments
    ///
    /// * `tile` - The tile to place a river on
    ///
    /// # Returns
    ///
    /// success - one edge of tile has a new river
    pub fn continue_river_on(tile: &Tile) -> bool {
        if !tile.is_land() {
            return false;
        }

        let tile_map = tile.tile_map;

        /// Helper to prioritize a tile edge for river placement - accesses tile as closure,
        /// and considers the edge common with other_tile in direction clock_position.
        ///
        /// Will consider two additional tiles - those that are neighbor to both tile and other_tile,
        /// and four other edges - those connecting to "our" edge.
        struct NeighborData<'a> {
            other_tile: &'a Tile,
            clock_position: i32,
            is_connected_by_river: bool,
            edge_leads_to_sea: bool,
            connected_river_count: i32,
            vertices_form_y_count: i32,
        }

        impl<'a> NeighborData<'a> {
            fn new(tile: &Tile, other_tile: &'a Tile) -> Self {
                let tile_map = tile.tile_map;
                let clock_position = tile_map.get_neighbor_tile_clock_position(tile, other_tile);
                let is_connected_by_river = tile.is_connected_by_river(other_tile);

                // Similar: private fn Tile.get_left_shared_neighbor in TileLayerBorders
                let left_shared_neighbor = tile_map.get_clock_position_neighbor_tile(
                    tile,
                    (clock_position - 2).rem_euclid(12)
                );

                let right_shared_neighbor = tile_map.get_clock_position_neighbor_tile(
                    tile,
                    (clock_position + 2).rem_euclid(12)
                );

                let edge_leads_to_sea = left_shared_neighbor.map_or(false, |t| t.is_water()) ||
                                       right_shared_neighbor.map_or(false, |t| t.is_water());

                let mut connected_river_count = 0;
                if left_shared_neighbor.map_or(false, |t| t.is_connected_by_river(tile)) {
                    connected_river_count += 1;
                }
                if left_shared_neighbor.map_or(false, |t| t.is_connected_by_river(other_tile)) {
                    connected_river_count += 1;
                }
                if right_shared_neighbor.map_or(false, |t| t.is_connected_by_river(tile)) {
                    connected_river_count += 1;
                }
                if right_shared_neighbor.map_or(false, |t| t.is_connected_by_river(other_tile)) {
                    connected_river_count += 1;
                }

                let mut vertices_form_y_count = 0;
                if left_shared_neighbor.map_or(false, |t| t.is_connected_by_river(tile) && t.is_connected_by_river(other_tile)) {
                    vertices_form_y_count += 1;
                }
                if right_shared_neighbor.map_or(false, |t| t.is_connected_by_river(tile) && t.is_connected_by_river(other_tile)) {
                    vertices_form_y_count += 1;
                }

                Self {
                    other_tile,
                    clock_position,
                    is_connected_by_river,
                    edge_leads_to_sea,
                    connected_river_count,
                    vertices_form_y_count,
                }
            }

            fn get_priority(&self, edge_to_sea_priority: i32) -> i32 {
                // Choose a priority - only order matters, not magnitude
                if self.is_connected_by_river {
                    -9 // Ensures this isn't chosen, otherwise "cannot place another river" would have bailed
                } else if self.edge_leads_to_sea {
                    edge_to_sea_priority + self.connected_river_count - 3 * self.vertices_form_y_count
                } else if self.vertices_form_y_count == 2 {
                    -2 // Connect two bends
                } else if self.vertices_form_y_count == 1 {
                    self.connected_river_count * 2 - 5 // Connect a bend with an open end or connect to one open end
                } else {
                    self.connected_river_count * 2 // Connect nothing or connect two open ends
                }
            }
        }

        // Collect data (includes tiles we already have a river edge with - need the stats)
        let viable_neighbors: Vec<NeighborData> = tile.neighbors()
            .iter()
            .filter(|t| t.is_land())
            .map(|t| NeighborData::new(tile, t))
            .collect();

        if viable_neighbors.iter().all(|n| n.is_connected_by_river) {
            return false; // Cannot place another river
        }

        // Greatly encourage connecting to sea unless the tile already has a river to sea, in which case slightly discourage another one
        let edge_to_sea_priority = if viable_neighbors.iter().none(|n| n.is_connected_by_river && n.edge_leads_to_sea) {
            9
        } else {
            -1
        };

        // Group by priority
        let mut priority_groups: HashMap<i32, Vec<&NeighborData>> = HashMap::new();
        for neighbor in &viable_neighbors {
            let priority = neighbor.get_priority(edge_to_sea_priority);
            priority_groups.entry(priority)
                .or_insert_with(Vec::new)
                .push(neighbor);
        }

        // Get the group with the highest priority
        let max_priority = priority_groups.keys().max().unwrap();
        let best_group = &priority_groups[max_priority];

        // Choose a random neighbor from the best group
        let mut rng = rand::thread_rng();
        let choice = &best_group[rng.gen_range(0..best_group.len())];

        tile.set_connected_by_river(choice.other_tile, true, true)
    }
}

/// Vector2 struct for 2D coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    /// Creates a new Vector2
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// MapGenerationRandomness struct for random number generation
pub struct MapGenerationRandomness {
    pub rng: rand::rngs::StdRng,
}

impl MapGenerationRandomness {
    /// Creates a new MapGenerationRandomness
    pub fn new(seed: u64) -> Self {
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Chooses spread out locations for rivers
    pub fn choose_spread_out_locations<'a>(
        &self,
        count: i32,
        tiles: &[&'a Tile],
        map_radius: i32
    ) -> Vec<&'a Tile> {
        if tiles.is_empty() || count <= 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(count as usize);
        let mut available_tiles = tiles.to_vec();

        // Choose the first tile randomly
        let mut rng = self.rng.clone();
        let first_index = rng.gen_range(0..available_tiles.len());
        let first_tile = available_tiles.remove(first_index);
        result.push(first_tile);

        // Choose the remaining tiles
        for _ in 1..count {
            if available_tiles.is_empty() {
                break;
            }

            // Find the tile that is farthest from all chosen tiles
            let mut max_min_distance = -1;
            let mut best_index = 0;

            for (i, tile) in available_tiles.iter().enumerate() {
                let mut min_distance = i32::MAX;

                for chosen_tile in &result {
                    let distance = tile.aerial_distance_to(chosen_tile);
                    min_distance = min_distance.min(distance);
                }

                if min_distance > max_min_distance {
                    max_min_distance = min_distance;
                    best_index = i;
                }
            }

            // Add the chosen tile to the result
            let chosen_tile = available_tiles.remove(best_index);
            result.push(chosen_tile);
        }

        result
    }
}