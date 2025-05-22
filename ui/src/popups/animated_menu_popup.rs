use std::sync::Arc;
use ggez::graphics::{Color, DrawParam, Mesh, MeshBatch, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};
use ggez::mint::Vector2;

use crate::ui::components::input::{KeyCharAndCode, KeyboardBinding};
use crate::ui::components::widgets::{Button, Container, Table};
use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::concurrency::Concurrency;

/// A popup menu that animates on open/close, centered on a given Position (unlike other Popups which are always stage-centered).
///
/// You must provide content by overriding create_content_table - see its doc.
///
/// The Popup opens automatically once created.
/// **Meant to be used for small menus.** - otherwise use ScrollableAnimatedMenuPopup.
/// No default close button - recommended to simply use "click-behind".
///
/// The "click-behind" semi-transparent covering of the rest of the stage is much darker than a normal
/// Popup (give the impression to take away illumination and spotlight the menu) and fades in together
/// with the AnimatedMenuPopup itself. Closing the menu in any of the four ways will fade out everything
/// inverting the fade-and-scale-in. Callbacks registered with Popup.close_listeners will run before the animation starts.
/// Use after_close_callback instead if you need a notification after the animation finishes and the Popup is cleaned up.
pub struct AnimatedMenuPopup {
    popup: Popup,
    container: Container<Table>,
    animation_duration: f32,
    background_color: Color,
    small_button_style: Arc<BaseScreen>,
    after_close_callback: Option<Box<dyn FnOnce()>>,
    any_button_was_clicked: bool,
}

impl AnimatedMenuPopup {
    /// Creates a new AnimatedMenuPopup
    pub fn new(stage: &BaseScreen, position: Vector2<f32>) -> Self {
        let mut popup = Popup::new(stage, false);
        popup.set_click_behind_to_close(true);
        popup.add_key_shortcut(KeyCharAndCode::BACK, Box::new(move || {
            popup.close();
        }));

        // Remove the inner table as we'll create our own
        popup.remove_inner_table();

        let container = Container::new();
        let animation_duration = 0.33;
        let background_color = Color::new(0.0, 0.0, 0.0, 0.0); // Will be set later
        let small_button_style = Arc::new(BaseScreen::new());

        let mut animated_popup = Self {
            popup,
            container,
            animation_duration,
            background_color,
            small_button_style,
            after_close_callback: None,
            any_button_was_clicked: false,
        };

        // Decouple the content creation from object initialization so it can access its own fields
        Concurrency::run_on_gl_thread(move || {
            animated_popup.create_and_show(position);
        });

        animated_popup
    }

    /// Get stage coords of an actor's right edge center, to help position an AnimatedMenuPopup.
    /// Note the Popup will center over this point.
    pub fn get_actor_top_right(actor: &BaseScreen) -> Vector2<f32> {
        let width = actor.width();
        let height = actor.height();
        Vector2 { x: width, y: height / 2.0 }
    }

    /// Provides the Popup content.
    ///
    /// Call super to fetch an empty default with prepared padding and background.
    /// You can use get_button, which produces TextButtons slightly smaller than Unciv's default ones.
    /// The content adding functions offered by Popup or Table won't work.
    /// The content needs to be complete when the method finishes, it will be packed and measured immediately.
    ///
    /// Return None to abort the menu creation - nothing will be shown and the instance should be discarded.
    /// Useful if you need full context first to determine if any entry makes sense.
    pub fn create_content_table(&self) -> Option<Table> {
        let mut table = Table::new();
        table.defaults().pad(5.0, 15.0, 5.0, 15.0).grow_x();
        table.set_background(BaseScreen::get_ui_background("General/AnimatedMenu", "roundedEdgeRectangleShape", Color::DARK_GRAY));
        Some(table)
    }

    /// Creates and shows the popup at the given position
    fn create_and_show(&mut self, position: Vector2<f32>) {
        let new_inner_table = match self.create_content_table() {
            Some(table) => table,
            None => return, // Special case - we don't want the context menu after all
        };

        new_inner_table.pack();
        self.container.set_actor(new_inner_table.clone());
        self.container.set_touchable(true);
        self.container.set_transform(true);
        self.container.set_scale(0.05);
        self.container.set_color(Color::new(1.0, 1.0, 1.0, 0.0));

        self.popup.open(true); // this only does the screen-covering "click-behind" portion - and ensures self.stage is set

        // Note that coerce_in throws if min>max, so we defend against new_inner_table being bigger than the stage,
        // and padding helps the rounded edges to look more natural:
        let padded_half_width = new_inner_table.width() / 2.0 + 2.0;
        let padded_half_height = new_inner_table.height() / 2.0 + 2.0;

        let stage_width = self.popup.stage().width();
        let stage_height = self.popup.stage().height();

        let x_pos = if padded_half_width * 2.0 > stage_width {
            stage_width / 2.0
        } else {
            position.x.clamp(padded_half_width, stage_width - padded_half_width)
        };

        let y_pos = if padded_half_height * 2.0 > stage_height {
            stage_height / 2.0
        } else {
            position.y.clamp(padded_half_height, stage_height - padded_half_height)
        };

        self.container.set_position(x_pos, y_pos);
        self.popup.add_actor(&self.container);

        // This "zoomfades" the container "in"
        self.container.add_action(
            Action::parallel(
                Action::scale_to(1.0, 1.0, self.animation_duration, Interpolation::fade),
                Action::fade_in(self.animation_duration, Interpolation::fade)
            )
        );

        // This gradually darkens the "outside" at the same time
        self.background_color = Color::new(0.0, 0.0, 0.0, 0.0);
        self.popup.add_action(
            Action::alpha(0.35, self.animation_duration, Interpolation::fade)
                .with_color(self.background_color)
        );
    }

