use std::time::{Duration, Instant};
use ggez::graphics::{Color, DrawParam, Image, Mesh, Rect};
use ggez::mint::Point2;
use std::sync::Arc;

use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::UncivGame;

/// Style configuration for the LoadingImage
#[derive(Clone)]
pub struct LoadingImageStyle {
    /// If not CLEAR, a Circle with this Color is layered at the bottom and the icons are resized to inner_size_factor * size
    pub circle_color: Color,
    /// Color for the animated "Loading" icon (drawn on top)
    pub loading_color: Color,
    /// If not CLEAR, another icon is layered between circle and loading, e.g. symbolizing 'idle' or 'done'
    pub idle_icon_color: Color,
    /// Minimum shown time in ms including fades
    pub min_show_time: i32,
    /// Texture name for the circle
    pub circle_image_name: String,
    /// Texture name for the idle icon
    pub idle_image_name: String,
    /// Texture name for the loading icon
    pub loading_image_name: String,
    /// Size scale for icons when a Circle is used
    pub inner_size_factor: f32,
    /// Duration of fade-in and fade-out in seconds
    pub fade_duration: f32,
    /// Duration of rotation - seconds per revolution
    pub rotation_duration: f32,
    /// While loading is shown, the idle icon is semitransparent
    pub idle_icon_hidden_alpha: f32,
}

impl Default for LoadingImageStyle {
    fn default() -> Self {
        Self {
            circle_color: Color::CLEAR,
            loading_color: Color::WHITE,
            idle_icon_color: Color::CLEAR,
            min_show_time: 0,
            circle_image_name: ImageGetter::circle_location(),
            idle_image_name: ImageGetter::white_dot_location(),
            loading_image_name: "OtherIcons/Loading".to_string(),
            inner_size_factor: 0.75,
            fade_duration: 0.2,
            rotation_duration: 4.0,
            idle_icon_hidden_alpha: 0.4,
        }
    }
}

/// Animated "double arrow" loading icon.
///
/// * By default, shows an empty transparent square, or a circle and/or an "idle" icon.
/// * When show() is called, the double-arrow loading icon is faded in and rotates.
/// * When hide() is called, the double-arrow fades out.
/// * When style.min_show_time is set, hide() will make sure the "busy status" can be seen even if it was very short.
/// * When GameSettings.continuous_rendering is off, fade and rotation animations are disabled.
/// * animated is public and can be used to override the 'continuous_rendering' setting.
pub struct LoadingImage {
    /// Fixed size for the component
    size: f32,
    /// Style configuration
    style: LoadingImageStyle,
    /// Whether animations are enabled
    pub animated: bool,
    /// When the loading started
    loading_started: Option<Instant>,
    /// The circle background image (if any)
    circle: Option<Image>,
    /// The idle icon image (if any)
    idle_icon: Option<Image>,
    /// The loading icon image
    loading_icon: Image,
    /// Current rotation angle in degrees
    rotation: f32,
    /// Current alpha value for the loading icon
    loading_alpha: f32,
    /// Current alpha value for the idle icon
    idle_alpha: f32,
    /// Whether the loading icon is visible
    is_visible: bool,
}

impl LoadingImage {
    /// Creates a new LoadingImage with the given size and style
    pub fn new(size: f32, style: LoadingImageStyle) -> Self {
        let base_screen = BaseScreen::get_instance();
        let animated = UncivGame::current().settings().continuous_rendering;

        let inner_size = if style.circle_color == Color::CLEAR {
            size
        } else {
            size * style.inner_size_factor
        };

        // Create circle if needed
        let circle = if style.circle_color != Color::CLEAR {
            let mut circle = ImageGetter::get_image(&style.circle_image_name);
            circle.set_color(style.circle_color);
            circle.set_size(size);
            Some(circle)
        } else {
            None
        };

        // Create idle icon if needed
        let idle_icon = if style.idle_icon_color != Color::CLEAR {
            let mut idle = ImageGetter::get_image(&style.idle_image_name);
            idle.set_color(style.idle_icon_color);
            idle.set_size(inner_size);
            Some(idle)
        } else {
            None
        };

        // Create loading icon
        let mut loading_icon = ImageGetter::get_image(&style.loading_image_name);
        loading_icon.set_color(style.loading_color);
        loading_icon.set_size(inner_size);
        loading_icon.set_origin(Point2 { x: 0.5, y: 0.5 });

        Self {
            size,
            style,
            animated,
            loading_started: None,
            circle,
            idle_icon,
            loading_icon,
            rotation: 0.0,
            loading_alpha: 0.0,
            idle_alpha: 1.0,
            is_visible: false,
        }
    }

