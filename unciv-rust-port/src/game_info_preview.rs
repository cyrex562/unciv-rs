use crate::game_info::GameInfo;
use crate::metadata::game_parameters::GameParameters;

/// Reduced variant of GameInfo used for load preview and multiplayer saves.
/// Contains additional data for multiplayer settings.
pub struct GameInfoPreview {
    /// The civilizations in the game
    pub civilizations: Vec<CivilizationInfoPreview>,

    /// The difficulty level of the game
    pub difficulty: String,

    /// The game parameters
    pub game_parameters: GameParameters,

    /// The current turn number
    pub turns: i32,

    /// The unique ID of the game
    pub game_id: String,

    /// The name of the current player
    pub current_player: String,

    /// The time when the current turn started
    pub current_turn_start_time: i64,
}

impl GameInfoPreview {
    /// Create a new GameInfoPreview
    pub fn new() -> Self {
        Self {
            civilizations: Vec::new(),
            difficulty: "Chieftain".to_string(),
            game_parameters: GameParameters::new(),
            turns: 0,
            game_id: String::new(),
            current_player: String::new(),
            current_turn_start_time: 0,
        }
    }

    /// Create a new GameInfoPreview from a GameInfo
    pub fn from_game_info(game_info: &GameInfo) -> Self {
        let mut preview = Self::new();
        preview.civilizations = game_info.get_civilizations_as_previews();
        preview.difficulty = game_info.difficulty.clone();
        preview.game_parameters = game_info.game_parameters.clone();
        preview.turns = game_info.turns;
        preview.game_id = game_info.game_id.clone();
        preview.current_player = game_info.current_player.clone();
        preview.current_turn_start_time = game_info.current_turn_start_time;

        preview
    }

    /// Get a civilization by name
    pub fn get_civilization(&self, civ_name: &str) -> Option<&CivilizationInfoPreview> {
        self.civilizations.iter().find(|c| c.civ_name == civ_name)
    }

    /// Get the current player's civilization
    pub fn get_current_player_civ(&self) -> Option<&CivilizationInfoPreview> {
        self.get_civilization(&self.current_player)
    }

    /// Get a player's civilization by player ID
    pub fn get_player_civ(&self, player_id: &str) -> Option<&CivilizationInfoPreview> {
        self.civilizations.iter().find(|c| c.player_id == player_id)
    }
}