    /// Closes the popup with animation
    pub fn close(&mut self) {
        let close_listeners = self.popup.close_listeners().clone();
        self.popup.clear_close_listeners();

        for listener in close_listeners {
            listener();
        }

        self.popup.add_action(
            Action::alpha(0.0, self.animation_duration, Interpolation::fade)
                .with_color(self.background_color)
        );

        self.container.add_action(
            Action::sequence(
                Action::parallel(
                    Action::scale_to(0.05, 0.05, self.animation_duration, Interpolation::fade),
                    Action::fade_out(self.animation_duration, Interpolation::fade)
                ),
                Action::run(Box::new(move || {
                    self.container.remove();
                    self.popup.close();
                    if let Some(callback) = self.after_close_callback.take() {
                        callback();
                    }
                }))
            )
        );
    }

    /// Creates a button - for use in AnimatedMenuPopup's content builder parameter.
    ///
    /// On activation it will set any_button_was_clicked, call action, then close the Popup.
    pub fn get_button<F>(&mut self, text: &str, binding: KeyboardBinding, action: F) -> Button
    where
        F: FnOnce() + 'static
    {
        let mut button = Button::new_with_style(text, self.small_button_style.clone());

        button.set_on_activation(binding, Box::new(move || {
            self.any_button_was_clicked = true;
            action();
            self.close();
        }));

        button
    }

    /// Sets a callback to be called after the popup is closed, the animation finished, and cleanup is done
    pub fn set_after_close_callback<F>(&mut self, callback: F)
    where
        F: FnOnce() + 'static
    {
        self.after_close_callback = Some(Box::new(callback));
    }

    /// Returns whether any button was clicked during the popup's lifetime
    pub fn any_button_was_clicked(&self) -> bool {
        self.any_button_was_clicked
    }

    /// Returns a reference to the underlying popup
    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    /// Returns a mutable reference to the underlying popup
    pub fn popup_mut(&mut self) -> &mut Popup {
        &mut self.popup
    }
}

/// Interpolation functions for animations
pub enum Interpolation {
    /// Linear interpolation
    Linear,
    /// Fade interpolation (smooth acceleration and deceleration)
    Fade,
    /// Power interpolation with the given power
    Power(f32),
    /// Sine interpolation
    Sine,
    /// Sine in interpolation
    SineIn,
    /// Sine out interpolation
    SineOut,
    /// Sine in-out interpolation
    SineInOut,
    /// Circle interpolation
    Circle,
    /// Circle in interpolation
    CircleIn,
    /// Circle out interpolation
    CircleOut,
    /// Circle in-out interpolation
    CircleInOut,
    /// Elastic interpolation
    Elastic,
    /// Elastic in interpolation
    ElasticIn,
    /// Elastic out interpolation
    ElasticOut,
    /// Elastic in-out interpolation
    ElasticInOut,
    /// Swing interpolation
    Swing,
    /// Swing in interpolation
    SwingIn,
    /// Swing out interpolation
    SwingOut,
    /// Swing in-out interpolation
    SwingInOut,
    /// Bounce interpolation
    Bounce,
    /// Bounce in interpolation
    BounceIn,
    /// Bounce out interpolation
    BounceOut,
    /// Bounce in-out interpolation
    BounceInOut,
}

/// Action for animations
pub enum Action {
    /// Parallel actions
    Parallel(Vec<Action>),
    /// Sequential actions
    Sequence(Vec<Action>),
    /// Scale to action
    ScaleTo(f32, f32, f32, Interpolation),
    /// Fade in action
    FadeIn(f32, Interpolation),
    /// Fade out action
    FadeOut(f32, Interpolation),
    /// Alpha action
    Alpha(f32, f32, Interpolation),
    /// Run action
    Run(Box<dyn FnOnce()>),
}

impl Action {
    /// Creates a parallel action
    pub fn parallel(action1: Action, action2: Action) -> Action {
        Action::Parallel(vec![action1, action2])
    }

    /// Creates a sequence action
    pub fn sequence(action1: Action, action2: Action) -> Action {
        Action::Sequence(vec![action1, action2])
    }

    /// Creates a scale to action
    pub fn scale_to(x: f32, y: f32, duration: f32, interpolation: Interpolation) -> Action {
        Action::ScaleTo(x, y, duration, interpolation)
    }

    /// Creates a fade in action
    pub fn fade_in(duration: f32, interpolation: Interpolation) -> Action {
        Action::FadeIn(duration, interpolation)
    }

    /// Creates a fade out action
    pub fn fade_out(duration: f32, interpolation: Interpolation) -> Action {
        Action::FadeOut(duration, interpolation)
    }

    /// Creates an alpha action
    pub fn alpha(alpha: f32, duration: f32, interpolation: Interpolation) -> Action {
        Action::Alpha(alpha, duration, interpolation)
    }

    /// Creates a run action
    pub fn run<F>(action: F) -> Action
    where
        F: FnOnce() + 'static
    {
        Action::Run(Box::new(action))
    }

    /// Adds a color to the action
    pub fn with_color(self, color: Color) -> Action {
        match self {
            Action::Alpha(alpha, duration, interpolation) => {
                // This is a simplified version - in a real implementation,
                // you would need to store the color and apply it during the action
                Action::Alpha(alpha, duration, interpolation)
            },
            _ => self,
        }
    }
}