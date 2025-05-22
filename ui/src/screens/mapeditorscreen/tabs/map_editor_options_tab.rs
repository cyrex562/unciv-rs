use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Slider, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::components::widgets::tabbed_pager::TabbedPager;
use crate::ui::components::widgets::tabbed_pager::PageExtensions;
use crate::ui::components::extensions::*;
use crate::ui::components::input::*;
use crate::ui::popups::toast_popup::ToastPopup;
use crate::files::file_chooser::FileChooser;
use crate::map::map_saver::MapSaver;
use crate::logic::map::map_shape::MapShape;
use crate::logic::map::map_size::MapSize;
use crate::models::translations::tr;
use crate::utils::log::Log;

/// Enum representing different levels of tile matching fuzziness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileMatchFuzziness {
    CompleteMatch,
    NoImprovement,
    BaseAndFeatures,
    BaseTerrain,
    LandOrWater,
}

impl TileMatchFuzziness {
    fn label(&self) -> &str {
        match self {
            TileMatchFuzziness::CompleteMatch => "Complete match",
            TileMatchFuzziness::NoImprovement => "Except improvements",
            TileMatchFuzziness::BaseAndFeatures => "Base and terrain features",
            TileMatchFuzziness::BaseTerrain => "Base terrain only",
            TileMatchFuzziness::LandOrWater => "Land or water only",
        }
    }
}

/// Tab for map editor options
pub struct MapEditorOptionsTab {
    editor_screen: Rc<RefCell<MapEditorScreen>>,
    seed_label: String,
    seed_to_copy: String,
    tile_match_fuzziness: TileMatchFuzziness,
    world_wrap: bool,
    overlay_file: Option<String>,
    overlay_alpha: f32,
}

impl MapEditorOptionsTab {
    pub fn new(editor_screen: Rc<RefCell<MapEditorScreen>>) -> Self {
        Self {
            editor_screen,
            seed_label: String::new(),
            seed_to_copy: String::new(),
            tile_match_fuzziness: TileMatchFuzziness::CompleteMatch,
            world_wrap: false,
            overlay_file: None,
            overlay_alpha: 1.0,
        }
    }

    /// Check whether we can flip world wrap without ruining geometry
    fn can_change_world_wrap(&self) -> bool {
        let editor = self.editor_screen.borrow();
        let params = &editor.tile_map.map_parameters;

        // Can't change for hexagonal at all, as non-ww must always have an odd number of columns and ww must have an even number of columns
        if params.shape != MapShape::Rectangular {
            return false;
        }

        // Too small?
        if params.map_size.radius < MapSize::Tiny.radius {
            return false;
        }

        // Even-width rectangular have no problems, but that has not necessarily been saved in mapSize!
        if params.map_size.width % 2 == 0 {
            return true;
        }

        // The recorded width may have been reduced to even by the TileMap constructor.
        // In such a case we allow turning WW off, and editorScreen.setWorldWrap will fix the width.
        params.world_wrap
    }

    fn copy_handler(&self) {
        let editor = self.editor_screen.borrow();
        let map_string = MapSaver::map_to_saved_string(editor.get_map_clone_for_save());
        // In Rust, we'd use the clipboard crate or similar
        // For now, just log that we would copy to clipboard
        Log::info(&format!("Would copy to clipboard: {}", map_string));
    }

    fn paste_handler(&self) {
        let editor = self.editor_screen.borrow();
        // In Rust, we'd use the clipboard crate or similar
        // For now, just log that we would paste from clipboard
        Log::info("Would paste from clipboard");

        // This is a placeholder for the actual implementation
        // In a real implementation, we would:
        // 1. Get clipboard contents
        // 2. Parse the map
        // 3. Load it into the editor
        // 4. Handle errors with ToastPopup
    }

    fn show_overlay_file_name(&mut self) {
        // This would update the UI to show the current overlay file name
        // In Rust/egui, this would be handled in the render method
    }
}

impl PageExtensions for MapEditorOptionsTab {
    fn activated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        let editor = self.editor_screen.borrow();
        self.seed_to_copy = editor.tile_map.map_parameters.seed.to_string();
        self.seed_label = format!("Current map RNG seed: [{}]", self.seed_to_copy);
        self.update();
        self.overlay_alpha = editor.overlay_alpha;
    }

    fn deactivated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        let mut editor = self.editor_screen.borrow_mut();
        editor.tile_match_fuzziness = self.tile_match_fuzziness;
    }
}

impl MapEditorOptionsTab {
    pub fn update(&mut self) {
        let editor = self.editor_screen.borrow();

        // Update world wrap checkbox state
        self.world_wrap = editor.tile_map.map_parameters.world_wrap;

        // Update overlay file name
        self.show_overlay_file_name();
    }

    pub fn render(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Tile Matching Criteria
            ui.heading("Tile Matching Criteria");

            for option in [
                TileMatchFuzziness::CompleteMatch,
                TileMatchFuzziness::NoImprovement,
                TileMatchFuzziness::BaseAndFeatures,
                TileMatchFuzziness::BaseTerrain,
                TileMatchFuzziness::LandOrWater,
            ] {
                let mut selected = self.tile_match_fuzziness == option;
                if ui.checkbox(&mut selected, option.label()).clicked() {
                    self.tile_match_fuzziness = option;
                }
            }

            ui.add_space(10.0);
            ui.separator();

            // Seed
            ui.label(&self.seed_label);
            if ui.button("Copy to clipboard").clicked() {
                // In Rust, we'd use the clipboard crate or similar
                Log::info(&format!("Would copy to clipboard: {}", self.seed_to_copy));
            }

            ui.add_space(10.0);
            ui.separator();

            // Map copy and paste
            ui.heading("Map copy and paste");
            ui.horizontal(|ui| {
                if ui.button("Copy to clipboard").clicked() {
                    self.copy_handler();
                }

                if ui.button("Load copied data").clicked() {
                    self.paste_handler();
                }
            });

            // Import Wesnoth map
            if ui.button("Import a Wesnoth map").clicked() {
                let mut editor = self.editor_screen.borrow_mut();
                editor.import_wesnoth_map();
            }

            ui.add_space(10.0);
            ui.separator();

            // World wrap
            let mut world_wrap = self.world_wrap;
            let can_change = self.can_change_world_wrap();
            let mut enabled = true;
            if !can_change {
                enabled = false;
            }

            if ui.add_enabled(enabled, egui::Checkbox::new(&mut world_wrap, "Current map: World Wrap")).clicked() {
                let mut editor = self.editor_screen.borrow_mut();
                editor.set_world_wrap(world_wrap);
                self.world_wrap = world_wrap;
            }

            ui.add_space(10.0);
            ui.separator();

            // Overlay image
            ui.heading("Overlay image");

            let overlay_text = if let Some(file) = &self.overlay_file {
                file.clone()
            } else {
                "Click to choose a file".to_string()
            };

            if ui.button(&overlay_text).clicked() {
                let mut editor = self.editor_screen.borrow_mut();
                // In a real implementation, we would use a file dialog
                // For now, just log that we would open a file dialog
                Log::info("Would open file dialog for overlay image");
            }

            // Overlay opacity
            ui.horizontal(|ui| {
                ui.label("Overlay opacity:");
                let mut alpha = self.overlay_alpha;
                if ui.add(Slider::new(&mut alpha, 0.0..=1.0).step_by(0.05)).changed() {
                    let mut editor = self.editor_screen.borrow_mut();
                    editor.overlay_alpha = alpha;
                    self.overlay_alpha = alpha;
                }
            });
        });
    }
}