use serde::{Deserialize, Serialize};

/// A UnitMove is a record of a unit moving from one tile to another.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UnitMove {
    pub unit_id: String,
    pub from_tile: String,
    pub to_tile: String,
    pub movement_points: u32,
}

impl UnitMove {
    pub fn new(unit_id: String, from_tile: &str, to_tile: &str) -> Self {
        Self {
            unit_id,
            from_tile: from_tile.to_string(),
            to_tile: to_tile.to_string(),
            movement_points,
        }
    }
}