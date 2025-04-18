use ggez::graphics::{Color, DrawParam, Drawable, Mesh, MeshBuilder};
use ggez::mint::Point2;
use std::sync::Arc;

use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;

/// A table with a customizable border and background.
///
/// Attention: UiElementDocsWriter parses source for usages of this, and is limited to recognize
/// string literals for the [path] parameter. No other expressions please, or your skinnable element
/// will not be documented.
///
/// Note: **This class breaks automatic getUiBackground recognition in UiElementDocsWriter**,
/// and therefore gets its own parser there. Any significant changes here **must** check whether
/// that parser still works!
pub struct BorderedTable {
    /// The base Table that this BorderedTable extends
    base: Table,

    /// The path for the background and border images
    path: String,

    /// The background color
    bg_color: Color,

    /// The border color
    bg_border_color: Color,

    /// The size of the border
    border_size: f32,

    /// Whether the border is drawn on top of the background
    border_on_top: bool,

    /// The inner background drawable
    bg_inner: Box<dyn Drawable>,

    /// The border drawable
    bg_border: Box<dyn Drawable>,
}

impl BorderedTable {
    /// Creates a new BorderedTable with the given path and default background shapes
    pub fn new(
        path: String,
        default_bg_shape: Option<String>,
        default_bg_border: Option<String>,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin_strings = base_screen.skin_strings();

        let default_bg_shape = default_bg_shape.unwrap_or_else(|| skin_strings.rectangle_with_outline_shape.clone());
        let default_bg_border = default_bg_border.unwrap_or_else(|| skin_strings.rectangle_with_outline_shape.clone());

        let bg_inner = skin_strings.get_ui_background(&path, &default_bg_shape);
        let bg_border = skin_strings.get_ui_background(&format!("{}Border", path), &default_bg_border);

        Self {
            base: Table::new(),
            path,
            bg_color: ImageGetter::CHARCOAL,
            bg_border_color: Color::WHITE,
            border_size: 5.0,
            border_on_top: false,
            bg_inner,
            bg_border,
        }
    }

    /// Creates a new BorderedTable with the given path
    pub fn with_path(path: String) -> Self {
        Self::new(path, None, None)
    }

    /// Gets the background color
    pub fn bg_color(&self) -> Color {
        self.bg_color
    }

    /// Sets the background color
    pub fn set_bg_color(&mut self, color: Color) {
        self.bg_color = color;
    }

    /// Gets the border color
    pub fn bg_border_color(&self) -> Color {
        self.bg_border_color
    }

    /// Sets the border color
    pub fn set_bg_border_color(&mut self, color: Color) {
        self.bg_border_color = color;
    }

    /// Gets the border size
    pub fn border_size(&self) -> f32 {
        self.border_size
    }

    /// Sets the border size
    pub fn set_border_size(&mut self, size: f32) {
        self.border_size = size;
    }

    /// Gets whether the border is drawn on top
    pub fn border_on_top(&self) -> bool {
        self.border_on_top
    }

    /// Sets whether the border is drawn on top
    pub fn set_border_on_top(&mut self, on_top: bool) {
        self.border_on_top = on_top;
    }

    /// Gets the path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Draws the background of this table
    fn draw_background(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        let position = self.base.get_position();
        let size = self.base.get_size();
        let color = self.base.get_color();
        let parent_alpha = param.color.a;

        if self.border_on_top {
            // Draw the inner background first
            let mut inner_param = param;
            inner_param.color = Color::new(
                self.bg_color.r * color.r,
                self.bg_color.g * color.g,
                self.bg_color.b * color.b,
                self.bg_color.a * color.a * parent_alpha,
            );
            self.bg_inner.draw(ctx, inner_param)?;

            // Then draw the border
            let mut border_param = param;
            border_param.color = Color::new(
                self.bg_border_color.r * color.r,
                self.bg_border_color.g * color.g,
                self.bg_border_color.b * color.b,
                self.bg_border_color.a * color.a * parent_alpha,
            );
            self.bg_border.draw(ctx, border_param)?;
        } else {
            // Draw the border first
            let mut border_param = param;
            border_param.color = Color::new(
                self.bg_border_color.r * color.r,
                self.bg_border_color.g * color.g,
                self.bg_border_color.b * color.b,
                self.bg_border_color.a * color.a * parent_alpha,
            );
            self.bg_border.draw(ctx, border_param)?;

            // Then draw the inner background
            let mut inner_param = param;
            inner_param.color = Color::new(
                self.bg_color.r * color.r,
                self.bg_color.g * color.g,
                self.bg_color.b * color.b,
                self.bg_color.a * color.a * parent_alpha,
            );
            self.bg_inner.draw(ctx, inner_param)?;
        }

        Ok(())
    }

    /// Creates a clone of this BorderedTable
    pub fn clone(&self) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin_strings = base_screen.skin_strings();

        let bg_inner = skin_strings.get_ui_background(&this.path, &skin_strings.rectangle_with_outline_shape);
        let bg_border = skin_strings.get_ui_background(&format!("{}Border", this.path), &skin_strings.rectangle_with_outline_shape);

