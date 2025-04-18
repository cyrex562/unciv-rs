use ggez::graphics::{self, Color, DrawParam, Drawable, Mesh, Rect, Text};
use ggez::{Context, GameResult};
use ggez::event::{MouseButton, MouseInput};
use ggez::input::mouse::MousePosition;
use ggez::mint::Vector2;
use std::time::{Duration, Instant};
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::components::input::{KeyCharAndCode, KeyboardBinding, KeyboardBindings};
use crate::models::translations::tr;

/// Duration of the fade/zoom-in/out animations in seconds
const TIP_ANIMATION_DURATION: f32 = 0.2;

/// Represents the different states a tooltip can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipState {
    /// Tooltip is not visible
    Hidden,
    /// Tooltip is currently animating to become visible
    Showing,
    /// Tooltip is fully visible
    Shown,
    /// Tooltip is currently animating to become hidden
    Hiding,
}

/// A replacement for the standard tooltip, with placement that doesn't follow the mouse
///
/// This tooltip can be attached to any actor and will display when the mouse enters the actor.
/// It supports animations for showing and hiding, and can be positioned relative to the target actor.
pub struct UncivTooltip<T: Drawable> {
    /// The target actor the tooltip is attached to
    target: Box<dyn Drawable>,

    /// The content to display in the tooltip
    content: T,

    /// The alignment point on the target actor
    target_align: Alignment,

    /// The alignment point on the tooltip
    tip_align: Alignment,

    /// Additional offset for tooltip position after alignment
    offset: Vector2<f32>,

    /// Whether to use animations when showing/hiding
    animate: bool,

    /// The current state of the tooltip
    state: TipState,

    /// The width of the content
    content_width: f32,

    /// The height of the content
    content_height: f32,

    /// Whether a touch down event has been seen (for Android compatibility)
    touch_down_seen: bool,

    /// Animation start time
    animation_start: Option<Instant>,

    /// Animation end state
    animation_end_state: Option<TipState>,

    /// Optional function to refresh content before showing
    content_refresher: Option<Box<dyn Fn() -> Option<Vector2<f32>>>>,
}

/// Represents alignment points for positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Top-left alignment
    TopLeft,
    /// Top-center alignment
    TopCenter,
    /// Top-right alignment
    TopRight,
    /// Center-left alignment
    CenterLeft,
    /// Center alignment
    Center,
    /// Center-right alignment
    CenterRight,
    /// Bottom-left alignment
    BottomLeft,
    /// Bottom-center alignment
    BottomCenter,
    /// Bottom-right alignment
    BottomRight,
}

impl<T: Drawable> UncivTooltip<T> {
    /// Creates a new tooltip
    ///
    /// # Arguments
    ///
    /// * `target` - The target actor the tooltip is attached to
    /// * `content` - The content to display in the tooltip
    /// * `target_align` - The alignment point on the target actor
    /// * `tip_align` - The alignment point on the tooltip
    /// * `offset` - Additional offset for tooltip position after alignment
    /// * `animate` - Whether to use animations when showing/hiding
    /// * `force_content_size` - Force virtual content width/height for alignment calculation
    /// * `content_refresher` - Function called just before showing the content to update it
    pub fn new(
        target: Box<dyn Drawable>,
        content: T,
        target_align: Alignment,
        tip_align: Alignment,
        offset: Vector2<f32>,
        animate: bool,
        force_content_size: Option<Vector2<f32>>,
        content_refresher: Option<Box<dyn Fn() -> Option<Vector2<f32>>>>,
    ) -> Self {
        // Get content dimensions
        let content_width = force_content_size.map_or_else(
            || content.bounds().w,
            |size| size.x
        );
        let content_height = force_content_size.map_or_else(
            || content.bounds().h,
            |size| size.y
        );

        Self {
            target,
            content,
            target_align,
            tip_align,
            offset,
            animate,
            state: TipState::Hidden,
            content_width,
            content_height,
            touch_down_seen: false,
            animation_start: None,
            animation_end_state: None,
            content_refresher,
        }
    }

