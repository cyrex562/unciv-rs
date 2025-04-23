use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

/// Represents detailed information about a multiplayer game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetails {
    /// Unique identifier for the game
    pub id: String,

    /// Name of the game
    pub name: String,

    /// Current number of players in the game
    pub current_players: usize,

    /// Maximum number of players allowed in the game
    pub max_players: usize,

    /// Whether the game is password protected
    pub is_password_protected: bool,

    /// Whether the game has started
    pub has_started: bool,

    /// Timestamp when the game was created
    pub created_at: u64,

    /// Additional game settings and metadata
    pub settings: GameSettings,

    /// List of player names currently in the game
    pub players: Vec<String>,

    /// Whether the game is currently in progress
    pub is_in_progress: bool,
}

// GameSettings has been moved to src/game_settings.rs

impl GameDetails {
    /// Creates a new GameDetails instance
    pub fn new(id: String, settings: GameSettings) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            name: settings.name,
            current_players: 0,
            max_players: settings.max_players as usize,
            is_password_protected: settings.is_password_protected,
            has_started: false,
            created_at: now,
            settings,
            players: Vec::new(),
            is_in_progress: false,
        }
    }

    /// Checks if the game is full
    pub fn is_full(&self) -> bool {
        self.current_players >= self.max_players
    }

    /// Checks if the game can be joined
    pub fn can_join(&self) -> bool {
        !self.has_started && !self.is_full()
    }

    /// Adds a player to the game
    pub fn add_player(&mut self, player_name: String) -> bool {
        if self.current_players >= self.max_players {
            return false;
        }

        self.players.push(player_name);
        self.current_players += 1;
        true
    }

    /// Removes a player from the game
    pub fn remove_player(&mut self, player_name: &str) -> bool {
        if let Some(pos) = self.players.iter().position(|p| p == player_name) {
            self.players.remove(pos);
            self.current_players -= 1;
            true
        } else {
            false
        }
    }

    /// Starts the game
    pub fn start_game(&mut self) {
        self.has_started = true;
        self.is_in_progress = true;
    }

    /// Ends the game
    pub fn end_game(&mut self) {
        self.is_in_progress = false;
    }
}