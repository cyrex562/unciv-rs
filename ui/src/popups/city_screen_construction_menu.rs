// Source: orig_src/core/src/com/unciv/ui/popups/CityScreenConstructionMenu.kt

use std::rc::Rc;
use eframe::egui::{self, Ui, Response};
use log::info;

use crate::ui::{
    popups::{Popup, AnimatedMenuPopup},
    screens::basescreen::BaseScreen,
    components::input::KeyboardBinding,
};
use crate::game::{
    city::City,
    city_constructions::CityConstructions,
};
use crate::ruleset::{
    building::Building,
    construction_new::{Construction, ConstructionType},
};

/// A context menu for City constructions - available by right-clicking (or long-press) in
/// City Screen, left side, available constructions or queue entries.
pub struct CityScreenConstructionMenu {
    base: AnimatedMenuPopup,
    city: Rc<City>,
    construction: Rc<Construction>,
    on_button_clicked: Option<Box<dyn FnOnce()>>,
    any_button_was_clicked: bool,
}

impl CityScreenConstructionMenu {
    /// Create a new CityScreenConstructionMenu
    pub fn new(
        screen: Rc<BaseScreen>,
        position_next_to: &egui::Response,
        city: Rc<City>,
        construction: Rc<Construction>,
        on_button_clicked: Option<Box<dyn FnOnce()>>,
    ) -> Self {
        let position = Self::get_actor_top_right(position_next_to);
        let mut base = AnimatedMenuPopup::new(&screen, position);

        // Add close listener to call on_button_clicked if any button was clicked
        let any_button_was_clicked = Rc::new(std::cell::RefCell::new(false));
        let any_button_was_clicked_clone = any_button_was_clicked.clone();

        base.set_after_close_callback(move || {
            if *any_button_was_clicked_clone.borrow() {
                if let Some(callback) = on_button_clicked {
                    callback();
                }
            }
        });

        Self {
            base,
            city,
            construction,
            on_button_clicked,
            any_button_was_clicked: false,
        }
    }

    /// Get the top-right position of an actor
    fn get_actor_top_right(actor: &egui::Response) -> egui::Pos2 {
        egui::Pos2::new(
            actor.rect.right(),
            actor.rect.top(),
        )
    }

    /// Get cities (including this one) where changing the construction queue makes sense
    /// (excludes isBeingRazed even though technically that would be allowed)
    fn candidate_cities(&self) -> Vec<Rc<City>> {
        self.city.civ.cities.iter()
            .filter(|city| !city.is_puppet && !city.is_in_resistance() && !city.is_being_razed)
            .cloned()
            .collect()
    }

    /// Check whether an "All cities" menu makes sense
    fn all_cities_entry_valid<F>(&self, predicate: F) -> bool
    where
        F: Fn(&CityConstructions) -> bool,
    {
        self.city.civ.cities.len() > 1 && // Yes any 2 cities
        !matches!(self.construction.construction_type, ConstructionType::Building(ref b) if b.is_any_wonder()) &&
        self.candidate_cities().iter()
            .map(|city| &city.city_constructions)
            .any(predicate)
    }

    /// Apply an action to all candidate cities
    fn for_all_cities<F>(&self, action: F)
    where
        F: Fn(&mut CityConstructions),
    {
        for city in self.candidate_cities() {
            action(&mut city.city_constructions);
        }
    }

    /// Check if we can move the construction to the top of the queue
    fn can_move_queue_top(&self) -> bool {
        if self.construction.is_perpetual {
            return false;
        }

        let my_index = self.city.city_constructions.construction_queue
            .iter()
            .position(|c| c.name == self.construction.name)
            .unwrap_or(usize::MAX);

        my_index > 0
    }

    /// Move the construction to the top of the queue
    fn move_queue_top(&mut self) {
        let my_index = self.city.city_constructions.construction_queue
            .iter()
            .position(|c| c.name == self.construction.name)
            .unwrap_or(usize::MAX);

        if my_index != usize::MAX {
            self.city.city_constructions.move_entry_to_top(my_index);
            self.any_button_was_clicked = true;
        }
    }

    /// Check if we can move the construction to the end of the queue
    fn can_move_queue_end(&self) -> bool {
        if self.construction.is_perpetual {
            return false;
        }

        let my_index = self.city.city_constructions.construction_queue
            .iter()
            .position(|c| c.name == self.construction.name)
            .unwrap_or(usize::MAX);

        let queue_size_without_perpetual = self.city.city_constructions.construction_queue
            .iter()
            .filter(|c| !c.is_perpetual)
            .count();

        my_index < queue_size_without_perpetual - 1
    }

    /// Move the construction to the end of the queue
    fn move_queue_end(&mut self) {
        let my_index = self.city.city_constructions.construction_queue
            .iter()
            .position(|c| c.name == self.construction.name)
            .unwrap_or(usize::MAX);

        if my_index != usize::MAX {
            self.city.city_constructions.move_entry_to_end(my_index);
            self.any_button_was_clicked = true;
        }
    }

