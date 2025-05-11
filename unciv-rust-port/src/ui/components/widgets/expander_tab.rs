use ggez::graphics::{Color, DrawParam};
use ggez::mint::Point2;
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::Constants;
use crate::ui::components::extensions::Scene2dExtensions;
use crate::ui::components::input::{KeyboardBinding, KeyShortcuts};
use crate::ui::components::widgets::scroll_pane::ScrollPane;
use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::{IconCircleGroup, ImageGetter};
use crate::ui::screens::basescreen::BaseScreen;
use crate::UncivGame;

/// A widget with a header that when clicked shows/hides a sub-Table.
///
/// # Arguments
///
/// * `title` - The header text, automatically translated.
/// * `font_size` - Size applied to header text (only)
/// * `icon` - Optional icon - please use [Image][com.badlogic.gdx.scenes.scene2d.ui.Image] or [IconCircleGroup]
/// * `default_pad` - Padding between content and wrapper.
/// * `header_pad` - Default padding for the header Table.
/// * `expander_width` - If set initializes header width
/// * `expander_height` - If set initializes header height
/// * `persistence_id` - If specified, the ExpanderTab will remember its open/closed state for the duration of one app run
/// * `on_change` - If specified, this will be called after the visual change for a change in [is_open] completes (e.g. to react to changed size)
/// * `init_content` - Optional lambda with [inner_table] as parameter, to help initialize content.
pub struct ExpanderTab {
    /// The base Table that this ExpanderTab extends
    base: Table,

    /// The title of the expander tab
    title: String,

    /// The header with label, header_content and icon, touchable to show/hide.
    /// This internal container is public to allow e.g. alignment changes.
    pub header: Table,

    /// Additional elements can be added to the `ExpanderTab`'s header using this container, empty by default.
    pub header_content: Table,

    /// The header label
    header_label: Arc<dyn Widget>,

    /// The header icon
    header_icon: Arc<dyn Widget>,

    /// Wrapper for inner_table, this is what will be shown/hidden
    content_wrapper: Table,

    /// The container where the client should add the content to toggle
    pub inner_table: Table,

    /// Indicates whether the contents are currently shown, changing this will animate the widget
    is_open: bool,

    /// The persistence ID for remembering the open/closed state
    persistence_id: Option<String>,

    /// Callback for when the open state changes
    on_change: Option<Box<dyn Fn() + Send + Sync>>,

    /// The toggle key binding
    toggle_key: KeyboardBinding,
}

impl ExpanderTab {
    // Constants
    const ARROW_SIZE: f32 = 18.0;
    const ARROW_IMAGE: &'static str = "OtherIcons/BackArrow";
    const ARROW_COLOR: Color = Color::new(1.0, 0.96, 0.75, 1.0);
    const ANIMATION_DURATION: f32 = 0.2;

    // Static map to store persisted states
    lazy_static! {
        static ref PERSISTED_STATES: std::sync::Mutex<HashMap<String, bool>> = std::sync::Mutex::new(HashMap::new());
    }

    /// Creates a new ExpanderTab with the given parameters
    pub fn new(
        title: String,
        font_size: Option<i32>,
        icon: Option<Arc<dyn Widget>>,
        starts_out_opened: Option<bool>,
        default_pad: Option<f32>,
        header_pad: Option<f32>,
        expander_width: Option<f32>,
        expander_height: Option<f32>,
        persistence_id: Option<String>,
        toggle_key: Option<KeyboardBinding>,
        on_change: Option<Box<dyn Fn() + Send + Sync>>,
        init_content: Option<Box<dyn Fn(&mut Table) + Send + Sync>>,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let font_size = font_size.unwrap_or(Constants::heading_font_size());
        let starts_out_opened = starts_out_opened.unwrap_or(true);
        let default_pad = default_pad.unwrap_or(10.0);
        let header_pad = header_pad.unwrap_or(10.0);
        let expander_width = expander_width.unwrap_or(0.0);
        let expander_height = expander_height.unwrap_or(0.0);
        let toggle_key = toggle_key.unwrap_or(KeyboardBinding::None);

        let mut header = Table::new(skin.clone());
        let header_content = Table::new(skin.clone());
        let header_label = title.to_label(Some(font_size), None, None, Some(true));
        let header_icon = ImageGetter::get_image(Self::ARROW_IMAGE);
        let content_wrapper = Table::new(skin.clone());
        let inner_table = Table::new(skin.clone());

        // Get persisted state if available
        let is_open = if let Some(id) = &persistence_id {
            PERSISTED_STATES.lock().unwrap().get(id).copied().unwrap_or(starts_out_opened)
        } else {
            starts_out_opened
        };

        let mut expander = Self {
            base: Table::new(skin),
            title,
            header,
            header_content,
            header_label,
            header_icon,
            content_wrapper,
            inner_table,
            is_open,
            persistence_id,
            on_change,
            toggle_key,
        };

        // Initialize the expander
        expander.init(
            icon,
            header_pad,
            expander_height,
            expander_width,
            default_pad,
            init_content,
        );

        expander
    }