    /// Shows the tooltip immediately or begins the animation
    pub fn show(&mut self, immediate: bool) {
        // If target is not in a stage, don't show
        if !self.target.has_parent() {
            return;
        }

        let use_animation = self.animate && !immediate;

        // Don't show if already shown or showing
        if self.state == TipState::Shown || (self.state == TipState::Showing && use_animation) {
            return;
        }

        // If currently animating, stop and reset
        if self.state == TipState::Showing || self.state == TipState::Hiding {
            self.animation_start = None;
            self.animation_end_state = None;
            self.state = TipState::Hidden;
        }

        // Refresh content if needed
        if let Some(refresher) = &self.content_refresher {
            if let Some(size) = refresher() {
                self.content_width = size.x;
                self.content_height = size.y;
            }
        }

        // Position the tooltip
        let target_pos = self.get_target_position();
        let origin = self.get_origin();

        // Set position
        self.content.set_position(
            target_pos.x - origin.x + self.offset.x,
            target_pos.y - origin.y + self.offset.y
        );

        // Set initial state for animation
        if use_animation {
            self.content.set_alpha(0.1);
            self.content.set_scale(0.1);
            self.animation_start = Some(Instant::now());
            self.animation_end_state = Some(TipState::Shown);
            self.state = TipState::Showing;
        } else {
            self.content.set_alpha(1.0);
            self.content.set_scale(1.0);
            self.state = TipState::Shown;
        }
    }

    /// Hides the tooltip immediately or begins the animation
    pub fn hide(&mut self, immediate: bool) {
        let use_animation = self.animate && !immediate;

        // Don't hide if already hidden or hiding
        if self.state == TipState::Hidden || (self.state == TipState::Hiding && use_animation) {
            return;
        }

        // If currently animating, stop and reset
        if self.state == TipState::Showing || self.state == TipState::Hiding {
            self.animation_start = None;
            self.animation_end_state = None;
            self.state = TipState::Shown;
        }

        if use_animation {
            self.animation_start = Some(Instant::now());
            self.animation_end_state = Some(TipState::Hidden);
            self.state = TipState::Hiding;
        } else {
            self.state = TipState::Hidden;
        }
    }

    /// Gets the target position based on alignment
    fn get_target_position(&self) -> Vector2<f32> {
        let target_bounds = self.target.bounds();
        let edge_point = self.get_edge_point(target_bounds, self.target_align);

        Vector2 {
            x: edge_point.x,
            y: edge_point.y,
        }
    }

    /// Gets the origin point based on alignment
    fn get_origin(&self) -> Vector2<f32> {
        let origin_x = match self.tip_align {
            Alignment::TopLeft | Alignment::CenterLeft | Alignment::BottomLeft => 0.0,
            Alignment::TopRight | Alignment::CenterRight | Alignment::BottomRight => self.content_width,
            _ => self.content_width / 2.0,
        };

        let origin_y = match self.tip_align {
            Alignment::BottomLeft | Alignment::BottomCenter | Alignment::BottomRight => 0.0,
            Alignment::TopLeft | Alignment::TopCenter | Alignment::TopRight => self.content_height,
            _ => self.content_height / 2.0,
        };

        Vector2 {
            x: origin_x,
            y: origin_y,
        }
    }

    /// Gets the edge point of a rectangle based on alignment
    fn get_edge_point(&self, bounds: Rect, align: Alignment) -> Vector2<f32> {
        let origin_x = match align {
            Alignment::TopLeft | Alignment::CenterLeft | Alignment::BottomLeft => 0.0,
            Alignment::TopRight | Alignment::CenterRight | Alignment::BottomRight => bounds.w,
            _ => bounds.w / 2.0,
        };

        let origin_y = match align {
            Alignment::BottomLeft | Alignment::BottomCenter | Alignment::BottomRight => 0.0,
            Alignment::TopLeft | Alignment::TopCenter | Alignment::TopRight => bounds.h,
            _ => bounds.h / 2.0,
        };

        Vector2 {
            x: bounds.x + origin_x,
            y: bounds.y + origin_y,
        }
    }