        Self {
            base: self.base.clone(),
            path: self.path.clone(),
            bg_color: self.bg_color,
            bg_border_color: self.bg_border_color,
            border_size: self.border_size,
            border_on_top: self.border_on_top,
            bg_inner,
            bg_border,
        }
    }
}

// Implement the necessary traits for BorderedTable
impl std::ops::Deref for BorderedTable {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for BorderedTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Implement the Widget trait for BorderedTable
impl Widget for BorderedTable {
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        // Draw the background
        self.draw_background(ctx, param)?;

        // Draw the base table
        self.base.draw(ctx, param)
    }

    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        self.base.update(ctx)
    }

    fn contains_point(&self, point: Point2<f32>) -> bool {
        self.base.contains_point(point)
    }

    fn get_stage(&self) -> Option<Arc<dyn crate::ui::components::widgets::stage::Stage>> {
        self.base.get_stage()
    }

    fn set_stage(&mut self, stage: Option<Arc<dyn crate::ui::components::widgets::stage::Stage>>) {
        self.base.set_stage(stage);
    }

    fn get_parent(&self) -> Option<Arc<dyn Widget>> {
        self.base.get_parent()
    }

    fn set_parent(&mut self, parent: Option<Arc<dyn Widget>>) {
        self.base.set_parent(parent);
    }

    fn get_position(&self) -> Point2<f32> {
        self.base.get_position()
    }

    fn set_position(&mut self, position: Point2<f32>) {
        self.base.set_position(position);
    }

    fn get_size(&self) -> Point2<f32> {
        self.base.get_size()
    }

    fn set_size(&mut self, size: Point2<f32>) {
        self.base.set_size(size);
    }

    fn get_visible(&self) -> bool {
        self.base.get_visible()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn get_enabled(&self) -> bool {
        self.base.get_enabled()
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.base.set_enabled(enabled);
    }

    fn get_name(&self) -> &str {
        self.base.get_name()
    }

    fn set_name(&mut self, name: String) {
        self.base.set_name(name);
    }

    fn get_user_data(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.base.get_user_data()
    }

    fn set_user_data(&mut self, user_data: Option<Arc<dyn std::any::Any + Send + Sync>>) {
        self.base.set_user_data(user_data);
    }

    fn get_children(&self) -> &[Arc<dyn Widget>] {
        self.base.get_children()
    }

    fn add_child(&mut self, child: Arc<dyn Widget>) {
        self.base.add_child(child);
    }

    fn remove_child(&mut self, child: &Arc<dyn Widget>) -> bool {
        self.base.remove_child(child)
    }

    fn clear_children(&mut self) {
        self.base.clear_children();
    }

    fn get_color(&self) -> Color {
        self.base.get_color()
    }

    fn set_color(&mut self, color: Color) {
        self.base.set_color(color);
    }

    fn handle_mouse_button_down_event(&mut self, button: ggez::event::MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_down_event(button, position)
    }

    fn handle_mouse_button_up_event(&mut self, button: ggez::event::MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_up_event(button, position)
    }

    fn handle_mouse_motion_event(&mut self, delta: Point2<f32>, position: Point2<f32>) -> bool {
        self.base.handle_mouse_motion_event(delta, position)
    }

    fn handle_mouse_wheel_event(&mut self, delta: ggez::event::MouseWheel, position: Point2<f32>) -> bool {
        self.base.handle_mouse_wheel_event(delta, position)
    }

    fn handle_key_down_event(&mut self, keycode: ggez::event::KeyCode, keymods: ggez::event::KeyMods, repeat: bool) -> bool {
        self.base.handle_key_down_event(keycode, keymods, repeat)
    }

    fn handle_key_up_event(&mut self, keycode: ggez::event::KeyCode, keymods: ggez::event::KeyMods) -> bool {
        self.base.handle_key_up_event(keycode, keymods)
    }

    fn handle_text_input_event(&mut self, character: char) -> bool {
        self.base.handle_text_input_event(character)
    }

    fn handle_focus_event(&mut self, focused: bool) -> bool {
        self.base.handle_focus_event(focused)
    }

    fn handle_scroll_focus_event(&mut self, focused: bool) -> bool {
        self.base.handle_scroll_focus_event(focused)
    }

    fn handle_cursor_enter_event(&mut self, entered: bool) -> bool {
        self.base.handle_cursor_enter_event(entered)
    }

    fn handle_cursor_leave_event(&mut self, entered: bool) -> bool {
        self.base.handle_cursor_leave_event(entered)
    }

    fn handle_cursor_motion_event(&mut self, position: Point2<f32>) -> bool {
        self.base.handle_cursor_motion_event(position)
    }

    fn handle_touch_event(&mut self, phase: ggez::event::TouchPhase, position: Point2<f32>, id: i64) -> bool {
        self.base.handle_touch_event(phase, position, id)
    }

    fn handle_resize_event(&mut self, width: f32, height: f32) -> bool {
        self.base.handle_resize_event(width, height)
    }

    fn handle_draw_event(&mut self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        self.base.handle_draw_event(ctx, param)
    }

    fn handle_update_event(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        self.base.handle_update_event(ctx)
    }
}