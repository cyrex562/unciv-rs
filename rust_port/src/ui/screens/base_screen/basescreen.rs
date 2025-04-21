// Source: orig_src/core/src/com/unciv/ui/screens/basescreen/BaseScreen.kt

use eframe::egui::{self, Context, Ui};
use std::rc::Rc;

/// Base screen class for all screens in the game
pub struct BaseScreen {
    ctx: Rc<Context>,
    width: f32,
    height: f32,
}

impl BaseScreen {
    /// Create a new BaseScreen
    pub fn new(ctx: Rc<Context>) -> Self {
        Self {
            ctx,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Get the context
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Get the width of the screen
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get the height of the screen
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Set the size of the screen
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// Show the screen
    pub fn show(&mut self, ui: &mut Ui) {
        // Base implementation does nothing
        // Subclasses will override this
    }
}