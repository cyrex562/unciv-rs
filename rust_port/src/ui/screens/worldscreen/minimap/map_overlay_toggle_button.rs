// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/minimap/MapOverlayToggleButton.kt

use std::rc::Rc;
use egui::{self, Color32, Button, Response, Ui};
use crate::ui::images::ImageGetter;
use crate::ui::screens::worldscreen::WorldScreen;

/// Class that unifies the behaviour of the little green map overlay toggle buttons shown next to the minimap.
pub struct MapOverlayToggleButton {
    /// The icon path
    icon_path: String,
    /// The icon size
    icon_size: f32,
    /// A function that returns the current backing state of the toggle
    getter: Rc<dyn Fn() -> bool>,
    /// A function for setting the backing state of the toggle
    setter: Rc<dyn Fn(bool)>,
    /// The button
    button: Option<Button>,
    /// The world screen
    world_screen: Rc<WorldScreen>,
}

impl MapOverlayToggleButton {
    /// Creates a new MapOverlayToggleButton
    pub fn new(
        icon_path: String,
        icon_size: f32,
        getter: Rc<dyn Fn() -> bool>,
        setter: Rc<dyn Fn(bool)>,
        world_screen: Rc<WorldScreen>,
    ) -> Self {
        Self {
            icon_path,
            icon_size,
            getter,
            setter,
            button: None,
            world_screen,
        }
    }

    /// Initializes the button
    pub fn init(&mut self) {
        // Create button with icon
        let mut button = Button::new(ImageGetter::get_image(&self.icon_path));
        button = button.small();
        button = button.min_size([self.icon_size, self.icon_size]);

        // Set initial color based on state
        self.update_color(&mut button);

        self.button = Some(button);
    }

    /// Toggle overlay. Called on click.
    pub fn toggle(&self) {
        let current_state = (self.getter)();
        (self.setter)(!current_state);
        self.world_screen.set_update_world_on_next_render();
        // Setting worldScreen.shouldUpdate implicitly causes this.update() to be called by the WorldScreen on the next update.
    }

    /// Update. Called via [WorldScreen.shouldUpdate] on toggle.
    pub fn update(&mut self) {
        if let Some(button) = &mut self.button {
            self.update_color(button);
        }
    }

    /// Updates the button color based on the current state
    fn update_color(&self, button: &mut Button) {
        let is_active = (self.getter)();
        let alpha = if is_active { 1.0 } else { 0.5 };

        // Set button color with alpha
        let mut color = Color32::from_rgb(0, 200, 0); // Green color
        color.a = (alpha * 255.0) as u8;
        button = button.fill(color);
    }

    /// Draws the button
    pub fn draw(&self, ui: &mut Ui) -> Response {
        if let Some(button) = &self.button {
            let response = ui.add(button.clone());
            if response.clicked() {
                self.toggle();
            }
            response
        } else {
            ui.available_response()
        }
    }
}