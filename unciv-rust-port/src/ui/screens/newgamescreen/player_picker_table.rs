use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, ScrollArea};
use uuid::Uuid;
use crate::models::metadata::{GameParameters, Player, PlayerType};
use crate::models::ruleset::{Ruleset, Nation};
use crate::models::ruleset::unique::UniqueType;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreens::PickerScreen;
use crate::ui::components::widgets::{ExpanderTab, WrappableLabel};
use crate::ui::popups::Popup;
use crate::utils::id_checker::IdChecker;
use crate::constants::Constants;
use crate::game::UncivGame;

/// This struct is used to pick or edit players information for new game creation.
/// Could be inserted to NewGameScreen, or any other BaseScreen
/// which provides GameSetupInfo and Ruleset.
/// Upon player changes updates property gameParameters. Also updates available nations when mod changes.
pub struct PlayerPickerTable {
    previous_screen: Rc<dyn IPreviousScreen>,
    game_parameters: Rc<RefCell<GameParameters>>,
    block_width: f32,
    player_list_table: Vec<PlayerTable>,
    random_number_label: Option<WrappableLabel>,
    locked: bool,
    no_random: bool,
    friend_list: FriendList,
}

struct PlayerTable {
    player: Player,
    nation_table: NationTable,
    player_type_button: egui::Button,
    remove_button: Option<egui::Button>,
    multiplayer_controls: Option<MultiplayerControls>,
}

struct NationTable {
    nation_image: String,
    nation_name: String,
}

struct MultiplayerControls {
    player_id_text: String,
    error_label: String,
    set_current_user_button: egui::Button,
    copy_from_clipboard_button: egui::Button,
    select_from_friends_button: Option<egui::Button>,
}

impl PlayerPickerTable {
    pub fn new(
        previous_screen: Rc<dyn IPreviousScreen>,
        game_parameters: Rc<RefCell<GameParameters>>,
        block_width: f32,
    ) -> Self {
        let mut table = Self {
            previous_screen,
            game_parameters,
            block_width,
            player_list_table: Vec::new(),
            random_number_label: None,
            locked: false,
            no_random: false,
            friend_list: FriendList::new(),
        };

        // Clear player IDs for multiplayer security
        let mut params = table.game_parameters.borrow_mut();
        for player in &mut params.players {
            player.player_id = String::new();
        }
        params.shuffle_player_order = false;

        table.update();
        table
    }

    pub fn update(&mut self, desired_civ: &str) {
        self.player_list_table.clear();
        let game_basics = &self.previous_screen.ruleset();

        self.reassign_removed_mod_references();
        let new_ruleset_playable_civs = game_basics.nations
            .values()
            .filter(|n| n.name != Constants::BARBARIANS && !n.has_unique(UniqueType::WillNotBeChosenForNewGames))
            .count();

        let mut params = self.game_parameters.borrow_mut();
        if params.players.len() > new_ruleset_playable_civs {
            params.players.truncate(new_ruleset_playable_civs);
        }

        if !desired_civ.is_empty() {
            self.assign_desired_civ(desired_civ);
        }

        for player in &params.players {
            self.player_list_table.push(self.get_player_table(player.clone()));
        }

        if params.random_number_of_players {
            let width = if self.block_width <= 10.0 {
                self.previous_screen.stage_width() / 3.0 - 5.0
            } else {
                self.block_width
            };

            self.random_number_label = Some(WrappableLabel::new(
                String::new(),
                width - 20.0,
                Color32::GOLD,
            ));
            self.update_random_number_label();
        }

        if !self.locked && params.players.len() < game_basics.nations.values()
            .filter(|n| n.is_major_civ)
            .count()
        {
            // Add player button logic here
        }

        // Enable start game when at least one human player and they're not alone
        let human_player_count = params.players.iter()
            .filter(|p| p.player_type == PlayerType::Human)
            .count();
        let is_valid = human_player_count >= 1 &&
            (params.random_number_of_players || params.players.len() >= 2);

        if let Some(picker) = self.previous_screen.as_any().downcast_ref::<PickerScreen>() {
            picker.set_right_side_button_enabled(is_valid);
        }
    }

    fn reassign_removed_mod_references(&mut self) {
        let mut params = self.game_parameters.borrow_mut();
        for player in &mut params.players {
            if !self.previous_screen.ruleset().nations.contains_key(&player.chosen_civ) ||
                self.previous_screen.ruleset().nations[&player.chosen_civ].is_city_state ||
                self.previous_screen.ruleset().nations[&player.chosen_civ].has_unique(UniqueType::WillNotBeChosenForNewGames)
            {
                player.chosen_civ = Constants::RANDOM.to_string();
            }
        }
    }

