use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::map_parameters::MapParameters;
use crate::tile::tile::Tile;
use crate::models::map_unit::MapUnit;
use crate::models::civilization::Civilization;
use crate::models::game_info::Position;
use crate::models::ruleset::Ruleset;

mod preview;

/// Represents a game map with all its properties and functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap {
    /// Map parameters like size, shape, etc.
    pub map_parameters: MapParameters,

    /// List of all tiles on the map
    pub tile_list: Vec<Tile>,

    /// Starting locations for civilizations
    pub starting_locations: Vec<StartingLocation>,

    /// Optional description of the map
    pub description: String,

    /// Transient fields (not serialized)
    #[serde(skip)]
    pub tile_matrix: Vec<Vec<Option<Tile>>>,

    #[serde(skip)]
    pub left_x: i32,

    #[serde(skip)]
    pub bottom_y: i32,

    #[serde(skip)]
    pub starting_locations_by_nation: HashMap<String, HashSet<Tile>>,

    #[serde(skip)]
    pub continent_sizes: HashMap<i32, i32>,
}

/// Represents a starting location for a civilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartingLocation {
    pub position: Position,
    pub nation: String,
    pub usage: StartingLocationUsage,
}

/// How a starting location may be used when the map is loaded for a new game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartingLocationUsage {
    /// Starting location only
    Normal,
    /// Use for "Select players from starting locations"
    Player,
    /// Use as first Human player
    Human,
}

impl Default for StartingLocationUsage {
    fn default() -> Self {
        StartingLocationUsage::Player
    }
}

impl TileMap {
    /// Creates a new empty TileMap
    pub fn new(initial_capacity: usize) -> Self {
        TileMap {
            map_parameters: MapParameters::default(),
            tile_list: Vec::with_capacity(initial_capacity),
            starting_locations: Vec::new(),
            description: String::new(),
            tile_matrix: Vec::new(),
            left_x: 0,
            bottom_y: 0,
            starting_locations_by_nation: HashMap::new(),
            continent_sizes: HashMap::new(),
        }
    }

    /// Creates a hexagonal map of given radius (filled with grassland)
    pub fn new_hexagonal(radius: i32, ruleset: &Ruleset, world_wrap: bool) -> Self {
        let mut tile_map = Self::new(0);
        tile_map.starting_locations.clear();

        // Get the first available land terrain
        let first_available_land_terrain = "Grassland"; // This would be determined by the ruleset

        // Create tiles in a hexagonal pattern
        // This is a simplified implementation
        for x in -radius..=radius {
            for y in -radius..=radius {
                // Check if the tile is within the hexagonal boundary
                if (x + y).abs() <= radius && (x - y).abs() <= radius {
                    let mut tile = Tile::new(x, y);
                    tile.base_terrain = Some(first_available_land_terrain.to_string());
                    tile_map.tile_list.push(tile);
                }
            }
        }

        // Set up the map parameters
        tile_map.map_parameters.shape = crate::models::map_parameters::MapShape::Hexagonal;
        tile_map.map_parameters.map_size.radius = radius;
        tile_map.map_parameters.world_wrap = world_wrap;

        // Set up transients
        tile_map.set_transients(ruleset, true);

        tile_map
    }

    /// Creates a rectangular map of given width and height (filled with grassland)
    pub fn new_rectangular(width: i32, height: i32, ruleset: &Ruleset, world_wrap: bool) -> Self {
        let mut tile_map = Self::new(0);
        tile_map.starting_locations.clear();

        // Get the first available land terrain
        let first_available_land_terrain = "Grassland"; // This would be determined by the ruleset

        // Adjust width for world wrap
        let wrap_adjusted_width = if world_wrap && width % 2 != 0 {
            width - 1
        } else {
            width
        };

        // Create tiles in a rectangular pattern
        for column in -wrap_adjusted_width / 2..=(wrap_adjusted_width - 1) / 2 {
            for row in -height / 2..=(height - 1) / 2 {
                let hex_coords = tile_map.column_row_to_hex_coords(column, row);
                let mut tile = Tile::new(hex_coords.x, hex_coords.y);
                tile.base_terrain = Some(first_available_land_terrain.to_string());
                tile_map.tile_list.push(tile);
            }
        }

        // Set up the map parameters
        tile_map.map_parameters.shape = crate::models::map_parameters::MapShape::Rectangular;
        tile_map.map_parameters.map_size.width = width;
        tile_map.map_parameters.map_size.height = height;
        tile_map.map_parameters.world_wrap = world_wrap;

        // Set up transients
        tile_map.set_transients(ruleset, true);

        tile_map
    }