    /// Shows the loading animation
    pub fn show(&mut self) {
        self.loading_started = Some(Instant::now());
        self.is_visible = true;

        if self.animated {
            self.loading_alpha = 0.0;
            self.rotation = 0.0;
        } else {
            self.loading_alpha = 1.0;
            if let Some(idle) = &mut self.idle_icon {
                idle.set_alpha(self.style.idle_icon_hidden_alpha);
            }
        }
    }

    /// Hides the loading animation
    pub fn hide<F: FnOnce() + Send + Sync + 'static>(&mut self, on_complete: Option<F>) {
        if self.animated {
            self.hide_animated(on_complete);
        } else {
            self.hide_delayed(on_complete);
        }
    }

    /// Returns whether the loading animation is currently showing
    pub fn is_showing(&self) -> bool {
        self.is_visible
    }

    /// Updates the animation state
    pub fn update(&mut self, delta: f32) {
        if !self.is_visible || !self.animated {
            return;
        }

        // Update fade
        if self.loading_alpha < 1.0 {
            self.loading_alpha = (self.loading_alpha + delta / self.style.fade_duration).min(1.0);
            if let Some(idle) = &mut self.idle_icon {
                idle.set_alpha(
                    (1.0 - self.loading_alpha) * (1.0 - self.style.idle_icon_hidden_alpha)
                        + self.style.idle_icon_hidden_alpha,
                );
            }
        }

        // Update rotation
        self.rotation = (self.rotation + 360.0 * delta / self.style.rotation_duration) % 360.0;
    }

    fn hide_animated<F: FnOnce() + Send + Sync + 'static>(&mut self, on_complete: Option<F>) {
        let wait_duration = self.get_wait_duration();
        if wait_duration == 0.0 {
            self.set_hidden();
            if let Some(callback) = on_complete {
                callback();
            }
            return;
        }

        // Start fade out
        self.loading_alpha = 1.0;
        let fade_duration = self.style.fade_duration;
        let on_complete = on_complete.clone();

        // Schedule the hide after the wait duration
        let hide_time = Instant::now() + Duration::from_secs_f32(wait_duration);
        self.loading_started = Some(hide_time);

        // The actual hiding will be done in update() when the time comes
    }

    fn hide_delayed<F: FnOnce() + Send + Sync + 'static>(&mut self, on_complete: Option<F>) {
        let wait_duration = self.get_wait_duration();
        if wait_duration == 0.0 {
            self.set_hidden();
            if let Some(callback) = on_complete {
                callback();
            }
            return;
        }

        // Schedule the hide after the wait duration
        let hide_time = Instant::now() + Duration::from_secs_f32(wait_duration);
        self.loading_started = Some(hide_time);
    }

    fn set_hidden(&mut self) {
        self.is_visible = false;
        self.loading_alpha = 0.0;
        if let Some(idle) = &mut self.idle_icon {
            idle.set_alpha(1.0);
        }
    }

    fn get_wait_duration(&self) -> f32 {
        if let Some(started) = self.loading_started {
            let elapsed = started.elapsed().as_millis() as i32;
            if elapsed >= self.style.min_show_time {
                return 0.0;
            }
            return (self.style.min_show_time - elapsed) as f32 * 0.001;
        }
        0.0
    }
}

impl Widget for LoadingImage {
    fn draw(&mut self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        // Draw circle background if present
        if let Some(circle) = &self.circle {
            circle.draw(ctx, param)?;
        }

        // Draw idle icon if present
        if let Some(idle) = &self.idle_icon {
            idle.draw(ctx, param)?;
        }

        // Draw loading icon if visible
        if self.is_visible {
            let mut loading_param = param;
            loading_param.color.a = self.loading_alpha;
            loading_param.rotation = self.rotation.to_radians();
            self.loading_icon.draw(ctx, loading_param)?;
        }

        Ok(())
    }

    fn update(&mut self, _ctx: &mut ggez::Context, delta: f32) -> ggez::GameResult {
        self.update(delta);
        Ok(())
    }

    fn get_pref_width(&self) -> f32 {
        self.size
    }

    fn get_pref_height(&self) -> f32 {
        self.size
    }

    fn get_max_width(&self) -> f32 {
        self.size
    }

    fn get_max_height(&self) -> f32 {
        self.size
    }
}

impl Clone for LoadingImage {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            style: self.style.clone(),
            animated: self.animated,
            loading_started: None,
            circle: self.circle.clone(),
            idle_icon: self.idle_icon.clone(),
            loading_icon: self.loading_icon.clone(),
            rotation: 0.0,
            loading_alpha: 0.0,
            idle_alpha: 1.0,
            is_visible: false,
        }
    }
}