    fn assign_desired_civ(&mut self, desired_civ: &str) {
        let mut params = self.game_parameters.borrow_mut();
        // No auto-select if desired_civ already used
        if params.players.iter().any(|p| p.chosen_civ == desired_civ) {
            return;
        }
        // Do auto-select, silently no-op if no suitable slot (human with 'random' choice)
        if let Some(player) = params.players.iter_mut()
            .find(|p| p.chosen_civ == Constants::RANDOM && p.player_type == PlayerType::Human)
        {
            player.chosen_civ = desired_civ.to_string();
        }
    }

    fn get_player_table(&self, player: Player) -> PlayerTable {
        let nation_table = self.get_nation_table(&player);
        let player_type_button = egui::Button::new(player.player_type.to_string());

        let mut table = PlayerTable {
            player,
            nation_table,
            player_type_button,
            remove_button: None,
            multiplayer_controls: None,
        };

        if !self.locked {
            table.remove_button = Some(egui::Button::new("-"));
        }

        if self.game_parameters.borrow().is_online_multiplayer &&
           table.player.player_type == PlayerType::Human {
            table.multiplayer_controls = Some(self.create_multiplayer_controls(&table.player));
        }

        table
    }

    fn get_nation_table(&self, player: &Player) -> NationTable {
        let nation_image_name = self.previous_screen.ruleset().nations.get(&player.chosen_civ);
        let (nation_image, nation_name) = if let Some(nation) = nation_image_name {
            (nation.get_portrait(40.0), nation.name.clone())
        } else {
            (self.get_random_nation_portrait(40.0), player.chosen_civ.clone())
        };

        NationTable {
            nation_image,
            nation_name,
        }
    }

    fn create_multiplayer_controls(&self, player: &Player) -> MultiplayerControls {
        MultiplayerControls {
            player_id_text: player.player_id.clone(),
            error_label: String::from("✘"),
            set_current_user_button: egui::Button::new("Set current user"),
            copy_from_clipboard_button: egui::Button::new("Player ID from clipboard"),
            select_from_friends_button: if !self.friend_list.list_of_friends.is_empty() {
                Some(egui::Button::new("Player ID from friends list"))
            } else {
                None
            },
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.checkbox(&mut self.game_parameters.borrow_mut().shuffle_player_order, "Shuffle Civ Order at Start");

            ScrollArea::vertical().show(ui, |ui| {
                for table in &mut self.player_list_table {
                    ui.horizontal(|ui| {
                        ui.add(egui::Image::new(table.nation_table.nation_image.clone()));
                        ui.label(&table.nation_table.nation_name);

                        if ui.button(&table.player_type_button.text).clicked() {
                            let mut params = self.game_parameters.borrow_mut();
                            if let Some(player) = params.players.iter_mut()
                                .find(|p| p.id == table.player.id)
                            {
                                player.player_type = player.player_type.toggle();
                                self.update("");
                            }
                        }

                        if let Some(ref remove_button) = table.remove_button {
                            if ui.button(remove_button.text).clicked() {
                                let mut params = self.game_parameters.borrow_mut();
                                params.players.retain(|p| p.id != table.player.id);
                                self.update("");
                            }
                        }
                    });

                    if let Some(ref controls) = table.multiplayer_controls {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut controls.player_id_text);
                            ui.label(&controls.error_label);

                            if ui.button(&controls.set_current_user_button.text).clicked() {
                                controls.player_id_text = UncivGame::current().settings.multiplayer.user_id.clone();
                                self.validate_player_id(&controls.player_id_text);
                            }

                            if ui.button(&controls.copy_from_clipboard_button.text).clicked() {
                                // Clipboard handling would go here
                                self.validate_player_id(&controls.player_id_text);
                            }

                            if let Some(ref select_button) = controls.select_from_friends_button {
                                if ui.button(select_button.text).clicked() {
                                    self.popup_friend_picker(&table.player);
                                }
                            }
                        });
                    }
                }
            });
        });
    }

    fn validate_player_id(&mut self, id: &str) {
        match Uuid::parse_str(&IdChecker::check_and_return_player_uuid(id)) {
            Ok(_) => {
                let mut params = self.game_parameters.borrow_mut();
                if let Some(player) = params.players.iter_mut()
                    .find(|p| p.player_id == id)
                {
                    player.player_id = id.trim().to_string();
                }
            }
            Err(_) => {
                // Handle invalid ID
            }
        }
    }

    fn popup_friend_picker(&mut self, player: &Player) {
        // Friend picker popup implementation would go here
        self.update("");
    }
}