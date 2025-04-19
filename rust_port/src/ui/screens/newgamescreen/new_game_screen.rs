use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, ScrollArea};
use crate::models::metadata::GameSetupInfo;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreens::PickerScreen;
use crate::ui::components::widgets::{ExpanderTab, WrappableLabel};
use crate::ui::popups::{Popup, ToastPopup, ConfirmPopup};
use crate::utils::concurrency::Concurrency;
use crate::game::UncivGame;
use crate::constants::Constants;
use super::{
    PlayerPickerTable,
    GameOptionsTable,
    MapOptionsTable,
    NewGameModCheckHelpers,
};

pub struct NewGameScreen {
    game_setup_info: Rc<RefCell<GameSetupInfo>>,
    ruleset: Rc<RefCell<Ruleset>>,
    player_picker_table: Rc<RefCell<PlayerPickerTable>>,
    new_game_options_table: Rc<RefCell<GameOptionsTable>>,
    map_options_table: Rc<RefCell<MapOptionsTable>>,
    is_portrait: bool,
}

impl NewGameScreen {
    pub fn new(game_setup_info: GameSetupInfo) -> Self {
        let game_setup_info = Rc::new(RefCell::new(game_setup_info));
        let ruleset = Rc::new(RefCell::new(Ruleset::default()));
        let is_portrait = Self::is_narrower_than_4to3();

        let mut screen = Self {
            game_setup_info: game_setup_info.clone(),
            ruleset: ruleset.clone(),
            player_picker_table: Rc::new(RefCell::new(PlayerPickerTable::new(
                game_setup_info.clone(),
                is_portrait,
            ))),
            new_game_options_table: Rc::new(RefCell::new(GameOptionsTable::new(
                game_setup_info.clone(),
                is_portrait,
            ))),
            map_options_table: Rc::new(RefCell::new(MapOptionsTable::new(
                game_setup_info.clone(),
            ))),
            is_portrait,
        };

        screen.try_update_ruleset(false);
        screen
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if self.is_portrait {
            self.show_portrait(ui);
        } else {
            self.show_landscape(ui);
        }
    }

