use std::path::PathBuf;
use std::fs;
use std::collections::{HashMap, BTreeMap};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Ui, Vec2, Button, ScrollArea};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::components::widgets::unciv_button::UncivButton;
use crate::ui::components::widgets::unciv_label::UncivLabel;
use crate::ui::components::extensions::*;
use crate::ui::images::image_getter::ImageGetter;
use crate::map::map_saver::MapSaver;
use crate::models::ruleset::ruleset_cache::RulesetCache;
use crate::utils::log::Log;

/// Table for displaying and selecting map files
pub struct MapEditorFilesTable {
    init_width: f32,
    include_mods: bool,
    on_select: Box<dyn Fn(PathBuf)>,
    on_double_click: Box<dyn Fn()>,
    selected_index: i32,
    sorted_files: Vec<ListEntry>,
    ui: egui::Ui,
}

/// Entry in the file list
#[derive(Clone)]
struct ListEntry {
    mod_name: String,
    file: PathBuf,
}

impl MapEditorFilesTable {
    pub fn new(
        init_width: f32,
        include_mods: bool,
        on_select: Box<dyn Fn(PathBuf)>,
        on_double_click: Box<dyn Fn()>,
    ) -> Self {
        Self {
            init_width,
            include_mods,
            on_select,
            on_double_click,
            selected_index: -1,
            sorted_files: Vec::new(),
            ui: egui::Ui::default(),
        }
    }

    fn mark_selection(&mut self, button: &mut UncivButton, row: i32) {
        // In a real implementation, we would update the button colors
        // For now, just log the selection
        Log::info(&format!("Selected file at index {}", row));

        self.selected_index = row;
        if row >= 0 && row < self.sorted_files.len() as i32 {
            (self.on_select)(self.sorted_files[row as usize].file.clone());
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let new_index = if self.selected_index + delta >= 0 &&
                          self.selected_index + delta < self.sorted_files.len() as i32 {
            self.selected_index + delta
        } else if self.selected_index + delta < 0 {
            self.sorted_files.len() as i32 - 1
        } else {
            0
        };

        // In a real implementation, we would update the button colors
        // For now, just log the selection change
        Log::info(&format!("Moved selection from {} to {}", self.selected_index, new_index));

        self.selected_index = new_index;
        if new_index >= 0 && new_index < self.sorted_files.len() as i32 {
            (self.on_select)(self.sorted_files[new_index as usize].file.clone());
        }
    }

    pub fn update(&mut self) {
        self.sorted_files.clear();

        // Get maps from the main maps directory
        let maps = MapSaver::get_maps();
        for map in maps {
            self.sorted_files.push(ListEntry {
                mod_name: String::new(),
                file: map,
            });
        }

        // Sort by last modified time (newest first)
        self.sorted_files.sort_by(|a, b| {
            let a_time = fs::metadata(&a.file)
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            let b_time = fs::metadata(&b.file)
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        // Include mod maps if requested
        if self.include_mods {
            for ruleset in RulesetCache::values() {
                if let Some(mod_folder) = &ruleset.folder_location {
                    let maps_folder = mod_folder.join(MapSaver::MAPS_FOLDER);
                    if maps_folder.exists() {
                        if let Ok(entries) = fs::read_dir(&maps_folder) {
                            for entry in entries.flatten() {
                                if let Ok(file_type) = entry.file_type() {
                                    if file_type.is_file() {
                                        self.sorted_files.push(ListEntry {
                                            mod_name: mod_folder.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            file: entry.path(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Sort mod maps by name
            let mod_maps: Vec<_> = self.sorted_files.iter()
                .filter(|e| !e.mod_name.is_empty())
                .cloned()
                .collect();

            let main_maps: Vec<_> = self.sorted_files.iter()
                .filter(|e| e.mod_name.is_empty())
                .cloned()
                .collect();

            self.sorted_files.clear();
            self.sorted_files.extend(main_maps);

            // Sort mod maps by name
            let mut sorted_mod_maps = mod_maps;
            sorted_mod_maps.sort_by(|a, b| a.file.file_name().unwrap_or_default().cmp(&b.file.file_name().unwrap_or_default()));

            self.sorted_files.extend(sorted_mod_maps);
        }
    }

    pub fn render(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            let mut last_mod = String::new();

            for (index, entry) in self.sorted_files.iter().enumerate() {
                let (mod_name, map_file) = &entry;

                if *mod_name != last_mod {
                    // One header per Mod
                    ui.horizontal(|ui| {
                        ui.add(ImageGetter::get_dot(Color32::from_gray(200)));
                        ui.label(mod_name);
                        ui.add(ImageGetter::get_dot(Color32::from_gray(200)));
                    });

                    last_mod = mod_name.clone();
                }

                let mut button = UncivButton::new(map_file.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(""));

                if ui.add(button.clone()).clicked() {
                    self.mark_selection(&mut button, index as i32);
                }

                if ui.add(button.clone()).double_clicked() {
                    self.mark_selection(&mut button, index as i32);
                    (self.on_double_click)();
                }
            }
        });
    }

    pub fn no_maps_available(&self) -> bool {
        if !MapSaver::get_maps().is_empty() {
            return false;
        }

        if !self.include_mods {
            return true;
        }

        for ruleset in RulesetCache::values() {
            if let Some(mod_folder) = &ruleset.folder_location {
                let maps_folder = mod_folder.join(MapSaver::MAPS_FOLDER);
                if maps_folder.exists() {
                    if let Ok(entries) = fs::read_dir(&maps_folder) {
                        if entries.count() > 0 {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }
}