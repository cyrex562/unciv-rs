// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/AutoPlayMenu.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Button, Ui, Response, Color32};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::next_turn_button::NextTurnButton;
use crate::ui::popups::AnimatedMenuPopup;
use crate::logic::automation::civilization::NextTurnAutomation;
use crate::logic::automation::unit::UnitAutomation;
use crate::logic::civilization::managers::TurnManager;
use crate::ui::components::input::KeyboardBinding;
use crate::utils::concurrency::Concurrency;

/// The "context" menu for the AutoPlay button
pub struct AutoPlayMenu {
    /// The world screen reference
    world_screen: Rc<RefCell<WorldScreen>>,
    /// The next turn button reference
    next_turn_button: Rc<RefCell<NextTurnButton>>,
    /// The auto play instance
    auto_play: Rc<RefCell<crate::ui::screens::worldscreen::unit::AutoPlay>>,
    /// The popup instance
    popup: Rc<RefCell<AnimatedMenuPopup>>,
}

impl AutoPlayMenu {
    /// Creates a new AutoPlayMenu
    pub fn new(
        world_screen: Rc<RefCell<WorldScreen>>,
        next_turn_button: Rc<RefCell<NextTurnButton>>,
        position: egui::Pos2,
    ) -> Self {
        let auto_play = Rc::new(RefCell::new(world_screen.borrow().auto_play.clone()));
        let popup = Rc::new(RefCell::new(AnimatedMenuPopup::new(
            world_screen.clone(),
            position,
        )));

        let menu = Self {
            world_screen,
            next_turn_button,
            auto_play,
            popup,
        };

        menu.init();
        menu
    }

    /// Initializes the menu
    fn init(&self) {
        // We need to activate the end turn button again after the menu closes
        let world_screen = self.world_screen.clone();
        self.popup.borrow_mut().set_after_close_callback(Box::new(move || {
            world_screen.borrow_mut().should_update = true;
        }));

        self.create_content_table();
    }

    /// Creates the content table for the menu
    fn create_content_table(&self) {
        let mut popup = self.popup.borrow_mut();
        let world_screen = self.world_screen.borrow();

        // Using the same keyboard binding for bypassing this menu and the default option
        if !world_screen.game_info.game_parameters.is_online_multiplayer {
            popup.add_button(
                "Start AutoPlay",
                KeyboardBinding::AutoPlay,
                Box::new(self.multiturn_auto_play())
            );
        }

        popup.add_button(
            "AutoPlay End Turn",
            KeyboardBinding::AutoPlayMenuEndTurn,
            Box::new(self.auto_play_end_turn())
        );

        popup.add_button(
            "AutoPlay Military Once",
            KeyboardBinding::AutoPlayMenuMilitary,
            Box::new(self.auto_play_military())
        );

        popup.add_button(
            "AutoPlay Civilians Once",
            KeyboardBinding::AutoPlayMenuCivilians,
            Box::new(self.auto_play_civilian())
        );

        popup.add_button(
            "AutoPlay Economy Once",
            KeyboardBinding::AutoPlayMenuEconomy,
            Box::new(self.auto_play_economy())
        );
    }

    /// Auto plays the end turn
    fn auto_play_end_turn(&self) -> Box<dyn Fn()> {
        let world_screen = self.world_screen.clone();
        let next_turn_button = self.next_turn_button.clone();
        let auto_play = self.auto_play.clone();

        Box::new(move || {
            let mut world_screen = world_screen.borrow_mut();
            let mut next_turn_button = next_turn_button.borrow_mut();
            let mut auto_play = auto_play.borrow_mut();

            next_turn_button.update();
            TurnManager::automate_turn(&world_screen.viewing_civ);
            auto_play.stop_auto_play();
            world_screen.next_turn();
        })
    }

    /// Starts multiturn auto play
    fn multiturn_auto_play(&self) -> Box<dyn Fn()> {
        let world_screen = self.world_screen.clone();
        let next_turn_button = self.next_turn_button.clone();
        let auto_play = self.auto_play.clone();

        Box::new(move || {
            let mut world_screen = world_screen.borrow_mut();
            let mut next_turn_button = next_turn_button.borrow_mut();
            let mut auto_play = auto_play.borrow_mut();

            auto_play.start_multiturn_auto_play();
            next_turn_button.update();
        })
    }

    /// Auto plays military units
    fn auto_play_military(&self) -> Box<dyn Fn()> {
        let world_screen = self.world_screen.clone();
        let auto_play = self.auto_play.clone();

        Box::new(move || {
            let mut world_screen = world_screen.borrow_mut();
            let mut auto_play = auto_play.borrow_mut();

            let civ_info = world_screen.viewing_civ.clone();
            let is_at_war = civ_info.is_at_war();

            // Sort military units by priority
            let mut sorted_units: Vec<_> = civ_info.units.get_civ_units()
                .into_iter()
                .filter(|unit| unit.is_military())
                .collect();

            sorted_units.sort_by(|a, b| {
                let priority_a = NextTurnAutomation::get_unit_priority(a, is_at_war);
                let priority_b = NextTurnAutomation::get_unit_priority(b, is_at_war);
                priority_b.partial_cmp(&priority_a).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Move each unit
            for unit in sorted_units {
                UnitAutomation::automate_unit_moves(&unit);
            }

            // Try to bombard enemy from cities
            for city in civ_info.cities.iter() {
                UnitAutomation::try_bombard_enemy(city);
            }

            world_screen.should_update = true;
        })
    }

    /// Auto plays civilian units
    fn auto_play_civilian(&self) -> Box<dyn Fn()> {
        let world_screen = self.world_screen.clone();
        let auto_play = self.auto_play.clone();

        Box::new(move || {
            let mut world_screen = world_screen.borrow_mut();
            let mut auto_play = auto_play.borrow_mut();

            let civ_info = world_screen.viewing_civ.clone();
            let is_at_war = civ_info.is_at_war();

            // Sort civilian units by priority
            let mut sorted_units: Vec<_> = civ_info.units.get_civ_units()
                .into_iter()
                .filter(|unit| unit.is_civilian())
                .collect();

            sorted_units.sort_by(|a, b| {
                let priority_a = NextTurnAutomation::get_unit_priority(a, is_at_war);
                let priority_b = NextTurnAutomation::get_unit_priority(b, is_at_war);
                priority_b.partial_cmp(&priority_a).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Move each unit
            for unit in sorted_units {
                UnitAutomation::automate_unit_moves(&unit);
            }

            world_screen.should_update = true;
        })
    }

    /// Auto plays economy
    fn auto_play_economy(&self) -> Box<dyn Fn()> {
        let world_screen = self.world_screen.clone();

        Box::new(move || {
            let mut world_screen = world_screen.borrow_mut();
            let civ_info = world_screen.viewing_civ.clone();

            NextTurnAutomation::automate_cities(&civ_info);
            world_screen.should_update = true;
            world_screen.render(0.0);
        })
    }

    /// Shows the menu
    pub fn show(&self) {
        self.popup.borrow().show();
    }
}