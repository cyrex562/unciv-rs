use std::collections::HashMap;

use crate::constants::Constants;
use crate::game::game_info::GameInfo;
use crate::simulation::mutable_int::MutableInt;

/// A single step in a game simulation
#[derive(Debug, Clone)]
pub struct SimulationStep {
    /// The civilizations in the game
    pub civilizations: Vec<String>,
    /// The current number of turns
    pub turns: usize,
    /// The type of victory achieved, if any
    pub victory_type: Option<String>,
    /// The winner of the game, if any
    pub winner: Option<String>,
    /// The current player
    pub current_player: String,
    /// Statistics for each civilization at each turn
    pub turn_stats: HashMap<String, HashMap<i32, MutableInt>>,
}

impl SimulationStep {
    /// Create a new SimulationStep
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info
    /// * `stat_turns` - The turns to collect statistics for
    ///
    /// # Returns
    ///
    /// A new SimulationStep
    pub fn new(game_info: &GameInfo, stat_turns: &[i32]) -> Self {
        let civilizations: Vec<String> = game_info
            .civilizations
            .iter()
            .filter(|c| c.civ_name != Constants::spectator)
            .map(|c| c.civ_name.clone())
            .collect();

        let mut turn_stats = HashMap::new();

        for civ in &civilizations {
            let mut civ_stats = HashMap::new();
            for &turn in stat_turns {
                civ_stats.insert(turn, MutableInt::new(-1));
            }
            civ_stats.insert(-1, MutableInt::new(-1)); // End of game
            turn_stats.insert(civ.clone(), civ_stats);
        }

        Self {
            civilizations,
            turns: game_info.turns,
            victory_type: game_info
                .get_current_player_civilization()
                .victory_manager
                .get_victory_type_achieved(),
            winner: None,
            current_player: game_info.current_player.clone(),
            turn_stats,
        }
    }

    /// Save the turn statistics
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info
    pub fn save_turn_stats(&mut self, game_info: &GameInfo) {
        self.victory_type = game_info
            .get_current_player_civilization()
            .victory_manager
            .get_victory_type_achieved();

        let turn = if self.victory_type.is_some() { -1 } else { game_info.turns as i32 };

        for civ in game_info
            .civilizations
            .iter()
            .filter(|c| c.civ_name != Constants::spectator)
        {
            let popsum = civ.cities.iter().map(|city| city.population.population).sum();
            self.turn_stats
                .get_mut(&civ.civ_name)
                .unwrap()
                .get_mut(&turn)
                .unwrap()
                .set(popsum);
        }
    }

    /// Update the simulation step with the current game state
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info
    pub fn update(&mut self, game_info: &GameInfo) {
        self.turns = game_info.turns;
        self.victory_type = game_info
            .get_current_player_civilization()
            .victory_manager
            .get_victory_type_achieved();
        self.current_player = game_info.current_player.clone();
    }
}