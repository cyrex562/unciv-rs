use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use crate::map::tile::Tile;

/// Records events throughout the game related to a tile.
///
/// Used for end of game replay.
///
/// # Properties
/// * `history` - History records by turn.
///
/// # See also
/// * `crate::ui::screens::victoryscreen::ReplayMap`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TileHistory {
    /// History records by turn
    history: BTreeMap<i32, TileHistoryState>,
}

/// Represents the state of a tile at a specific turn
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TileHistoryState {
    /// The name of the civilization owning this tile or `None` if there is no owner.
    pub owning_civ_name: Option<String>,
    /// The type of city center on this tile, or `None` if there is no city center.
    pub city_center_type: CityCenterType,
}

/// Represents the type of city center on a tile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CityCenterType {
    /// No city center
    None,
    /// Regular city center
    Regular,
    /// Capital city center
    Capital,
}

impl CityCenterType {
    /// Returns the serialized representation of this city center type
    pub fn serialized_representation(&self) -> &str {
        match self {
            CityCenterType::None => "N",
            CityCenterType::Regular => "R",
            CityCenterType::Capital => "C",
        }
    }

    /// Deserializes a city center type from its string representation
    pub fn deserialize(s: &str) -> Self {
        match s {
            "R" => CityCenterType::Regular,
            "C" => CityCenterType::Capital,
            _ => CityCenterType::None,
        }
    }
}

impl TileHistoryState {
    /// Creates a new TileHistoryState from a tile
    pub fn from_tile(tile: &Tile) -> Self {
        let owning_civ_name = tile.get_owner().map(|civ| civ.civ_name.clone());

        let city_center_type = if !tile.is_city_center() {
            CityCenterType::None
        } else if let Some(city) = tile.get_city() {
            if city.is_capital() {
                CityCenterType::Capital
            } else {
                CityCenterType::Regular
            }
        } else {
            CityCenterType::None
        };

        Self {
            owning_civ_name,
            city_center_type,
        }
    }
}

impl TileHistory {
    /// Creates a new empty TileHistory
    pub fn new() -> Self {
        Self {
            history: BTreeMap::new(),
        }
    }

    /// Records a change in tile ownership
    pub fn record_take_ownership(&mut self, tile: &Tile) {
        self.history.insert(
            tile.tile_map.game_info.turns,
            TileHistoryState::from_tile(tile)
        );
    }

    /// Records relinquishing ownership of a tile
    pub fn record_relinquish_ownership(&mut self, tile: &Tile) {
        self.history.insert(
            tile.tile_map.game_info.turns,
            TileHistoryState::default()
        );
    }

    /// Gets the state of the tile at the specified turn
    pub fn get_state(&self, turn: i32) -> &TileHistoryState {
        // Find the entry with the highest key that is less than or equal to turn
        if let Some((_, state)) = self.history.range(..=turn).next_back() {
            state
        } else {
            // If no entry is found, return a default state
            &TileHistoryState::default()
        }
    }

    /// Returns an iterator over the history entries
    pub fn iter(&self) -> impl Iterator<Item = (&i32, &TileHistoryState)> {
        self.history.iter()
    }
}

impl PartialEq for TileHistory {
    fn eq(&self, other: &Self) -> bool {
        if self.history.len() != other.history.len() {
            return false;
        }

        for (turn, state) in self.history.iter() {
            if other.history.get(turn) != Some(state) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_history_equality() {
        let mut history1 = TileHistory::new();
        let mut history2 = TileHistory::new();

        // Empty histories should be equal
        assert_eq!(history1, history2);

        // Add some entries to history1
        history1.history.insert(0, TileHistoryState {
            owning_civ_name: Some("Spain".to_string()),
            city_center_type: CityCenterType::Capital,
        });

        // Histories with different sizes should not be equal
        assert_ne!(history1, history2);

        // Add the same entry to history2
        history2.history.insert(0, TileHistoryState {
            owning_civ_name: Some("Spain".to_string()),
            city_center_type: CityCenterType::Capital,
        });

        // Histories with the same entries should be equal
        assert_eq!(history1, history2);

        // Add a different entry to history2
        history2.history.insert(1, TileHistoryState {
            owning_civ_name: Some("China".to_string()),
            city_center_type: CityCenterType::Regular,
        });

        // Histories with different entries should not be equal
        assert_ne!(history1, history2);
    }
}