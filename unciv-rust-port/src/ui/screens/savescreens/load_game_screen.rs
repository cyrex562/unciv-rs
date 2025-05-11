// Source: orig_src/core/src/com/unciv/ui/screens/savescreens/LoadGameScreen.kt

use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, RichText, Vec2};
use crate::models::files::UncivFiles;
use crate::models::game_info::GameInfo;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::translation::tr;
use crate::ui::screens::savescreens::load_or_save_screen::LoadOrSaveScreen;

/// Screen for loading saved games
pub struct LoadGameScreen {
    base: LoadOrSaveScreen,
    load_button: Button,
    game_info: Option<GameInfo>,
}

impl LoadGameScreen {
    /// Creates a new load game screen
    pub fn new() -> Self {
        let mut screen = Self {
            base: LoadOrSaveScreen::new(Some("Select a save to load")),
            load_button: Button::new(RichText::new(tr("Load"))),
            game_info: None,
        };

        screen.init();
        screen
    }

    /// Initializes the screen
    fn init(&mut self) {
        // Set up load button
        self.load_button.set_enabled(false);
        self.load_button.on_click(|| {
            self.load_game();
        });

        // Add load button to right side table
        self.base.get_right_side_table().add(self.load_button.clone());

        // Set up double click action
        self.base.get_picker_screen().set_double_click_action(|| {
            self.load_game();
        });
    }

    /// Loads the selected game
    fn load_game(&mut self) {
        if let Some(save_path) = self.base.get_selected_save() {
            self.base.get_picker_screen().set_description_label_text(&tr("Loading..."));

            let save_path_clone = save_path.clone();
            Concurrency::run("LoadGame", move || {
                match UncivFiles::load_game_from_file(&save_path_clone) {
                    Ok(game_info) => {
                        Concurrency::run_on_gl_thread(move || {
                            self.game_info = Some(game_info);
                            self.base.get_picker_screen().close();
                        });
                    },
                    Err(e) => {
                        Concurrency::run_on_gl_thread(move || {
                            self.base.get_picker_screen().set_description_label_text(&tr(&format!("Failed to load game: {}", e)));
                        });
                    }
                }
            });
        }
    }

    /// Gets the loaded game info
    pub fn get_game_info(&self) -> Option<GameInfo> {
        self.game_info.clone()
    }

    /// Shows the screen
    pub fn show(&mut self, ui: &mut Ui) {
        self.base.show(ui);
    }
}

impl LoadOrSaveScreen for LoadGameScreen {
    /// Called when an existing save is selected
    fn on_existing_save_selected(&mut self, save_game_file: PathBuf) {
        self.load_button.set_enabled(true);
    }

    /// Called on double click
    fn double_click_action(&mut self) {
        self.load_game();
    }
}