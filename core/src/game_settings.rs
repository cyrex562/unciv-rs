use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Game settings that can be modified by the user.
/// 
/// This consolidates settings from multiple areas of the game including
/// general gameplay preferences, multiplayer settings, and visual/audio options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    // Basic settings
    /// List of enabled visual mods
    pub visual_mods: Vec<String>,
    
    /// Screen mode setting
    pub screen_mode: i32,
    
    /// Whether to use continuous rendering
    pub continuous_rendering: bool,
    
    /// The currently selected tileset
    pub tile_set: String,
    
    /// Font family data
    pub font_family_data: String,
    
    /// Whether this is the first time the game has been run
    pub is_freshly_created: bool,
    
    /// Whether to enable autoplay
    pub auto_play: bool,
    
    // Multiplayer settings
    /// Multiplayer-specific configuration
    pub multiplayer: MultiplayerSettings,
    
    // Game specific settings (used when creating/joining games)
    /// Map being used for the game
    pub map: String,
    
    /// Game rules being used
    pub rules: String,
    
    /// Victory conditions for the game
    pub victory_conditions: Vec<String>,
    
    /// Whether the game is turn-based
    pub is_turn_based: bool,
    
    /// Time limit per turn in seconds (if applicable)
    pub turn_time_limit: Option<u32>,
    
    /// Name of the game
    pub name: String,
    
    /// Maximum number of players allowed
    pub max_players: u32,
    
    /// Whether the game is password protected
    pub is_password_protected: bool,
    
    /// Additional game settings as key-value pairs
    #[serde(flatten)]
    pub additional_settings: HashMap<String, String>,
}

/// Multiplayer-specific settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiplayerSettings {
    /// Unique ID for the user in multiplayer
    pub user_id: String,
    
    /// User's display name
    pub username: String,
    
    /// Server URL
    pub server_url: String,
    
    /// Whether anyone can spectate games
    pub anyone_can_spectate: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            visual_mods: Vec::new(),
            screen_mode: 0, // Default screen mode
            continuous_rendering: true,
            tile_set: "FantasyHex".to_string(), // Assuming this is the default
            font_family_data: "".to_string(),
            is_freshly_created: true,
            auto_play: false,
            multiplayer: MultiplayerSettings {
                user_id: "".to_string(),
                username: "Player".to_string(),
                server_url: "https://uncivserver.xyz".to_string(),
                anyone_can_spectate: false,
            },
            map: "Default".to_string(),
            rules: "Default".to_string(),
            victory_conditions: vec!["Domination".to_string()],
            is_turn_based: true,
            turn_time_limit: None,
            name: "New Game".to_string(),
            max_players: 8,
            is_password_protected: false,
            additional_settings: HashMap::new(),
        }
    }
}

impl GameSettings {
    /// Save the current settings
    pub fn save(&self) {
        // Implementation to save settings to persistent storage
    }
    
    /// Get the current font size
    pub fn get_font_size(&self) -> i32 {
        // This would be implemented to extract font size from font_family_data
        // or return a default value
        12
    }
}
