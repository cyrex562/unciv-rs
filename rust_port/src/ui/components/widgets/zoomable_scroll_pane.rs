use std::f32;
use std::option::Option;
use std::sync::Arc;

use ggez::graphics::{Color, DrawParam, Drawable, Mesh, MeshBuilder, Rect, Text};
use ggez::input::mouse::MouseInput;
use ggez::mint::{Point2, Vector2};
use ggez::Context;
use ggez::event::MouseButton;
use ggez::graphics::DrawMode;
use ggez::graphics::Align;

use crate::ui::components::widgets::scroll_pane::ScrollPane;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::components::extensions::add_to_center;
use crate::ui::components::extensions::center_x;
use crate::ui::components::extensions::color_from_rgb;
use crate::ui::components::extensions::set_size;
use crate::ui::components::extensions::surround_with_circle;
use crate::ui::components::extensions::surround_with_thin_circle;
use crate::ui::components::zoom_gesture_listener::ZoomGestureListener;
use crate::ui::components::keyboard_panning_listener::KeyboardPanningListener;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::basescreen::UncivGame;
use crate::ui::screens::basescreen::GUI;
use crate::ui::screens::basescreen::GameSettings;

/// A scroll pane that supports zooming and panning
pub struct ZoomableScrollPane {
    /// The base ScrollPane that this ZoomableScrollPane extends
    base: ScrollPane,

    /// Extra culling in X direction
    extra_culling_x: f32,

    /// Extra culling in Y direction
    extra_culling_y: f32,

    /// Minimum zoom level
    min_zoom: f32,

    /// Maximum zoom level
    max_zoom: f32,

    /// Whether continuous scrolling is enabled in X direction
    continuous_scrolling_x: bool,

    /// Callback for when viewport changes
    on_viewport_changed_listener: Option<Box<dyn Fn(f32, f32, Rect)>>,

    /// Callback for when panning stops
    on_pan_stop_listener: Option<Box<dyn Fn()>>,

    /// Callback for when panning starts
    on_pan_start_listener: Option<Box<dyn Fn()>>,

    /// Callback for when zooming stops
    on_zoom_stop_listener: Option<Box<dyn Fn()>>,

    /// Callback for when zooming starts
    on_zoom_start_listener: Option<Box<dyn Fn()>>,

    /// The zoom listener
    zoom_listener: ZoomListener,

    /// Whether auto-scrolling is enabled
    is_auto_scroll_enabled: bool,

    /// Map panning speed
    map_panning_speed: f32,

    /// Current scrolling destination
    scrolling_to: Option<Point2<f32>>,

    /// Current scrolling action
    scrolling_action: Option<Box<dyn Action>>,
}

/// A listener for zoom gestures
struct ZoomListener {
    /// The parent ZoomableScrollPane
    parent: Arc<ZoomableScrollPane>,

    /// Whether zooming is in progress
    is_zooming: bool,

    /// Current zoom action
    zoom_action: Option<Box<ZoomAction>>,
}

/// An action for zooming
struct ZoomAction {
    /// The parent ZoomableScrollPane
    parent: Arc<ZoomableScrollPane>,

    /// Starting zoom level
    starting_zoom: f32,

    /// Finishing zoom level
    finishing_zoom: f32,

    /// Current zoom level
    current_zoom: f32,

    /// Duration of the action
    duration: f32,

    /// Current time
    current_time: f32,

    /// Interpolation function
    interpolation: Box<dyn Fn(f32) -> f32>,
}

/// A listener for flick scrolling
struct FlickScrollListener {
    /// The parent ZoomableScrollPane
    parent: Arc<ZoomableScrollPane>,

    /// Whether panning is in progress
    is_panning: bool,
}

/// An action for scrolling to a position
struct ScrollToAction {
    /// The parent ZoomableScrollPane
    parent: Arc<ZoomableScrollPane>,

    /// Original scroll X position
    original_scroll_x: f32,

    /// Original scroll Y position
    original_scroll_y: f32,

