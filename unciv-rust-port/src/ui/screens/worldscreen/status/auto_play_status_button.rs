// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/AutoPlayStatusButton.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Button, Image, Response, Ui, Color32, Vec2};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::next_turn_button::NextTurnButton;
use crate::ui::screens::worldscreen::status::auto_play_menu::AutoPlayMenu;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;

/// Button for auto play status
pub struct AutoPlayStatusButton {
    /// The world screen reference
    world_screen: Rc<RefCell<WorldScreen>>,
    /// The next turn button reference
    next_turn_button: Rc<RefCell<NextTurnButton>>,
    /// The auto play image
    auto_play_image: Image,
    /// The button
    button: Button,
    /// Whether the button is pressed
    is_pressed: bool,
}

impl AutoPlayStatusButton {
    /// Creates a new AutoPlayStatusButton
    pub fn new(
        world_screen: Rc<RefCell<WorldScreen>>,
        next_turn_button: Rc<RefCell<NextTurnButton>>,
    ) -> Self {
        let auto_play_image = Self::create_autoplay_image();
        let button = Button::new("")
            .min_size([50.0, 50.0])
            .fill(Color32::from_rgba_premultiplied(0, 0, 0, 0));

        let mut button_instance = Self {
            world_screen,
            next_turn_button,
            auto_play_image,
            button,
            is_pressed: false,
        };

        button_instance.init();
        button_instance
    }

    /// Initializes the button
    fn init(&mut self) {
        // Add image to button
        self.button = self.button.clone().image(self.auto_play_image.clone());

        // Set up keyboard shortcuts
        let world_screen = self.world_screen.clone();
        let next_turn_button = self.next_turn_button.clone();

        // Handle activation (click)
        self.button = self.button.clone().on_click(move |ui| {
            let world_screen = world_screen.borrow();
            let auto_play = world_screen.auto_play.clone();

            if auto_play.is_auto_playing() {
                auto_play.stop_auto_play();
            } else if world_screen.is_players_turn {
                let position = ui.cursor().min;
                let menu = AutoPlayMenu::new(
                    world_screen.clone(),
                    next_turn_button.clone(),
                    position,
                );
                menu.show();
            }
        });

        // Handle right click
        let world_screen = self.world_screen.clone();
        let next_turn_button = self.next_turn_button.clone();

        self.button = self.button.clone().on_right_click(move |ui| {
            let world_screen = world_screen.borrow();

            if !world_screen.game_info.game_parameters.is_online_multiplayer
                && world_screen.viewing_civ == world_screen.game_info.current_player_civ {
                let mut auto_play = world_screen.auto_play.clone();
                auto_play.start_multiturn_auto_play();
                next_turn_button.borrow_mut().update();
            }
        });

        // Add keyboard shortcut
        let world_screen = self.world_screen.clone();
        let next_turn_button = self.next_turn_button.clone();

        BaseScreen::add_keyboard_shortcut(
            KeyboardBinding::AutoPlay,
            Box::new(move || {
                let world_screen = world_screen.borrow();

                if !world_screen.game_info.game_parameters.is_online_multiplayer
                    && world_screen.viewing_civ == world_screen.game_info.current_player_civ {
                    let mut auto_play = world_screen.auto_play.clone();
                    auto_play.start_multiturn_auto_play();
                    next_turn_button.borrow_mut().update();
                }
            }),
        );
    }

    /// Creates the autoplay image
    fn create_autoplay_image() -> Image {
        let img = ImageGetter::get_image("OtherIcons/NationSwap");
        img.set_size(40.0);
        img
    }

    /// Draws the button
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let response = ui.add(self.button.clone());

        if response.clicked() {
            self.is_pressed = true;
        } else if response.released() {
            self.is_pressed = false;
        }

        response
    }

    /// Disposes of the button
    pub fn dispose(&self) {
        if self.is_pressed {
            let world_screen = self.world_screen.borrow();
            let auto_play = world_screen.auto_play.clone();

            if auto_play.is_auto_playing() {
                auto_play.stop_auto_play();
            }
        }
    }
}