use ggez::event::{self, Event, MouseButton, MouseInput};
use ggez::input::mouse::MousePosition;
use ggez::mint::Vector2;
use ggez::{Context, GameResult};
use std::time::Duration;

/// A listener for zoom gestures, particularly for pinch-to-zoom functionality
///
/// This class handles zoom gestures for the game UI, particularly for pinch-to-zoom functionality.
/// It tracks the focal point of the zoom and calculates the appropriate translation and scale changes.
pub struct ZoomGestureListener {
    /// The gesture detector that processes raw input events
    detector: GestureDetector,

    /// Function that returns the current stage size
    stage_size: Box<dyn Fn() -> Vector2<f32>>,
}

impl ZoomGestureListener {
    /// Creates a new ZoomGestureListener with default parameters
    ///
    /// # Arguments
    ///
    /// * `stage_size` - Function that returns the current stage size
    pub fn new(stage_size: Box<dyn Fn() -> Vector2<f32>>) -> Self {
        Self::with_params(
            20.0, // half_tap_square_size
            0.4,  // tap_count_interval
            1.1,  // long_press_duration
            f32::MAX, // max_fling_delay
            stage_size,
        )
    }

    /// Creates a new ZoomGestureListener with custom parameters
    ///
    /// # Arguments
    ///
    /// * `half_tap_square_size` - Half the size of the tap square
    /// * `tap_count_interval` - Interval between taps for multi-tap detection
    /// * `long_press_duration` - Duration for long press detection
    /// * `max_fling_delay` - Maximum delay for fling detection
    /// * `stage_size` - Function that returns the current stage size
    pub fn with_params(
        half_tap_square_size: f32,
        tap_count_interval: f32,
        long_press_duration: f32,
        max_fling_delay: f32,
        stage_size: Box<dyn Fn() -> Vector2<f32>>,
    ) -> Self {
        let detector = GestureDetector::new(
            half_tap_square_size,
            tap_count_interval,
            long_press_duration,
            max_fling_delay,
            Box::new(GestureAdapter::new(stage_size.clone())),
        );

        Self {
            detector,
            stage_size,
        }
    }

    /// Handles a scroll event
    ///
    /// # Arguments
    ///
    /// * `amount_x` - The amount scrolled horizontally
    /// * `amount_y` - The amount scrolled vertically
    ///
    /// # Returns
    ///
    /// Whether the event was handled
    pub fn scrolled(&mut self, amount_x: f32, amount_y: f32) -> bool {
        false
    }

    /// Handles a pinch gesture
    ///
    /// # Arguments
    ///
    /// * `translation` - The translation vector in stage coordinates
    /// * `scale_change` - The change in scale
    pub fn pinch(&mut self, translation: Vector2<f32>, scale_change: f32) {
        // Default implementation does nothing
    }

    /// Handles the end of a pinch gesture
    pub fn pinch_stop(&mut self) {
        // Default implementation does nothing
    }

    /// Handles an event
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Whether the event was handled
    pub fn handle(&mut self, event: &Event) -> bool {
        match event {
            Event::MouseInput {
                button,
                state,
                position,
                ..
            } => {
                match (button, state) {
                    (MouseButton::Left, MouseInput::Down) => {
                        self.detector.touch_down(position.x, position.y, 0, 0);
                        true
                    },
                    (MouseButton::Left, MouseInput::Up) => {
                        self.detector.touch_up(position.x, position.y, 0, 0);
                        true
                    },
                    _ => false,
                }
            },
            Event::MouseMotion {
                delta,
                position,
                ..
            } => {
                self.detector.touch_dragged(position.x, position.y, 0);
                true
            },
            Event::MouseWheel {
                x,
                y,
                ..
            } => {
                self.scrolled(*x, *y)
            },
            _ => false,
        }
    }
}

/// A gesture detector that processes raw input events
struct GestureDetector {
    /// Half the size of the tap square
    half_tap_square_size: f32,

    /// Interval between taps for multi-tap detection
    tap_count_interval: f32,

    /// Duration for long press detection
    long_press_duration: f32,

    /// Maximum delay for fling detection
    max_fling_delay: f32,