    /// Starts the show/hide animation
    fn start_animation(&mut self, end_state: TipState) {
        self.animation_start = Some(Instant::now());
        self.animation_end_state = Some(end_state);

        match end_state {
            TipState::Shown => {
                self.state = TipState::Showing;
            },
            TipState::Hidden => {
                self.state = TipState::Hiding;
            },
            _ => {},
        }
    }

    /// Updates the animation
    fn update_animation(&mut self, delta: f32) {
        if let (Some(start), Some(end_state)) = (self.animation_start, self.animation_end_state) {
            let elapsed = start.elapsed();
            let duration = Duration::from_secs_f32(TIP_ANIMATION_DURATION);

            if elapsed >= duration {
                // Animation complete
                self.state = end_state;
                self.animation_start = None;
                self.animation_end_state = None;

                if end_state == TipState::Hidden {
                    // Remove from parent if hidden
                    self.content.remove_from_parent();
                }
            } else {
                // Update animation
                let percent = elapsed.as_secs_f32() / TIP_ANIMATION_DURATION;

                match end_state {
                    TipState::Shown => {
                        // Fade in and scale up
                        let value = percent * 0.9 + 0.1;
                        self.content.set_alpha(value);
                        self.content.set_scale(value);
                    },
                    TipState::Hidden => {
                        // Fade out and scale down
                        let value = 1.0 - percent * 0.9;
                        self.content.set_alpha(value);
                        self.content.set_scale(value);
                    },
                    _ => {},
                }
            }
        }
    }

    /// Handles mouse enter event
    pub fn on_mouse_enter(&mut self, from_actor: Option<&dyn Drawable>) {
        // Don't show if coming from a descendant of the target
        if let Some(from) = from_actor {
            if from.is_descendant_of(&*self.target) {
                return;
            }
        }

        self.show(false);
    }

    /// Handles mouse exit event
    pub fn on_mouse_exit(&mut self, to_actor: Option<&dyn Drawable>) {
        // Don't hide if going to a descendant of the target
        if let Some(to) = to_actor {
            if to.is_descendant_of(&*self.target) && !self.touch_down_seen {
                return;
            }
        }

        self.touch_down_seen = false;
        self.hide(false);
    }

    /// Handles mouse down event
    pub fn on_mouse_down(&mut self, _button: MouseButton, _position: MousePosition) -> bool {
        self.touch_down_seen = true;
        self.content.to_front();
        false
    }

    /// Updates the tooltip
    pub fn update(&mut self, _ctx: &mut Context, delta: f32) -> GameResult {
        // Update animation if needed
        self.update_animation(delta);

        Ok(())
    }

    /// Draws the tooltip
    pub fn draw(&self, ctx: &mut Context) -> GameResult {
        // Only draw if visible or animating
        if self.state == TipState::Hidden {
            return Ok(());
        }

        // Draw the content
        graphics::draw(ctx, &self.content, DrawParam::default())?;

        Ok(())
    }
}

/// Extension trait for adding tooltips to actors
pub trait TooltipExt {
    /// Adds a tooltip with text to an actor
    fn add_tooltip(
        &mut self,
        text: &str,
        size: f32,
        always: bool,
        target_align: Alignment,
        tip_align: Alignment,
        hide_icons: bool,
        dynamic_text_provider: Option<Box<dyn Fn() -> String>>,
    );

    /// Adds a tooltip with a single character to an actor
    fn add_char_tooltip(&mut self, char: char, size: f32);

    /// Adds a tooltip with a key to an actor
    fn add_key_tooltip(&mut self, key: KeyCharAndCode, size: f32);

    /// Adds a tooltip with a keyboard binding to an actor
    fn add_binding_tooltip(&mut self, binding: KeyboardBinding, size: f32);
}

