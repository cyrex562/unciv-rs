use std::collections::{HashMap, HashSet};

/// The immutable properties and mutable game state of an individual unit present on the map
pub struct MapUnit {
    // Persisted fields
    pub owner: String,
    pub original_owner: Option<String>,
    pub name: String,
    pub instance_name: Option<String>,
    pub current_movement: f32,
    pub health: i32,
    pub id: i32,
    pub action: Option<String>,
    pub automated: bool,
    pub escorting: bool,
    pub automated_road_connection_destination: Option<Vector2>,
    pub automated_road_connection_path: Option<Vec<Vector2>>,
    pub attacks_this_turn: i32,
    pub promotions: UnitPromotions,
    pub due: bool,
    pub is_transported: bool,
    pub turns_fortified: i32,
    pub ability_to_times_used: HashMap<String, i32>,
    pub religion: Option<String>,
    pub religious_strength_lost: i32,
    pub movement_memories: Vec<UnitMovementMemory>,
    pub most_recent_move_type: UnitMovementMemoryType,
    pub attacks_since_turn_start: Vec<Vector2>,
    pub status_map: HashMap<String, UnitStatus>,

    // Transient fields
    pub civ: Civilization,
    pub base_unit: BaseUnit,
    pub current_tile: Tile,
    temp_uniques_map: UniqueMap,
    non_unit_uniques_map: UniqueMap,
    pub movement: UnitMovement,
    pub is_destroyed: bool,
    pub cache: MapUnitCache,
    pub viewable_tiles: HashSet<Tile>,
}

impl MapUnit {
    /// Creates a new MapUnit instance
    pub fn new(
        owner: String,
        name: String,
        id: i32,
        civ: Civilization,
        base_unit: BaseUnit,
        current_tile: Tile,
    ) -> Self {
        let mut unit = Self {
            owner,
            original_owner: None,
            name,
            instance_name: None,
            current_movement: 0.0,
            health: 100,
            id,
            action: None,
            automated: false,
            escorting: false,
            automated_road_connection_destination: None,
            automated_road_connection_path: None,
            attacks_this_turn: 0,
            promotions: UnitPromotions::new(),
            due: false,
            is_transported: false,
            turns_fortified: 0,
            ability_to_times_used: HashMap::new(),
            religion: None,
            religious_strength_lost: 0,
            movement_memories: Vec::new(),
            most_recent_move_type: UnitMovementMemoryType::None,
            attacks_since_turn_start: Vec::new(),
            status_map: HashMap::new(),
            civ,
            base_unit,
            current_tile,
            temp_uniques_map: UniqueMap::new(),
            non_unit_uniques_map: UniqueMap::new(),
            movement: UnitMovement::new(),
            is_destroyed: false,
            cache: MapUnitCache::new(),
            viewable_tiles: HashSet::new(),
        };
        
        unit.reset_movement();
        unit
    }

    /// Resets the unit's movement for a new turn
    pub fn reset_movement(&mut self) {
        self.current_movement = self.get_max_movement().into();
        self.attacks_this_turn = 0;
        self.attacks_since_turn_start.clear();
    }

    /// Gets the maximum movement points for this unit
    pub fn get_max_movement(&self) -> i32 {
        let base_movement = self.base_unit.movement;
        let bonus_from_promotions = self.promotions.get_movement_bonus();
        base_movement + bonus_from_promotions
    }

    /// Checks if the unit can move to the specified tile
    pub fn can_move_to(&self, tile: &Tile) -> bool {
        if self.current_movement <= 0.0 {
            return false;
        }

        if tile.is_impassable() {
            return false;
        }

        // Additional movement logic...
        
        true
    }

    /// Gets the name of the unit for display
    pub fn get_display_name(&self) -> String {
        if let Some(instance_name) = &self.instance_name {
            instance_name.clone()
        } else {
            self.name.clone()
        }
    }

    /// Updates the unit's status
    pub fn update_status(&mut self, status: UnitStatus) {
        self.status_map.insert(status.name.clone(), status);
    }

    /// Clears a specific status from the unit
    pub fn clear_status(&mut self, status_name: &str) {
        self.status_map.remove(status_name);
    }

    /// Checks if the unit has a specific status
    pub fn has_status(&self, status_name: &str) -> bool {
        self.status_map.contains_key(status_name)
    }

    /// Gets all unique abilities for this unit
    pub fn get_uniques(&self) -> Vec<String> {
        let mut uniques = self.base_unit.uniques.clone();
        
        // Add promotion uniques
        for promotion in self.promotions.promotions() {
            uniques.extend(promotion.uniques.clone());
        }
        
        // Add any terrain or other contextual uniques
        uniques.extend(self.temp_uniques_map.get_all_uniques());
        
        uniques
    }

    /// Updates the viewable tiles for this unit
    pub fn update_viewable_tiles(&mut self, map: &Map) {
        self.viewable_tiles = map.get_viewable_tiles_for_unit(self);
    }
}

// Placeholder type definitions to make the code compile
pub struct Vector2 {
    pub x: i32,
    pub y: i32,
}

pub struct UnitPromotions {
    promotions_list: Vec<Promotion>,
}

impl UnitPromotions {
    pub fn new() -> Self {
        Self {
            promotions_list: Vec::new(),
        }
    }

    pub fn get_movement_bonus(&self) -> i32 {
        // Implementation would calculate movement bonuses from promotions
        0
    }

    pub fn promotions(&self) -> &Vec<Promotion> {
        &self.promotions_list
    }
}

pub struct Promotion {
    pub name: String,
    pub uniques: Vec<String>,
}

pub enum UnitMovementMemoryType {
    None,
    Regular,
    Teleport,
    Airlift,
}

pub struct UnitMovementMemory {
    pub position: Vector2,
    pub memory_type: UnitMovementMemoryType,
}

pub struct UnitStatus {
    pub name: String,
    pub turns_left: i32,
}



pub struct BaseUnit {
    pub name: String,
    pub movement: i32,
    pub uniques: Vec<String>,
}



pub struct UniqueMap {
    uniques: Vec<String>,
}

impl UniqueMap {
    pub fn new() -> Self {
        Self {
            uniques: Vec::new(),
        }
    }

    pub fn get_all_uniques(&self) -> Vec<String> {
        self.uniques.clone()
    }
}

pub struct UnitMovement {
    // Movement-related fields
}

impl UnitMovement {
    pub fn new() -> Self {
        Self {}
    }
}

// MapUnitCache has been moved to src/map/unit_cache.rs
use crate::map::unit_cache::MapUnitCache;
use crate::tile::tile::Tile;

pub struct Map {
    // Map data
}

impl Map {
    pub fn get_viewable_tiles_for_unit(&self, unit: &MapUnit) -> HashSet<Tile> {
        // Implementation would determine which tiles a unit can see
        HashSet::new()
    }
}
