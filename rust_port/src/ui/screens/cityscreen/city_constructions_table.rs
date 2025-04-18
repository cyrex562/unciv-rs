use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashMap;

use crate::constants::Constants;
use crate::logic::city::{City, CityConstructions};
use crate::models::ruleset::{Building, IConstruction, INonPerpetualConstruction, PerpetualConstruction, RejectionReason};
use crate::ui::components::widgets::{ExpanderTab, ImageGetter};
use crate::ui::components::extensions::to_label;
use crate::ui::components::input::{KeyboardBinding, on_activation};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::utils::translations::tr;

/// Data transfer object for construction buttons
struct ConstructionButtonDTO {
    construction: Box<dyn IConstruction>,
    button_text: String,
    resources_required: Option<HashMap<String, i32>>,
    rejection_reason: Option<RejectionReason>,
}

/// Table for managing city constructions, including queue and available constructions
pub struct CityConstructionsTable<'a> {
    city_screen: &'a CityScreen,
    selected_queue_entry: i32,
    queue_expander: ExpanderTab,
    buy_button_factory: BuyButtonFactory<'a>,
}

impl<'a> CityConstructionsTable<'a> {
    /// Create a new CityConstructionsTable
    pub fn new(city_screen: &'a CityScreen) -> Self {
        Self {
            city_screen,
            selected_queue_entry: -1,
            queue_expander: ExpanderTab::new(
                tr("Construction queue"),
                Constants::DEFAULT_FONT_SIZE,
                "CityScreen/CityConstructionTable/QueueExpander",
                false,
                KeyboardBinding::ConstructionQueue,
                Some(Box::new(move |_| {
                    city_screen.update();
                })),
                |_| {},
            ),
            buy_button_factory: BuyButtonFactory::new(city_screen),
        }
    }

    /// Update the table contents
    pub fn update(&mut self, ui: &mut egui::Ui, selected_construction: Option<&dyn IConstruction>) {
        self.update_queue_and_buttons(ui, selected_construction);
        self.update_available_constructions(ui);
    }

    /// Update the construction queue and buttons
    fn update_queue_and_buttons(&mut self, ui: &mut egui::Ui, construction: Option<&dyn IConstruction>) {
        self.update_buttons(ui, construction);
        self.update_construction_queue(ui);
    }

    /// Update the construction buttons
    fn update_buttons(&mut self, ui: &mut egui::Ui, construction: Option<&dyn IConstruction>) {
        if !self.city_screen.can_change_state() {
            return;
        }

        // Check for puppet city restrictions
        if self.city_screen.city.is_puppet && !self.city_screen.city.has_unique("MayBuyConstructionsInPuppets") {
            return;
        }

        ui.clear();

        // Buy buttons
        for button in self.buy_button_factory.get_buy_buttons(construction) {
            ui.add(button);
        }

        // Queue management buttons
        let queue = &self.city_screen.city.city_constructions.construction_queue;
        if self.selected_queue_entry >= 0 && self.selected_queue_entry < queue.len() as i32 && queue.len() > 1 {
            let construction_name = &queue[self.selected_queue_entry as usize];

            // Raise priority button
            if self.city_screen.can_city_be_changed() && self.selected_queue_entry > 0 {
                let raise_button = self.get_raise_priority_button(
                    self.selected_queue_entry,
                    construction_name,
                    &self.city_screen.city,
                );
                ui.add(raise_button);
            }

            // Lower priority button
            if self.selected_queue_entry != (queue.len() - 1) as i32 && self.city_screen.can_city_be_changed() {
                let lower_button = self.get_lower_priority_button(
                    self.selected_queue_entry,
                    construction_name,
                    &self.city_screen.city,
                );
                ui.add(lower_button);
            }

            // Remove button
            if self.city_screen.can_city_be_changed() && !self.queue_expander.is_open() &&
               (1..=4).contains(&self.selected_queue_entry) {
                let remove_button = self.get_remove_from_queue_button(
                    self.selected_queue_entry,
                    &self.city_screen.city,
                );
                ui.add(remove_button);
            }
        }
    }

    /// Update the construction queue display
    fn update_construction_queue(&mut self, ui: &mut egui::Ui) {
        let city = &self.city_screen.city;
        let city_constructions = &city.city_constructions;
        let current_construction = city_constructions.current_construction_from_queue();
        let queue = &city_constructions.construction_queue;

        ui.clear();

        // Current construction
        if !current_construction.is_empty() {
            self.add_queue_entry(ui, 0, current_construction);
        } else {
            ui.label(tr("Pick a construction"));
        }

        // Queue entries
        self.queue_expander.update(ui, |ui| {
            for (i, construction_name) in queue.iter().enumerate() {
                if i > 0 { // Skip current construction as it's already displayed
                    self.add_queue_entry(ui, i as i32, construction_name);
                }
            }
        });
    }

