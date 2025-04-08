use crate::models::tile::Tile;
use crate::models::ruleset::BaseUnit;
use crate::models::movement::Movement;

/// Represents a unit on the map.
pub struct MapUnit {
    pub base_unit: BaseUnit,
    pub current_tile: Box<Tile>,
    pub health: i32,
    pub movement: Movement,
    pub is_civilian: bool,
    pub civ: String,
}

impl MapUnit {
    /// Creates a new MapUnit.
    pub fn new(base_unit: BaseUnit, current_tile: Box<Tile>, is_civilian: bool, civ: String) -> Self {
        MapUnit {
            base_unit,
            current_tile,
            health: 100,
            movement: Movement::new(),
            is_civilian,
            civ,
        }
    }

    /// Checks if the unit is a civilian unit.
    pub fn is_civilian(&self) -> bool {
        self.is_civilian
    }

    /// Checks if the unit has any movement points remaining.
    pub fn has_movement(&self) -> bool {
        self.movement.has_movement()
    }

    /// Attempts to fortify the unit if possible.
    pub fn fortify_if_can(&self) {
        // Implementation would go here
    }
}