    /// Duration of the action
    duration: f32,

    /// Current time
    current_time: f32,

    /// Interpolation function
    interpolation: Box<dyn Fn(f32) -> f32>,
}

/// A trait for actions
trait Action {
    /// Updates the action
    fn update(&mut self, dt: f32) -> bool;
}

impl ZoomableScrollPane {
    /// Creates a new ZoomableScrollPane
    ///
    /// # Arguments
    ///
    /// * `extra_culling_x` - Extra culling in X direction
    /// * `extra_culling_y` - Extra culling in Y direction
    /// * `min_zoom` - Minimum zoom level
    /// * `max_zoom` - Maximum zoom level
    pub fn new(
        extra_culling_x: f32,
        extra_culling_y: f32,
        min_zoom: f32,
        max_zoom: f32,
    ) -> Self {
        let base = ScrollPane::new(WidgetGroup::new());

        let mut pane = Self {
            base,
            extra_culling_x,
            extra_culling_y,
            min_zoom,
            max_zoom,
            continuous_scrolling_x: false,
            on_viewport_changed_listener: None,
            on_pan_stop_listener: None,
            on_pan_start_listener: None,
            on_zoom_stop_listener: None,
            on_zoom_start_listener: None,
            zoom_listener: ZoomListener {
                parent: Arc::new(Self::default()),
                is_zooming: false,
                zoom_action: None,
            },
            is_auto_scroll_enabled: false,
            map_panning_speed: 6.0,
            scrolling_to: None,
            scrolling_action: None,
        };

        // Set up the zoom listener
        pane.zoom_listener.parent = Arc::new(pane.clone());

        // Add the zoom listener
        pane.base.add_listener(Box::new(pane.zoom_listener.clone()));

        pane
    }

    /// Reloads the maximum zoom level
    pub fn reload_max_zoom(&mut self) {
        let settings = UncivGame::current().settings();
        self.max_zoom = settings.max_world_zoom_out();
        self.min_zoom = 1.0 / self.max_zoom;

        // Since normally min isn't reached exactly, only powers of 0.8
        if self.base.scale_x() < self.min_zoom {
            self.zoom(1.0);
        }
    }

    /// Gets the horizontal padding
    fn horizontal_padding(&self) -> f32 {
        self.base.width() / 2.0
    }

    /// Gets the vertical padding
    fn vertical_padding(&self) -> f32 {
        self.base.height() / 2.0
    }

    /// Gets the actor
    pub fn get_actor(&self) -> Option<Box<dyn Widget>> {
        let group = self.base.get_actor().downcast_ref::<WidgetGroup>()?;
        if group.has_children() {
            Some(group.children()[0].clone())
        } else {
            None
        }
    }

    /// Sets the actor
    pub fn set_actor(&mut self, content: Option<Box<dyn Widget>>) {
        if let Some(group) = self.base.get_actor().downcast_ref::<WidgetGroup>() {
            group.clear_children();
            if let Some(content) = content {
                group.add_actor(content);
            }
        } else {
            self.base.set_actor(content);
        }
    }

    /// Scrolls in X direction
    pub fn scroll_x(&mut self, pixels_x: f32) {
        self.base.scroll_x(pixels_x);
        self.update_culling();
        self.on_viewport_changed();
    }

    /// Sets the scroll X position
    pub fn set_scroll_x(&mut self, pixels: f32) {
        let mut result = pixels;

        if self.continuous_scrolling_x {
            if result < 0.0 {
                result += self.base.max_x();
            } else if result > self.base.max_x() {
                result -= self.base.max_x();
            }
        }

        self.base.set_scroll_x(result);
    }

    /// Scrolls in Y direction
    pub fn scroll_y(&mut self, pixels_y: f32) {
        self.base.scroll_y(pixels_y);
        self.update_culling();
        self.on_viewport_changed();
    }

    /// Called when size changes
    pub fn size_changed(&mut self) {
        self.update_padding();
        self.base.size_changed();
        self.update_culling();
    }