    /// Initializes the expander tab
    fn init(
        &mut self,
        icon: Option<Arc<dyn Widget>>,
        header_pad: f32,
        expander_height: f32,
        expander_width: f32,
        default_pad: f32,
        init_content: Option<Box<dyn Fn(&mut Table) + Send + Sync>>,
    ) {
        // Set up header
        self.header.defaults().pad(header_pad);
        if expander_height > 0.0 {
            self.header.defaults().height(expander_height);
        }

        // Set up header icon
        self.header_icon.set_size(Self::ARROW_SIZE, Self::ARROW_SIZE);
        self.header_icon.set_origin(ggez::graphics::Align::Center);
        self.header_icon.set_rotation(0.0);
        self.header_icon.set_color(Self::ARROW_COLOR);

        // Set up header background
        let base_screen = BaseScreen::get_instance();
        let skin_strings = base_screen.skin_strings();
        self.header.set_background(
            skin_strings.get_ui_background(
                "General/ExpanderTab",
                Some(skin_strings.skin_config().base_color()),
            ),
        );

        // Add components to header
        if let Some(icon) = icon {
            self.header.add_child(icon);
        }
        self.header.add_child(self.header_label.clone());
        self.header.add_child(self.header_content.clone()).grow_x();
        self.header.add_child(self.header_icon.clone()).size(Self::ARROW_SIZE).align(ggez::graphics::Align::Center);

        // Make header touchable
        self.header.set_touchable(true);

        // Set up activation handler
        let mut key_shortcuts = KeyShortcuts::new();
        key_shortcuts.add(self.toggle_key);
        self.header.set_key_shortcuts(key_shortcuts);

        // Set up base table
        if expander_width != 0.0 {
            self.base.defaults().min_width(expander_width);
        }
        self.base.defaults().grow_x();

        // Set up content wrapper
        self.content_wrapper.defaults().grow_x().pad(default_pad);

        // Set up inner table
        self.inner_table.defaults().grow_x();

        // Add components to base table
        self.base.add_child(self.header.clone()).fill().row();
        self.base.add_child(self.content_wrapper.clone());

        // Add inner table to content wrapper
        self.content_wrapper.add_child(self.inner_table.clone());

        // Initialize content if provided
        if let Some(init_content) = init_content {
            init_content(&mut self.inner_table);
        }

        // Set header width to match content width if expander_width is 0
        if expander_width == 0.0 {
            if self.inner_table.needs_layout() {
                self.content_wrapper.pack();
            }
            self.base.get_cell(&self.header).min_width(self.content_wrapper.width());
        }

        // Update the expander
        self.update(true);
    }

    /// Updates the expander tab
    fn update(&mut self, no_animation: bool) {
        // Save state to persisted states if persistence_id is set
        if let Some(id) = &self.persistence_id {
            PERSISTED_STATES.lock().unwrap().insert(id.clone(), self.is_open);
        }

        // Handle no animation or continuous rendering disabled
        if no_animation || !UncivGame::current().settings().continuous_rendering() {
            self.content_wrapper.clear_children();
            if self.is_open {
                self.content_wrapper.add_child(self.inner_table.clone());
            }
            self.header_icon.set_rotation(if self.is_open { 90.0 } else { 0.0 });
            if !no_animation {
                if let Some(on_change) = &self.on_change {
                    on_change();
                }
            }
            return;
        }

        // Create animation action
        let start_value = if self.is_open { 0.0 } else { 90.0 };
        let end_value = if self.is_open { 90.0 } else { 0.0 };

        // In a real implementation, this would use ggez's animation system
        // For now, we'll just set the rotation directly
        self.header_icon.set_rotation(end_value);
        self.content_wrapper.clear_children();
        if self.is_open {
            self.content_wrapper.add_child(self.inner_table.clone());
        }
        if let Some(on_change) = &self.on_change {
            on_change();
        }
    }

    /// Toggles the open state of the expander tab
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;

