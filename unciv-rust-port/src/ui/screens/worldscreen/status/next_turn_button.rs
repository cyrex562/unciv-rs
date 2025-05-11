// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/NextTurnButton.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Button, Label, Response, Ui, Color32, Vec2};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::next_turn_menu::NextTurnMenu;
use crate::ui::screens::worldscreen::status::next_turn_progress::NextTurnProgress;
use crate::ui::screens::worldscreen::status::next_turn_action::NextTurnAction;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::images::ImageGetter;
use crate::logic::civilization::managers::TurnManager;
use crate::utils::concurrency::Concurrency;
use crate::utils::launch_on_gl_thread;

/// Button for next turn actions
pub struct NextTurnButton {
    /// The world screen reference
    world_screen: Rc<RefCell<WorldScreen>>,
    /// The next turn action
    next_turn_action: NextTurnAction,
    /// The units due label
    units_due_label: Label,
    /// The button
    button: Button,
    /// The progress bar
    progress_bar: Option<Rc<RefCell<NextTurnProgress>>>,
}

impl NextTurnButton {
    /// Creates a new NextTurnButton
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        let units_due_label = Label::new("");
        let button = Button::new("")
            .min_size([50.0, 50.0])
            .fill(Color32::from_rgba_premultiplied(0, 0, 0, 0));

        let mut button_instance = Self {
            world_screen,
            next_turn_action: NextTurnAction::Default,
            units_due_label,
            button,
            progress_bar: None,
        };

        button_instance.init();
        button_instance
    }

    /// Initializes the button
    fn init(&mut self) {
        // Set up button padding
        self.button = self.button.clone().padding(15.0);

        // Set up click handler
        let world_screen = self.world_screen.clone();
        let next_turn_action = self.next_turn_action.clone();

        self.button = self.button.clone().on_click(move |_| {
            next_turn_action.action(&world_screen.borrow());
        });

        // Set up right click handler
        let world_screen = self.world_screen.clone();
        let button = self.button.clone();

        self.button = self.button.clone().on_right_click(move |ui| {
            let position = ui.cursor().min;
            let menu = NextTurnMenu::new(
                world_screen.clone(),
                button.clone(),
                position,
            );
            menu.show();
        });

        // Set up keyboard shortcuts
        let world_screen = self.world_screen.clone();
        let next_turn_action = self.next_turn_action.clone();

        crate::ui::screens::basescreen::BaseScreen::add_keyboard_shortcut(
            KeyboardBinding::NextTurn,
            Box::new(move || {
                next_turn_action.action(&world_screen.borrow());
            }),
        );

        let world_screen = self.world_screen.clone();
        let next_turn_action = self.next_turn_action.clone();

        crate::ui::screens::basescreen::BaseScreen::add_keyboard_shortcut(
            KeyboardBinding::NextTurnAlternate,
            Box::new(move || {
                next_turn_action.action(&world_screen.borrow());
            }),
        );

        // Set up units due label
        self.button = self.button.clone().add_label(self.units_due_label.clone());
    }

    /// Updates the button
    pub fn update(&mut self) {
        let world_screen = self.world_screen.borrow();
        self.next_turn_action = Self::get_next_turn_action(&world_screen);
        self.update_button(self.next_turn_action.clone());

        let auto_play = world_screen.auto_play.clone();
        if auto_play.should_continue_auto_playing() && world_screen.is_players_turn
            && !world_screen.waiting_for_autosave && !world_screen.is_next_turn_update_running() {
            let world_screen_clone = self.world_screen.clone();
            let auto_play_clone = auto_play.clone();

            auto_play.run_auto_play_job_in_new_thread("MultiturnAutoPlay", world_screen_clone.clone(), false, move || {
                TurnManager::automate_turn(&world_screen_clone.borrow().viewing_civ);
                launch_on_gl_thread(move || {
                    world_screen_clone.borrow_mut().next_turn();
                });
                auto_play_clone.end_turn_multiturn_auto_play();
            });
        }

        let is_enabled = self.next_turn_action.get_text(&world_screen) == "AutoPlay"
            || (!world_screen.has_open_popups() && world_screen.is_players_turn
                && !world_screen.waiting_for_autosave && !world_screen.is_next_turn_update_running());

        self.button = self.button.clone().enabled(is_enabled);

        if is_enabled {
            self.button = self.button.clone().tooltip(KeyboardBinding::NextTurn.to_string());
        } else {
            self.button = self.button.clone().tooltip("");
        }

        // Update small unit button
        world_screen.small_unit_button.update();
    }

    /// Updates the button appearance
    fn update_button(&mut self, next_turn_action: NextTurnAction) {
        let world_screen = self.world_screen.borrow();

        // Update button text
        self.button = self.button.clone().text(next_turn_action.get_text(&world_screen));

        // Update button color
        self.button = self.button.clone().fill(next_turn_action.color);

        // Update button icon
        if let Some(icon) = next_turn_action.icon {
            if ImageGetter::image_exists(&icon) {
                let image = ImageGetter::get_image(&icon);
                image.set_size(30.0);
                image.set_color(next_turn_action.color);
                self.button = self.button.clone().image(image);
            } else {
                self.button = self.button.clone().clear_image();
            }
        } else {
            self.button = self.button.clone().clear_image();
        }

        // Update units due label
        if let Some(sub_text) = next_turn_action.get_sub_text(&world_screen) {
            self.units_due_label.set_text(sub_text);
            self.button = self.button.clone().add_label(self.units_due_label.clone());
        } else {
            self.button = self.button.clone().clear_label();
        }
    }

    /// Gets the next turn action
    fn get_next_turn_action(world_screen: &WorldScreen) -> NextTurnAction {
        // Guaranteed to return a non-null NextTurnAction because the last isChoice always returns true
        NextTurnAction::entries().iter()
            .find(|action| action.is_choice(world_screen))
            .cloned()
            .unwrap_or(NextTurnAction::Default)
    }

    /// Checks if the next turn action is a unit action
    pub fn is_next_unit_action(&self) -> bool {
        self.next_turn_action == NextTurnAction::NextUnit
    }

    /// Draws the button
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let response = ui.add(self.button.clone());

        // Draw progress bar if it exists
        if let Some(progress_bar) = &self.progress_bar {
            let progress_response = progress_bar.borrow().draw(ui);
            return response.union(progress_response);
        }

        response
    }

    /// Sets the progress bar
    pub fn set_progress_bar(&mut self, progress_bar: Rc<RefCell<NextTurnProgress>>) {
        self.progress_bar = Some(progress_bar);
    }

    /// Gets the button
    pub fn get_button(&self) -> Button {
        self.button.clone()
    }

    /// Gets the world screen
    pub fn get_world_screen(&self) -> Rc<RefCell<WorldScreen>> {
        self.world_screen.clone()
    }
}