impl TooltipExt for Box<dyn Drawable> {
    fn add_tooltip(
        &mut self,
        text: &str,
        size: f32,
        always: bool,
        target_align: Alignment,
        tip_align: Alignment,
        hide_icons: bool,
        dynamic_text_provider: Option<Box<dyn Fn() -> String>>,
    ) {
        // Remove any existing tooltips
        self.remove_tooltips();

        // Don't add tooltip if text is empty
        if text.is_empty() {
            return;
        }

        // Create the label
        let label_color = BaseScreen::get_skin_color();
        let label = if hide_icons {
            // Create a simple label without icons
            let mut text = Text::new(text);
            text.set_color(label_color);
            text.set_font_size(38);
            text
        } else {
            // Create a color markup label
            ColorMarkupLabel::new(text, label_color, 38)
        };

        // Create the background
        let background = BaseScreen::get_ui_background(
            "General/Tooltip",
            "roundedEdgeRectangleShape",
            Color::LIGHT_GRAY
        );

        // Set padding based on text content
        let skew_pad_descenders = if text.contains(|c| ",;gjpqy".contains(c)) { 0.0 } else { 2.5 };
        let horizontal_pad = if text.len() > 1 { 10.0 } else { 6.0 };

        // Create a container for the label and background
        let mut container = Box::new(Container::new(label));
        container.set_background(background);
        container.set_padding(4.0 + skew_pad_descenders, horizontal_pad, 8.0 - skew_pad_descenders, horizontal_pad);

        // Calculate size based on text
        let multi_row_size = size * (1.0 + text.matches('\n').count() as f32);
        let width_height_ratio = {
            container.pack();
            container.set_scale(1.0);
            let ratio = container.bounds().w / container.bounds().h;
            container.set_scale(multi_row_size / container.bounds().h);
            ratio
        };

        let content_size = Vector2 {
            x: multi_row_size * width_height_ratio,
            y: multi_row_size,
        };

        // Create content refresher if needed
        let content_refresher = dynamic_text_provider.map(|provider| {
            Box::new(move || {
                let new_text = provider();
                // Update label text
                // This is simplified - in a real implementation, you would need to
                // update the label text and recalculate the size
                Some(content_size)
            }) as Box<dyn Fn() -> Option<Vector2<f32>>>
        });

        // Create the tooltip
        let tooltip = UncivTooltip::new(
            self.clone(),
            container,
            target_align,
            tip_align,
            Vector2 {
                x: -multi_row_size / 4.0,
                y: size / 4.0,
            },
            true,
            Some(content_size),
            content_refresher,
        );

        // Add the tooltip to the target
        self.add_tooltip(tooltip);
    }

    fn add_char_tooltip(&mut self, char: char, size: f32) {
        let char_str = if char == 'i' || char == 'I' {
            "i"
        } else {
            &char.to_uppercase().to_string()
        };

        self.add_tooltip(
            char_str,
            size,
            false,
            Alignment::TopRight,
            Alignment::Top,
            false,
            None,
        );
    }

    fn add_key_tooltip(&mut self, key: KeyCharAndCode, size: f32) {
        if key != KeyCharAndCode::UNKNOWN {
            self.add_tooltip(
                &key.to_string(),
                size,
                false,
                Alignment::TopRight,
                Alignment::Top,
                false,
                None,
            );
        }
    }

    fn add_binding_tooltip(&mut self, binding: KeyboardBinding, size: f32) {
        let get_text = || KeyboardBindings::get(binding).to_string();

        self.add_tooltip(
            &get_text(),
            size,
            false,
            Alignment::TopRight,
            Alignment::Top,
            false,
            Some(Box::new(get_text)),
        );
    }
}

/// A simple container for UI elements
pub struct Container<T: Drawable> {
    content: T,
    background: Option<Mesh>,
    padding: (f32, f32, f32, f32), // top, right, bottom, left
    scale: f32,
    alpha: f32,
    position: Vector2<f32>,
}

