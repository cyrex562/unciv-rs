use std::collections::HashSet;
use std::sync::Arc;
use ggez::event::{self, KeyCode, KeyMods};
use ggez::input::keyboard::KeyInput;
use ggez::mint::Point2;
use crate::ui::components::input::keyboard_binding::{KeyboardBinding, GlobalKeyboardBindings};
use crate::ui::components::widgets::zoomable_scroll_pane::ZoomableScrollPane;

/// Listens for keyboard input to pan the map
pub struct KeyboardPanningListener {
    /// The map holder that will be panned
    map_holder: Arc<ZoomableScrollPane>,
    /// Whether to allow WASD keys for panning
    allow_wasd: bool,
    /// The currently pressed keys
    pressed_keys: HashSet<i32>,
    /// The action that is currently running
    infinite_action: Option<ggez::event::Action>,
    /// The key codes for panning
    keycode_up: i32,
    keycode_left: i32,
    keycode_down: i32,
    keycode_right: i32,
    keycode_up_alt: i32,
    keycode_left_alt: i32,
    keycode_down_alt: i32,
    keycode_right_alt: i32,
    /// The allowed keys for panning
    allowed_keys: HashSet<i32>,
}

impl KeyboardPanningListener {
    /// The delay between panning steps
    const DELTA_TIME: f32 = 0.01;

    /// Create a new KeyboardPanningListener
    pub fn new(map_holder: Arc<ZoomableScrollPane>, allow_wasd: bool) -> Self {
        let keycode_up = GlobalKeyboardBindings::get(KeyboardBinding::PanUp).code;
        let keycode_left = GlobalKeyboardBindings::get(KeyboardBinding::PanLeft).code;
        let keycode_down = GlobalKeyboardBindings::get(KeyboardBinding::PanDown).code;
        let keycode_right = GlobalKeyboardBindings::get(KeyboardBinding::PanRight).code;
        let keycode_up_alt = GlobalKeyboardBindings::get(KeyboardBinding::PanUpAlternate).code;
        let keycode_left_alt = GlobalKeyboardBindings::get(KeyboardBinding::PanLeftAlternate).code;
        let keycode_down_alt = GlobalKeyboardBindings::get(KeyboardBinding::PanDownAlternate).code;
        let keycode_right_alt = GlobalKeyboardBindings::get(KeyboardBinding::PanRightAlternate).code;

        let mut allowed_keys = HashSet::new();
        allowed_keys.insert(keycode_up);
        allowed_keys.insert(keycode_left);
        allowed_keys.insert(keycode_down);
        allowed_keys.insert(keycode_right);

        if allow_wasd {
            allowed_keys.insert(keycode_up_alt);
            allowed_keys.insert(keycode_left_alt);
            allowed_keys.insert(keycode_down_alt);
            allowed_keys.insert(keycode_right_alt);
        }

        Self {
            map_holder,
            allow_wasd,
            pressed_keys: HashSet::new(),
            infinite_action: None,
            keycode_up,
            keycode_left,
            keycode_down,
            keycode_right,
            keycode_up_alt,
            keycode_left_alt,
            keycode_down_alt,
            keycode_right_alt,
            allowed_keys,
        }
    }

    /// Handle key down event
    pub fn key_down(&mut self, keycode: i32, mods: KeyMods) -> bool {
        // Skip if control key is pressed
        if mods.contains(KeyMods::CTRL) {
            return false;
        }

        // Skip if key is not in allowed keys
        if !self.allowed_keys.contains(&keycode) {
            return false;
        }

        self.pressed_keys.insert(keycode);
        self.start_loop();
        true
    }

    /// Handle key up event
    pub fn key_up(&mut self, keycode: i32) -> bool {
        // Skip if key is not in allowed keys
        if !self.allowed_keys.contains(&keycode) {
            return false;
        }

        self.pressed_keys.remove(&keycode);
        if self.pressed_keys.is_empty() {
            self.stop_loop();
        }
        true
    }

    /// Start the panning loop
    fn start_loop(&mut self) {
        if self.infinite_action.is_some() {
            return;
        }

        // In a real implementation, this would create an action that runs every DELTA_TIME seconds
        // For now, we'll just simulate it with a simple action
        self.infinite_action = Some(ggez::event::Action::new(
            "panning_loop",
            Self::DELTA_TIME,
            Box::new(move || {
                // This would be called every DELTA_TIME seconds
                // In a real implementation, this would call while_key_pressed_loop
            }),
        ));
    }

    /// Stop the panning loop
    fn stop_loop(&mut self) {
        if let Some(action) = self.infinite_action.take() {
            // In a real implementation, this would stop the action
            // For now, we'll just remove it
        }
    }

    /// The loop that runs while keys are pressed
    fn while_key_pressed_loop(&self) {
        let mut delta_x = 0.0;
        let mut delta_y = 0.0;

        for &keycode in &self.pressed_keys {
            match keycode {
                k if k == self.keycode_up || k == self.keycode_up_alt => delta_y -= 1.0,
                k if k == self.keycode_down || k == self.keycode_down_alt => delta_y += 1.0,
                k if k == self.keycode_left || k == self.keycode_left_alt => delta_x += 1.0,
                k if k == self.keycode_right || k == self.keycode_right_alt => delta_x -= 1.0,
                _ => {}
            }
        }

        self.map_holder.do_key_or_mouse_panning(delta_x, delta_y);
    }
}

// Implement the event handler trait
impl event::EventHandler<ggez::GameError> for KeyboardPanningListener {
    fn key_down_event(
        &mut self,
        _ctx: &mut ggez::Context,
        keycode: KeyCode,
        keymods: KeyMods,
        _repeat: bool,
    ) -> Result<(), ggez::GameError> {
        // Convert KeyCode to i32
        let keycode_i32 = keycode as i32;

        // Skip if the target is a text field
        // In a real implementation, this would check if the target is a text field
        // For now, we'll just assume it's not

        self.key_down(keycode_i32, keymods);
        Ok(())
    }

    fn key_up_event(
        &mut self,
        _ctx: &mut ggez::Context,
        keycode: KeyCode,
        _keymods: KeyMods,
    ) -> Result<(), ggez::GameError> {
        // Convert KeyCode to i32
        let keycode_i32 = keycode as i32;

        self.key_up(keycode_i32);
        Ok(())
    }

    fn update(&mut self, _ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        // If there are pressed keys, call the panning loop
        if !self.pressed_keys.is_empty() {
            self.while_key_pressed_loop();
        }
        Ok(())
    }
}