        // In the common case where the expander is hosted in a Table within a ScrollPane...
        // try scrolling our header so it is visible (when toggled by keyboard)
        if let Some(parent) = self.base.parent() {
            if let Some(parent_parent) = parent.parent() {
                if parent.is::<Table>() && parent_parent.is::<ScrollPane>() {
                    self.try_auto_scroll(parent_parent.downcast_ref::<ScrollPane>().unwrap());
                }
                // But - our Actor.addBorder extension can ruin that, so cater for that special case too...
                else if self.test_for_bordered_table() {
                    if let Some(parent_parent_parent) = parent_parent.parent() {
                        if parent_parent_parent.is::<ScrollPane>() {
                            self.try_auto_scroll(parent_parent_parent.downcast_ref::<ScrollPane>().unwrap());
                        }
                    }
                }
            }
        }
    }

    /// Tests if the parent is a bordered table
    fn test_for_bordered_table(&self) -> bool {
        if let Some(parent) = self.base.parent() {
            if !parent.is::<Table>() {
                return false;
            }

            if let Some(parent_parent) = parent.parent() {
                let border_table = parent_parent.downcast_ref::<Table>()?;

                if let Some(parent_parent_parent) = parent_parent.parent() {
                    if !parent_parent_parent.is::<ScrollPane>() {
                        return false;
                    }

                    return border_table.cells().len() == 1
                        && border_table.background().is_some()
                        && border_table.pad_top() == 2.0;
                }
            }
        }

        false
    }

    /// Tries to auto-scroll the scroll pane to show the header
    fn try_auto_scroll(&self, scroll_pane: &ScrollPane) {
        if scroll_pane.is_scrolling_disabled_y() {
            return;
        }

        // As the "opening" is animated, and right now the animation has just started,
        // a scroll-to-visible won't work, so limit it to showing the header for now.
        let height_to_show = self.header.height();

        // Coords as seen by "this" expander relative to parent and as seen by scrollPane may differ by the border size
        // Also make area to show relative to top
        let y_to_show = self.base.y() + self.base.height() - height_to_show +
            if let Some(parent) = self.base.parent() {
                if scroll_pane.actor().as_ref() == Some(parent.as_ref()) {
                    0.0
                } else {
                    parent.y()
                }
            } else {
                0.0
            };

        // scrollTo does the y axis inversion for us, and also will do nothing if the requested area is already fully visible
        scroll_pane.scroll_to(0.0, y_to_show, self.header.width(), height_to_show);
    }

    /// Changes the header label text after initialization (does not auto-translate)
    pub fn set_text(&mut self, text: String) {
        if let Some(label) = self.header_label.downcast_ref::<crate::ui::components::widgets::label::Label>() {
            label.set_text(text);
        }
    }

    /// Gets whether the header icon is visible
    pub fn is_header_icon_visible(&self) -> bool {
        self.header_icon.visible()
    }

    /// Sets whether the header icon is visible
    pub fn set_header_icon_visible(&mut self, visible: bool) {
        self.header_icon.set_visible(visible);
    }

    /// Gets whether the expander is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Sets whether the expander is open
    pub fn set_is_open(&mut self, is_open: bool) {
        if self.is_open == is_open {
            return;
        }
        self.is_open = is_open;
        self.update(false);
    }

    /// Creates a clone of this ExpanderTab
    pub fn clone(&self) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let mut header = Table::new(skin.clone());
        let header_content = Table::new(skin.clone());
        let header_label = self.title.to_label(Some(Constants::heading_font_size()), None, None, Some(true));
        let header_icon = ImageGetter::get_image(Self::ARROW_IMAGE);
        let content_wrapper = Table::new(skin.clone());
        let inner_table = Table::new(skin.clone());

        let mut expander = Self {
            base: Table::new(skin),
            title: self.title.clone(),
            header,
            header_content,
            header_label,
            header_icon,
            content_wrapper,
            inner_table,
            is_open: self.is_open,
            persistence_id: self.persistence_id.clone(),
            on_change: self.on_change.clone(),
            toggle_key: self.toggle_key,
        };

        // Initialize the expander
        expander.init(
            None,
            10.0,
            0.0,
            0.0,
            10.0,
            None,
        );

        expander
    }
}

// Implement the necessary traits for ExpanderTab
impl std::ops::Deref for ExpanderTab {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ExpanderTab {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Implement the Widget trait for ExpanderTab
impl Widget for ExpanderTab {
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

    fn get_color(&self) -> Color {
        self.base.get_color()
    }

    fn set_color(&mut self, color: Color) {
        self.base.set_color(color);
    }

    fn handle_mouse_button_down_event(&mut self, button: ggez::event::MouseButton, position: Point2<f32>) -> bool {
        let result = self.base.handle_mouse_button_down_event(button, position);
        if result && self.header.contains_point(position) {
            self.toggle();
        }
        result
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
        let result = self.base.handle_key_down_event(keycode, keymods, repeat);
        if result && self.toggle_key.matches(keycode, keymods) {
            self.toggle();
        }
        result
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