    /// Check if we can add the construction to the top of the queue
    fn can_add_queue_top(&self) -> bool {
        !self.construction.is_perpetual &&
        self.city.city_constructions.can_add_to_queue(&self.construction)
    }

    /// Add the construction to the top of the queue
    fn add_queue_top(&mut self) {
        self.city.city_constructions.add_to_queue(&self.construction, true);
        self.any_button_was_clicked = true;
    }

    /// Check if we can add the construction to all queues
    fn can_add_all_queues(&self) -> bool {
        self.all_cities_entry_valid(|constructions| {
            constructions.can_add_to_queue(&self.construction) &&
            // A Perpetual that is already queued can still be added says canAddToQueue, but here we don't want to count that
            !(self.construction.is_perpetual &&
              constructions.is_being_constructed_or_enqueued(&self.construction.name))
        })
    }

    /// Add the construction to all queues
    fn add_all_queues(&mut self) {
        self.for_all_cities(|constructions| {
            constructions.add_to_queue(&self.construction);
        });
        self.any_button_was_clicked = true;
    }

    /// Check if we can add the construction to the top of all queues
    fn can_add_all_queues_top(&self) -> bool {
        !self.construction.is_perpetual &&
        self.all_cities_entry_valid(|constructions| {
            constructions.can_add_to_queue(&self.construction) ||
            constructions.is_enqueued_for_later(&self.construction.name)
        })
    }

    /// Add the construction to the top of all queues
    fn add_all_queues_top(&mut self) {
        self.for_all_cities(|constructions| {
            let index = constructions.construction_queue
                .iter()
                .position(|c| c.name == self.construction.name)
                .unwrap_or(usize::MAX);

            if index > 0 {
                constructions.move_entry_to_top(index);
            } else {
                constructions.add_to_queue(&self.construction, true);
            }
        });
        self.any_button_was_clicked = true;
    }

    /// Check if we can remove the construction from all queues
    fn can_remove_all_queues(&self) -> bool {
        self.all_cities_entry_valid(|constructions| {
            constructions.is_being_constructed_or_enqueued(&self.construction.name)
        })
    }

    /// Remove the construction from all queues
    fn remove_all_queues(&mut self) {
        self.for_all_cities(|constructions| {
            constructions.remove_all_by_name(&self.construction.name);
        });
        self.any_button_was_clicked = true;
    }

    /// Check if we can disable the construction
    fn can_disable(&self) -> bool {
        let settings = self.city.civ.game.settings();
        !settings.disabled_auto_assign_constructions.contains(&self.construction.name) &&
        !matches!(self.construction.construction_type, ConstructionType::Idle)
    }

    /// Disable the construction
    fn disable_entry(&mut self) {
        let settings = self.city.civ.game.settings_mut();
        settings.disabled_auto_assign_constructions.insert(self.construction.name.clone());
        settings.save();
        self.any_button_was_clicked = true;
    }

    /// Check if we can enable the construction
    fn can_enable(&self) -> bool {
        let settings = self.city.civ.game.settings();
        settings.disabled_auto_assign_constructions.contains(&self.construction.name)
    }

    /// Enable the construction
    fn enable_entry(&mut self) {
        let settings = self.city.civ.game.settings_mut();
        settings.disabled_auto_assign_constructions.remove(&self.construction.name);
        settings.save();
        self.any_button_was_clicked = true;
    }
}

impl Popup for CityScreenConstructionMenu {
    fn show(&mut self, ui: &mut Ui) -> bool {
        let mut should_close = false;

        // Create a frame for the popup
        egui::Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(200.0);

                // Add title
                ui.heading("Construction Menu");

                ui.add_space(10.0);

                // Add buttons based on available actions
                if self.can_move_queue_top() {
                    if ui.button("Move to the top of the queue").clicked() {
                        self.move_queue_top();
                        should_close = true;
                    }
                }

                if self.can_move_queue_end() {
                    if ui.button("Move to the end of the queue").clicked() {
                        self.move_queue_end();
                        should_close = true;
                    }
                }

                if self.can_add_queue_top() {
                    if ui.button("Add to the top of the queue").clicked() {
                        self.add_queue_top();
                        should_close = true;
                    }
                }

                if self.can_add_all_queues() {
                    if ui.button("Add to the queue in all cities").clicked() {
                        self.add_all_queues();
                        should_close = true;
                    }
                }

                if self.can_add_all_queues_top() {
                    if ui.button("Add or move to the top in all cities").clicked() {
                        self.add_all_queues_top();
                        should_close = true;
                    }
                }

                if self.can_remove_all_queues() {
                    if ui.button("Remove from the queue in all cities").clicked() {
                        self.remove_all_queues();
                        should_close = true;
                    }
                }

                if self.can_disable() {
                    if ui.button("Disable").clicked() {
                        self.disable_entry();
                        should_close = true;
                    }
                }

                if self.can_enable() {
                    if ui.button("Enable").clicked() {
                        self.enable_entry();
                        should_close = true;
                    }
                }
            });

        should_close
    }

    fn title(&self) -> String {
        String::from("Construction Menu")
    }
}