// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/IdleUnitButton.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Image, Response, Ui};
use crate::game::unit::Unit;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::image_getter::ImageGetter;

/// Button that displays idle units and allows selecting them
pub struct IdleUnitButton {
    /// Reference to the world screen
    world_screen: Rc<RefCell<WorldScreen>>,

    /// The unit this button represents
    unit: Rc<RefCell<Unit>>,

    /// The button's size
    size: f32,

    /// The button's color
    color: Color32,

    /// Whether the button is currently pressed
    is_pressed: bool,

    /// The button's image
    image: Option<Image>,
}

impl IdleUnitButton {
    /// Creates a new IdleUnitButton
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>, unit: Rc<RefCell<Unit>>, size: f32) -> Self {
        Self {
            world_screen,
            unit,
            size,
            color: Color32::WHITE,
            is_pressed: false,
            image: None,
        }
    }

    /// Initializes the button
    pub fn init(&mut self) {
        self.image = Some(ImageGetter::get_unit_image(&self.unit.borrow()));
    }

    /// Draws the button
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let response = ui.add(egui::ImageButton::new(
            self.image.as_ref().unwrap().clone(),
            egui::vec2(self.size, self.size)
        ).tint(self.color));

        if response.clicked() {
            self.is_pressed = true;
            self.world_screen.borrow_mut().select_unit(&self.unit.borrow());
        }

        response
    }

    /// Disposes of the button
    pub fn dispose(&mut self) {
        if self.is_pressed {
            self.world_screen.borrow_mut().deselect_unit();
        }
    }
}