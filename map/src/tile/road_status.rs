use crate::models::ruleset::Ruleset;

/// Represents the road status of a tile.
///
/// You can use RoadStatus.name to identify [Road] and [Railroad]
/// in string-based identification, as done in [improvement].
///
/// Note: Order is important, [ordinal] _is_ compared - please interpret as "roadLevel".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoadStatus {
    /// No road on the tile
    None,
    /// A basic road on the tile
    Road,
    /// A railroad on the tile
    Railroad,
}

impl RoadStatus {
    /// Returns the upkeep cost of the road
    pub fn upkeep(&self) -> i32 {
        match self {
            RoadStatus::None => 0,
            RoadStatus::Road => 1,
            RoadStatus::Railroad => 2,
        }
    }

    /// Returns the movement cost for units on this road
    pub fn movement(&self) -> f32 {
        match self {
            RoadStatus::None => 1.0,
            RoadStatus::Road => 0.5,
            RoadStatus::Railroad => 0.1,
        }
    }

    /// Returns the movement cost for units with road movement improvement on this road
    pub fn movement_improved(&self) -> f32 {
        match self {
            RoadStatus::None => 1.0,
            RoadStatus::Road => 1.0 / 3.0,
            RoadStatus::Railroad => 0.1,
        }
    }

    /// Returns the action to remove this road, or None if there is no road
    pub fn remove_action(&self) -> Option<&str> {
        match self {
            RoadStatus::None => None,
            RoadStatus::Road => Some("Remove Road"),
            RoadStatus::Railroad => Some("Remove Railroad"),
        }
    }

    /// Returns the improvement object for this road status, or None if there is no road
    pub fn improvement(&self, ruleset: &Ruleset) -> Option<&crate::models::ruleset::tile::TileImprovement> {
        match self {
            RoadStatus::None => None,
            _ => ruleset.tile_improvements.get(self.name()),
        }
    }

    /// Returns the name of this road status
    pub fn name(&self) -> &str {
        match self {
            RoadStatus::None => "None",
            RoadStatus::Road => "Road",
            RoadStatus::Railroad => "Railroad",
        }
    }
}

impl Default for RoadStatus {
    fn default() -> Self {
        RoadStatus::None
    }
}