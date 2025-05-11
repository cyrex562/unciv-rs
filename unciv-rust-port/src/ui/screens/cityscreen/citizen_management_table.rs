use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashMap;

use crate::constants::Constants;
use crate::logic::city::CityFocus;
use crate::ui::components::widgets::ExpanderTab;
use crate::ui::components::extensions::to_label;
use crate::ui::components::input::{KeyboardBinding, on_activation};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::utils::translations::tr;

/// Table for managing city citizens, including focus settings and growth controls
pub struct CitizenManagementTable<'a> {
    city_screen: &'a CityScreen,
    num_col: i32,
}

impl<'a> CitizenManagementTable<'a> {
    /// Create a new CitizenManagementTable
    pub fn new(city_screen: &'a CityScreen) -> Self {
        Self {
            city_screen,
            num_col: 4,
        }
    }

    /// Update the table contents
    pub fn update(&mut self, ui: &mut egui::Ui) {
        ui.clear();

        let color_selected = BaseScreen::skin().get_color("selection");
        let color_button = BaseScreen::skin().get_color("color");

        // Top table for reset and avoid growth buttons
        let mut top_table = egui::Grid::new("top_table");
        top_table.show(ui, |ui| {
            // Reset Citizens button
            let mut reset_cell = egui::Frame::none()
                .fill(color_button)
                .inner_margin(5.0)
                .outer_margin(3.0);

            reset_cell.show(ui, |ui| {
                let reset_label = to_label("Reset Citizens");
                ui.add(reset_label);

                if self.city_screen.can_city_be_changed() {
                    if ui.button("").clicked() {
                        self.city_screen.city.reassign_population(true);
                        self.city_screen.update();
                    }
                }
            });

            // Avoid Growth button
            let mut avoid_cell = egui::Frame::none()
                .fill(if self.city_screen.city.avoid_growth { color_selected } else { color_button })
                .inner_margin(5.0)
                .outer_margin(3.0);

            avoid_cell.show(ui, |ui| {
                let avoid_label = to_label("Avoid Growth");
                ui.add(avoid_label);

                if self.city_screen.can_city_be_changed() {
                    if ui.button("").clicked() {
                        self.city_screen.city.avoid_growth = !self.city_screen.city.avoid_growth;
                        self.city_screen.city.reassign_population();
                        self.city_screen.update();
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Citizen Focus header
        let mut focus_cell = egui::Frame::none()
            .inner_margin(5.0)
            .outer_margin(3.0);

        focus_cell.show(ui, |ui| {
            let focus_label = to_label("Citizen Focus");
            ui.add(focus_label);
        });

        ui.add_space(10.0);

        // Focus buttons grid
        let mut curr_col = self.num_col;
        let mut default_table = egui::Grid::new("default_table");

        for focus in CityFocus::values() {
            if !focus.table_enabled {
                continue;
            }

            if focus == CityFocus::FaithFocus && !self.city_screen.city.civ.game_info.is_religion_enabled() {
                continue;
            }

            let label = to_label(&focus.label);
            let mut cell = egui::Frame::none()
                .fill(if self.city_screen.city.get_city_focus() == focus { color_selected } else { color_button })
                .inner_margin(5.0)
                .outer_margin(3.0);

            if focus != CityFocus::NoFocus && focus != CityFocus::Manual {
                cell = cell.inner_margin(egui::style::Margin::same(5.0).with_top(10.0));
            }

            cell.show(ui, |ui| {
                ui.add(label);

                if self.city_screen.can_city_be_changed() {
                    if ui.button("").clicked() {
                        self.city_screen.city.set_city_focus(focus);
                        self.city_screen.city.reassign_population();
                        self.city_screen.update();
                    }
                }
            });

            // Special handling for NoFocus and Manual
            if focus == CityFocus::NoFocus {
                default_table.show(ui, |ui| {
                    ui.add(cell);
                });
            } else if focus == CityFocus::Manual {
                default_table.show(ui, |ui| {
                    ui.add(cell);
                });
                ui.add_space(10.0);
            } else {
                ui.add(cell);
                curr_col -= 1;

                if curr_col == 0 {
                    ui.add_space(10.0);
                    curr_col = self.num_col;
                }
            }
        }
    }

    /// Create an expander tab for this table
    pub fn as_expander<F>(&self, on_change: Option<F>) -> ExpanderTab
    where
        F: FnOnce(&mut egui::Ui) + 'static,
    {
        ExpanderTab::new(
            tr("Citizen Management"),
            Constants::DEFAULT_FONT_SIZE,
            "CityStatsTable.CitizenManagement",
            false,
            KeyboardBinding::CitizenManagement,
            on_change,
            move |ui| {
                self.update(ui);
            }
        )
    }
}