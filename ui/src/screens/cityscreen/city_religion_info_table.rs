use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui};
use std::collections::HashMap;

use crate::constants::Constants;
use crate::logic::city::managers::CityReligionManager;
use crate::models::religion::Religion;
use crate::ui::components::widgets::{ExpanderTab, ImageGetter};
use crate::ui::components::extensions::to_label;
use crate::ui::components::input::{KeyboardBinding, on_activation};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::overviewscreen::{EmpireOverviewCategories, EmpireOverviewScreen};
use crate::utils::translations::tr;

/// Table for displaying religious information for a city
pub struct CityReligionInfoTable<'a> {
    religion_manager: &'a CityReligionManager,
    show_majority: bool,
}

impl<'a> CityReligionInfoTable<'a> {
    /// Create a new CityReligionInfoTable
    pub fn new(religion_manager: &'a CityReligionManager, show_majority: bool) -> Self {
        Self {
            religion_manager,
            show_majority,
        }
    }

    /// Draw the religion info table
    pub fn draw(&self, ui: &mut Ui) {
        let grid_color = Color32::from_gray(64); // DARK_GRAY equivalent
        let followers = self.religion_manager.get_number_of_followers();
        let future_pressures = self.religion_manager.get_pressures_from_surrounding_cities();

        // Draw majority religion if requested
        if self.show_majority {
            let majority_religion = self.religion_manager.get_majority_religion();
            let (icon_name, label) = self.get_icon_and_label(majority_religion);

            ui.horizontal(|ui| {
                ui.add(self.linked_religion_icon(icon_name, majority_religion.map(|r| r.name)));
                ui.label(format!("Majority Religion: [{}]", label));
            });
        }

        // Draw holy city information
        if let Some(holy_city_religion) = self.religion_manager.religion_this_is_the_holy_city_of {
            let (icon_name, label) = self.get_icon_and_label(Some(holy_city_religion));

            ui.horizontal(|ui| {
                ui.add(self.linked_religion_icon(icon_name, Some(holy_city_religion)));
                if !self.religion_manager.is_blocked_holy_city {
                    ui.label(format!("Holy City of: [{}]", label));
                } else {
                    ui.label(format!("Former Holy City of: [{}]", label));
                }
            });
        }

        // Draw followers and pressure information
        if !followers.is_empty() {
            ui.add_space(5.0);

            // Header row
            ui.horizontal(|ui| {
                ui.add_space(30.0); // Space for icon
                ui.vertical(|ui| {
                    ui.add_separator();
                    ui.label("Followers");
                });
                ui.vertical(|ui| {
                    ui.add_separator();
                    ui.label("Pressure");
                });
            });

            ui.add_separator();

            // Sort followers by count in descending order
            let mut sorted_followers: Vec<_> = followers.iter().collect();
            sorted_followers.sort_by(|a, b| b.1.cmp(a.1));

            // Draw each religion's followers and pressure
            for (religion, follower_count) in sorted_followers {
                let icon_name = self.religion_manager.city.civ.game_info.religions[religion]
                    .get_icon_name();

                ui.horizontal(|ui| {
                    ui.add(self.linked_religion_icon(icon_name, Some(religion)));
                    ui.label(follower_count.to_string());

                    if let Some(pressure) = future_pressures.get(religion) {
                        ui.label(format!("+ [{}] pressure", pressure));
                    }
                });
            }
        }
    }

    /// Get icon name and display label for a religion
    fn get_icon_and_label(&self, religion: Option<&Religion>) -> (String, String) {
        match religion {
            None => ("Religion".to_string(), "None".to_string()),
            Some(religion) => (
                religion.get_icon_name(),
                religion.get_religion_display_name(),
            ),
        }
    }

    /// Create a clickable religion icon
    fn linked_religion_icon(&self, icon_name: String, religion: Option<&str>) -> egui::ImageButton {
        let mut button = ImageGetter::get_religion_portrait(&icon_name, 30.0);

        if let Some(religion_name) = religion {
            if religion_name == icon_name {
                button = button.on_click(move || {
                    let new_screen = EmpireOverviewScreen::new(
                        self.religion_manager.city.civ.get_viewing_player(),
                        EmpireOverviewCategories::Religion,
                        Some(religion_name),
                    );
                    // Push the new screen to the game stack
                    // This would need to be implemented based on your game's screen management
                });
            } else {
                // This is used only for Pantheons
                button = button.on_click(move || {
                    // Open the civilopedia for this belief
                    // This would need to be implemented based on your game's civilopedia system
                });
            }
        }

        button
    }

    /// Convert this table to an expander tab
    pub fn as_expander<F>(&self, on_change: Option<F>) -> ExpanderTab
    where
        F: Fn() + 'static,
    {
        let (icon, label) = self.get_icon_and_label(self.religion_manager.get_majority_religion());

        ExpanderTab::new(
            format!("Majority Religion: [{}]", label),
            Constants::DEFAULT_FONT_SIZE,
            ImageGetter::get_religion_portrait(&icon, 30.0),
            0.0, // default_pad
            "CityStatsTable.Religion",
            false, // starts_out_opened
            KeyboardBinding::ReligionDetail,
            on_change,
        )
    }
}