use std::collections::{HashMap, HashSet};
use std::option::Option;
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use crate::models::map_unit::MapUnit;
use crate::models::game_info::Position;
use crate::models::ruleset::{ResourceType, Terrain, TerrainType};
use crate::models::city::City;
use crate::unique_type::UniqueType;

/// Represents a tile on the game map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    // Core properties
    pub position: Position,
    pub base_terrain: Option<String>,
    pub terrain_features: Vec<String>,
    pub natural_wonder: Option<String>,
    pub resource: Option<String>,
    pub resource_amount: i32,
    pub improvement: Option<String>,
    pub improvement_is_pillaged: bool,
    pub road_status: Option<String>,
    pub road_is_pillaged: bool,
    pub road_owner: String,

    // River properties
    pub has_bottom_right_river: bool,
    pub has_bottom_river: bool,
    pub has_bottom_left_river: bool,

    // Units
    pub military_unit: Option<Rc<MapUnit>>,
    pub civilian_unit: Option<Rc<MapUnit>>,
    pub air_units: Vec<Rc<MapUnit>>,

    // Exploration and ownership
    pub explored_by: HashSet<String>,
    pub owning_city: Option<Rc<City>>,

    // Cached properties
    #[serde(skip)]
    pub is_land: bool,
    #[serde(skip)]
    pub is_water: bool,
    #[serde(skip)]
    pub is_ocean: bool,
    #[serde(skip)]
    pub is_city_center: bool,
    #[serde(skip)]
    pub unit_height: i32,
    #[serde(skip)]
    pub tile_height: i32,

    // History
    pub history: TileHistory,

    // New fields from the code block
    pub river_corners: Vec<u8>,
    pub continent: i32,
    pub defense_bonus: f32,
    pub movement_cost: f32,
    pub owner_name: Option<String>,
    pub improvement_stats: HashMap<String, f32>,
}

/// Represents the status of a road on a tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoadStatus {
    None,
    Road,
    Railroad,
}

impl Tile {
    /// Creates a new empty tile
    pub fn new(x: i32, y: i32) -> Self {
        Tile {
            position: Position { x, y },
            base_terrain: None,
            terrain_features: Vec::new(),
            natural_wonder: None,
            resource: None,
            resource_amount: 0,
            improvement: None,
            improvement_is_pillaged: false,
            road_status: None,
            road_is_pillaged: false,
            road_owner: String::new(),
            has_bottom_right_river: false,
            has_bottom_river: false,
            has_bottom_left_river: false,
            military_unit: None,
            civilian_unit: None,
            air_units: Vec::new(),
            explored_by: HashSet::new(),
            owning_city: None,
            is_land: false,
            is_water: false,
            is_ocean: false,
            is_city_center: false,
            unit_height: 0,
            tile_height: 0,
            history: TileHistory::default(),
            river_corners: Vec::new(),
            continent: -1,
            defense_bonus: 0.0,
            movement_cost: 1.0,
            owner_name: None,
            improvement_stats: HashMap::new(),
        }
    }

    /// Sets the base terrain of the tile
    pub fn set_base_terrain(&mut self, terrain: &Terrain) {
        self.base_terrain = Some(terrain.name.clone());
        self.update_terrain_properties();
    }

    /// Sets the terrain features of the tile
    pub fn set_terrain_features(&mut self, features: Vec<String>) {
        self.terrain_features = features;
        self.update_terrain_properties();
    }

    /// Updates cached terrain properties
    fn update_terrain_properties(&mut self) {
        // Implementation would update is_land, is_water, is_ocean based on terrain
        // This is a placeholder - actual implementation would depend on terrain rules
        self.is_land = true; // Placeholder
        self.is_water = false; // Placeholder
        self.is_ocean = false; // Placeholder
    }

    /// Checks if this tile is a coastal tile
    pub fn is_coastal_tile(&self) -> bool {
        self.is_land && self.neighbors.iter().any(|n| n.is_water)
    }

    /// Gets the first unit on this tile
    pub fn get_first_unit(&self) -> Option<&MapUnit> {
        self.military_unit.as_ref().map(|u| u.as_ref())
            .or_else(|| self.civilian_unit.as_ref().map(|u| u.as_ref()))
    }

    /// Checks if this tile has a luxury resource
    pub fn has_luxury_resource(&self) -> bool {
        self.resource.is_some() && self.resource_type() == Some(ResourceType::Luxury)
    }

    /// Checks if this tile has a strategic resource
    pub fn has_strategic_resource(&self) -> bool {
        self.resource.is_some() && self.resource_type() == Some(ResourceType::Strategic)
    }

    /// Gets the resource type of this tile
    pub fn resource_type(&self) -> Option<ResourceType> {
        // Implementation would depend on resource rules
        None // Placeholder
    }

    /// Gets all tiles within the given distance of this tile
    pub fn get_tiles_in_distance(&self, distance: i32) -> Vec<&Tile> {
        // Implementation would use HexMath to find tiles
        Vec::new() // Placeholder
    }

    /// Calculates the aerial distance to another tile
    pub fn aerial_distance_to(&self, other: &Tile) -> f32 {
        let dx = (self.position.x - other.position.x) as f32;
        let dy = (self.position.y - other.position.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn add_terrain_feature(&mut self, feature: String) {
        if !self.terrain_features.contains(&feature) {
            self.terrain_features.push(feature);
            self.update_terrain_properties();
        }
    }

    pub fn remove_terrain_feature(&mut self, feature: &str) {
        if let Some(index) = self.terrain_features.iter().position(|x| x == feature) {
            self.terrain_features.remove(index);
            self.update_terrain_properties();
        }
    }

    pub fn set_improvement(&mut self, improvement: String) {
        self.improvement = Some(improvement);
        self.update_terrain_properties();
    }

    pub fn remove_improvement(&mut self) {
        self.improvement = None;
        self.improvement_stats.clear();
        self.update_terrain_properties();
    }

    pub fn add_river_corner(&mut self, corner: u8) {
        if !self.river_corners.contains(&corner) {
            self.river_corners.push(corner);
            self.update_terrain_properties();
        }
    }

    pub fn has_river(&self) -> bool {
        !self.river_corners.is_empty()
    }

    pub fn set_continent(&mut self, continent_id: i32) {
        self.continent = continent_id;
    }

    pub fn update_improvement_stat(&mut self, stat_name: String, value: f32) {
        self.improvement_stats.insert(stat_name, value);
    }

    pub fn set_road_status(&mut self, status: String) {
        self.road_status = Some(status);
        self.update_terrain_properties();
    }

    pub fn remove_road(&mut self) {
        self.road_status = None;
        self.update_terrain_properties();
    }

    pub fn set_owner(&mut self, owner: String) {
        self.owner_name = Some(owner);
    }

    pub fn remove_owner(&mut self) {
        self.owner_name = None;
    }

    pub fn is_owned(&self) -> bool {
        self.owner_name.is_some()
    }

    pub fn update_defense_bonus(&mut self, bonus: f32) {
        self.defense_bonus = bonus;
    }

    pub fn update_movement_cost(&mut self, cost: f32) {
        self.movement_cost = cost;
    }
}

impl std::hash::Hash for Tile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position.x.hash(state);
        self.position.y.hash(state);
    }
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Eq for Tile {}

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