    fn show_portrait(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("Game Options");
            ui.add_space(10.0);
            self.new_game_options_table.borrow_mut().show(ui);

            ui.add_space(10.0);
            ui.heading("Map Options");
            ui.add_space(10.0);
            self.map_options_table.borrow_mut().show(ui);

            ui.add_space(10.0);
            ui.heading("Civilizations");
            ui.add_space(10.0);
            self.player_picker_table.borrow_mut().show(ui);
        });
    }

    fn show_landscape(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Game Options");
                ui.add_space(10.0);
                self.new_game_options_table.borrow_mut().show(ui);
            });

            ui.vertical(|ui| {
                ui.heading("Map Options");
                ui.add_space(10.0);
                self.map_options_table.borrow_mut().show(ui);
            });

            ui.vertical(|ui| {
                ui.heading("Civilizations");
                ui.add_space(10.0);
                self.player_picker_table.borrow_mut().show(ui);
            });
        });
    }

    fn try_update_ruleset(&mut self, update_ui: bool) -> bool {
        let mut success = true;
        let mut new_ruleset = Ruleset::default();

        match RulesetCache::get_complex_ruleset(&self.game_setup_info.borrow().game_parameters) {
            Ok(ruleset) => {
                new_ruleset = ruleset;
            }
            Err(e) => {
                success = false;
                ToastPopup::new(
                    format!("Your previous options needed to be reset to defaults.\n\n{}", e),
                    self.clone(),
                    5000,
                );

                let mut params = self.game_setup_info.borrow_mut().game_parameters;
                params.mods.clear();
                params.base_ruleset = "Civ_V_GnK".to_string();
                new_ruleset = RulesetCache::get("Civ_V_GnK").unwrap();
            }
        }

        *this.ruleset.borrow_mut() = new_ruleset;

        if update_ui {
            this.new_game_options_table.borrow_mut().update_ruleset(&this.ruleset.borrow());
        }

        success
    }

    fn is_narrower_than_4to3() -> bool {
        // Implementation would depend on window/screen size handling
        false
    }

    pub fn start_game(&mut self) {
        // Disable input while checking
        // Implementation would depend on input handling system

        Concurrency::run(|| {
            if let Some(error_message) = this.get_error_message() {
                Concurrency::run_on_gl_thread(|| {
                    let mut popup = Popup::new(this.clone());
                    popup.add_label(&error_message);
                    popup.add_close_button();
                    popup.show();
                });
                return;
            }

            // Check mod compatibility
            let mod_check_result = this.new_game_options_table.borrow().mod_checkboxes.saved_modcheck_result.clone();
            this.new_game_options_table.borrow_mut().mod_checkboxes.saved_modcheck_result = None;

            if let Some(result) = mod_check_result {
                Concurrency::run_on_gl_thread(|| {
                    AcceptModErrorsPopup::new(
                        this.clone(),
                        result,
                        || this.new_game_options_table.borrow_mut().reset_ruleset(),
                        || {
                            this.game_setup_info.borrow_mut().game_parameters.accepted_mod_check_errors = Some(result);
                            this.start_game();
                        },
                    ).show();
                });
                return;
            }

            // Start new game
            Concurrency::run(|| {
                this.start_new_game();
            });
        });
    }

    fn get_error_message(&self) -> Option<String> {
        let params = this.game_setup_info.borrow().game_parameters;

        if params.is_online_multiplayer {
            if !this.check_connection_to_multiplayer_server() {
                return Some(if Multiplayer::uses_custom_server() {
                    "Couldn't connect to Multiplayer Server!".to_string()
                } else {
                    "Couldn't connect to Dropbox!".to_string()
                });
            }

            // Validate player IDs
            for player in params.players.iter().filter(|p| p.player_type == PlayerType::Human) {
                if Uuid::parse_str(&IdChecker::check_and_return_player_uuid(&player.player_id)).is_err() {
                    return Some("Invalid player ID!".to_string());
                }
            }

            // Check spectator permissions
            if !params.anyone_can_spectate {
                if !params.players.iter().any(|p| p.player_id == UncivGame::current().settings.multiplayer.user_id) {
                    return Some("You are not allowed to spectate!".to_string());
                }
            }
        }

        // Check for human players
        if !params.players.iter().any(|p| {
            p.player_type == PlayerType::Human &&
            !(p.chosen_civ == Constants::SPECTATOR && params.is_online_multiplayer)
        }) {
            return Some("No human players selected!".to_string());
        }

        // Check victory conditions
        if params.victory_types.is_empty() {
            return Some("No victory conditions were selected!".to_string());
        }

        // Check map compatibility
        if this.map_options_table.borrow().map_type_select_box.selected == MapGeneratedMainType::Custom {
            match MapSaver::load_map(&this.game_setup_info.borrow().map_file.unwrap()) {
                Ok(map) => {
                    let incompatibilities = map.get_ruleset_incompatibility(&this.ruleset.borrow());
                    if !incompatibilities.is_empty() {
                        return Some(format!(
                            "Map is incompatible with the chosen ruleset!\n{}",
                            incompatibilities.join("\n")
                        ));
                    }
                }
                Err(_) => return Some("Could not load map".to_string()),
            }
        } else {
            // Check generated map parameters
            let map_size = this.game_setup_info.borrow().map_parameters.map_size;
            if let Some(message) = map_size.fix_undesired_sizes(this.game_setup_info.borrow().map_parameters.world_wrap) {
                return Some(message);
            }
        }

        None
    }

    fn check_connection_to_multiplayer_server(&self) -> bool {
        // Implementation would depend on network handling
        true
    }

    fn start_new_game(&mut this) {
        let popup = Popup::new(this.clone());
        Concurrency::run_on_gl_thread(|| {
            popup.add_label(Constants::WORKING);
            popup.show();
        });

        let new_game = match this.map_options_table.borrow().get_selected_scenario() {
            None => GameStarter::start_new_game(&this.game_setup_info.borrow()),
            Some(scenario) => {
                let mut game_info = this.game.files.load_game_from_file(&scenario.file);
                // Convert spectator to AI
                if let Some(spectator) = game_info.civilizations.iter_mut()
                    .find(|c| c.civ_name == Constants::SPECTATOR)
                {
                    spectator.player_type = PlayerType::AI;
                }
                // Update player types
                for player_info in this.game_setup_info.borrow().game_parameters.players.iter() {
                    if let Some(civ) = game_info.civilizations.iter_mut()
                        .find(|c| c.civ_name == player_info.chosen_civ)
                    {
                        civ.player_type = player_info.player_type;
                    }
                }
                game_info
            }
        };

        if this.game_setup_info.borrow().game_parameters.is_online_multiplayer {
            new_game.is_up_to_date = true;
            match this.game.online_multiplayer.create_game(new_game.clone()) {
                Ok(_) => {
                    this.game.files.autosaves.request_auto_save(new_game);
                }
                Err(FileStorageRateLimitReached { limit_remaining_seconds }) => {
                    Concurrency::run_on_gl_thread(|| {
                        let mut popup = Popup::new(this.clone());
                        popup.add_label(&format!("Server limit reached! Please wait for {} seconds", limit_remaining_seconds));
                        popup.show();
                    });
                    return;
                }
                Err(e) => {
                    log::error!("Error while creating game: {}", e);
                    Concurrency::run_on_gl_thread(|| {
                        let mut popup = Popup::new(this.clone());
                        popup.add_label("Could not upload game!");
                        popup.show();
                    });
                    return;
                }
            }
        }

        let world_screen = this.game.load_game(new_game.clone());

        if new_game.game_parameters.is_online_multiplayer {
            Concurrency::run_on_gl_thread(|| {
                // Copy game ID to clipboard
                // Implementation would depend on clipboard handling
                ToastPopup::new("Game ID copied to clipboard!", world_screen, 2500);
            });
        }
    }
}