    /// Updates the padding
    fn update_padding(&mut self) {
        if let Some(content) = self.get_actor() {
            // Padding is always [dimension / 2] because we want to be able to have the center of the scrollPane at the very edge of the content
            content.set_x(self.horizontal_padding());
            content.set_y(self.vertical_padding());

            if let Some(group) = self.base.get_actor().downcast_ref::<WidgetGroup>() {
                group.set_width(content.width() + self.horizontal_padding() * 2.0);
                group.set_height(content.height() + self.vertical_padding() * 2.0);
            }
        }
    }

    /// Updates the culling
    pub fn update_culling(&self) {
        if let Some(content) = self.get_actor() {
            if let Some(cullable) = content.as_any().downcast_ref::<dyn Cullable>() {
                let viewport = self.get_viewport();
                let mut culling_area = viewport;

                // Add in all directions
                culling_area.x -= self.extra_culling_x;
                culling_area.y -= self.extra_culling_y;
                culling_area.width += self.extra_culling_x * 2.0;
                culling_area.height += self.extra_culling_y * 2.0;

                cullable.set_culling_area(culling_area);
            }
        }
    }

    /// Zooms to the specified scale
    pub fn zoom(&mut self, zoom_scale: f32) {
        let new_zoom = zoom_scale.clamp(self.min_zoom, self.max_zoom);
        let old_zoom_x = self.base.scale_x();
        let old_zoom_y = self.base.scale_y();

        if new_zoom == old_zoom_x {
            return;
        }

        let new_width = self.base.width() * old_zoom_x / new_zoom;
        let new_height = self.base.height() * old_zoom_y / new_zoom;

        // When we scale, the width & height values stay the same. However, after scaling up/down, the width will be rendered wider/narrower than before.
        // But we want to keep the size of the pane the same, so we do need to adjust the width & height: smaller if the scale increased, larger if it decreased.
        self.base.set_scale(new_zoom);
        self.base.set_size(new_width, new_height);

        self.on_viewport_changed();
        // The size increase/decrease kept scrollX and scrollY (i.e. the top edge and left edge) the same - but changing the scale & size should have changed
        // where the right and bottom edges are. This would mean our visual center moved. To correct this, we theoretically need to update the scroll position
        // by half (i.e. middle) of what our size changed.
        // However, we also changed the padding, which is exactly equal to half of our size change, so we actually don't need to move our center at all.
    }

    /// Zooms in
    ///
    /// # Arguments
    ///
    /// * `immediate` - Whether to zoom immediately
    pub fn zoom_in(&mut self, immediate: bool) {
        if immediate {
            self.zoom(self.base.scale_x() / 0.8);
        } else {
            self.zoom_listener.zoom_in(0.8);
        }
    }

    /// Zooms out
    ///
    /// # Arguments
    ///
    /// * `immediate` - Whether to zoom immediately
    pub fn zoom_out(&mut self, immediate: bool) {
        if immediate {
            self.zoom(self.base.scale_x() * 0.8);
        } else {
            self.zoom_listener.zoom_out(0.8);
        }
    }

    /// Checks if zooming is in progress
    pub fn is_zooming(&self) -> bool {
        self.zoom_listener.is_zooming
    }

    /// Restricts the X scroll position
    pub fn restrict_x(&self, delta_x: f32) -> f32 {
        self.base.scroll_x() - delta_x
    }

    /// Restricts the Y scroll position
    pub fn restrict_y(&self, delta_y: f32) -> f32 {
        self.base.scroll_y() + delta_y
    }

    /// Performs keyboard WASD or mouse-at-edge panning
    ///
    /// # Arguments
    ///
    /// * `delta_x` - Delta X (positive = left)
    /// * `delta_y` - Delta Y (positive = down)
    pub fn do_key_or_mouse_panning(&mut self, delta_x: f32, delta_y: f32) {
        if delta_x == 0.0 && delta_y == 0.0 {
            return;
        }

        let amount_to_move = self.map_panning_speed / self.base.scale_x();
        self.base.set_scroll_x(self.restrict_x(delta_x * amount_to_move));
        self.base.set_scroll_y(self.restrict_y(delta_y * amount_to_move));
        self.base.update_visual_scroll();
    }

