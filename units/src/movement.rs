use crate::tile::tile::Tile;

/// Handles unit movement on the map.
pub struct Movement {
    movement_points: i32,
    max_movement: i32,
}

impl Movement {
    /// Creates a new Movement instance with default values.
    pub fn new() -> Self {
        Movement {
            movement_points: 2,
            max_movement: 2,
        }
    }

    /// Creates a new Movement instance with the given values.
    pub fn with_values(movement_points: i32, max_movement: i32) -> Self {
        Movement {
            movement_points,
            max_movement,
        }
    }

    /// Checks if the unit has any movement points remaining.
    pub fn has_movement(&self) -> bool {
        self.movement_points > 0
    }

    /// Checks if the unit can reach a tile.
    pub fn can_reach(&self, tile: &Tile) -> bool {
        // Implementation would go here
        true
    }

    /// Makes the unit head towards a tile.
    pub fn head_towards(&mut self, tile: &Tile) {
        // Implementation would go here
    }
}