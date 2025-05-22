use std::rc::Rc;
use egui::{Ui, Slider, Color32};
use crate::ui::components::widgets::ExpanderTab;
use crate::game::map::MapParameters;

pub struct MapOptionsTable {
    map_parameters: Rc<MapParameters>,
    persistence_id: String,
}

impl MapOptionsTable {
    pub fn new(map_parameters: Rc<MapParameters>, persistence_id: String) -> Self {
        Self {
            map_parameters,
            persistence_id,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ExpanderTab::new("Map Options", &self.persistence_id, false)
            .show(ui, |ui| {
                ui.add_space(5.0);

                // Map size slider
                ui.horizontal(|ui| {
                    ui.label("Map Size:");
                    ui.add(Slider::new(&mut self.map_parameters.size, 10..=100)
                        .text("size"));
                });
                ui.add_space(5.0);

                // Map type selection
                ui.horizontal(|ui| {
                    ui.label("Map Type:");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.map_parameters.map_type.to_string())
                        .show_ui(ui, |ui| {
                            for map_type in ["Continents", "Pangaea", "Archipelago", "Fractal"] {
                                ui.selectable_value(
                                    &mut self.map_parameters.map_type,
                                    map_type.to_string(),
                                    map_type,
                                );
                            }
                        });
                });
                ui.add_space(5.0);

                // Map wrap selection
                ui.horizontal(|ui| {
                    ui.label("Map Wrap:");
                    egui::ComboBox::from_label("")
                        .selected_text(&self.map_parameters.map_wrap.to_string())
                        .show_ui(ui, |ui| {
                            for map_wrap in ["None", "Horizontal", "Vertical", "Both"] {
                                ui.selectable_value(
                                    &mut self.map_parameters.map_wrap,
                                    map_wrap.to_string(),
                                    map_wrap,
                                );
                            }
                        });
                });
                ui.add_space(5.0);

                // Resource frequency slider
                ui.horizontal(|ui| {
                    ui.label("Resource Frequency:");
                    ui.add(Slider::new(&mut self.map_parameters.resource_frequency, 0.0..=1.0)
                        .text("frequency"));
                });
                ui.add_space(5.0);

                // Terrain type toggles
                ui.label("Terrain Types:");
                for terrain_type in &mut self.map_parameters.terrain_types {
                    ui.checkbox(&mut terrain_type.enabled, &terrain_type.name);
                }
            });
    }
}