    /// Gets the flick scroll listener
    pub fn get_flick_scroll_listener(&self) -> Box<dyn ActorGestureListener> {
        Box::new(FlickScrollListener {
            parent: Arc::new(self.clone()),
            is_panning: false,
        })
    }

    /// Checks if scrolling is in progress
    pub fn is_scrolling(&self) -> bool {
        self.scrolling_action.is_some() && self.base.actions().iter().any(|action| {
            action.as_any().type_id() == self.scrolling_action.as_ref().unwrap().as_any().type_id()
        })
    }

    /// Gets the scrolling destination
    pub fn scrolling_destination(&self) -> Point2<f32> {
        if self.is_scrolling() {
            self.scrolling_to.unwrap()
        } else {
            Point2::new(self.base.scroll_x(), self.base.scroll_y())
        }
    }

    /// Scrolls to the specified position
    ///
    /// # Arguments
    ///
    /// * `x` - X position
    /// * `y` - Y position
    /// * `immediately` - Whether to scroll immediately
    ///
    /// # Returns
    ///
    /// `true` if scroll position got changed or started being changed, `false` if already centered there or already scrolling there
    pub fn scroll_to(&mut self, x: f32, y: f32, immediately: bool) -> bool {
        let destination = Point2::new(x, y);
        if self.scrolling_destination() == destination {
            return false;
        }

        // Remove the current scrolling action
        if let Some(action) = &self.scrolling_action {
            self.base.remove_action(action.as_any());
        }

        if immediately {
            self.base.set_scroll_x(x);
            self.base.set_scroll_y(y);
            self.base.update_visual_scroll();
        } else {
            self.scrolling_to = Some(destination);
            let action = Box::new(ScrollToAction {
                parent: Arc::new(self.clone()),
                original_scroll_x: self.base.scroll_x(),
                original_scroll_y: self.base.scroll_y(),
                duration: 0.4,
                current_time: 0.0,
                interpolation: Box::new(|t| (1.0 - (t * std::f32::consts::PI).cos()) / 2.0), // sine interpolation
            });
            self.base.add_action(action.as_any());
            self.scrolling_action = Some(action);
        }

        true
    }

    /// Gets the viewport
    ///
    /// # Arguments
    ///
    /// * `rect` - Rectangle to fill with viewport
    pub fn get_viewport(&self, rect: &mut Rect) {
        let viewport_from_left = self.base.scroll_x();
        // In the default coordinate system, the y origin is at the bottom, but scrollY is from the top, so we need to invert.
        let viewport_from_bottom = self.base.max_y() - self.base.scroll_y();
        rect.x = viewport_from_left - self.horizontal_padding();
        rect.y = viewport_from_bottom - self.vertical_padding();
        rect.w = self.base.width();
        rect.h = self.base.height();
    }

    /// Gets the viewport
    ///
    /// # Returns
    ///
    /// The viewport rectangle
    fn get_viewport_rect(&self) -> Rect {
        let mut rect = Rect::new(0.0, 0.0, 0.0, 0.0);
        self.get_viewport(&mut rect);
        rect
    }

    /// Called when viewport changes
    pub fn on_viewport_changed(&self) {
        if let Some(listener) = &self.on_viewport_changed_listener {
            listener(self.base.max_x(), self.base.max_y(), self.get_viewport_rect());
        }
    }

    /// Sets the on viewport changed listener
    pub fn set_on_viewport_changed_listener<F>(&mut self, listener: F)
    where
        F: Fn(f32, f32, Rect) + 'static,
    {
        self.on_viewport_changed_listener = Some(Box::new(listener));
    }

