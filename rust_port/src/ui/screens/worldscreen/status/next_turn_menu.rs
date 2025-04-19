// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/NextTurnMenu.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Button, Response, Ui, Vec2};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::next_turn_button::NextTurnButton;
use crate::ui::screens::worldscreen::status::next_turn_action::NextTurnAction;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::popups::AnimatedMenuPopup;

/// Menu for next turn actions
pub struct NextTurnMenu {
    /// The world screen reference
    world_screen: Rc<RefCell<WorldScreen>>,
    /// The next turn button reference
    next_turn_button: Rc<RefCell<NextTurnButton>>,
    /// The popup
    popup: Rc<RefCell<AnimatedMenuPopup>>,
}

impl NextTurnMenu {
    /// Creates a new NextTurnMenu
    pub fn new(
        world_screen: Rc<RefCell<WorldScreen>>,
        next_turn_button: Rc<RefCell<NextTurnButton>>,
        position: Vec2,
    ) -> Self {
        let popup = Rc::new(RefCell::new(AnimatedMenuPopup::new(
            world_screen.clone(),
            position,
        )));

        let menu = Self {
            world_screen,
            next_turn_button,
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

    /// Creates the content table
    fn create_content_table(&self) {
        let mut popup = self.popup.borrow_mut();
        let world_screen = self.world_screen.borrow();

        // Add next turn button
        popup.add_button(
            "Next Turn",
            KeyboardBinding::NextTurnMenuNextTurn,
            Box::new(move || {
                world_screen.borrow_mut().next_turn();
            }),
        );

        // Add move automated units button if applicable
        let automate_units_action = NextTurnAction::MoveAutomatedUnits;
        if automate_units_action.is_choice(&world_screen) {
            let world_screen_clone = self.world_screen.clone();
            let action_clone = automate_units_action.clone();

            popup.add_button(
                "Move automated units",
                KeyboardBinding::NextTurnMenuMoveAutomatedUnits,
                Box::new(move || {
                    action_clone.action(&world_screen_clone.borrow());
                }),
            );
        }
    }

    /// Shows the menu
    pub fn show(&self) {
        self.popup.borrow().show();
    }
}