use ggez::event::{MouseButton, MouseWheel};
use ggez::graphics::DrawParam;
use ggez::input::mouse::MousePosition;
use ggez::mint::Point2;
use std::sync::Arc;

use crate::ui::components::widgets::scroll_pane::{ScrollPane, ScrollPaneStyle};
use crate::ui::components::widgets::skin::Skin;
use crate::ui::components::widgets::widget::Widget;

/// A scroll pane that automatically handles mouse wheel scrolling when the mouse is over it.
///
/// ** Problem **
/// Standard ScrollPane widgets support vertical scrolling by a mouse wheel.
/// That works once they have 'Scroll focus' (there's keyboard focus, too) - e.g. once they
/// are dragged or clicked once. However, the user expects the mouse wheel to affect
/// a scrollable widget as soon as the mouse points to it.
///
/// ** Approach **
/// Listen to enter and exit events and set focus as needed.
/// The old focus is saved on enter and restored on exit to make this as side-effect free as possible.
///
/// ** Implementation **
/// The listener is attached per widget (and not, say, to an upper container or the screen, where
/// one listener would suffice but we'd have to do coordinate to target resolution ourselves).
/// This is accomplished by subclassing the ScrollPane and replacing usages,
/// which in turn can be done either by using this class as drop-in replacement per widget
/// or by importing this using an import alias per file.
///
/// ** Notes **
/// This should not be used in cases where the mouse wheel should do something else,
/// e.g. zooming. For panes scrolling only horizontally, using this class is redundant.
pub struct AutoScrollPane {
    /// The base ScrollPane that this AutoScrollPane extends
    base: ScrollPane,

    /// The saved focus to restore when the mouse leaves
    saved_focus: Option<Arc<dyn Widget>>,

    /// Whether the mouse is currently over this pane
    mouse_over: bool,
}

impl AutoScrollPane {
    /// Creates a new AutoScrollPane with the given widget and style
    pub fn new(widget: Option<Arc<dyn Widget>>, style: ScrollPaneStyle) -> Self {
        let mut pane = Self {
            base: ScrollPane::new(widget, style),
            saved_focus: None,
            mouse_over: false,
        };

        // Ensure the listener is attached
        pane.ensure_listener();

        pane
    }

    /// Creates a new AutoScrollPane with the given widget and skin
    pub fn with_skin(widget: Option<Arc<dyn Widget>>, skin: &Skin) -> Self {
        Self::new(widget, skin.get_scroll_pane_style())
    }

    /// Creates a new AutoScrollPane with the given widget, skin, and style name
    pub fn with_style_name(widget: Option<Arc<dyn Widget>>, skin: &Skin, style_name: &str) -> Self {
        Self::new(widget, skin.get_scroll_pane_style_by_name(style_name))
    }

    /// Sets whether scrolling is disabled in the x and y directions
    pub fn set_scrolling_disabled(&mut self, x: bool, y: bool) {
        self.base.set_scrolling_disabled(x, y);
        self.ensure_listener();
    }

    /// Ensures the mouse over listener is attached or removed as needed
    fn ensure_listener(&mut self) {
        // If scrolling is disabled in both directions, we don't need the listener
        if self.base.is_scrolling_disabled_x() && self.base.is_scrolling_disabled_y() {
            self.mouse_over = false;
            self.saved_focus = None;
        }
    }

    /// Handles mouse enter events
    pub fn handle_mouse_enter(&mut self, position: Point2<f32>) -> bool {
        // If the mouse is already over this pane, do nothing
        if self.mouse_over {
            return false;
        }

        // If scrolling is disabled in both directions, do nothing
        if self.base.is_scrolling_disabled_x() && self.base.is_scrolling_disabled_y() {
            return false;
        }

        // Check if the mouse is over this pane
        if !self.base.contains_point(position) {
            return false;
        }

        // Set the mouse over flag
        self.mouse_over = true;

        // Save the current scroll focus
        if self.saved_focus.is_none() {
            self.saved_focus = self.base.get_stage().and_then(|stage| stage.get_scroll_focus());
        }

        // Set this pane as the scroll focus
        if let Some(stage) = self.base.get_stage() {
            stage.set_scroll_focus(Some(Arc::new(self.clone())));
        }

        true
    }

    /// Handles mouse exit events
    pub fn handle_mouse_exit(&mut self, position: Point2<f32>) -> bool {
        // If the mouse is not over this pane, do nothing
        if !self.mouse_over {
            return false;
        }

        // Check if the mouse is still over this pane
        if self.base.contains_point(position) {
            return false;
        }

        // Reset the mouse over flag
        self.mouse_over = false;

        // Restore the saved scroll focus
        if let Some(stage) = self.base.get_stage() {
            if stage.get_scroll_focus().as_ref().map_or(false, |focus| Arc::ptr_eq(focus, &Arc::new(self.clone()))) {
                stage.set_scroll_focus(self.saved_focus.clone());
            }
        }

        self.saved_focus = None;

        true
    }

    /// Handles mouse wheel events
    pub fn handle_mouse_wheel(&mut self, delta: f32) -> bool {
        // If the mouse is not over this pane, do nothing
        if !self.mouse_over {
            return false;
        }

        // If scrolling is disabled in both directions, do nothing
        if self.base.is_scrolling_disabled_x() && self.base.is_scrolling_disabled_y() {
            return false;
        }

        // Handle the mouse wheel event
        self.base.handle_mouse_wheel(delta)
    }

    /// Gets the base ScrollPane
    pub fn base(&self) -> &ScrollPane {
        &self.base
    }

    /// Gets a mutable reference to the base ScrollPane
    pub fn base_mut(&mut self) -> &mut ScrollPane {
        &mut self.base
    }

    /// Creates a clone of this AutoScrollPane
    pub fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            saved_focus: None, // Don't clone the saved focus
            mouse_over: false, // Reset the mouse over flag
        }
    }
}

// Implement the necessary traits for AutoScrollPane
impl std::ops::Deref for AutoScrollPane {
    type Target = ScrollPane;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for AutoScrollPane {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Implement the Widget trait for AutoScrollPane
impl Widget for AutoScrollPane {
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
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

    fn handle_mouse_button_down_event(&mut self, button: MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_down_event(button, position)
    }

    fn handle_mouse_button_up_event(&mut self, button: MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_up_event(button, position)
    }

    fn handle_mouse_motion_event(&mut self, delta: Point2<f32>, position: Point2<f32>) -> bool {
        // Check for mouse enter/exit
        let was_over = self.mouse_over;
        let is_over = self.base.contains_point(position);

        if is_over && !was_over {
            self.handle_mouse_enter(position);
        } else if !is_over && was_over {
            self.handle_mouse_exit(position);
        }

        self.base.handle_mouse_motion_event(delta, position)
    }

    fn handle_mouse_wheel_event(&mut self, delta: MouseWheel, position: Point2<f32>) -> bool {
        // Check for mouse enter/exit
        let was_over = self.mouse_over;
        let is_over = self.base.contains_point(position);

        if is_over && !was_over {
            self.handle_mouse_enter(position);
        } else if !is_over && was_over {
            self.handle_mouse_exit(position);
        }

        // If the mouse is over this pane, handle the wheel event
        if self.mouse_over {
            match delta {
                MouseWheel::Y(delta) => self.handle_mouse_wheel(delta),
                _ => false,
            }
        } else {
            self.base.handle_mouse_wheel_event(delta, position)
        }
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