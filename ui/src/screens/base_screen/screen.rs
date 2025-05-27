// Source: orig_src/core/src/com/unciv/ui/screens/basescreen/BaseScreen.kt
use egui::Ui;
use ggez::{Context, graphics::Color};
use std::rc::Rc;
use uuid::Uuid;
/// Base screen class for all screens in the game

pub struct Screen {
    id: Uuid,
    ctx: Rc<Context>,
    clear_color: Color,
    frame: bool,
    width: f32,
    height: f32,
}

impl Screen {
    /// Create a new BaseScreen
    pub fn new(ctx: Rc<Context>, clear_color: Color, frame: bool, width: f32, height: f32) -> Self {
        Self {
            ctx,
            clear_color,
            frame,
            width,
            height,
            id: Uuid::now_v7()
        }
    }

    /// Set the size of the screen
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Base implementation does nothing
        // Subclasses will override this
        unimplemented!();
    }
}
