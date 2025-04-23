use std::collections::HashMap;
use crate::models::tile::Tile;
use crate::models::barbarians::Barbarians;

/// Contains game state information.
pub struct GameInfo {
    pub tile_map: HashMap<Position, Tile>,
    pub barbarians: Barbarians,
}

/// Represents a position on the map.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl GameInfo {
    /// Gets a tile at the specified position.
    pub fn get_tile(&self, position: Position) -> Option<&Tile> {
        self.tile_map.get(&position)
    }
}