    /// Sets the on pan stop listener
    pub fn set_on_pan_stop_listener<F>(&mut self, listener: F)
    where
        F: Fn() + 'static,
    {
        self.on_pan_stop_listener = Some(Box::new(listener));
    }

    /// Sets the on pan start listener
    pub fn set_on_pan_start_listener<F>(&mut self, listener: F)
    where
        F: Fn() + 'static,
    {
        self.on_pan_start_listener = Some(Box::new(listener));
    }

    /// Sets the on zoom stop listener
    pub fn set_on_zoom_stop_listener<F>(&mut self, listener: F)
    where
        F: Fn() + 'static,
    {
        self.on_zoom_stop_listener = Some(Box::new(listener));
    }

    /// Sets the on zoom start listener
    pub fn set_on_zoom_start_listener<F>(&mut self, listener: F)
    where
        F: Fn() + 'static,
    {
        self.on_zoom_start_listener = Some(Box::new(listener));
    }

    /// Sets whether continuous scrolling is enabled in X direction
    pub fn set_continuous_scrolling_x(&mut self, continuous_scrolling_x: bool) {
        self.continuous_scrolling_x = continuous_scrolling_x;
    }

    /// Sets whether auto-scrolling is enabled
    pub fn set_auto_scroll_enabled(&mut self, is_auto_scroll_enabled: bool) {
        self.is_auto_scroll_enabled = is_auto_scroll_enabled;
    }

    /// Sets the map panning speed
    pub fn set_map_panning_speed(&mut self, map_panning_speed: f32) {
        this.map_panning_speed = map_panning_speed;
    }
}

impl Default for ZoomableScrollPane {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.5, 2.0)
    }
}

impl Clone for ZoomableScrollPane {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            extra_culling_x: self.extra_culling_x,
            extra_culling_y: self.extra_culling_y,
            min_zoom: self.min_zoom,
            max_zoom: self.max_zoom,
            continuous_scrolling_x: self.continuous_scrolling_x,
            on_viewport_changed_listener: None, // Can't clone closures
            on_pan_stop_listener: None,
            on_pan_start_listener: None,
            on_zoom_stop_listener: None,
            on_zoom_start_listener: None,
            zoom_listener: self.zoom_listener.clone(),
            is_auto_scroll_enabled: self.is_auto_scroll_enabled,
            map_panning_speed: self.map_panning_speed,
            scrolling_to: self.scrolling_to,
            scrolling_action: None, // Can't clone actions
        }
    }
}

