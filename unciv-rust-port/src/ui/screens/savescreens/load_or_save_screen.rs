// Source: orig_src/core/src/com/unciv/ui/screens/savescreens/LoadOrSaveScreen.kt

use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, Checkbox, TextEdit, RichText, Vec2};
use chrono::{DateTime, Local};
use crate::models::files::UncivFiles;
use crate::models::game_info::GameInfo;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::ui::popups::ConfirmPopup;
use crate::utils::concurrency::Concurrency;
use crate::utils::translation::tr;
use crate::ui::screens::savescreens::vertical_file_list_scroll_pane::VerticalFileListScrollPane;

/// Abstract base class for load and save screens
pub struct LoadOrSaveScreen {
    file_list_header_text: Option<String>,
    selected_save: Option<PathBuf>,
    saves_scroll_pane: VerticalFileListScrollPane,
    right_side_table: egui::Grid,
    delete_save_button: Button,
    show_autosaves_checkbox: Checkbox,
    picker_screen: PickerScreen,
}

impl LoadOrSaveScreen {
    /// Creates a new load or save screen
    pub fn new(file_list_header_text: Option<String>) -> Self {
        let mut screen = Self {
            file_list_header_text,
            selected_save: None,
            saves_scroll_pane: VerticalFileListScrollPane::new(),
            right_side_table: egui::Grid::new("right_side_table"),
            delete_save_button: Button::new(RichText::new(tr("Delete save"))),
            show_autosaves_checkbox: Checkbox::new(tr("Show autosaves")),
            picker_screen: PickerScreen::new(true),
        };

        screen.init();
        screen
    }

    /// Initializes the screen
    fn init(&mut self) {
        // Set up saves scroll pane
        self.saves_scroll_pane.on_change(|save_game_file| {
            self.select_existing_save(save_game_file);
        });

        // Set up right side table
        self.right_side_table.set_padding(5.0, 10.0);

        // Set up show autosaves checkbox
        let show_autosaves = UncivFiles::get_settings().show_autosaves;
        self.show_autosaves_checkbox.set_checked(show_autosaves);
        self.show_autosaves_checkbox.on_change(|checked| {
            self.update_shown_saves(checked);
            UncivFiles::get_settings_mut().show_autosaves = checked;
        });

        // Set up delete save button
        self.delete_save_button.set_enabled(false);
        self.delete_save_button.on_click(|| {
            self.on_delete_clicked();
        });

        // Add file list header if provided
        if let Some(header_text) = &self.file_list_header_text {
            self.picker_screen.add_to_top_table(
                egui::Label::new(RichText::new(tr(header_text))).padding(10.0)
            );
        }

        // Update shown saves
        self.update_shown_saves(show_autosaves);

        // Add saves scroll pane and right side table
        self.picker_screen.add_to_top_table(self.saves_scroll_pane.clone());
        self.picker_screen.add_to_top_table(self.right_side_table.clone());
    }

    /// Resets the window state
    pub fn reset_window_state(&mut self) {
        self.update_shown_saves(self.show_autosaves_checkbox.is_checked());
        self.delete_save_button.set_enabled(false);
        self.picker_screen.set_description_label_text("");
    }

    /// Handles delete button click
    fn on_delete_clicked(&mut self) {
        if self.selected_save.is_none() {
            return;
        }

        let save_path = self.selected_save.clone().unwrap();
        let name = save_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        let mut popup = ConfirmPopup::new(
            self.picker_screen.clone(),
            tr("Are you sure you want to delete this save?"),
            tr("Delete save"),
            true,
            None,
            Some(Box::new(move || {
                self.delete_save(save_path.clone());
            }))
        );

        popup.open();
    }

    /// Deletes a save file
    fn delete_save(&mut self, save_path: PathBuf) {
        let name = save_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        let result = match UncivFiles::delete_save(&save_path) {
            Ok(_) => {
                self.reset_window_state();
                format!("[{}] deleted successfully.", name)
            },
            Err(e) => {
                if e.to_string().contains("Permission denied") {
                    format!("Insufficient permissions to delete [{}].", name)
                } else {
                    format!("Failed to delete [{}].", name)
                }
            }
        };

        self.picker_screen.set_description_label_text(&tr(&result));
    }

    /// Updates the shown saves
    fn update_shown_saves(&mut self, show_autosaves: bool) {
        self.saves_scroll_pane.update_save_games(&UncivFiles::get_instance(), show_autosaves);
    }

    /// Selects an existing save
    fn select_existing_save(&mut self, save_game_file: PathBuf) {
        self.delete_save_button.set_enabled(true);

        self.selected_save = Some(save_game_file.clone());
        self.show_save_info(save_game_file.clone());
        self.picker_screen.set_right_side_button_visible(true);
        self.on_existing_save_selected(save_game_file);
    }

    /// Shows save info
    fn show_save_info(&mut self, save_game_file: PathBuf) {
        self.picker_screen.set_description_label_text(&tr("Loading..."));

        Concurrency::run("LoadMetaData", move || {
            // Even loading the game to get its metadata can take a long time on older phones
            let text_to_set = match UncivFiles::load_game_preview_from_file(&save_game_file) {
                Ok(game) => {
                    let saved_at = match save_game_file.metadata() {
                        Ok(metadata) => {
                            metadata.modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| DateTime::<Local>::from(std::time::UNIX_EPOCH + d))
                                .unwrap_or_else(|| Local::now())
                        },
                        Err(_) => Local::now(),
                    };

                    let player_civ_names = game.civilizations
                        .iter()
                        .filter(|civ| civ.is_player_civilization())
                        .map(|civ| tr(&civ.civ_name))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let mods = if game.game_parameters.mods.is_empty() {
                        String::new()
                    } else {
                        format!("\n{} {}", tr("Mods:"), game.game_parameters.mods.join(", "))
                    };

                    format!(
                        "{}\n{}: {}\n{}, {}, {} {}\n{} {}",
                        save_game_file.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown"),
                        tr("Saved at"),
                        saved_at.format("%Y-%m-%d %H:%M:%S"),
                        player_civ_names,
                        tr(&game.difficulty),
                        tr("Turn"),
                        game.turns,
                        tr("Base ruleset:"),
                        game.game_parameters.base_ruleset,
                        mods
                    )
                },
                Err(_) => format!("\n{}", tr("Could not load game")),
            };

            Concurrency::run_on_gl_thread(move || {
                self.picker_screen.set_description_label_text(&tr(&text_to_set));
            });
        });
    }

    /// Called when an existing save is selected
    fn on_existing_save_selected(&mut self, save_game_file: PathBuf);

    /// Called on double click
    fn double_click_action(&mut self);

    /// Shows the screen
    pub fn show(&mut self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }

    /// Gets the selected save
    pub fn get_selected_save(&self) -> Option<PathBuf> {
        self.selected_save.clone()
    }

    /// Gets the right side table
    pub fn get_right_side_table(&mut self) -> &mut egui::Grid {
        &mut self.right_side_table
    }

    /// Gets the delete save button
    pub fn get_delete_save_button(&mut self) -> &mut Button {
        &mut self.delete_save_button
    }

    /// Gets the show autosaves checkbox
    pub fn get_show_autosaves_checkbox(&mut self) -> &mut Checkbox {
        &mut self.show_autosaves_checkbox
    }

    /// Gets the picker screen
    pub fn get_picker_screen(&mut self) -> &mut PickerScreen {
        &mut self.picker_screen
    }
}