use serde::{Deserialize, Serialize};

use crate::constants::RANDOM;
use crate::logic::IsPartOfGameInfoSerialization;
use crate::logic::civilization::PlayerType;

/// Represents a player in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Player {
    /// The chosen civilization for the player
    pub chosen_civ: String,
    /// The type of player (AI or Human)
    pub player_type: PlayerType,
    /// The unique identifier for the player
    pub player_id: String,
}

impl Player {
    /// Creates a new Player with the given parameters
    pub fn new(
        chosen_civ: String,
        player_type: PlayerType,
        player_id: String,
    ) -> Self {
        Player {
            chosen_civ,
            player_type,
            player_id,
        }
    }

    /// Creates a new Player with default values
    pub fn default() -> Self {
        Player {
            chosen_civ: RANDOM.to_string(),
            player_type: PlayerType::AI,
            player_id: String::new(),
        }
    }
}

impl IsPartOfGameInfoSerialization for Player {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_default() {
        let player = Player::default();
        assert_eq!(player.chosen_civ, RANDOM);
        assert_eq!(player.player_type, PlayerType::AI);
        assert!(player.player_id.is_empty());
    }

    #[test]
    fn test_player_new() {
        let player = Player::new(
            "Rome".to_string(),
            PlayerType::Human,
            "player1".to_string(),
        );
        assert_eq!(player.chosen_civ, "Rome");
        assert_eq!(player.player_type, PlayerType::Human);
        assert_eq!(player.player_id, "player1");
    }
}