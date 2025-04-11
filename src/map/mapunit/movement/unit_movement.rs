use std::collections::{HashMap, HashSet};
use crate::map::tile::Tile;
use crate::map::tile_map::TileMap;
use crate::map::mapunit::MapUnit;
use crate::models::UnitActionType;
use crate::models::ruleset::unique::UniqueType;
use crate::constants::Constants;
use crate::utils::Vector2;

/// Represents a parent tile and the total movement cost to reach it
pub struct ParentTileAndTotalMovement<'a> {
    pub tile: &'a Tile,
    pub parent_tile: &'a Tile,
    pub total_movement: f32,
}

/// Cache for pathfinding results to optimize performance
pub struct PathfindingCache<'a> {
    shortest_path_cache: Vec<&'a Tile>,
    destination: Option<&'a Tile>,
    distance_to_tiles_cache: HashMap<bool, PathsToTilesWithinTurn<'a>>,
    movement: f32,
    current_tile: Option<&'a Tile>,
}

impl<'a> PathfindingCache<'a> {
    pub fn new() -> Self {
        Self {
            shortest_path_cache: Vec::new(),
            destination: None,
            distance_to_tiles_cache: HashMap::new(),
            movement: -1.0,
            current_tile: None,
        }
    }

    fn is_valid(&self, unit: &MapUnit) -> bool {
        self.movement == unit.current_movement && unit.get_tile() == self.current_tile
    }

    pub fn clear(&mut self, unit: &'a MapUnit) {
        self.distance_to_tiles_cache.clear();
        self.movement = unit.current_movement;
        self.current_tile = Some(unit.get_tile());
        self.destination = None;
        self.shortest_path_cache.clear();
    }
}

/// Collection of paths to tiles reachable within the current turn
pub struct PathsToTilesWithinTurn<'a> {
    paths: HashMap<&'a Tile, ParentTileAndTotalMovement<'a>>,
}

impl<'a> PathsToTilesWithinTurn<'a> {
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
        }
    }

    pub fn get_path_to_tile(&self, tile: &'a Tile) -> Result<Vec<&'a Tile>, String> {
        if !self.paths.contains_key(tile) {
            return Err("Can't reach this tile!".to_string());
        }

        let mut reverse_path_list = Vec::new();
        let mut current_tile = tile;

        while self.paths[current_tile].parent_tile != current_tile {
            reverse_path_list.push(current_tile);
            current_tile = self.paths[current_tile].parent_tile;
        }

        reverse_path_list.reverse();
        Ok(reverse_path_list)
    }
}

/// Handles unit movement, including pathfinding and movement costs
pub struct UnitMovement<'a> {
    unit: &'a MapUnit,
    pathfinding_cache: PathfindingCache<'a>,
}

impl<'a> UnitMovement<'a> {
    pub fn new(unit: &'a MapUnit) -> Self {
        Self {
            unit,
            pathfinding_cache: PathfindingCache::new(),
        }
    }

    /// Checks if an unknown tile should be assumed passable
    pub fn is_unknown_tile_we_should_assume_to_be_passable(&self, tile: &Tile) -> bool {
        !self.unit.civ.has_explored(tile)
    }

    /// Gets the tiles the unit could move to at the given position with the specified movement points
    pub fn get_movement_to_tiles_at_position(
        &self,
        position: &Vector2,
        unit_movement: f32,
        consider_zone_of_control: bool,
        tiles_to_ignore: Option<&HashSet<&'a Tile>>,
        pass_through_cache: &mut HashMap<&'a Tile, bool>,
        movement_cost_cache: &mut HashMap<(&'a Tile, &'a Tile), f32>,
        include_other_escort_unit: bool,
    ) -> PathsToTilesWithinTurn<'a> {
        let mut distance_to_tiles = PathsToTilesWithinTurn::new();

        let current_unit_tile = self.unit.current_tile;
        let unit_tile = if position == &current_unit_tile.position {
            current_unit_tile
        } else {
            current_unit_tile.tile_map.get_tile_at_position(position)
        };

        distance_to_tiles.paths.insert(
            unit_tile,
            ParentTileAndTotalMovement {
                tile: unit_tile,
                parent_tile: unit_tile,
                total_movement: 0.0,
            },
        );