impl std::ops::Deref for ZoomableScrollPane {
    type Target = ScrollPane;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ZoomableScrollPane {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Widget for ZoomableScrollPane {
    fn draw(&mut self, ctx: &mut Context, parent_alpha: f32) -> ggez::GameResult {
        if self.is_auto_scroll_enabled && !ctx.mouse.button_pressed(MouseButton::Left) {
            let pos_x = ctx.mouse.position().x;
            let pos_y = ctx.mouse.position().y; // Viewport coord: goes down, unlike world coordinates

            let delta_x = if pos_x <= 2.0 {
                1
            } else if pos_x >= ctx.gfx.window().inner_size().width as f32 - 2.0 {
                -1
            } else {
                0
            };

            let delta_y = if pos_y <= 6.0 {
                -1
            } else if pos_y >= ctx.gfx.window().inner_size().height as f32 - 6.0 {
                1
            } else {
                0
            };

            if delta_x != 0 || delta_y != 0 {
                // if Gdx deltaTime is > KeyboardPanningListener.deltaTime, then mouse auto scroll would be slower
                // (Gdx deltaTime is measured, not a constant, depends on framerate)
                // The extra factor is empirical to make mouse and WASD keyboard feel the same
                let relative_speed = ctx.time.delta().as_secs_f32() / KeyboardPanningListener::delta_time() * 0.3;
                self.do_key_or_mouse_panning(delta_x as f32 * relative_speed, delta_y as f32 * relative_speed);
            }
        }

        self.base.draw(ctx, parent_alpha)
    }

    fn update(&mut self, _ctx: &mut Context) -> ggez::GameResult {
        // Update actions
        let mut actions_to_remove = Vec::new();

        for (i, action) in self.base.actions().iter().enumerate() {
            if let Some(action) = action.as_any().downcast_ref::<dyn Action>() {
                if !action.update(ctx.time.delta().as_secs_f32()) {
                    actions_to_remove.push(i);
                }
            }
        }

        // Remove finished actions
        for i in actions_to_remove.iter().rev() {
            self.base.remove_action_at(*i);
        }

        Ok(())
    }
}

impl ZoomListener {
    /// Zooms out
    ///
    /// # Arguments
    ///
    /// * `zoom_multiplier` - Zoom multiplier
    pub fn zoom_out(&mut self, zoom_multiplier: f32) {
        if self.parent.base.scale_x() <= self.parent.min_zoom {
            if let Some(action) = &mut self.zoom_action {
                action.finish();
            }
            return;
        }

        if let Some(action) = &mut self.zoom_action {
            action.starting_zoom = action.current_zoom;
            action.finishing_zoom *= zoom_multiplier;
            action.restart();
        } else {
            let mut action = Box::new(ZoomAction {
                parent: self.parent.clone(),
                starting_zoom: self.parent.base.scale_x(),
                finishing_zoom: self.parent.base.scale_x() * zoom_multiplier,
                current_zoom: self.parent.base.scale_x(),
                duration: 0.3,
                current_time: 0.0,
                interpolation: Box::new(|t| {
                    // fastSlow interpolation
                    let t = t * 2.0;
                    if t <= 1.0 {
                        0.5 * t * t
                    } else {
                        let t = t - 1.0;
                        -0.5 * (t * (t - 2.0) - 1.0)
                    }
                }),
            });
            self.parent.base.add_action(action.as_any());
            self.zoom_action = Some(action);
        }
    }

    /// Zooms in
    ///
    /// # Arguments
    ///
    /// * `zoom_multiplier` - Zoom multiplier
    pub fn zoom_in(&mut self, zoom_multiplier: f32) {
        if self.parent.base.scale_x() >= self.parent.max_zoom {
            if let Some(action) = &mut self.zoom_action {
                action.finish();
            }
            return;
        }

        if let Some(action) = &mut self.zoom_action {
            action.starting_zoom = action.current_zoom;
            action.finishing_zoom /= zoom_multiplier;
            action.restart();
        } else {
            let mut action = Box::new(ZoomAction {
                parent: self.parent.clone(),
                starting_zoom: self.parent.base.scale_x(),
                finishing_zoom: self.parent.base.scale_x() / zoom_multiplier,
                current_zoom: self.parent.base.scale_x(),
                duration: 0.3,
                current_time: 0.0,
                interpolation: Box::new(|t| {
                    // fastSlow interpolation
                    let t = t * 2.0;
                    if t <= 1.0 {
                        0.5 * t * t
                    } else {
                        let t = t - 1.0;
                        -0.5 * (t * (t - 2.0) - 1.0)
                    }
                }),
            });
            self.parent.base.add_action(action.as_any());
            self.zoom_action = Some(action);
        }
    }
}

impl Clone for ZoomListener {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            is_zooming: self.is_zooming,
            zoom_action: None, // Can't clone actions
        }
    }
}

impl ActorGestureListener for ZoomListener {
    fn pinch(&mut self, translation: Vector2<f32>, scale_change: f32) {
        if !self.is_zooming {
            self.is_zooming = true;
            if let Some(listener) = &self.parent.on_zoom_start_listener {
                listener();
            }
        }

        self.parent.scroll_to(
            self.parent.base.scroll_x() - translation.x / self.parent.base.scale_x(),
            self.parent.base.scroll_y() + translation.y / self.parent.base.scale_y(),
            true,
        );

        self.parent.zoom(self.parent.base.scale_x() * scale_change);
    }

