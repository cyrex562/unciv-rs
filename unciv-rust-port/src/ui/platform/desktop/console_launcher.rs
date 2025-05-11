// Source: orig_src/desktop/src/com/unciv/app/desktop/ConsoleLauncher.kt
// Ported to Rust

use std::collections::HashMap;
use std::time::Instant;
use crate::constants::Constants;
use crate::game::UncivGame;
use crate::game::game_starter::GameStarter;
use crate::models::civilization::PlayerType;
use crate::models::map::{MapParameters, MapSize, MirroringType};
use crate::models::metadata::{GameParameters, GameSetupInfo};
use crate::models::ruleset::{RulesetCache, Speed};
use crate::models::ruleset::nation::Nation;
use crate::models::skins::SkinCache;
use crate::models::tilesets::TileSetCache;
use crate::simulation::Simulation;
use crate::utils::log::{Log, DesktopLogBackend};

/// Launches the game in console mode for simulation purposes
pub struct ConsoleLauncher;

impl ConsoleLauncher {
    /// Main entry point for console launcher
    pub fn main(args: Vec<String>) {
        // Set up logging
        Log::set_backend(Box::new(DesktopLogBackend::new()));

        // Initialize game
        let mut game = UncivGame::new(true);
        UncivGame::set_current(game.clone());

        // Configure game settings
        let mut settings = game.settings();
        settings.show_tutorials = false;
        settings.turns_between_autosaves = 10000;
        game.set_settings(settings);

        // Load game resources
        RulesetCache::load_rulesets(true);
        TileSetCache::load_tile_set_configs(true);
        SkinCache::load_skin_configs(true);

        // Run simulation
        Self::run_simulation();
    }

    /// Runs a simulation of multiple games
    fn run_simulation() {
        let ruleset_name = "Civ_V_GnK";
        let ruleset = RulesetCache::get_ruleset(ruleset_name).expect("Ruleset not found");

        // Set up simulation civilizations
        let simulation_civ1 = Constants::SIMULATION_CIV1;
        let simulation_civ2 = Constants::SIMULATION_CIV2;

        // Create nations for simulation
        let mut nation1 = Nation::new();
        nation1.name = simulation_civ1.to_string();
        let mut nation2 = Nation::new();
        nation2.name = simulation_civ2.to_string();

        // Add nations to ruleset
        ruleset.nations.insert(simulation_civ1.to_string(), nation1);
        ruleset.nations.insert(simulation_civ2.to_string(), nation2);

        // Get game parameters
        let game_parameters = Self::get_game_parameters(&[simulation_civ1, simulation_civ2]);
        let map_parameters = Self::get_map_parameters();
        let game_setup_info = GameSetupInfo::new(game_parameters, map_parameters);

        // Start new game
        let mut new_game = GameStarter::start_new_game(game_setup_info);

        // Set victory types
        new_game.game_parameters.victory_types = new_game.ruleset.victories.keys().cloned().collect();

        // Set current game
        UncivGame::current().set_game_info(new_game.clone());

        // Create and start simulation
        let simulation = Simulation::new(new_game, 50, 8);
        simulation.start();
    }

    /// Creates map parameters for simulation
    fn get_map_parameters() -> MapParameters {
        let mut params = MapParameters::new();
        params.map_size = MapSize::Small;
        params.no_ruins = true;
        params.no_natural_wonders = true;
        params.mirroring = MirroringType::AroundCenterTile;
        params
    }

    /// Creates game parameters for simulation
    fn get_game_parameters(civilizations: &[&str]) -> GameParameters {
        let mut params = GameParameters::new();
        params.difficulty = "Prince".to_string();
        params.number_of_city_states = 0;
        params.speed = Speed::DEFAULT;
        params.no_barbarians = true;

        // Add players
        let mut players = Vec::new();
        for civ in civilizations {
            players.push(Player::new(civ.to_string(), PlayerType::AI));
        }
        players.push(Player::new(Constants::SPECTATOR.to_string(), PlayerType::Human));
        params.players = players;

        params
    }
}