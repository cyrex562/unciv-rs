// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/TechButton.kt

use std::rc::Rc;
use egui::{Ui, Color32, Align, Response, Button, Image, RichText, Vec2};
use crate::models::civilization::managers::TechManager;
use crate::ui::images::ImageGetter;
use crate::ui::object_descriptions::TechnologyDescriptions;
use crate::ui::screens::basescreen::BaseScreen;
use crate::utils::translation::tr;

/// A button representing a technology in the tech picker screen
pub struct TechButton {
    tech_name: String,
    tech_manager: Rc<TechManager>,
    is_world_screen: bool,
    text: String,
    turns: String,
    background_color: Color32,
    width: f32,
    height: f32,
    position: Vec2,
}

impl TechButton {
    /// Creates a new tech button
    pub fn new(tech_name: String, tech_manager: Rc<TechManager>, is_world_screen: bool) -> Self {
        let mut button = Self {
            tech_name,
            tech_manager,
            is_world_screen,
            text: String::new(),
            turns: String::new(),
            background_color: Color32::TRANSPARENT,
            width: 0.0,
            height: 0.0,
            position: Vec2::ZERO,
        };

        button.init();
        button
    }

    /// Initializes the button
    fn init(&mut self) {
        // Set up text
        self.text = tr(&self.tech_name);

        // Set up turns text if not in world screen
        if !self.is_world_screen {
            let tech_cost = self.tech_manager.cost_of_tech(&self.tech_name);
            let remaining_tech = self.tech_manager.remaining_science_to_tech(&self.tech_name);
            let tech_this_turn = self.tech_manager.civ_info.stats.stats_for_next_turn.science;

            let percent_complete = (tech_cost - remaining_tech) as f32 / tech_cost as f32;
            let percent_will_be_complete = (tech_cost - (remaining_tech - tech_this_turn)) as f32 / tech_cost as f32;

            // Set turns text
            let turns_to_tech = self.tech_manager.turns_to_tech(&self.tech_name);
            self.turns = format!("{}t", turns_to_tech);
        }
    }

    /// Sets the button color
    pub fn set_button_color(&mut self, color: Color32) {
        self.background_color = color;
    }

    /// Renders the button
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let mut response = Response::default();

        // Create button frame
        let button_response = ui.add_space(5.0);

        // Add tech icon
        let icon = ImageGetter::get_tech_icon_portrait(&self.tech_name, 46.0);
        ui.add_space(2.0);
        ui.add(Image::new(icon).size(Vec2::new(46.0, 46.0)));
        ui.add_space(5.0);

        // Add progress bar if in world screen
        if self.is_world_screen {
            let tech_cost = self.tech_manager.cost_of_tech(&self.tech_name);
            let remaining_tech = self.tech_manager.remaining_science_to_tech(&self.tech_name);
            let tech_this_turn = self.tech_manager.civ_info.stats.stats_for_next_turn.science;

            let percent_complete = (tech_cost - remaining_tech) as f32 / tech_cost as f32;
            let percent_will_be_complete = (tech_cost - (remaining_tech - tech_this_turn)) as f32 / tech_cost as f32;

            // Create progress bar
            let progress_bar = ui.add(egui::ProgressBar::new(percent_complete)
                .text(format!("{}/{}", tech_cost - remaining_tech, tech_cost))
                .show_percentage());

            // Add semi-progress (will be complete next turn)
            if percent_will_be_complete > percent_complete {
                let semi_progress = ui.add(egui::ProgressBar::new(percent_will_be_complete)
                    .text(format!("{}/{}", tech_cost - (remaining_tech - tech_this_turn), tech_cost))
                    .show_percentage());
            }
        }

        // Add text and turns
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            ui.label(RichText::new(&self.text).size(14.0));
            ui.add_space(10.0);
            ui.label(RichText::new(&self.turns).size(14.0));
        });

        // Add tech enabled icons
        self.add_tech_enabled_icons(ui);

        // Store button dimensions
        self.width = ui.available_rect_before_wrap().width();
        self.height = ui.available_rect_before_wrap().height();
        self.position = ui.cursor().min;

        response
    }

    /// Adds icons for technologies enabled by this tech
    fn add_tech_enabled_icons(&self, ui: &mut Ui) {
        let civ = &self.tech_manager.civ_info;
        let tech = &civ.game_info.ruleset.technologies[&self.tech_name];

        // Get enabled icons
        let enabled_icons = TechnologyDescriptions::get_tech_enabled_icons(tech, civ, 30.0);

        // Add icons (limit to 5)
        ui.horizontal(|ui| {
            for icon in enabled_icons.iter().take(5) {
                ui.add(Image::new(icon.clone()).size(Vec2::new(30.0, 30.0)));
                ui.add_space(5.0);
            }
        });
    }

    /// Gets the button width
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Gets the button height
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Gets the button position
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// Sets up click handler
    pub fn on_click<F: Fn() + 'static>(&mut self, ui: &mut Ui, callback: F) {
        if ui.button(RichText::new(&self.text).size(14.0)).clicked() {
            callback();
        }
    }

    /// Sets up right click handler
    pub fn on_right_click<F: Fn() + 'static>(&mut self, ui: &mut Ui, callback: F) {
        if ui.button(RichText::new(&self.text).size(14.0)).secondary_clicked() {
            callback();
        }
    }

    /// Sets up double click handler
    pub fn on_double_click<F: Fn() + 'static>(&mut self, ui: &mut Ui, callback: F) {
        if ui.button(RichText::new(&self.text).size(14.0)).double_clicked() {
            callback();
        }
    }
}