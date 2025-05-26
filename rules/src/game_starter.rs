use std::collections::HashMap;
use rand::Rng;
use uuid::Uuid;

use crate::game_info::GameInfo;
use crate::civilization::Civilization;
use crate::models::metadata::GameParameters;
use crate::models::ruleset::RulesetCache;
use crate::logic::map::TileMap;
use crate::constants::Constants;

/// Responsible for starting new games
pub struct GameStarter {
    /// The game parameters
    game_parameters: GameParameters,
    /// The ruleset
    ruleset: Option<Ruleset>,
    /// The game info
    game_info: Option<GameInfo>,
}

impl GameStarter {
    /// Create a new GameStarter
    pub fn new(game_parameters: GameParameters) -> Self {
        let mut starter = Self {
            game_parameters,
            ruleset: None,
            game_info: None,
        };
        starter.ruleset = Some(RulesetCache::get_complex_ruleset(&starter.game_parameters));
        starter
    }

    /// Start a new game
    pub fn start_new_game(&mut self) -> GameInfo {
        let mut game = GameInfo::new();
        game.game_parameters = self.game_parameters.clone();
        game.difficulty = self.game_parameters.difficulty.clone();
        game.ruleset = self.ruleset.clone();

        // Create the map
        game.tile_map = TileMap::new();
        game.tile_map.map_parameters = self.game_parameters.map_parameters.clone();
        game.tile_map.generate_new_map();

        // Add civilizations
        self.add_civilizations(&mut game);

        // Set up barbarians
        if !self.game_parameters.no_barbarians {
            game.barbarians.game_info = Some(game.clone());
        }

        // Set up initial game state
        game.set_transients();
        game.update_civilization_state();

        // Set current player
        game.current_player = game.civilizations.iter()
            .find(|c| c.is_human())
            .map(|c| c.civ_name.clone())
            .unwrap_or_default();

        game.current_player_civ = Some(game.get_civilization(&game.current_player).clone());

        self.game_info = Some(game.clone());
        game
    }

    /// Add civilizations to the game
    fn add_civilizations(&self, game: &mut GameInfo) {
        let mut rng = rand::thread_rng();
        let mut taken_civs = HashMap::new();

        // Add player civilizations
        for player_civ in &self.game_parameters.player_civilizations {
            let civ = self.create_civilization(
                game,
                &player_civ.civilization_name,
                &player_civ.player_type,
                &player_civ.nation_name,
                &mut taken_civs,
                &mut rng,
            );
            game.civilizations.push(civ);
        }

        // Add AI civilizations
        let num_civs = self.game_parameters.number_of_civilizations;
        while game.civilizations.len() < num_civs {
            let civ = self.create_ai_civilization(game, &mut taken_civs, &mut rng);
            game.civilizations.push(civ);
        }

        // Add city states
        let num_city_states = self.game_parameters.number_of_city_states;
        for _ in 0..num_city_states {
            let civ = self.create_city_state(game, &mut taken_civs, &mut rng);
            game.civilizations.push(civ);
        }

        // Set up initial diplomacy
        for civ in &mut game.civilizations {
            civ.meet_other_civilizations();
            civ.init_diplomacy();
        }
    }

    /// Create a civilization
    fn create_civilization(
        &self,
        game: &GameInfo,
        civ_name: &str,
        player_type: &PlayerType,
        nation_name: &str,
        taken_civs: &mut HashMap<String, bool>,
        rng: &mut impl Rng,
    ) -> Civilization {
        let mut civ = Civilization::new();
        civ.player_type = player_type.clone();
        civ.is_player_civilization = true;
        civ.player_id = Uuid::new_v4().to_string();
        civ.civ_name = civ_name.to_string();
        civ.nation = nation_name.to_string();
        civ.game_info = Some(game.clone());

        taken_civs.insert(nation_name.to_string(), true);

        // Set up initial state
        civ.init_unit_types();
        civ.init_tech();
        civ.init_policies();
        civ.init_diplomacy();
        civ.init_notification_history();
        civ.init_great_people();
        civ.init_religion();

        civ
    }

    /// Create an AI civilization
    fn create_ai_civilization(
        &self,
        game: &GameInfo,
        taken_civs: &mut HashMap<String, bool>,
        rng: &mut impl Rng,
    ) -> Civilization {
        let available_nations: Vec<_> = self.ruleset.as_ref().unwrap().nations.values()
            .filter(|n| !n.is_spectator && !n.is_city_state && !taken_civs.contains_key(&n.name))
            .collect();

        let nation = available_nations[rng.gen_range(0..available_nations.len())];
        self.create_civilization(
            game,
            &nation.name,
            &PlayerType::AI,
            &nation.name,
            taken_civs,
            rng,
        )
    }

