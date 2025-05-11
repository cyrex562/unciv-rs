use std::collections::{HashSet, LinkedHashSet};
use std::fmt;

use crate::logic::IsPartOfGameInfoSerialization;
use crate::logic::civilization::PlayerType;
use crate::models::ruleset::Speed;
use crate::metadata::base_ruleset::BaseRuleset;

/// Represents the parameters for a game of Unciv.
/// Default values are set for a new game.
#[derive(Clone, Debug)]
pub struct GameParameters {
    pub difficulty: String,
    pub speed: Speed,

    pub random_number_of_players: bool,
    pub min_number_of_players: i32,
    pub max_number_of_players: i32,
    pub players: Vec<Player>,

    pub random_number_of_city_states: bool,
    pub min_number_of_city_states: i32,
    pub max_number_of_city_states: i32,
    pub number_of_city_states: i32,

    pub enable_random_nations_pool: bool,
    pub random_nations_pool: Vec<String>,

    pub no_city_razing: bool,
    pub no_barbarians: bool,
    pub raging_barbarians: bool,
    pub one_city_challenge: bool,
    pub god_mode: bool,
    pub nuclear_weapons_enabled: bool,
    pub espionage_enabled: bool,
    pub no_start_bias: bool,
    pub shuffle_player_order: bool,

    pub victory_types: Vec<String>,
    pub starting_era: String,

    // Multiplayer parameters
    pub is_online_multiplayer: bool,
    pub multiplayer_server_url: Option<String>,
    pub anyone_can_spectate: bool,
    /// After this amount of minutes, anyone can choose to 'skip turn' of the current player to keep the game going
    pub minutes_until_skip_turn: i32,

    pub base_ruleset: String,
    pub mods: LinkedHashSet<String>,

    pub max_turns: i32,

    pub accepted_mod_check_errors: String,
}

impl Default for GameParameters {
    fn default() -> Self {
        let mut players = Vec::new();
        players.push(Player::new(PlayerType::Human));
        for _ in 0..3 {
            players.push(Player::new(PlayerType::AI));
        }

        GameParameters {
            difficulty: "Prince".to_string(),
            speed: Speed::DEFAULT,

            random_number_of_players: false,
            min_number_of_players: 3,
            max_number_of_players: 3,
            players,

            random_number_of_city_states: false,
            min_number_of_city_states: 6,
            max_number_of_city_states: 6,
            number_of_city_states: 6,

            enable_random_nations_pool: false,
            random_nations_pool: Vec::new(),

            no_city_razing: false,
            no_barbarians: false,
            raging_barbarians: false,
            one_city_challenge: false,
            god_mode: false,
            nuclear_weapons_enabled: true,
            espionage_enabled: false,
            no_start_bias: false,
            shuffle_player_order: false,

            victory_types: Vec::new(),
            starting_era: "Ancient era".to_string(),

            is_online_multiplayer: false,
            multiplayer_server_url: None,
            anyone_can_spectate: true,
            minutes_until_skip_turn: 60 * 24,

            base_ruleset: BaseRuleset::CivVGnK.full_name().to_string(),
            mods: LinkedHashSet::new(),

            max_turns: 500,

            accepted_mod_check_errors: String::new(),
        }
    }
}

impl GameParameters {
    /// Creates a deep copy of the game parameters.
    pub fn clone(&self) -> Self {
        GameParameters {
            difficulty: self.difficulty.clone(),
            speed: self.speed,

            random_number_of_players: self.random_number_of_players,
            min_number_of_players: self.min_number_of_players,
            max_number_of_players: self.max_number_of_players,
            players: self.players.clone(),

            random_number_of_city_states: self.random_number_of_city_states,
            min_number_of_city_states: self.min_number_of_city_states,
            max_number_of_city_states: self.max_number_of_city_states,
            number_of_city_states: self.number_of_city_states,

            enable_random_nations_pool: self.enable_random_nations_pool,
            random_nations_pool: self.random_nations_pool.clone(),

            no_city_razing: self.no_city_razing,
            no_barbarians: self.no_barbarians,
            raging_barbarians: self.raging_barbarians,
            one_city_challenge: self.one_city_challenge,
            // god_mode intentionally reset on clone
            god_mode: false,
            nuclear_weapons_enabled: self.nuclear_weapons_enabled,
            espionage_enabled: self.espionage_enabled,
            no_start_bias: self.no_start_bias,
            shuffle_player_order: self.shuffle_player_order,

            victory_types: self.victory_types.clone(),
            starting_era: self.starting_era.clone(),

            is_online_multiplayer: self.is_online_multiplayer,
            multiplayer_server_url: self.multiplayer_server_url.clone(),
            anyone_can_spectate: self.anyone_can_spectate,
            minutes_until_skip_turn: self.minutes_until_skip_turn,

            base_ruleset: self.base_ruleset.clone(),
            mods: self.mods.clone(),

            max_turns: self.max_turns,

            accepted_mod_check_errors: self.accepted_mod_check_errors.clone(),
        }
    }