impl<T: Drawable> Container<T> {
    /// Creates a new container
    pub fn new(content: T) -> Self {
        Self {
            content,
            background: None,
            padding: (0.0, 0.0, 0.0, 0.0),
            scale: 1.0,
            alpha: 1.0,
            position: Vector2 { x: 0.0, y: 0.0 },
        }
    }

    /// Sets the background of the container
    pub fn set_background(&mut self, background: Mesh) {
        self.background = Some(background);
    }

    /// Sets the padding of the container
    pub fn set_padding(&mut self, top: f32, right: f32, bottom: f32, left: f32) {
        self.padding = (top, right, bottom, left);
    }

    /// Packs the container to fit its content
    pub fn pack(&mut self) {
        // In a real implementation, this would adjust the container size
        // to fit its content plus padding
    }

    /// Sets the scale of the container
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Sets the position of the container
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = Vector2 { x, y };
    }

    /// Gets the bounds of the container
    pub fn bounds(&self) -> Rect {
        // In a real implementation, this would return the actual bounds
        // For now, we'll return a dummy value
        Rect::new(0.0, 0.0, 100.0, 100.0)
    }

    /// Brings the container to the front
    pub fn to_front(&mut self) {
        // In a real implementation, this would bring the container to the front
    }

    /// Removes the container from its parent
    pub fn remove_from_parent(&mut self) {
        // In a real implementation, this would remove the container from its parent
    }
}

impl<T: Drawable> Drawable for Container<T> {
    fn bounds(&self) -> Rect {
        self.bounds()
    }

    fn draw(&self, ctx: &mut Context) -> GameResult {
        // Draw background if present
        if let Some(background) = &self.background {
            graphics::draw(ctx, background, DrawParam::default()
                .dest(self.position)
                .scale([self.scale, self.scale])
                .color(Color::new(1.0, 1.0, 1.0, self.alpha))
            )?;
        }

        // Draw content
        graphics::draw(ctx, &self.content, DrawParam::default()
            .dest(self.position)
            .scale([self.scale, self.scale])
            .color(Color::new(1.0, 1.0, 1.0, self.alpha))
        )?;

        Ok(())
    }

    fn has_parent(&self) -> bool {
        // In a real implementation, this would check if the container has a parent
        true
    }

    fn is_descendant_of(&self, _actor: &dyn Drawable) -> bool {
        // In a real implementation, this would check if the container is a descendant of the actor
        false
    }

    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    fn remove_from_parent(&mut self) {
        self.remove_from_parent();
    }
}

/// Trait for drawable objects
pub trait Drawable {
    /// Gets the bounds of the drawable
    fn bounds(&self) -> Rect;

    /// Draws the drawable
    fn draw(&self, ctx: &mut Context) -> GameResult;

    /// Checks if the drawable has a parent
    fn has_parent(&self) -> bool;

    /// Checks if the drawable is a descendant of another drawable
    fn is_descendant_of(&self, actor: &dyn Drawable) -> bool;

    /// Sets the alpha of the drawable
    fn set_alpha(&mut self, alpha: f32);

    /// Sets the scale of the drawable
    fn set_scale(&mut self, scale: f32);

    /// Removes the drawable from its parent
    fn remove_from_parent(&mut self);

    /// Sets the position of the drawable
    fn set_position(&mut self, x: f32, y: f32);

    /// Brings the drawable to the front
    fn to_front(&mut self);
}

/// Extension trait for removing tooltips from actors
pub trait RemoveTooltipsExt {
    /// Removes all tooltips from an actor
    fn remove_tooltips(&mut self);

    /// Adds a tooltip to an actor
    fn add_tooltip<T: Drawable>(&mut self, tooltip: UncivTooltip<T>);
}

impl RemoveTooltipsExt for Box<dyn Drawable> {
    fn remove_tooltips(&mut self) {
        // In a real implementation, this would remove all tooltips from the actor
    }

    fn add_tooltip<T: Drawable>(&mut self, _tooltip: UncivTooltip<T>) {
        // In a real implementation, this would add the tooltip to the actor
    }
}