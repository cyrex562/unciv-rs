// Source: orig_src/core/src/com/unciv/ui/screens/savescreens/VerticalFileListScrollPane.kt

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, ScrollArea, Vec2, RichText};
use crate::models::files::UncivFiles;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::utils::concurrency::Concurrency;

/// A widget holding buttons vertically in a scroll area, with methods to
/// hold file names and paths in those buttons. Used to display existing saves in the Load and Save game dialogs.
pub struct VerticalFileListScrollPane {
    saves_per_button: HashMap<Button, PathBuf>,
    previous_selection: Option<Button>,
    on_change_listener: Option<Box<dyn Fn(PathBuf)>>,
    existing_saves_table: egui::Grid,
}

impl VerticalFileListScrollPane {
    /// Creates a new vertical file list scroll pane
    pub fn new() -> Self {
        Self {
            saves_per_button: HashMap::new(),
            previous_selection: None,
            on_change_listener: None,
            existing_saves_table: egui::Grid::new("file_list_grid"),
        }
    }

    /// Sets the change listener
    pub fn on_change<F: Fn(PathBuf) + 'static>(&mut self, action: F) {
        self.on_change_listener = Some(Box::new(action));
    }

    /// Repopulate with existing saved games
    pub fn update_save_games(&mut self, files: &UncivFiles, show_autosaves: bool) {
        let saves = files.get_saves(show_autosaves);
        let mut sorted_saves: Vec<_> = saves.collect();
        sorted_saves.sort_by(|a, b| {
            b.metadata()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .cmp(&a.metadata()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0))
        });
        self.update(sorted_saves);
    }

    /// Repopulate from a sequence of paths - for other sources than saved games
    pub fn update(&mut self, files: Vec<PathBuf>) {
        self.existing_saves_table.clear();
        self.previous_selection = None;

        // Show loading indicator
        let loading_image = ImageGetter::get_image("OtherIcons/Load");
        // TODO: Implement loading animation

        // Apparently, even just getting the list of saves can cause ANRs -
        // not sure how many saves these guys had but Google Play reports this to have happened hundreds of times
        Concurrency::run("GetSaves", move || {
            // Materialize the result of the sequence
            let saves = files;

            Concurrency::run_on_gl_thread(move || {
                // Clear loading animation
                // TODO: Implement loading animation reset

                self.existing_saves_table.clear();
                self.saves_per_button.clear();

                for save_game_file in saves {
                    let file_name = save_game_file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");

                    let button = Button::new(RichText::new(file_name));
                    self.saves_per_button.insert(button.clone(), save_game_file.clone());

                    // Set up click handler
                    let file_path = save_game_file.clone();
                    button.on_click(move || {
                        self.select_existing_save(button.clone(), file_path.clone());
                    });

                    self.existing_saves_table.add(button).padding(5.0);
                }
            });
        });
    }

    /// Selects an existing save
    fn select_existing_save(&mut self, button: Button, save_game_file: PathBuf) {
        // Reset previous selection color
        if let Some(prev) = &self.previous_selection {
            prev.set_color(Color32::WHITE);
        }

        // Set new selection color
        button.set_color(Color32::GREEN);
        self.previous_selection = Some(button.clone());

        // Notify listener
        if let Some(listener) = &self.on_change_listener {
            listener(save_game_file);
        }
    }

    /// Shows the scroll pane
    pub fn show(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            self.existing_saves_table.show(ui);
        });
    }

    /// Handles keyboard navigation
    pub fn handle_keyboard(&mut self, key: egui::Key) {
        match key {
            egui::Key::ArrowUp => self.on_arrow_key(-1),
            egui::Key::ArrowDown => self.on_arrow_key(1),
            egui::Key::PageUp => self.on_page_key(-1),
            egui::Key::PageDown => self.on_page_key(1),
            egui::Key::Home => self.on_home_end_key(0),
            egui::Key::End => self.on_home_end_key(1),
            _ => {}
        }
    }

    /// Handles arrow key navigation
    fn on_arrow_key(&mut self, direction: i32) {
        // TODO: Implement arrow key navigation
        // This would require tracking button positions and scroll position
    }

    /// Handles page key navigation
    fn on_page_key(&mut self, direction: i32) {
        // TODO: Implement page key navigation
    }

    /// Handles home/end key navigation
    fn on_home_end_key(&mut self, direction: i32) {
        // TODO: Implement home/end key navigation
    }
}