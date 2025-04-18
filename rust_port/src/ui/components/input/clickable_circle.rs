use gdx::math::Vector2;
use gdx::scenes::scene2d::{Actor, Group, Touchable};

/// Invisible Widget that supports detecting clicks in a circular area.
///
/// (An Image Actor does not respect alpha for its hit area, it's always square, but we want a clickable _circle_)
///
/// Usage: instantiate, position and overlay on something with `add_actor`, add listener using `on_activation`.
/// Does not implement Layout at the moment - usage e.g. in a Table Cell may need that.
///
/// Note this is a `Group` that is supposed to have no children - as a simple `Actor` the Scene2D framework won't know to call our `hit` method.
pub struct ClickableCircle {
    /// The base group functionality
    base: Group,

    /// The center of the circle
    center: Vector2,

    /// The squared maximum distance from the center (squared radius)
    max_dst2: f32,
}

impl ClickableCircle {
    /// Creates a new ClickableCircle with the given size
    pub fn new(size: f32) -> Self {
        let center = Vector2::new(size / 2.0, size / 2.0);
        let max_dst2 = size * size / 4.0; // squared radius

        let mut circle = Self {
            base: Group::new(),
            center,
            max_dst2,
        };

        // Set the touchable property to enabled
        circle.base.set_touchable(Touchable::Enabled);

        // Set the size
        circle.base.set_size(size, size);

        circle
    }

    /// Gets the center of the circle
    pub fn center(&self) -> &Vector2 {
        &self.center
    }

    /// Gets the squared maximum distance from the center
    pub fn max_dst2(&self) -> f32 {
        self.max_dst2
    }
}

impl Actor for ClickableCircle {
    /// Override the hit method to only return this actor if the hit point is within the circle
    fn hit(&self, x: f32, y: f32, touchable: bool) -> Option<&dyn Actor> {
        if self.center.dst2(x, y) < self.max_dst2 {
            Some(self)
        } else {
            None
        }
    }

    /// Delegate all other Actor methods to the base Group
    fn x(&self) -> f32 {
        self.base.x()
    }

    fn set_x(&mut self, x: f32) {
        self.base.set_x(x);
    }

    fn y(&self) -> f32 {
        self.base.y()
    }

    fn set_y(&mut self, y: f32) {
        self.base.set_y(y);
    }

    fn width(&self) -> f32 {
        self.base.width()
    }

    fn set_width(&mut self, width: f32) {
        self.base.set_width(width);
    }

    fn height(&self) -> f32 {
        self.base.height()
    }

    fn set_height(&mut self, height: f32) {
        self.base.set_height(height);
    }

    fn visible(&self) -> bool {
        self.base.visible()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn touchable(&self) -> Touchable {
        self.base.touchable()
    }

    fn set_touchable(&mut self, touchable: Touchable) {
        self.base.set_touchable(touchable);
    }

    fn color(&self) -> &gdx::graphics::Color {
        self.base.color()
    }

    fn color_mut(&mut self) -> &mut gdx::graphics::Color {
        self.base.color_mut()
    }

    fn set_color(&mut self, color: gdx::graphics::Color) {
        self.base.set_color(color);
    }

    fn parent(&self) -> Option<&dyn Actor> {
        self.base.parent()
    }

    fn stage(&self) -> Option<&gdx::scenes::scene2d::Stage> {
        self.base.stage()
    }

    fn add_listener(&mut self, listener: Box<dyn gdx::scenes::scene2d::EventListener>) {
        self.base.add_listener(listener);
    }

    fn remove_listener(&mut self, listener: &dyn gdx::scenes::scene2d::EventListener) {
        self.base.remove_listener(listener);
    }

    fn clear_listeners(&mut self) {
        self.base.clear_listeners();
    }

    fn has_listener(&self, listener: &dyn gdx::scenes::scene2d::EventListener) -> bool {
        self.base.has_listener(listener)
    }

    fn fire(&mut self, event: &mut gdx::scenes::scene2d::Event) -> bool {
        self.base.fire(event)
    }

    fn local_to_stage_coordinates(&self, local_coords: &mut Vector2) {
        self.base.local_to_stage_coordinates(local_coords);
    }

    fn stage_to_local_coordinates(&self, stage_coordinates: &mut Vector2) {
        self.base.stage_to_local_coordinates(stage_coordinates);
    }

    fn local_to_ascendant_coordinates(&self, ascendant: &dyn Actor, local_coords: &mut Vector2) {
        self.base.local_to_ascendant_coordinates(ascendant, local_coords);
    }

    fn local_to_parent_coordinates(&self, local_coords: &mut Vector2) {
        self.base.local_to_parent_coordinates(local_coords);
    }

    fn parent_to_local_coordinates(&self, parent_coords: &mut Vector2) {
        self.base.parent_to_local_coordinates(parent_coords);
    }

    fn screen_to_local_coordinates(&self, screen_coords: &mut Vector2) {
        self.base.screen_to_local_coordinates(screen_coords);
    }

    fn local_to_screen_coordinates(&self, local_coords: &mut Vector2) {
        self.base.local_to_screen_coordinates(local_coords);
    }

    fn local_to_ascendant_coordinates_without_parent(&self, ascendant: &dyn Actor, local_coords: &mut Vector2) {
        self.base.local_to_ascendant_coordinates_without_parent(ascendant, local_coords);
    }

    fn user_object(&self) -> Option<&dyn std::any::Any> {
        self.base.user_object()
    }

    fn user_object_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        self.base.user_object_mut()
    }

    fn set_user_object(&mut self, user_object: Box<dyn std::any::Any>) {
        self.base.set_user_object(user_object);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}