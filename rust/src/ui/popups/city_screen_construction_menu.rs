use ggez::graphics::{DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::ui::popups::{AnimatedMenuPopup, Popup};
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::components::input::KeyboardBinding;
use crate::logic::city::{City, CityConstructions};
use crate::models::ruleset::{Building, IConstruction, PerpetualConstruction};
use crate::gui::GUI;

/// A context menu for city constructions that appears when right-clicking on construction items
pub struct CityScreenConstructionMenu {
    base: AnimatedMenuPopup,
    city: City,
    construction: Box<dyn IConstruction>,
    on_button_clicked: Box<dyn Fn()>,
    any_button_was_clicked: bool,
}

impl CityScreenConstructionMenu {
    /// Creates a new CityScreenConstructionMenu
    pub fn new(
        screen: &BaseScreen,
        position_next_to: Point2<f32>,
        city: City,
        construction: Box<dyn IConstruction>,
        on_button_clicked: Box<dyn Fn()>,
    ) -> Self {
        let mut menu = Self {
            base: AnimatedMenuPopup::new(screen, position_next_to),
            city,
            construction,
            on_button_clicked,
            any_button_was_clicked: false,
        };

        menu.setup_ui();
        menu
    }

    fn setup_ui(&mut self) {
        // Add close listener
        self.base.add_close_listener(Box::new(move |_| {
            if self.any_button_was_clicked {
                (self.on_button_clicked)();
            }
        }));

        // Create content table
        let mut table = self.base.create_content_table();

        // Add buttons based on conditions
        if self.can_move_queue_top() {
            table.add_button(
                "Move to the top of the queue",
                KeyboardBinding::RaisePriority,
                Box::new(move |_| self.move_queue_top()),
            );
        }

        if self.can_move_queue_end() {
            table.add_button(
                "Move to the end of the queue",
                KeyboardBinding::LowerPriority,
                Box::new(move |_| self.move_queue_end()),
            );
        }

        if self.can_add_queue_top() {
            table.add_button(
                "Add to the top of the queue",
                KeyboardBinding::AddConstructionTop,
                Box::new(move |_| self.add_queue_top()),
            );
        }

        if self.can_add_all_queues() {
            table.add_button(
                "Add to the queue in all cities",
                KeyboardBinding::AddConstructionAll,
                Box::new(move |_| self.add_all_queues()),
            );
        }

        if self.can_add_all_queues_top() {
            table.add_button(
                "Add or move to the top in all cities",
                KeyboardBinding::AddConstructionAllTop,
                Box::new(move |_| self.add_all_queues_top()),
            );
        }

        if self.can_remove_all_queues() {
            table.add_button(
                "Remove from the queue in all cities",
                KeyboardBinding::RemoveConstructionAll,
                Box::new(move |_| self.remove_all_queues()),
            );
        }

        if self.can_disable() {
            table.add_button(
                "Disable",
                KeyboardBinding::BuildDisabled,
                Box::new(move |_| self.disable_entry()),
            );
        }

        if self.can_enable() {
            table.add_button(
                "Enable",
                KeyboardBinding::BuildDisabled,
                Box::new(move |_| self.enable_entry()),
            );
        }

        self.base.set_content_table(table);
    }

    // Helper methods
    fn city_constructions(&self) -> &CityConstructions {
        &self.city.city_constructions
    }

    fn construction_name(&self) -> &str {
        self.construction.name()
    }

    fn queue_size_without_perpetual(&self) -> usize {
        self.city_constructions()
            .construction_queue
            .iter()
            .filter(|&c| !PerpetualConstruction::is_perpetual(c))
            .count()
    }

    fn my_index(&self) -> usize {
        self.city_constructions()
            .construction_queue
            .iter()
            .position(|&c| c == self.construction_name())
            .unwrap_or(0)
    }

    fn candidate_cities(&self) -> Vec<&City> {
        self.city
            .civ
            .cities
            .iter()
            .filter(|&c| !c.is_puppet && !c.is_in_resistance() && !c.is_being_razed)
            .collect()
    }

    fn all_cities_entry_valid<F>(&self, predicate: F) -> bool
    where
        F: Fn(&CityConstructions) -> bool,
    {
        self.city.civ.cities.len() > 1
            && !(self.construction.as_any().downcast_ref::<Building>()
                .map_or(false, |b| b.is_any_wonder()))
            && self
                .candidate_cities()
                .iter()
                .map(|c| &c.city_constructions)
                .any(predicate)
    }

    fn for_all_cities<F>(&self, action: F)
    where
        F: Fn(&mut CityConstructions),
    {
        for city in self.candidate_cities() {
            action(&mut city.city_constructions);
        }
    }

    // Button condition methods
    fn can_move_queue_top(&self) -> bool {
        !self.construction.is_perpetual() && self.my_index() > 0
    }

    fn move_queue_top(&mut self) {
        self.city_constructions().move_entry_to_top(self.my_index());
        self.any_button_was_clicked = true;
    }

    fn can_move_queue_end(&self) -> bool {
        !self.construction.is_perpetual() && self.my_index() < self.queue_size_without_perpetual() - 1
    }

    fn move_queue_end(&mut self) {
        self.city_constructions().move_entry_to_end(self.my_index());
        self.any_button_was_clicked = true;
    }

    fn can_add_queue_top(&self) -> bool {
        !self.construction.is_perpetual() && self.city_constructions().can_add_to_queue(&self.construction)
    }

    fn add_queue_top(&mut self) {
        self.city_constructions().add_to_queue(&self.construction, true);
        self.any_button_was_clicked = true;
    }

    fn can_add_all_queues(&self) -> bool {
        self.all_cities_entry_valid(|cc| {
            cc.can_add_to_queue(&self.construction)
                && !(self.construction.is_perpetual()
                    && cc.is_being_constructed_or_enqueued(self.construction_name()))
        })
    }

    fn add_all_queues(&mut self) {
        self.for_all_cities(|cc| cc.add_to_queue(&self.construction));
        self.any_button_was_clicked = true;
    }

    fn can_add_all_queues_top(&self) -> bool {
        !self.construction.is_perpetual()
            && self.all_cities_entry_valid(|cc| {
                cc.can_add_to_queue(&self.construction)
                    || cc.is_enqueued_for_later(self.construction_name())
            })
    }

    fn add_all_queues_top(&mut self) {
        self.for_all_cities(|cc| {
            let index = cc
                .construction_queue
                .iter()
                .position(|&c| c == self.construction_name())
                .unwrap_or(0);
            if index > 0 {
                cc.move_entry_to_top(index);
            } else {
                cc.add_to_queue(&self.construction, true);
            }
        });
        self.any_button_was_clicked = true;
    }

    fn can_remove_all_queues(&self) -> bool {
        self.all_cities_entry_valid(|cc| cc.is_being_constructed_or_enqueued(self.construction_name()))
    }

    fn remove_all_queues(&mut self) {
        self.for_all_cities(|cc| cc.remove_all_by_name(self.construction_name()));
        self.any_button_was_clicked = true;
    }

    fn can_disable(&self) -> bool {
        let settings = GUI::get_settings();
        !settings
            .disabled_auto_assign_constructions
            .contains(self.construction_name())
            && !self.construction.is_idle()
    }

    fn disable_entry(&mut self) {
        let settings = GUI::get_settings();
        settings
            .disabled_auto_assign_constructions
            .insert(self.construction_name().to_string());
        settings.save();
        self.any_button_was_clicked = true;
    }

    fn can_enable(&self) -> bool {
        let settings = GUI::get_settings();
        settings
            .disabled_auto_assign_constructions
            .contains(self.construction_name())
    }

    fn enable_entry(&mut self) {
        let settings = GUI::get_settings();
        settings
            .disabled_auto_assign_constructions
            .remove(self.construction_name());
        settings.save();
        self.any_button_was_clicked = true;
    }
}

impl Popup for CityScreenConstructionMenu {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.base.draw(ctx)
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.base.update(ctx)
    }
}