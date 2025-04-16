use gdx::scenes::scene2d::{Actor, InputEvent};
use gdx::scenes::scene2d::utils::ActorGestureListener;
use crate::ui::components::input::activation_extensions::ActorActivationExt;
use crate::ui::components::input::activation_types::ActivationTypes;

/// A gesture listener that handles activation events for actors
///
/// This listener handles tap and long press gestures, converting them to activation events
/// that can be processed by the actor's activation handlers.
pub struct ActivationListener {
    /// The base gesture listener functionality
    base: ActorGestureListener,
}

impl ActivationListener {
    /// Creates a new ActivationListener with default parameters
    ///
    /// Default parameters are:
    /// - half_tap_square_size = 20.0
    /// - tap_count_interval = 0.25
    /// - long_press_duration = 1.1
    /// - max_fling_delay = f32::MAX
    pub fn new() -> Self {
        Self {
            base: ActorGestureListener::new(20.0, 0.25, 1.1, f32::MAX),
        }
    }

    /// Creates a new ActivationListener with custom parameters
    ///
    /// # Arguments
    ///
    /// * `half_tap_square_size` - Half the size of the square around the tap point
    /// * `tap_count_interval` - The maximum time between taps for a multi-tap
    /// * `long_press_duration` - The duration of a long press in seconds
    /// * `max_fling_delay` - The maximum delay for a fling gesture
    pub fn with_params(
        half_tap_square_size: f32,
        tap_count_interval: f32,
        long_press_duration: f32,
        max_fling_delay: f32,
    ) -> Self {
        Self {
            base: ActorGestureListener::new(
                half_tap_square_size,
                tap_count_interval,
                long_press_duration,
                max_fling_delay,
            ),
        }
    }
}

impl ActorGestureListener for ActivationListener {
    /// Handles tap events, converting them to activation events
    ///
    /// # Arguments
    ///
    /// * `event` - The input event
    /// * `x` - The x coordinate of the tap
    /// * `y` - The y coordinate of the tap
    /// * `count` - The number of taps
    /// * `button` - The button that was pressed
    fn tap(&self, event: Option<&InputEvent>, x: f32, y: f32, count: i32, button: i32) {
        // Get the actor from the event
        let actor = match event.and_then(|e| e.listener_actor()) {
            Some(actor) => actor,
            None => return,
        };

        // Find the activation type that matches the tap count and button
        let activation_type = match ActivationTypes::values().iter().find(|&t| {
            t.is_gesture() && t.tap_count() == count && t.button() == button
        }) {
            Some(t) => t,
            None => return,
        };

        // Activate the actor with the found activation type
        actor.activate(activation_type);
    }

    /// Handles long press events, converting them to activation events
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor that was long pressed
    /// * `x` - The x coordinate of the long press
    /// * `y` - The y coordinate of the long press
    ///
    /// # Returns
    ///
    /// `true` if the event was handled, `false` otherwise
    fn long_press(&self, actor: Option<&Actor>, x: f32, y: f32) -> bool {
        // Get the actor
        let actor = match actor {
            Some(actor) => actor,
            None => return false,
        };

        // See #10050 - when a tap discards its actor or ascendants, Gdx can't cancel the longpress timer
        if actor.stage().is_none() {
            return false;
        }

        // Activate the actor with the Longpress activation type
        actor.activate(ActivationTypes::Longpress)
    }
}