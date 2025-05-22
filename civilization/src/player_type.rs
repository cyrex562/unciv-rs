use serde::{Serialize, Deserialize};

/// Represents the type of player in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerType {
    /// AI-controlled player
    AI,
    /// Human-controlled player
    Human,
}

impl PlayerType {
    /// Toggles between AI and Human player types
    pub fn toggle(&self) -> Self {
        match self {
            Self::AI => Self::Human,
            Self::Human => Self::AI,
        }
    }
}