        // If unit can't move, return immediately
        if unit_movement == 0.0 || self.unit.cache.cannot_move {
            return distance_to_tiles;
        }

        // If escort can't move, return immediately
        if include_other_escort_unit
            && self.unit.is_escorting()
            && self.unit.get_other_escort_unit().map_or(true, |u| u.current_movement == 0.0) {
            return distance_to_tiles;
        }

        // Main pathfinding logic will be implemented here
        // This will include:
        // - Iterating through neighboring tiles
        // - Calculating movement costs
        // - Handling zone of control
        // - Managing escort units
        // - Updating the paths collection

        distance_to_tiles
    }

    /// Gets the shortest path to the destination tile
    pub fn get_shortest_path(
        &mut self,
        destination: &'a Tile,
        avoid_damaging_terrain: bool,
    ) -> Vec<&'a Tile> {
        if self.unit.cache.cannot_move {
            return Vec::new();
        }

        // Try damage-free path first if applicable
        if !avoid_damaging_terrain
            && self.unit.civ.pass_through_impassable_unlocked
            && self.unit.base_unit.is_land_unit {
            let damage_free_path = self.get_shortest_path(destination, true);
            if !damage_free_path.is_empty() {
                return damage_free_path;
            }
        }

        // Check if destination is reachable
        if destination.neighbors().iter().all(|tile| {
            !self.is_unknown_tile_we_should_assume_to_be_passable(tile)
            && !self.can_pass_through(tile)
        }) {
            self.pathfinding_cache.shortest_path_cache = Vec::new();
            return Vec::new();
        }

        // Rest of pathfinding implementation will go here
        // Including:
        // - Cache checking
        // - BFS/pathfinding algorithm
        // - Handling special cases (air units, paradrops)
        // - Movement cost calculations
        // - Path validation and return

        Vec::new()
    }

    /// Checks if a unit can pass through a tile
    pub fn can_pass_through(&self, tile: &Tile, include_other_escort_unit: bool) -> bool {
        if tile.is_impassible() {
            // Handle impassable terrain exceptions
            if !self.unit.cache.can_pass_through_impassable_tiles
                && !(self.unit.cache.can_enter_ice_tiles && tile.terrain_features.contains(&Constants::ICE.to_string()))
                && !(self.unit.civ.pass_through_impassable_unlocked
                    && self.unit.civ.passable_impassables.contains(&tile.last_terrain.name)) {
                return false;
            }
        }

        // Water unit checks
        if tile.is_land
            && self.unit.base_unit.is_water_unit
            && !tile.is_city_center() {
            return false;
        }

        // Land unit on water checks
        if tile.is_water && self.unit.base_unit.is_land_unit && !self.unit.cache.can_move_on_water {
            if !self.unit.civ.tech.units_can_embark {
                return false;
            }
            if tile.is_ocean
                && !self.unit.civ.tech.embarked_units_can_enter_ocean
                && !self.unit_specific_allow_ocean() {
                return false;
            }
        }

        // Ocean tile checks
        if tile.is_ocean && !self.unit.civ.tech.all_units_can_enter_ocean {
            if !self.unit_specific_allow_ocean() && self.unit.cache.cannot_enter_ocean_tiles {
                return false;
            }
        }

        // Territory and escort checks
        if self.unit.cache.can_enter_city_states && tile.get_owner().map_or(false, |owner| owner.is_city_state()) {
            return true;
        }
        if !self.unit.cache.can_enter_foreign_terrain && !tile.can_civ_pass_through(&self.unit.civ) {
            return false;
        }
        if include_other_escort_unit
            && self.unit.is_escorting()
            && !self.unit.get_other_escort_unit()
                .map_or(false, |escort| escort.movement.can_pass_through(tile, false)) {
            return false;
        }

        true
    }

    /// Helper method to check if unit specifically can enter ocean tiles
    fn unit_specific_allow_ocean(&self) -> bool {
        self.unit.civ.tech.specific_units_can_enter_ocean
            && self.unit.civ
                .get_matching_uniques(UniqueType::UnitsMayEnterOcean)
                .iter()
                .any(|unique| self.unit.matches_filter(&unique.params[0]))
    }
}