    /// Create a city state
    fn create_city_state(
        &self,
        game: &GameInfo,
        taken_civs: &mut HashMap<String, bool>,
        rng: &mut impl Rng,
    ) -> Civilization {
        let available_city_states: Vec<_> = self.ruleset.as_ref().unwrap().nations.values()
            .filter(|n| n.is_city_state && !taken_civs.contains_key(&n.name))
            .collect();

        let city_state = available_city_states[rng.gen_range(0..available_city_states.len())];
        let mut civ = self.create_civilization(
            game,
            &city_state.name,
            &PlayerType::AI,
            &city_state.name,
            taken_civs,
            rng,
        );

        civ.is_city_state = true;
        civ.city_state_type = Some(city_state.city_state_type.clone());
        civ.personality = city_state.personality.clone();

        civ
    }

    /// Start a game from a scenario
    pub fn start_game_from_scenario(&mut self, scenario_name: &str) -> GameInfo {
        let mut game = GameInfo::new();
        game.game_parameters = self.game_parameters.clone();
        game.difficulty = self.game_parameters.difficulty.clone();
        game.ruleset = self.ruleset.clone();

        // Load scenario
        let scenario = self.ruleset.as_ref().unwrap().scenarios.get(scenario_name)
            .expect("Scenario not found");

        // Set up map from scenario
        game.tile_map = scenario.map_file.clone();
        game.tile_map.map_parameters = self.game_parameters.map_parameters.clone();

        // Add civilizations from scenario
        for (nation_name, start_location) in &scenario.civilizations {
            let mut civ = self.create_civilization_from_scenario(
                &game,
                nation_name,
                start_location,
            );
            game.civilizations.push(civ);
        }

        // Set up barbarians
        if !self.game_parameters.no_barbarians {
            game.barbarians.game_info = Some(game.clone());
        }

        // Apply scenario modifiers
        self.apply_scenario_modifiers(&mut game, scenario);

        // Set up initial game state
        game.set_transients();
        game.update_civilization_state();

        // Set current player
        game.current_player = game.civilizations.iter()
            .find(|c| c.is_human())
            .map(|c| c.civ_name.clone())
            .unwrap_or_default();

        game.current_player_civ = Some(game.get_civilization(&game.current_player).clone());

        self.game_info = Some(game.clone());
        game
    }

    /// Create a civilization from a scenario
    fn create_civilization_from_scenario(
        &self,
        game: &GameInfo,
        nation_name: &str,
        start_location: &StartLocation,
    ) -> Civilization {
        let mut civ = Civilization::new();

        // Set basic info
        civ.nation = nation_name.to_string();
        civ.civ_name = nation_name.to_string();
        civ.player_type = if start_location.is_player {
            PlayerType::Human
        } else {
            PlayerType::AI
        };
        civ.is_player_civilization = start_location.is_player;
        civ.player_id = Uuid::new_v4().to_string();
        civ.game_info = Some(game.clone());

        // Set starting position
        civ.start_location = Some(start_location.clone());

        // Initialize state
        civ.init_unit_types();
        civ.init_tech();
        civ.init_policies();
        civ.init_diplomacy();
        civ.init_notification_history();
        civ.init_great_people();
        civ.init_religion();

        // Apply scenario-specific modifiers
        if let Some(tech_modifiers) = &start_location.tech_modifiers {
            for tech in tech_modifiers {
                civ.tech_manager.research_technology(tech);
            }
        }

        if let Some(policy_modifiers) = &start_location.policy_modifiers {
            for policy in policy_modifiers {
                civ.policies.adopt_policy(policy);
            }
        }

        civ
    }

    /// Apply scenario modifiers to the game
    fn apply_scenario_modifiers(&self, game: &mut GameInfo, scenario: &Scenario) {
        // Apply global modifiers
        if let Some(turn) = scenario.starting_turn {
            game.turns = turn;
        }

        if let Some(era) = &scenario.starting_era {
            game.game_parameters.starting_era = era.clone();
        }

        // Apply civilization-specific modifiers
        for civ in &mut game.civilizations {
            if let Some(modifiers) = scenario.civ_modifiers.get(&civ.nation) {
                // Apply tech modifiers
                for tech in &modifiers.technologies {
                    civ.tech_manager.research_technology(tech);
                }

                // Apply policy modifiers
                for policy in &modifiers.policies {
                    civ.policies.adopt_policy(policy);
                }

                // Apply resource modifiers
                for (resource, amount) in &modifiers.resources {
                    civ.resources.add_resource(resource, *amount);
                }

                // Apply gold modifier
                if let Some(gold) = modifiers.gold {
                    civ.gold = gold;
                }

                // Apply culture modifier
                if let Some(culture) = modifiers.culture {
                    civ.culture = culture;
                }

                // Apply faith modifier
                if let Some(faith) = modifiers.faith {
                    civ.faith = faith;
                }
            }
        }
    }

    /// Load a game from a save file
    pub fn load_game(&mut self, save_data: &str) -> GameInfo {
        let mut game: GameInfo = serde_json::from_str(save_data)
            .expect("Failed to parse save data");

        // Update game parameters
        game.game_parameters = self.game_parameters.clone();
        game.ruleset = self.ruleset.clone();

        // Set up transients
        game.set_transients();
        game.update_civilization_state();

        self.game_info = Some(game.clone());
        game
    }
}