    /// The gesture adapter that receives processed gestures
    adapter: Box<dyn GestureAdapter>,

    /// The current state of the detector
    state: GestureState,
}

impl GestureDetector {
    /// Creates a new GestureDetector
    ///
    /// # Arguments
    ///
    /// * `half_tap_square_size` - Half the size of the tap square
    /// * `tap_count_interval` - Interval between taps for multi-tap detection
    /// * `long_press_duration` - Duration for long press detection
    /// * `max_fling_delay` - Maximum delay for fling detection
    /// * `adapter` - The gesture adapter that receives processed gestures
    fn new(
        half_tap_square_size: f32,
        tap_count_interval: f32,
        long_press_duration: f32,
        max_fling_delay: f32,
        adapter: Box<dyn GestureAdapter>,
    ) -> Self {
        Self {
            half_tap_square_size,
            tap_count_interval,
            long_press_duration,
            max_fling_delay,
            adapter,
            state: GestureState::Idle,
        }
    }

    /// Handles a touch down event
    ///
    /// # Arguments
    ///
    /// * `x` - The x coordinate
    /// * `y` - The y coordinate
    /// * `pointer` - The pointer ID
    /// * `button` - The button ID
    fn touch_down(&mut self, x: f32, y: f32, pointer: i32, button: i32) {
        // Implementation would track touch points and detect gestures
        // For simplicity, we'll just pass the coordinates to the adapter
        self.adapter.touch_down(x, y, pointer, button);
    }

    /// Handles a touch up event
    ///
    /// # Arguments
    ///
    /// * `x` - The x coordinate
    /// * `y` - The y coordinate
    /// * `pointer` - The pointer ID
    /// * `button` - The button ID
    fn touch_up(&mut self, x: f32, y: f32, pointer: i32, button: i32) {
        // Implementation would track touch points and detect gestures
        // For simplicity, we'll just pass the coordinates to the adapter
        self.adapter.touch_up(x, y, pointer, button);
    }

    /// Handles a touch dragged event
    ///
    /// # Arguments
    ///
    /// * `x` - The x coordinate
    /// * `y` - The y coordinate
    /// * `pointer` - The pointer ID
    fn touch_dragged(&mut self, x: f32, y: f32, pointer: i32) {
        // Implementation would track touch points and detect gestures
        // For simplicity, we'll just pass the coordinates to the adapter
        self.adapter.touch_dragged(x, y, pointer);
    }

    /// Resets the detector state
    fn reset(&mut self) {
        self.state = GestureState::Idle;
    }
}

/// The state of a gesture detector
enum GestureState {
    /// No gesture is being detected
    Idle,

    /// A tap is being detected
    Tapping,

    /// A long press is being detected
    LongPressing,

    /// A fling is being detected
    Flinging,

    /// A pinch is being detected
    Pinching,
}

/// A trait for gesture adapters
trait GestureAdapter {
    /// Handles a touch down event
    fn touch_down(&mut self, x: f32, y: f32, pointer: i32, button: i32);

    /// Handles a touch up event
    fn touch_up(&mut self, x: f32, y: f32, pointer: i32, button: i32);

    /// Handles a touch dragged event
    fn touch_dragged(&mut self, x: f32, y: f32, pointer: i32);

    /// Handles a pinch gesture
    fn pinch(
        &mut self,
        stage_initial_pointer1: Vector2<f32>,
        stage_initial_pointer2: Vector2<f32>,
        stage_pointer1: Vector2<f32>,
        stage_pointer2: Vector2<f32>,
    ) -> bool;

    /// Handles the end of a pinch gesture
    fn pinch_stop(&mut self);
}

/// A gesture adapter for zoom gestures
struct GestureAdapter {
    /// Function that returns the current stage size
    stage_size: Box<dyn Fn() -> Vector2<f32>>,

    /// The last focal point of the pinch
    last_focus: Option<Vector2<f32>>,

    /// The last distance between the two pointers
    last_distance: f32,
}

impl GestureAdapter {
    /// Creates a new GestureAdapter
    ///
    /// # Arguments
    ///
    /// * `stage_size` - Function that returns the current stage size
    fn new(stage_size: Box<dyn Fn() -> Vector2<f32>>) -> Self {
        Self {
            stage_size,
            last_focus: None,
            last_distance: 0.0,
        }
    }

