use ggez::graphics::{self, Color, DrawParam, Image};
use ggez::mint::Point2;
use ggez::Context;
use ggez::GameResult;

use crate::ui::components::non_transform_group::NonTransformGroup;
use crate::ui::images::image_getter::ImageGetter;

/// A group that displays an actor centered inside a circular background.
pub struct IconCircleGroup {
    group: NonTransformGroup,
    circle: Image,
    actor: Box<dyn Actor>,
    color: Color,
    size: f32,
}

/// Trait for actors that can be drawn and positioned.
pub trait Actor: Send + Sync {
    /// Draws the actor.
    fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult;

    /// Gets the width of the actor.
    fn get_width(&self) -> f32;

    /// Gets the height of the actor.
    fn get_height(&self) -> f32;

    /// Sets the position of the actor.
    fn set_position(&mut self, x: f32, y: f32);

    /// Sets the size of the actor.
    fn set_size(&mut self, width: f32, height: f32);

    /// Sets the origin of the actor.
    fn set_origin(&mut self, x: f32, y: f32);
}

impl IconCircleGroup {
    /// Creates a new IconCircleGroup with the given size, actor, and optional parameters.
    pub fn new(
        size: f32,
        actor: Box<dyn Actor>,
        resize_actor: bool,
        color: Color,
        circle_image: &str,
    ) -> Self {
        let mut group = NonTransformGroup::new();
        group.set_size(size, size);

        let circle = ImageGetter::get_image(circle_image);
        let mut circle_clone = circle.clone();
        circle_clone.set_size(size, size);
        circle_clone.set_color(color);

        let mut actor_clone = actor;
        if resize_actor {
            actor_clone.set_size(size * 0.75, size * 0.75);
        }

        // Center the actor in the group
        let actor_width = actor_clone.get_width();
        let actor_height = actor_clone.get_height();
        let actor_x = (size - actor_width) / 2.0;
        let actor_y = (size - actor_height) / 2.0;
        actor_clone.set_position(actor_x, actor_y);

        // Set the origin to the center
        actor_clone.set_origin(actor_width / 2.0, actor_height / 2.0);

        // Add the circle and actor to the group
        group.add_child(Box::new(circle_clone));
        group.add_child(actor_clone);

        Self {
            group,
            circle,
            actor: actor_clone,
            color,
            size,
        }
    }

    /// Draws the IconCircleGroup.
    pub fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult {
        self.group.draw(ctx, parent_alpha * self.color.a)
    }

    /// Gets the width of the IconCircleGroup.
    pub fn get_width(&self) -> f32 {
        self.size
    }

    /// Gets the height of the IconCircleGroup.
    pub fn get_height(&self) -> f32 {
        self.size
    }

    /// Sets the position of the IconCircleGroup.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.group.set_position(x, y);
    }

    /// Sets the size of the IconCircleGroup.
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.size = width;
        self.group.set_size(width, height);

        // Update the circle size
        if let Some(child) = self.group.get_child(0) {
            if let Some(circle) = child.as_any().downcast_ref::<Image>() {
                circle.set_size(width, height);
            }
        }

        // Update the actor size and position
        if let Some(child) = self.group.get_child(1) {
            if let Some(actor) = child.as_any().downcast_ref::<Box<dyn Actor>>() {
                let actor_width = width * 0.75;
                let actor_height = height * 0.75;
                actor.set_size(actor_width, actor_height);

                let actor_x = (width - actor_width) / 2.0;
                let actor_y = (height - actor_height) / 2.0;
                actor.set_position(actor_x, actor_y);

                actor.set_origin(actor_width / 2.0, actor_height / 2.0);
            }
        }
    }

    /// Sets the color of the IconCircleGroup.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;

        // Update the circle color
        if let Some(child) = self.group.get_child(0) {
            if let Some(circle) = child.as_any().downcast_ref::<Image>() {
                circle.set_color(color);
            }
        }
    }
}