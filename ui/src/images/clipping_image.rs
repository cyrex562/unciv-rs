use ggez::graphics::{self, DrawParam, Image, Rect};
use ggez::mint::Point2;
use ggez::Context;
use ggez::GameResult;

/// Image that is constrained by the size of its parent. Instead of spilling over if it is larger than the parent,
/// the spilling parts simply get clipped off.
pub struct ClippingImage {
    image: Image,
    parent_width: f32,
    parent_height: f32,
}

impl ClippingImage {
    /// Creates a new ClippingImage with the given image.
    pub fn new(image: Image) -> Self {
        Self {
            image,
            parent_width: 0.0,
            parent_height: 0.0,
        }
    }

    /// Sets the parent dimensions for clipping.
    pub fn set_parent_size(&mut self, width: f32, height: f32) {
        self.parent_width = width;
        self.parent_height = height;
    }

    /// Draws the image with clipping applied.
    pub fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult {
        // Save the current scissor box
        let old_scissor = graphics::get_scissor(ctx)?;

        // Set the new scissor box to the parent's bounds
        graphics::set_scissor(
            ctx,
            Rect::new(0.0, 0.0, self.parent_width, self.parent_height),
        )?;

        // Draw the image
        self.image.draw(
            ctx,
            DrawParam::new()
                .dest(Point2::from([0.0, 0.0]))
                .color([1.0, 1.0, 1.0, parent_alpha]),
        )?;

        // Restore the old scissor box
        graphics::set_scissor(ctx, old_scissor)?;

        Ok(())
    }
}