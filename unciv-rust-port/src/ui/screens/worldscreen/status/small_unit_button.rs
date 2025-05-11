use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Response, Sense, Ui, Vec2};

use crate::core::unit::Unit;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::NextTurnButton;

/// A small button representing a unit in the world screen
pub struct SmallUnitButton {
    world_screen: Rc<RefCell<WorldScreen>>,
    next_turn_button: Rc<RefCell<NextTurnButton>>,
    unit: Rc<RefCell<Unit>>,
    size: Vec2,
    background_color: Color32,
    border_color: Color32,
    border_width: f32,
    is_pressed: bool,
}

impl SmallUnitButton {
    /// Creates a new small unit button
    pub fn new(
        world_screen: Rc<RefCell<WorldScreen>>,
        next_turn_button: Rc<RefCell<NextTurnButton>>,
        unit: Rc<RefCell<Unit>>,
    ) -> Self {
        Self {
            world_screen,
            next_turn_button,
            unit,
            size: Vec2::new(30.0, 30.0),
            background_color: Color32::from_rgba_premultiplied(0, 0, 0, 180),
            border_color: Color32::from_rgba_premultiplied(255, 255, 255, 180),
            border_width: 1.0,
            is_pressed: false,
        }
    }

    /// Initializes the button
    pub fn init(&mut self) {
        // Set up click handler
        let world_screen = self.world_screen.clone();
        let unit = self.unit.clone();

        // TODO: Add click handler to select unit
    }

    /// Draws the button
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Pointer);
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Draw background
            painter.rect_filled(
                rect,
                0.0,
                self.background_color,
            );

            // Draw border
            painter.rect_stroke(
                rect,
                0.0,
                self.border_width,
                self.border_color,
            );

            // Draw unit icon
            // TODO: Implement unit icon drawing
        }

        response
    }

    /// Disposes of the button
    pub fn dispose(&mut self) {
        // Clean up any resources
    }
}