use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::fs;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Slider, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::{MapEditorScreen, MapEditorFilesTable};
use crate::ui::components::widgets::tabbed_pager::TabbedPager;
use crate::ui::components::widgets::tabbed_pager::PageExtensions;
use crate::ui::components::widgets::unciv_text_field::UncivTextField;
use crate::ui::components::widgets::auto_scroll_pane::AutoScrollPane;
use crate::ui::components::extensions::*;
use crate::ui::popups::{toast_popup::ToastPopup, confirm_popup::ConfirmPopup, popup::Popup};
use crate::logic::files::map_saver::MapSaver;
use crate::logic::map::map_generated_main_type::MapGeneratedMainType;
use crate::logic::map::tile_map::TileMap;
use crate::models::translations::tr;
use crate::utils::concurrency::Concurrency;
use crate::utils::log::Log;
use crate::constants::Constants;

/// Tab for saving and loading maps
pub struct MapEditorSaveTab {
    editor_screen: Rc<RefCell<MapEditorScreen>>,
    header_height: f32,
    map_files: MapEditorFilesTable,
    save_button: egui::Button,
    delete_button: egui::Button,
    quit_button: egui::Button,
    map_name_text_field: UncivTextField,
    chosen_map: Option<PathBuf>,
}

impl MapEditorSaveTab {
    pub fn new(editor_screen: Rc<RefCell<MapEditorScreen>>, header_height: f32) -> Self {
        let tools_width = editor_screen.borrow().get_tools_width() - 40.0;

        let map_files = MapEditorFilesTable::new(
            tools_width,
            false,
            Box::new(|file| {
                // This will be set in the render method
            }),
            Box::new(|| {
                // This will be set in the render method
            })
        );

        let mut map_name_text_field = UncivTextField::new("Map Name");
        map_name_text_field.max_length = 100;
        map_name_text_field.text_field_filter = Some(Box::new(|_, char| char != '\\' && char != '/'));
        map_name_text_field.select_all();

        Self {
            editor_screen,
            header_height,
            map_files,
            save_button: egui::Button::new("Save map"),
            delete_button: egui::Button::new("Delete map"),
            quit_button: egui::Button::new("Exit map editor"),
            map_name_text_field,
            chosen_map: None,
        }
    }

    fn set_save_button(&mut self, enabled: bool) {
        self.save_button.enabled = enabled;
        self.save_button.text = if enabled {
            "Save map".to_string()
        } else {
            Constants::WORKING.to_string()
        };
    }

    fn save_handler(&mut self) {
        if self.map_name_text_field.text.is_blank() {
            return;
        }

        let mut editor = self.editor_screen.borrow_mut();
        editor.tile_map.map_parameters.name = self.map_name_text_field.text.clone();
        editor.tile_map.map_parameters.type_ = MapGeneratedMainType::Custom;
        editor.tile_map.description = editor.description_text_field.text.clone();

        self.set_save_button(false);

        // Start background job
        editor.start_background_job("MapSaver", false, Box::new(|| {
            self.saver_thread();
        }));
    }

    fn delete_handler(&mut self) {
        if self.chosen_map.is_none() {
            return;
        }

        let chosen_map = self.chosen_map.clone().unwrap();

        ConfirmPopup::new(
            &self.editor_screen.borrow(),
            "Are you sure you want to delete this map?",
            "Delete map",
            true,
            None,
            Box::new(move || {
                if let Err(e) = fs::remove_file(&chosen_map) {
                    Log::error("Failed to delete map", &e);
                }
                // Update the file list
                // In a real implementation, we would call map_files.update()
            })
        ).show();
    }

    fn select_file(&mut self, file: Option<PathBuf>) {
        self.chosen_map = file.clone();

        if let Some(file) = file {
            self.map_name_text_field.text = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
        } else {
            let editor = self.editor_screen.borrow();
            self.map_name_text_field.text = editor.tile_map.map_parameters.name.clone();
        }

        if self.map_name_text_field.text.is_blank() {
            self.map_name_text_field.text = "My new map".to_string();
        }

        // Set selection to end of text
        self.map_name_text_field.set_selection(i32::MAX, i32::MAX);

        // Set keyboard focus
        // In a real implementation, we would set the keyboard focus to the text field

        self.save_button.enabled = true;
        self.delete_button.enabled = file.is_some();

        // Set button color
        if file.is_some() {
            self.delete_button.color = Color32::from_rgb(255, 0, 0); // SCARLET
        } else {
            self.delete_button.color = Color32::from_rgb(139, 69, 19); // BROWN
        }
    }

    fn saver_thread(&self) {
        let editor = self.editor_screen.borrow();
        let map_to_save = editor.get_map_clone_for_save();

        // Check if the coroutine is still active
        // In a real implementation, we would check if the coroutine is still active

        // Assign continents
        if let Err(e) = map_to_save.assign_continents(TileMap::AssignContinentsMode::Reassign) {
            Log::error("Failed to assign continents", &e);
            return;
        }

        // Check if the coroutine is still active
        // In a real implementation, we would check if the coroutine is still active

        // Save the map
        match MapSaver::save_map(&self.map_name_text_field.text, &map_to_save) {
            Ok(_) => {
                // Run on GL thread
                Concurrency::run_on_gl_thread(Box::new(|| {
                    ToastPopup::new("Map saved successfully!", &self.editor_screen.borrow()).show();
                }));

                let mut editor = self.editor_screen.borrow_mut();
                editor.is_dirty = false;

                // Run on GL thread
                Concurrency::run_on_gl_thread(Box::new(|| {
                    self.set_save_button(true);
                }));
            },
            Err(e) => {
                Log::error("Failed to save map", &e);

                // Run on GL thread
                Concurrency::run_on_gl_thread(Box::new(|| {
                    let mut popup = Popup::new(&self.editor_screen.borrow());
                    popup.add_good_sized_label("It looks like your map can't be saved!");
                    popup.add_close_button();
                    popup.show(true);

                    self.set_save_button(true);
                }));
            }
        }
    }
}

impl PageExtensions for MapEditorSaveTab {
    fn activated(&mut self, _index: usize, _caption: &str, pager: &mut TabbedPager) {
        pager.set_scroll_disabled(true);

        // Update the file list
        // In a real implementation, we would call map_files.update()

        self.select_file(None);
    }

    fn deactivated(&mut self, _index: usize, _caption: &str, pager: &mut TabbedPager) {
        pager.set_scroll_disabled(false);

        // Set keyboard focus to null
        // In a real implementation, we would set the keyboard focus to null
    }
}

impl MapEditorSaveTab {
    pub fn render(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Map name text field
            ui.add_space(10.0);
            ui.add(self.map_name_text_field.clone());

            // Buttons
            ui.horizontal(|ui| {
                ui.add_space(10.0);

                if ui.add(self.save_button.clone()).clicked() {
                    self.save_handler();
                }

                if ui.add(self.delete_button.clone()).clicked() {
                    self.delete_handler();
                }

                if ui.add(self.quit_button.clone()).clicked() {
                    let mut editor = self.editor_screen.borrow_mut();
                    editor.close_editor();
                }
            });

            // File table
            let file_table_height = self.editor_screen.borrow().stage.height -
                self.header_height -
                self.map_name_text_field.pref_height -
                22.0;

            let mut scroll_pane = AutoScrollPane::new(&self.map_files);
            scroll_pane.set_overscroll(false, true);

            ui.add_space(file_table_height);
            ui.add(scroll_pane);
        });
    }
}