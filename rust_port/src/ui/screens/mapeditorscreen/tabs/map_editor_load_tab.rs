use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::ui::components::*;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::{MapEditorFilesTable, MapEditorScreen};
use crate::files::MapSaver;
use crate::logic::{MissingModsException, UncivShowableException};
use crate::models::ruleset::RulesetCache;
use crate::utils::concurrency::Concurrency;
use crate::utils::logging::Log;
use crate::utils::translations::tr;

pub struct MapEditorLoadTab {
    editor_screen: Entity,
    header_height: f32,
    map_files: MapEditorFilesTable,
    load_button: TextButton,
    delete_button: TextButton,
    chosen_map: Option<PathBuf>,
}

impl MapEditorLoadTab {
    pub fn new(editor_screen: Entity, header_height: f32) -> Self {
        let mut tab = Self {
            editor_screen,
            header_height,
            map_files: MapEditorFilesTable::new(
                editor_screen.get_tools_width() - 20.0,
                true,
                Arc::new(move |file| tab.select_file(file)),
                Arc::new(move || tab.load_handler()),
            ),
            load_button: TextButton::new("Load map".tr()),
            delete_button: TextButton::new("Delete map".tr()),
            chosen_map: None,
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        let mut button_table = Table::new();
        button_table.defaults().pad(10.0).fill_x();

        self.load_button.on_activation(|| self.load_handler());
        self.load_button.add_keyboard_shortcut(KeyCharAndCode::RETURN);
        button_table.add(self.load_button.clone());

        self.delete_button.on_activation(|| self.delete_handler());
        self.delete_button.add_keyboard_shortcut(KeyCharAndCode::DEL);
        button_table.add(self.delete_button.clone());

        button_table.pack();

        let file_table_height = self.editor_screen.stage.height - self.header_height - button_table.height - 2.0;
        let mut scroll_pane = AutoScrollPane::new(self.map_files.clone());
        scroll_pane.set_overscroll(false, true);

        self.add(scroll_pane)
            .size(self.editor_screen.get_tools_width() - 20.0, file_table_height)
            .pad_top(10.0)
            .row();
        self.add(button_table).row();
    }

    fn load_handler(&mut self) {
        if self.chosen_map.is_none() {
            return;
        }

        self.editor_screen.ask_if_dirty_for_load(|| {
            self.editor_screen.start_background_job("MapLoader", move || self.loader_thread());
        });
    }

    fn delete_handler(&mut self) {
        if self.chosen_map.is_none() {
            return;
        }

        ConfirmPopup::new(
            &self.editor_screen,
            "Are you sure you want to delete this map?".tr(),
            "Delete map".tr(),
        )
        .on_confirm(|| {
            if let Some(map_path) = &self.chosen_map {
                std::fs::remove_file(map_path).unwrap();
                self.map_files.update();
            }
        })
        .open();
    }

    fn select_file(&mut self, file: Option<PathBuf>) {
        self.chosen_map = file;
        self.load_button.set_enabled(file.is_some());
        self.delete_button.set_enabled(file.is_some());
        self.delete_button.set_color(if file.is_some() { Color::SCARLET } else { Color::BROWN });
    }

    fn loader_thread(&mut self) {
        let mut popup: Option<Popup> = None;
        let mut need_popup = true; // loadMap can fail faster than postRunnable runs

        Concurrency::run_on_gl_thread(|| {
            if !need_popup {
                return;
            }
            popup = Some(LoadingPopup::new(&self.editor_screen));
        });

        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let map_path = self.chosen_map.as_ref().unwrap();
            let map = MapSaver::load_map(map_path)?;

            // For deprecated maps, set the base ruleset field if it's still saved in the mods field
            let mod_base_ruleset = map.map_parameters.mods.iter()
                .find(|&mod_name| {
                    RulesetCache::get(mod_name)
                        .map(|ruleset| ruleset.mod_options.is_base_ruleset)
                        .unwrap_or(false)
                });

            if let Some(base_ruleset) = mod_base_ruleset {
                map.map_parameters.base_ruleset = base_ruleset.clone();
                map.map_parameters.mods.retain(|mod_name| mod_name != base_ruleset);
            }

            let missing_mods: Vec<_> = (std::iter::once(&map.map_parameters.base_ruleset)
                .chain(map.map_parameters.mods.iter()))
                .filter(|&mod_name| !RulesetCache::contains(mod_name))
                .collect();

            if !missing_mods.is_empty() {
                return Err(Box::new(MissingModsException::new(missing_mods)));
            }

            Concurrency::run_on_gl_thread(|| {
                // This is to stop ANRs happening here, until the map editor screen sets up.
                self.editor_screen.set_input_processor(None);

                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let ruleset = RulesetCache::get_complex_ruleset(&map.map_parameters);
                    let ruleset_incompatibilities = map.get_ruleset_incompatibility(&ruleset);

                    if !ruleset_incompatibilities.is_empty() {
                        map.remove_missing_terrain_mod_references(&ruleset);
                        let message = format!(
                            "{}\n\n{}\n\n{}",
                            "This map has errors:".tr(),
                            ruleset_incompatibilities.iter()
                                .sorted()
                                .map(|e| e.tr())
                                .collect::<Vec<_>>()
                                .join("\n"),
                            "The incompatible elements have been removed.".tr()
                        );
                        ToastPopup::new(message, &self.editor_screen, 4000);
                    }

                    self.editor_screen.load_map(map, ruleset);
                    need_popup = false;
                    if let Some(p) = &popup {
                        p.close();
                    }
                    Ok(())
                })();

                if let Err(e) = result {
                    need_popup = false;
                    if let Some(p) = &popup {
                        p.close();
                    }
                    Log::error(&format!("Error displaying map \"{:?}\"", self.chosen_map), &e);
                    self.editor_screen.set_input_processor(Some(self.editor_screen.stage.clone()));
                    ToastPopup::new("Error loading map!".tr(), &self.editor_screen);
                }
            });

            Ok(())
        })();

        if let Err(e) = result {
            need_popup = false;
            Concurrency::run_on_gl_thread(|| {
                if let Some(p) = &popup {
                    p.close();
                }
                Log::error(&format!("Error loading map \"{:?}\"", self.chosen_map), &e);

                let error_message = if e.is::<UncivShowableException>() {
                    format!("{}\n{}", "Error loading map!".tr(), e.to_string())
                } else {
                    "Error loading map!".tr().to_string()
                };

                ToastPopup::new(error_message, &self.editor_screen);
            });
        }
    }

    pub fn no_maps_available(&self) -> bool {
        self.map_files.no_maps_available()
    }
}

// Extension trait for TabbedPager to support the IPageExtensions interface
pub trait TabbedPagerExtensions {
    fn activated(&mut self, index: usize, caption: &str, pager: &mut TabbedPager);
    fn deactivated(&mut self, index: usize, caption: &str, pager: &mut TabbedPager);
}

impl TabbedPagerExtensions for MapEditorLoadTab {
    fn activated(&mut self, _index: usize, _caption: &str, pager: &mut TabbedPager) {
        pager.set_scroll_disabled(true);
        self.map_files.update();
        self.select_file(None);
    }

    fn deactivated(&mut self, _index: usize, _caption: &str, pager: &mut TabbedPager) {
        pager.set_scroll_disabled(false);
    }
}