    /// Gets a tile at the specified position
    pub fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        let matrix_x = x - self.left_x;
        let matrix_y = y - self.bottom_y;

        if matrix_x < 0 || matrix_y < 0 || matrix_x >= self.tile_matrix.len() as i32 || matrix_y >= self.tile_matrix[0].len() as i32 {
            return None;
        }

        self.tile_matrix[matrix_x as usize][matrix_y as usize].as_ref()
    }

    /// Checks if a position exists on the map
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_some()
    }

    /// Gets a tile at the specified position, respecting world wrap
    pub fn get_if_tile_exists_or_null(&self, x: i32, y: i32) -> Option<&Tile> {
        let tile = self.get(x, y);
        if tile.is_some() {
            return tile;
        }

        if !self.map_parameters.world_wrap {
            return None;
        }

        let radius = if self.map_parameters.shape == crate::models::map_parameters::MapShape::Rectangular {
            self.map_parameters.map_size.width / 2
        } else {
            self.map_parameters.map_size.radius
        };

        // Try wrapping around from right to left
        let right_side_tile = self.get(x + radius, y - radius);
        if right_side_tile.is_some() {
            return right_side_tile;
        }

        // Try wrapping around from left to right
        let left_side_tile = self.get(x - radius, y + radius);
        if left_side_tile.is_some() {
            return left_side_tile;
        }

        None
    }

    /// Gets all tiles within a certain distance of a position
    pub fn get_tiles_in_distance(&self, origin: Position, distance: i32) -> Vec<&Tile> {
        self.get_tiles_in_distance_range(origin, 0..=distance)
    }

    /// Gets all tiles within a range of distances from a position
    pub fn get_tiles_in_distance_range(&self, origin: Position, range: std::ops::RangeInclusive<i32>) -> Vec<&Tile> {
        let mut result = Vec::new();
        for distance in range {
            result.extend(self.get_tiles_at_distance(origin, distance));
        }
        result
    }

    /// Gets all tiles at a specific distance from a position
    pub fn get_tiles_at_distance(&self, origin: Position, distance: i32) -> Vec<&Tile> {
        if distance <= 0 {
            // If distance is 0 or negative, just return the origin tile
            if let Some(tile) = self.get(origin.x, origin.y) {
                return vec![tile];
            }
            return Vec::new();
        }

        let mut result = Vec::new();
        let center_x = origin.x;
        let center_y = origin.y;

        // Start from 6 O'clock point which means (-distance, -distance) away from the center point
        let mut current_x = center_x - distance;
        let mut current_y = center_y - distance;

        // From 6 to 8
        for _ in 0..distance {
            if let Some(tile) = self.get_if_tile_exists_or_null(current_x, current_y) {
                result.push(tile);
            }
            // Get the tile on the other side of the clock
            if let Some(tile) = self.get_if_tile_exists_or_null(2 * center_x - current_x, 2 * center_y - current_y) {
                result.push(tile);
            }
            current_x += 1; // Going upwards to the left, towards 8 o'clock
        }

        // From 8 to 10
        for _ in 0..distance {
            if let Some(tile) = self.get_if_tile_exists_or_null(current_x, current_y) {
                result.push(tile);
            }
            if let Some(tile) = self.get_if_tile_exists_or_null(2 * center_x - current_x, 2 * center_y - current_y) {
                result.push(tile);
            }
            current_x += 1;
            current_y += 1; // Going up the left side of the hexagon so we're going "up" - +1,+1
        }

        // From 10 to 12
        for _ in 0..distance {
            if let Some(tile) = self.get_if_tile_exists_or_null(current_x, current_y) {
                result.push(tile);
            }
            if let Some(tile) = self.get_if_tile_exists_or_null(2 * center_x - current_x, 2 * center_y - current_y) {
                result.push(tile);
            }
            current_y += 1; // Going up the top left side of the hexagon so we're heading "up and to the right"
        }

        result
    }

    /// Gets all tiles within a rectangle
    pub fn get_tiles_in_rectangle(&self, x: i32, y: i32, width: i32, height: i32) -> Vec<&Tile> {
        let mut result = Vec::new();
        for world_column_number in x..(x + width) {
            for world_row_number in y..(y + height) {
                // Convert rectangular coordinates to hex coordinates
                let hex_coords = self.column_row_to_hex_coords(world_column_number, world_row_number);
                if let Some(tile) = self.get_if_tile_exists_or_null(hex_coords.x, hex_coords.y) {
                    result.push(tile);
                }
            }
        }
        result
    }

    /// Converts column and row coordinates to hex coordinates
    fn column_row_to_hex_coords(&self, column: i32, row: i32) -> Position {
        // This is a simplified conversion - the actual implementation would depend on the map's orientation
        Position {
            x: column - row / 2,
            y: row,
        }
    }

    /// Gets the clock position of a neighbor tile
    pub fn get_neighbor_tile_clock_position(&self, tile: &Tile, other_tile: &Tile) -> i32 {
        let radius = if self.map_parameters.shape == crate::models::map_parameters::MapShape::Rectangular {
            self.map_parameters.map_size.width / 2
        } else {
            self.map_parameters.map_size.radius
        };

        let x1 = tile.position.x;
        let y1 = tile.position.y;
        let x2 = other_tile.position.x;
        let y2 = other_tile.position.y;

        let x_difference = x1 - x2;
        let y_difference = y1 - y2;
        let x_wrap_difference_bottom = if radius < 3 { 0 } else { x1 - (x2 - radius) };
        let y_wrap_difference_bottom = if radius < 3 { 0 } else { y1 - (y2 - radius) };
        let x_wrap_difference_top = if radius < 3 { 0 } else { x1 - (x2 + radius) };
        let y_wrap_difference_top = if radius < 3 { 0 } else { y1 - (y2 + radius) };

        if x_difference == 1 && y_difference == 1 {
            return 6; // otherTile is below
        } else if x_difference == -1 && y_difference == -1 {
            return 12; // otherTile is above
        } else if x_difference == 1 || x_wrap_difference_bottom == 1 {
            return 4; // otherTile is bottom-right
        } else if y_difference == 1 || y_wrap_difference_bottom == 1 {
            return 8; // otherTile is bottom-left
        } else if x_difference == -1 || x_wrap_difference_top == -1 {
            return 10; // otherTile is top-left
        } else if y_difference == -1 || y_wrap_difference_top == -1 {
            return 2; // otherTile is top-right
        } else {
            return -1; // Not neighbors
        }
    }

    /// Gets the neighbor tile at a specific clock position
    pub fn get_clock_position_neighbor_tile(&self, tile: &Tile, clock_position: i32) -> Option<&Tile> {
        let difference = self.clock_position_to_hex_vector(clock_position);
        if difference.x == 0 && difference.y == 0 {
            return None;
        }

        let possible_neighbor_position = Position {
            x: tile.position.x + difference.x,
            y: tile.position.y + difference.y,
        };

        self.get_if_tile_exists_or_null(possible_neighbor_position.x, possible_neighbor_position.y)
    }

    /// Converts a clock position to a hex vector
    fn clock_position_to_hex_vector(&self, clock_position: i32) -> Position {
        match clock_position {
            2 => Position { x: 0, y: -1 },  // top-right
            4 => Position { x: 1, y: 0 },   // bottom-right
            6 => Position { x: 1, y: 1 },   // bottom
            8 => Position { x: 0, y: 1 },   // bottom-left
            10 => Position { x: -1, y: 0 }, // top-left
            12 => Position { x: -1, y: -1 }, // top
            _ => Position { x: 0, y: 0 },   // invalid position
        }
    }

    /// Gets the unwrapped position for a given position
    pub fn get_unwrapped_position(&self, position: Position) -> Position {
        if !self.contains(position.x, position.y) {
            return position; // The position is outside the map so it's unwrapped already
        }

        let radius = if self.map_parameters.shape == crate::models::map_parameters::MapShape::Rectangular {
            self.map_parameters.map_size.width / 2
        } else {
            self.map_parameters.map_size.radius
        };

        let vector_unwrapped_left = Position {
            x: position.x + radius,
            y: position.y - radius,
        };

        let vector_unwrapped_right = Position {
            x: position.x - radius,
            y: position.y + radius,
        };

        // Return the position with the smaller magnitude
        if (vector_unwrapped_right.x * vector_unwrapped_right.x + vector_unwrapped_right.y * vector_unwrapped_right.y) <
           (vector_unwrapped_left.x * vector_unwrapped_left.x + vector_unwrapped_left.y * vector_unwrapped_left.y) {
            vector_unwrapped_right
        } else {
            vector_unwrapped_left
        }
    }

    /// Gets all viewable tiles from a position
    pub fn get_viewable_tiles(&self, position: Position, sight_distance: i32, for_attack: bool) -> Vec<&Tile> {
        // This is a simplified implementation
        // The actual implementation would be more complex and would depend on the game's mechanics

        let mut result = Vec::new();
        let origin_tile = self.get(position.x, position.y).unwrap();
        let unit_height = origin_tile.unit_height;

        // Add the origin tile
        result.push(origin_tile);

        // Add tiles within sight distance
        for distance in 1..=sight_distance {
            let tiles_at_distance = self.get_tiles_at_distance(position, distance);
            for tile in tiles_at_distance {
                // Check if the tile is visible based on height
                if unit_height >= tile.tile_height || for_attack {
                    result.push(tile);
                }
            }
        }

        result
    }

    /// Sets up the transient fields
    pub fn set_transients(&mut self, ruleset: &Ruleset, set_unit_civ_transients: bool) {
        // Initialize the tile matrix if it's empty
        if self.tile_matrix.is_empty() {
            let top_y = self.tile_list.iter().map(|t| t.position.y).max().unwrap_or(0);
            self.bottom_y = self.tile_list.iter().map(|t| t.position.y).min().unwrap_or(0);
            let right_x = self.tile_list.iter().map(|t| t.position.x).max().unwrap_or(0);
            self.left_x = self.tile_list.iter().map(|t| t.position.x).min().unwrap_or(0);

            // Initialize arrays with enough capacity to avoid re-allocations
            self.tile_matrix = vec![vec![None; (top_y - self.bottom_y + 1) as usize]; (right_x - self.left_x + 1) as usize];
        } else {
            // Check if the tile matrix size is appropriate
            let expected_size = -2 * self.left_x..(3 - 2 * self.left_x);
            if !expected_size.contains(&(self.tile_matrix.len() as i32)) {
                panic!("TileMap.set_transients called on existing tileMatrix of different size");
            }
        }

        // Fill the tile matrix
        for tile in &self.tile_list {
            let matrix_x = (tile.position.x - self.left_x) as usize;
            let matrix_y = (tile.position.y - self.bottom_y) as usize;
            self.tile_matrix[matrix_x][matrix_y] = Some(tile.clone());
        }

        // Set up tile transients
        for tile in &mut self.tile_list {
            // Set up terrain transients
            tile.update_terrain_properties();

            // Set up unit transients if needed
            if set_unit_civ_transients {
                // This would be implemented in the Tile struct
                // tile.set_unit_transients();
            }
        }
    }

    /// Sets up the neutral transients
    pub fn set_neutral_transients(&mut self) {
        for tile in &mut self.tile_list {
            // This would be implemented in the Tile struct
            // tile.set_owner_transients();
        }
    }

    /// Removes missing terrain mod references
    pub fn remove_missing_terrain_mod_references(&mut self, ruleset: &Ruleset) {
        for tile in &mut self.tile_list {
            // This would be implemented in the Tile struct
            // tile.remove_missing_terrain_mod_references(ruleset);
        }

        // Remove starting locations for nations that don't exist in the ruleset
        self.starting_locations.retain(|loc| {
            // Check if the nation exists in the ruleset
            // This would depend on how nations are stored in the ruleset
            true // Placeholder
        });
    }

    /// Places a unit near a tile
    pub fn place_unit_near_tile(&mut self, position: Position, unit_name: &str, civ_info: &Civilization, unit_id: Option<i32>) -> Option<MapUnit> {
        // This is a simplified implementation
        // The actual implementation would be more complex and would depend on the game's mechanics

        // Create a unit
        let mut unit = MapUnit::new(unit_name, civ_info, unit_id);

        // Try to place the unit at the original position
        let current_tile = self.get(position.x, position.y)?;

        // Check if the unit can move to the current tile
        if unit.can_move_to(current_tile) {
            unit.current_tile = Some(current_tile.clone());
            return Some(unit);
        }

        // If not, try to find a suitable tile nearby
        let mut try_count = 0;
        let mut potential_candidates = self.get_tiles_at_distance(position, 1);

        while try_count < 10 {
            // Find a suitable tile among the candidates
            let suitable_tile = potential_candidates.iter()
                .filter(|t| unit.can_move_to(t))
                .next();

            if let Some(tile) = suitable_tile {
                unit.current_tile = Some(tile.clone());
                return Some(unit);
            }

            // If no suitable tile found, expand the search
            let mut new_candidates = Vec::new();
            for tile in potential_candidates {
                new_candidates.extend(self.get_tiles_at_distance(tile.position, 1));
            }
            potential_candidates = new_candidates;

            try_count += 1;
        }

        // No suitable tile found
        None
    }

    /// Strips all units and starting locations for a player
    pub fn strip_player(&mut self, player_civ: &str) {
        // Remove units belonging to the player
        for tile in &mut self.tile_list {
            // This would be implemented in the Tile struct
            // tile.remove_units_by_owner(player_civ);
        }

        // Remove starting locations for the player
        self.starting_locations.retain(|loc| loc.nation != player_civ);

        // Update the starting locations by nation
        if let Some(tiles) = self.starting_locations_by_nation.get_mut(player_civ) {
            tiles.clear();
        }
    }

    /// Switches a player's nation
    pub fn switch_players_nation(&mut self, player_civ: &str, new_nation: &str) {
        // This is a simplified implementation
        // The actual implementation would be more complex and would depend on the game's mechanics

        // Create a new civilization
        let new_civ = Civilization::new(new_nation);

        // Switch units belonging to the player
        for tile in &mut self.tile_list {
            // This would be implemented in the Tile struct
            // tile.switch_units_owner(player_civ, new_nation, &new_civ);
        }

        // Switch starting locations for the player
        for loc in &mut self.starting_locations {
            if loc.nation == player_civ {
                loc.nation = new_nation.to_string();
            }
        }

        // Update the starting locations by nation
        if let Some(tiles) = self.starting_locations_by_nation.remove(player_civ) {
            self.starting_locations_by_nation.insert(new_nation.to_string(), tiles);
        }

        // Set up the starting locations transients
        self.set_starting_locations_transients();
    }

    /// Sets up the starting locations transients
    pub fn set_starting_locations_transients(&mut self) {
        self.starting_locations_by_nation.clear();
        for loc in &self.starting_locations {
            if let Some(tile) = self.get(loc.position.x, loc.position.y) {
                self.starting_locations_by_nation
                    .entry(loc.nation.clone())
                    .or_insert_with(HashSet::new)
                    .insert(tile.clone());
            }
        }
    }

    /// Adds a starting location
    pub fn add_starting_location(&mut self, nation_name: &str, tile: &Tile, usage: StartingLocationUsage) -> bool {
        // Check if the starting location already exists
        if self.starting_locations_by_nation
            .get(nation_name)
            .map_or(false, |tiles| tiles.contains(tile)) {
            return false;
        }

        // Add the starting location
        self.starting_locations.push(StartingLocation {
            position: tile.position,
            nation: nation_name.to_string(),
            usage,
        });

        // Update the starting locations by nation
        self.starting_locations_by_nation
            .entry(nation_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(tile.clone());

        true
    }

    /// Removes a starting location
    pub fn remove_starting_location(&mut self, nation_name: &str, tile: &Tile) -> bool {
        // Check if the starting location exists
        if !self.starting_locations_by_nation
            .get(nation_name)
            .map_or(false, |tiles| tiles.contains(tile)) {
            return false;
        }

        // Remove the starting location
        self.starting_locations.retain(|loc| {
            !(loc.nation == nation_name && loc.position == tile.position)
        });

        // Update the starting locations by nation
        if let Some(tiles) = self.starting_locations_by_nation.get_mut(nation_name) {
            tiles.remove(tile);
        }

        true
    }

    /// Removes all starting locations for a nation
    pub fn remove_starting_locations(&mut self, nation_name: &str) {
        if let Some(tiles) = self.starting_locations_by_nation.get(nation_name) {
            for tile in tiles.clone() {
                self.starting_locations.retain(|loc| {
                    !(loc.nation == nation_name && loc.position == tile.position)
                });
            }
            tiles.clear();
        }
    }

    /// Removes all starting locations at a position
    pub fn remove_starting_locations_at_position(&mut self, position: Position) {
        self.starting_locations.retain(|loc| loc.position != position);
        self.set_starting_locations_transients();
    }

    /// Clears all starting locations
    pub fn clear_starting_locations(&mut self) {
        self.starting_locations.clear();
        self.starting_locations_by_nation.clear();
    }

    /// Assigns continents to tiles
    pub fn assign_continents(&mut self, mode: AssignContinentsMode) {
        match mode {
            AssignContinentsMode::Clear => {
                // Clear all continent data
                for tile in &mut self.tile_list {
                    tile.continent = -1;
                }
                self.continent_sizes.clear();
                return;
            },
            AssignContinentsMode::Ensure => {
                // Check if continents are already assigned
                if !self.continent_sizes.is_empty() {
                    return;
                }

                // Try to regenerate continent sizes from tile data
                for tile in &self.tile_list {
                    let continent = tile.continent;
                    if continent == -1 {
                        continue;
                    }
                    *self.continent_sizes.entry(continent).or_insert(0) += 1;
                }

                // If continents are already assigned, return
                if !self.continent_sizes.is_empty() {
                    return;
                }

                // Otherwise, assign continents
                // This will fall through to the Assign case
            },
            AssignContinentsMode::Reassign => {
                // Clear all continent data
                for tile in &mut self.tile_list {
                    tile.continent = -1;
                }
                self.continent_sizes.clear();
                // This will fall through to the Assign case
            },
            AssignContinentsMode::Assign => {
                // Check if any land tile already has a continent
                for tile in &self.tile_list {
                    if tile.is_land && tile.continent != -1 {
                        panic!("Cannot assign continents: some land tiles already have continents");
                    }
                }
                // This will fall through to the actual assignment
            }
        }

        // Get all land tiles that are not impassible
        let mut land_tiles: Vec<&mut Tile> = self.tile_list.iter_mut()
            .filter(|t| t.is_land && !t.is_impassible())
            .collect();

        let mut current_continent = 0;
        self.continent_sizes.clear();

        // Assign continents using breadth-first search
        while !land_tiles.is_empty() {
            // Pick a random tile to start a new continent
            let start_tile_index = 0; // In a real implementation, this would be random
            let start_tile = land_tiles.remove(start_tile_index);
            start_tile.continent = current_continent;

            // Use BFS to find all connected tiles
            let mut continent_tiles = vec![start_tile];
            let mut queue = vec![start_tile];

            while !queue.is_empty() {
                let current_tile = queue.remove(0);

                // Get all neighbors
                let neighbors = self.get_tiles_at_distance(current_tile.position, 1);

                // Add unassigned land neighbors to the continent
                for neighbor in neighbors {
                    if neighbor.is_land && !neighbor.is_impassible() && neighbor.continent == -1 {
                        // Find the neighbor in land_tiles
                        if let Some(index) = land_tiles.iter().position(|t| t.position == neighbor.position) {
                            let mut neighbor_tile = land_tiles.remove(index);
                            neighbor_tile.continent = current_continent;
                            continent_tiles.push(neighbor_tile);
                            queue.push(neighbor_tile);
                        }
                    }
                }
            }

            // Record the continent size
            self.continent_sizes.insert(current_continent, continent_tiles.len() as i32);

            current_continent += 1;
        }
    }
}

/// Behavior of assign_continents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignContinentsMode {
    /// Initial assign, throw if tiles have continents
    Assign,
    /// Clear continent data and redo for map editor
    Reassign,
    /// Regenerate continent sizes from tile data, and if that is empty, Assign
    Ensure,
    /// Clear all continent data
    Clear,
}

/// Class to parse only the parameters and starting locations out of a map file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMapPreview {
    pub map_parameters: MapParameters,
    starting_locations: Vec<StartingLocation>,
}

impl TileMapPreview {
    /// Gets the declared nations
    pub fn get_declared_nations(&self) -> Vec<String> {
        self.starting_locations.iter()
            .filter(|loc| loc.usage != StartingLocationUsage::Normal)
            .map(|loc| loc.nation.clone())
            .collect()
    }

    /// Gets the nations for human player
    pub fn get_nations_for_human_player(&self) -> Vec<String> {
        self.starting_locations.iter()
            .filter(|loc| loc.usage == StartingLocationUsage::Human)
            .map(|loc| loc.nation.clone())
            .collect()
    }
}