    /// Calculates the distance between two points
    ///
    /// # Arguments
    ///
    /// * `p1` - The first point
    /// * `p2` - The second point
    ///
    /// # Returns
    ///
    /// The distance between the points
    fn distance(p1: &Vector2<f32>, p2: &Vector2<f32>) -> f32 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Adds two vectors
    ///
    /// # Arguments
    ///
    /// * `v1` - The first vector
    /// * `v2` - The second vector
    ///
    /// # Returns
    ///
    /// The sum of the vectors
    fn add(v1: &Vector2<f32>, v2: &Vector2<f32>) -> Vector2<f32> {
        Vector2 {
            x: v1.x + v2.x,
            y: v1.y + v2.y,
        }
    }

    /// Subtracts two vectors
    ///
    /// # Arguments
    ///
    /// * `v1` - The first vector
    /// * `v2` - The second vector
    ///
    /// # Returns
    ///
    /// The difference of the vectors
    fn sub(v1: &Vector2<f32>, v2: &Vector2<f32>) -> Vector2<f32> {
        Vector2 {
            x: v1.x - v2.x,
            y: v1.y - v2.y,
        }
    }

    /// Scales a vector by a factor
    ///
    /// # Arguments
    ///
    /// * `v` - The vector
    /// * `scale` - The scale factor
    ///
    /// # Returns
    ///
    /// The scaled vector
    fn scale(v: &Vector2<f32>, scale: f32) -> Vector2<f32> {
        Vector2 {
            x: v.x * scale,
            y: v.y * scale,
        }
    }
}

impl GestureAdapter for GestureAdapter {
    fn touch_down(&mut self, _x: f32, _y: f32, _pointer: i32, _button: i32) {
        // Implementation would track touch points
    }

    fn touch_up(&mut self, _x: f32, _y: f32, _pointer: i32, _button: i32) {
        // Implementation would track touch points
    }

    fn touch_dragged(&mut self, _x: f32, _y: f32, _pointer: i32) {
        // Implementation would track touch points
    }

    fn pinch(
        &mut self,
        stage_initial_pointer1: Vector2<f32>,
        stage_initial_pointer2: Vector2<f32>,
        stage_pointer1: Vector2<f32>,
        stage_pointer2: Vector2<f32>,
    ) -> bool {
        // If this is the first pinch, initialize the last focus and distance
        if self.last_focus.is_none() {
            let sum = Self::add(&stage_initial_pointer1, &stage_initial_pointer2);
            self.last_focus = Some(Self::scale(&sum, 0.5));
            self.last_distance = Self::distance(&stage_initial_pointer1, &stage_initial_pointer2);
        }

        // The current focal point is the center of the two pointers
        let current_focus = Self::scale(
            &Self::add(&stage_pointer1, &stage_pointer2),
            0.5
        );

        // Translation caused by moving the focal point
        let mut translation = Self::sub(&current_focus, &self.last_focus.unwrap());
        self.last_focus = Some(current_focus);

        // Scale change caused by changing distance of the two pointers
        let current_distance = Self::distance(&stage_pointer1, &stage_pointer2);
        let scale_change = current_distance / self.last_distance;
        self.last_distance = current_distance;

        // Calculate the translation (dx, dy) needed to direct the zoom towards the
        // current focal point. Without this correction, the zoom would be directed
        // towards the center of the stage.
        let stage_size = (self.stage_size)();
        let dx = (stage_size.x / 2.0 - current_focus.x) * (scale_change - 1.0);
        let dy = (stage_size.y / 2.0 - current_focus.y) * (scale_change - 1.0);

        // Add the translation caused by changing the scale (dx, dy) to the translation
        // caused by changing the position of the focal point.
        translation.x += dx;
        translation.y += dy;

        // Notify the listener of the pinch
        // In a real implementation, this would call a callback
        // For now, we'll just return true to indicate the event was handled
        true
    }

    fn pinch_stop(&mut self) {
        self.last_focus = None;
        // In a real implementation, this would call a callback
    }
}