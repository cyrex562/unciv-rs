use ggez::graphics::{DrawParam, Drawable};
use ggez::Context;

/// A performance-optimized group that doesn't use transformations.
/// This is useful for UI elements that don't need to be transformed,
/// as it avoids the overhead of checking and applying transformations.
pub struct NonTransformGroup {
    /// The children of this group
    children: Vec<Box<dyn Drawable>>,
    /// Whether this group is visible
    visible: bool,
}

impl NonTransformGroup {
    /// Creates a new NonTransformGroup
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            visible: true,
        }
    }

    /// Adds a child to this group
    pub fn add_child(&mut self, child: Box<dyn Drawable>) {
        self.children.push(child);
    }

    /// Removes a child from this group
    pub fn remove_child(&mut self, index: usize) -> Option<Box<dyn Drawable>> {
        if index < self.children.len() {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    /// Clears all children from this group
    pub fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Sets the visibility of this group
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Returns whether this group is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Drawable for NonTransformGroup {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> ggez::GameResult {
        if !self.visible {
            return Ok(());
        }

        // Draw all children without applying transformations
        for child in &self.children {
            child.draw(ctx, param)?;
        }

        Ok(())
    }

    fn dimensions(&self, _ctx: &Context) -> Option<ggez::graphics::Rect> {
        // This is a simplified implementation
        // In a real implementation, you would calculate the bounding box of all children
        None
    }
}