    /// Add a queue entry to the UI
    fn add_queue_entry(&mut self, ui: &mut egui::Ui, queue_index: i32, construction_name: &str) {
        let city = &self.city_screen.city;
        let city_constructions = &city.city_constructions;
        let construction = city_constructions.get_construction(construction_name);
        let is_first_of_kind = city_constructions.is_first_construction_of_its_kind(queue_index, construction_name);

        let mut frame = egui::Frame::none();
        if queue_index == self.selected_queue_entry {
            frame = frame.fill(BaseScreen::skin().get_color("selection"));
        }

        frame.show(ui, |ui| {
            // Progress bar
            if is_first_of_kind {
                ui.add(self.get_progress_bar(construction_name));
            }

            // Construction icon
            ui.add(ImageGetter::get_construction_portrait(construction_name, 40.0));

            // Construction info
            let mut text = tr(construction_name);
            if construction_name == "PerpetualConstruction" {
                text += "\n∞";
            } else {
                text += "\n" + &city_constructions.get_turns_to_construction_string(construction, is_first_of_kind);
            }

            // Resource requirements
            let resources = if construction.is_unit() {
                construction.get_resource_requirements_per_turn(&city.civ.state)
            } else {
                construction.get_resource_requirements_per_turn(&city.state)
            };

            for (resource_name, amount) in resources {
                if let Some(resource) = city_constructions.city.get_ruleset().tile_resources.get(&resource_name) {
                    text += &format!("\n{}", tr(&resource_name.get_consumes_amount_string(amount, resource.is_stockpiled)));
                }
            }

            ui.label(text);

            // Queue management buttons
            if self.queue_expander.is_open() {
                if queue_index > 0 && self.city_screen.can_city_be_changed() {
                    ui.add(self.get_raise_priority_button(queue_index, construction_name, city));
                }
                if queue_index != (city_constructions.construction_queue.len() - 1) as i32 &&
                   self.city_screen.can_city_be_changed() {
                    ui.add(self.get_lower_priority_button(queue_index, construction_name, city));
                }
            }
            if self.city_screen.can_city_be_changed() {
                ui.add(self.get_remove_from_queue_button(queue_index, city));
            }
        });
    }

    /// Get the progress bar for a construction
    fn get_progress_bar(&self, construction_name: &str) -> egui::ProgressBar {
        let city_constructions = &this.city_screen.city.city_constructions;
        let construction = city_constructions.get_construction(construction_name);

        if construction.is_perpetual() || city_constructions.get_work_done(construction_name) == 0 {
            return egui::ProgressBar::new(0.0);
        }

        let construction_percentage = city_constructions.get_work_done(construction_name) as f32 /
            construction.get_production_cost(&city_constructions.city.civ, &city_constructions.city) as f32;

        egui::ProgressBar::new(construction_percentage)
            .text(format!("{:.0}%", construction_percentage * 100.0))
    }

    /// Get the raise priority button
    fn get_raise_priority_button(&self, queue_index: i32, name: &str, city: &City) -> egui::Button {
        self.get_move_priority_button(
            "up",
            KeyboardBinding::RaisePriority,
            queue_index,
            name,
            |i| city.city_constructions.raise_priority(i),
        )
    }

    /// Get the lower priority button
    fn get_lower_priority_button(&self, queue_index: i32, name: &str, city: &City) -> egui::Button {
        self.get_move_priority_button(
            "down",
            KeyboardBinding::LowerPriority,
            queue_index,
            name,
            |i| city.city_constructions.lower_priority(i),
        )
    }

    /// Get a move priority button
    fn get_move_priority_button<F>(
        &self,
        direction: &str,
        binding: KeyboardBinding,
        queue_index: i32,
        name: &str,
        move_priority: F,
    ) -> egui::Button
    where
        F: Fn(i32) -> i32,
    {
        let mut button = egui::Button::new(ImageGetter::get_arrow_image(direction));

        if self.selected_queue_entry == queue_index {
            button = button.shortcut_text(binding.to_string());
        }

        button.on_click(move || {
            self.selected_queue_entry = move_priority(queue_index);
            self.city_screen.select_construction(name);
            self.city_screen.city.reassign_population();
            self.city_screen.update();
        });

        button
    }

    /// Get the remove from queue button
    fn get_remove_from_queue_button(&self, queue_index: i32, city: &City) -> egui::Button {
        let mut button = egui::Button::new(ImageGetter::get_image("OtherIcons/Stop"));

        button.on_click(move || {
            city.city_constructions.remove_from_queue(queue_index, false);
            self.city_screen.clear_selection();
            self.city_screen.city.reassign_population();
            self.select_queue_entry(queue_index.min(city.city_constructions.construction_queue.len() as i32 - 1));
        });

        button
    }

    /// Select a queue entry
    fn select_queue_entry(&mut self, queue_index: i32) {
        if queue_index >= 0 && queue_index < self.city_screen.city.city_constructions.construction_queue.len() as i32 {
            self.city_screen.select_construction_from_queue(queue_index);
            self.selected_queue_entry = queue_index;
        } else {
            self.city_screen.clear_selection();
            self.selected_queue_entry = -1;
        }
        self.city_screen.update();
        self.ensure_queue_entry_visible();
    }

    /// Ensure the selected queue entry is visible
    fn ensure_queue_entry_visible(&self) {
        if let Some(button) = self.get_selected_queue_button() {
            // Scroll to make the button visible
            // Note: This would need to be implemented based on your UI framework's scrolling capabilities
        }
    }

    /// Get the selected queue button
    fn get_selected_queue_button(&self) -> Option<egui::Button> {
        if self.selected_queue_entry == 0 {
            // Return the current construction button
            None // This would need to be implemented based on your UI structure
        } else if self.selected_queue_entry > 0 &&
                  self.selected_queue_entry < self.city_screen.city.city_constructions.construction_queue.len() as i32 {
            // Return the queue entry button
            None // This would need to be implemented based on your UI structure
        } else {
            None
        }
    }
}

/// Factory for creating buy buttons
struct BuyButtonFactory<'a> {
    city_screen: &'a CityScreen,
}

impl<'a> BuyButtonFactory<'a> {
    /// Create a new BuyButtonFactory
    fn new(city_screen: &'a CityScreen) -> Self {
        Self { city_screen }
    }

    /// Get buy buttons for a construction
    fn get_buy_buttons(&self, construction: Option<&dyn IConstruction>) -> Vec<egui::Button> {
        // This would need to be implemented based on your buy button logic
        Vec::new()
    }
}