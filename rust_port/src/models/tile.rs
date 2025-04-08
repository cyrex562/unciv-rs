use std::option::Option;
use std::collections::VecDeque;
use std::rc::Rc;
use crate::models::map_unit::MapUnit;
use crate::models::game_info::Position;
use crate::models::ruleset::UniqueType;

/// Represents a tile on the game map.
pub struct Tile {
    pub position: Position,
    pub is_land: bool,
    pub is_water: bool,
    pub is_impassible: bool,
    pub is_city_center: bool,
    pub resource: Option<String>,
    pub improvement: Option<String>,
    pub terrain_feature_objects: Vec<TerrainFeature>,
    pub civilian_unit: Option<Rc<MapUnit>>,
    pub military_unit: Option<Rc<MapUnit>>,
    pub neighbors: Vec<Box<Tile>>,
}

impl Tile {
    /// Creates a new Tile.
    pub fn new() -> Self {
        Tile {
            position: Position { x: 0, y: 0 },
            is_land: true,
            is_water: false,
            is_impassible: false,
            is_city_center: false,
            resource: None,
            improvement: None,
            terrain_feature_objects: Vec::new(),
            civilian_unit: None,
            military_unit: None,
            neighbors: Vec::new(),
        }
    }

    /// Calculates the aerial distance to another tile.
    pub fn aerial_distance_to(&self, other: &Tile) -> f32 {
        // Implementation would go here
        0.0
    }

    /// Checks if this tile is a coastal tile.
    pub fn is_coastal_tile(&self) -> bool {
        self.is_land && self.neighbors.iter().any(|n| n.is_water)
    }

    /// Gets all tiles within the given distance of this tile.
    pub fn get_tiles_in_distance(&self, distance: i32) -> Vec<Tile> {
        // Placeholder implementation
        Vec::new()
    }

    /// Gets the first unit on this tile.
    pub fn get_first_unit(&self) -> Option<&MapUnit> {
        self.military_unit.as_ref().map(|u| u.as_ref()).or_else(|| self.civilian_unit.as_ref().map(|u| u.as_ref()))
    }

    /// Checks if this tile has the given unique type.
    pub fn terrain_has_unique(&self, unique_type: UniqueType) -> bool {
        self.terrain_feature_objects.iter().any(|feature| feature.has_unique(unique_type))
    }
}

/// Represents a terrain feature on a tile.
pub struct TerrainFeature {
    pub uniques: Vec<UniqueType>,
}

impl TerrainFeature {
    /// Checks if this terrain feature has the given unique type.
    pub fn has_unique(&self, unique_type: UniqueType) -> bool {
        self.uniques.contains(&unique_type)
    }
}