    /// Get all mods including base ruleset.
    ///
    /// The returned Set is ordered base first, then in the order they are stored in a save.
    /// This creates a fresh instance, and the caller is allowed to mutate it.
    pub fn get_mods_and_base_ruleset(&self) -> LinkedHashSet<String> {
        let mut result = LinkedHashSet::with_capacity(self.mods.len() + 1);
        result.insert(self.base_ruleset.clone());
        for mod_name in &self.mods {
            result.insert(mod_name.clone());
        }
        result
    }
}

impl fmt::Display for GameParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        parts.push(format!("{} {} {}", self.difficulty, self.speed, self.starting_era));

        let human_count = self.players.iter().filter(|p| p.player_type == PlayerType::Human).count();
        let ai_count = self.players.iter().filter(|p| p.player_type == PlayerType::AI).count();
        parts.push(format!("{} {}", human_count, PlayerType::Human));
        parts.push(format!("{} {}", ai_count, PlayerType::AI));

        if self.random_number_of_players {
            parts.push(format!("Random number of Players: {}..{}",
                self.min_number_of_players, self.max_number_of_players));
        }

        if self.random_number_of_city_states {
            parts.push(format!("Random number of City-States: {}..{}",
                self.min_number_of_city_states, self.max_number_of_city_states));
        } else {
            parts.push(format!("{} CS", self.number_of_city_states));
        }

        if self.is_online_multiplayer {
            parts.push("Online Multiplayer".to_string());
        }

        if self.no_barbarians {
            parts.push("No barbs".to_string());
        }

        if self.raging_barbarians {
            parts.push("Raging barbs".to_string());
        }

        if self.one_city_challenge {
            parts.push("OCC".to_string());
        }

        if !self.nuclear_weapons_enabled {
            parts.push("No nukes".to_string());
        }

        if self.god_mode {
            parts.push("God mode".to_string());
        }

        parts.push(format!("Enabled Victories: {}", self.victory_types.join(", ")));
        parts.push(self.base_ruleset.clone());

        if self.mods.is_empty() {
            parts.push("no mods".to_string());
        } else {
            let mods_str = if self.mods.len() > 6 {
                format!("mods=({}, ...)", self.mods.iter().take(6).collect::<Vec<_>>().join(","))
            } else {
                format!("mods=({})", self.mods.iter().collect::<Vec<_>>().join(","))
            };
            parts.push(mods_str);
        }

        write!(f, "({})", parts.join(" "))
    }
}

/// Represents a player in the game.
#[derive(Clone, Debug)]
pub struct Player {
    pub player_type: PlayerType,
}

impl Player {
    /// Creates a new player with the specified player type.
    pub fn new(player_type: PlayerType) -> Self {
        Player { player_type }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_game_parameters() {
        let params = GameParameters::default();
        assert_eq!(params.difficulty, "Prince");
        assert_eq!(params.players.len(), 4);
        assert_eq!(params.players[0].player_type, PlayerType::Human);
        assert_eq!(params.number_of_city_states, 6);
    }

    #[test]
    fn test_clone() {
        let mut params = GameParameters::default();
        params.god_mode = true;

        let cloned = params.clone();
        assert_eq!(cloned.difficulty, params.difficulty);
        assert_eq!(cloned.players.len(), params.players.len());
        assert!(!cloned.god_mode); // god_mode should be reset on clone
    }

    #[test]
    fn test_get_mods_and_base_ruleset() {
        let mut params = GameParameters::default();
        params.mods.insert("mod1".to_string());
        params.mods.insert("mod2".to_string());

        let mods = params.get_mods_and_base_ruleset();
        assert_eq!(mods.len(), 3); // base ruleset + 2 mods
        assert!(mods.contains(&params.base_ruleset));
        assert!(mods.contains(&"mod1".to_string()));
        assert!(mods.contains(&"mod2".to_string()));
    }
}