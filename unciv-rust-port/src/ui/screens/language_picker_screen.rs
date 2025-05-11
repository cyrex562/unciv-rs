// Source: orig_src/core/src/com/unciv/ui/screens/LanguagePickerScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Button, ScrollArea, RichText};
use crate::constants::ENGLISH;
use crate::ui::components::language_table::{LanguageTable, add_language_tables, add_language_key_shortcuts};
use crate::ui::screens::picker_screen::PickerScreen;
use crate::ui::screens::main_menu_screen::MainMenuScreen;
use crate::ui::popups::options::OptionsPopup;
use crate::logic::game::GameInfo;

/// A PickerScreen to select a language, used once on the initial run after a fresh install.
/// After that, OptionsPopup provides the functionality.
/// Reusable code is in LanguageTable and add_language_tables.
pub struct LanguagePickerScreen {
    game_info: Rc<RefCell<GameInfo>>,
    chosen_language: String,
    language_tables: Vec<LanguageTable>,
    close_button_visible: bool,
    right_side_button_text: String,
    right_side_button_enabled: bool,
}

impl LanguagePickerScreen {
    pub fn new(game_info: Rc<RefCell<GameInfo>>) -> Self {
        let mut screen = Self {
            game_info,
            chosen_language: ENGLISH.to_string(),
            language_tables: Vec::new(),
            close_button_visible: false,
            right_side_button_text: "Pick language".to_string(),
            right_side_button_enabled: false,
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Close button is not visible in this screen
        self.close_button_visible = false;

        // Create language tables
        let stage_width = 800.0; // TODO: Get actual stage width
        self.language_tables = add_language_tables(stage_width - 60.0);

        // Set up language key shortcuts
        add_language_key_shortcuts(&self.language_tables, &self.chosen_language, |language| {
            self.on_choice(language);
            // TODO: Implement scroll to selected table
        });

        self.right_side_button_text = "Pick language".to_string();
    }

    pub fn update(&mut self) {
        for table in &mut self.language_tables {
            table.update(&self.chosen_language);
        }
    }

    fn on_choice(&mut self, choice: String) {
        self.chosen_language = choice;
        self.right_side_button_enabled = true;
        self.update();
    }

    fn pick_language(&mut self) {
        let mut game_info = self.game_info.borrow_mut();
        game_info.settings.language = self.chosen_language.clone();
        game_info.settings.update_locale_from_language();
        game_info.settings.is_freshly_created = false; // mark so the picker isn't called next launch
        game_info.settings.save();

        game_info.translations.try_read_translation_for_current_language();

        // TODO: Replace current screen with MainMenuScreen
        // game_info.replace_current_screen(MainMenuScreen::new(game_info.clone()));
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Draw the language tables
        ScrollArea::vertical().show(ui, |ui| {
            for table in &mut self.language_tables {
                if ui.button(RichText::new(table.language.clone())).clicked() {
                    self.on_choice(table.language.clone());
                }
            }
        });

        // Draw the pick language button
        if ui.button(RichText::new(self.right_side_button_text.clone())).enabled(self.right_side_button_enabled).clicked() {
            self.pick_language();
        }
    }
}