    fn pinch_stop(&mut self) {
        self.is_zooming = false;
        if let Some(listener) = &self.parent.on_zoom_stop_listener {
            listener();
        }
    }

    fn scrolled(&mut self, amount_x: f32, amount_y: f32) -> bool {
        if amount_x > 0.0 || amount_y > 0.0 {
            self.zoom_out(0.82);
        } else {
            self.zoom_in(0.82);
        }
        true
    }
}

impl Action for ZoomAction {
    fn update(&mut self, dt: f32) -> bool {
        self.current_time += dt;
        let percent = (self.current_time / self.duration).min(1.0);

        if percent >= 1.0 {
            self.parent.zoom(self.finishing_zoom);
            return false;
        }

        let interpolated_percent = (self.interpolation)(percent);
        self.current_zoom = self.starting_zoom + (self.finishing_zoom - self.starting_zoom) * interpolated_percent;
        self.parent.zoom(self.current_zoom);

        true
    }
}

impl ZoomAction {
    /// Finishes the action
    fn finish(&mut self) {
        self.current_time = self.duration;
    }

    /// Restarts the action
    fn restart(&mut self) {
        self.current_time = 0.0;
    }
}

impl ActorGestureListener for FlickScrollListener {
    fn pan(&mut self, _event: &MouseInput, x: f32, y: f32, delta_x: f32, delta_y: f32) {
        if !self.is_panning {
            self.is_panning = true;
            if let Some(listener) = &self.parent.on_pan_start_listener {
                listener();
            }
        }

        self.parent.base.set_scrollbars_visible(true);
        self.parent.base.set_scroll_x(self.parent.restrict_x(delta_x));
        self.parent.base.set_scroll_y(self.parent.restrict_y(delta_y));

        // clamp() call is missing here but it doesn't seem to make any big difference in this case
    }

    fn pan_stop(&mut self, _event: Option<&MouseInput>, _x: f32, _y: f32, _pointer: i32, _button: MouseButton) {
        if self.parent.zoom_listener.is_zooming {
            self.parent.zoom_listener.is_zooming = false;
        }

        self.is_panning = false;
        if let Some(listener) = &self.parent.on_pan_stop_listener {
            listener();
        }
    }
}

impl Action for ScrollToAction {
    fn update(&mut self, dt: f32) -> bool {
        self.current_time += dt;
        let percent = (self.current_time / self.duration).min(1.0);

        if percent >= 1.0 {
            self.parent.base.set_scroll_x(self.parent.scrolling_to.unwrap().x);
            self.parent.base.set_scroll_y(self.parent.scrolling_to.unwrap().y);
            self.parent.base.update_visual_scroll();
            return false;
        }

        let interpolated_percent = (self.interpolation)(percent);
        self.parent.base.set_scroll_x(
            self.parent.scrolling_to.unwrap().x * interpolated_percent + self.original_scroll_x * (1.0 - interpolated_percent)
        );
        self.parent.base.set_scroll_y(
            self.parent.scrolling_to.unwrap().y * interpolated_percent + self.original_scroll_y * (1.0 - interpolated_percent)
        );
        self.parent.base.update_visual_scroll();

        true
    }
}

/// A trait for cullable objects
pub trait Cullable {
    /// Sets the culling area
    fn set_culling_area(&mut self, area: Rect);
}

/// A trait for actor gesture listeners
pub trait ActorGestureListener {
    /// Called when a pinch gesture is detected
    fn pinch(&mut self, translation: Vector2<f32>, scale_change: f32);

    /// Called when a pinch gesture stops
    fn pinch_stop(&mut self);

    /// Called when a scroll gesture is detected
    fn scrolled(&mut self, amount_x: f32, amount_y: f32) -> bool;

    /// Called when a pan gesture is detected
    fn pan(&mut self, event: &MouseInput, x: f32, y: f32, delta_x: f32, delta_y: f32);

    /// Called when a pan gesture stops
    fn pan_stop(&mut self, event: Option<&MouseInput>, x: f32, y: f32, pointer: i32, button: MouseButton);
}