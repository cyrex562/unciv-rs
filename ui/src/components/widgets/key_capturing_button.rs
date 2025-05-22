use std::rc::Rc;
use sdl2::keyboard::Keycode;
use sdl2::event::Event;
use sdl2::pixels::Color;
use crate::ui::components::input::KeyCharAndCode;
use crate::ui::components::fonts::Fonts;
use crate::ui::images::ImageGetter;
use crate::ui::screens::base_screen::BaseScreen;

/// Style configuration for the KeyCapturingButton
pub struct KeyCapturingButtonStyle {
    /// Size for the image part
    pub image_size: f32,
    /// Name for the image part
    pub image_name: String,
    /// Tint color for the normal state
    pub image_up_tint: Color,
    /// Tint color for the hover state
    pub image_over_tint: Color,
    /// Minimum width of the button
    pub min_width: f32,
    /// Minimum height of the button
    pub min_height: f32,
}

impl Default for KeyCapturingButtonStyle {
    fn default() -> Self {
        Self {
            image_size: 24.0,
            image_name: "OtherIcons/Keyboard".to_string(),
            image_up_tint: Color::RGBA(0, 0, 0, 0), // CLEAR
            image_over_tint: Color::RGB(0, 255, 0), // LIME
            min_width: 150.0,
            min_height: 24.0,
        }
    }
}

/// A button that captures keyboard input
pub struct KeyCapturingButton {
    /// The default key state
    default: KeyCharAndCode,
    /// The current key state
    current: KeyCharAndCode,
    /// Whether to mark the button as having a conflict
    mark_conflict: bool,
    /// The style configuration
    style: KeyCapturingButtonStyle,
    /// Callback for when a key is hit
    on_key_hit: Option<Rc<dyn Fn(KeyCharAndCode)>>,
    /// The saved keyboard focus
    saved_focus: Option<Box<dyn std::any::Any>>,
    /// The normal style
    normal_style: ButtonStyle,
    /// The default style (grayed)
    default_style: ButtonStyle,
    /// The conflict style (red)
    conflict_style: ButtonStyle,
}

/// Style for the button
struct ButtonStyle {
    /// Font color
    font_color: Color,
    /// Background color
    background_color: Color,
}

impl KeyCapturingButton {
    /// Create a new KeyCapturingButton
    pub fn new(
        default: KeyCharAndCode,
        style: KeyCapturingButtonStyle,
        on_key_hit: Option<Rc<dyn Fn(KeyCharAndCode)>>,
    ) -> Self {
        let normal_style = ButtonStyle {
            font_color: Color::WHITE,
            background_color: Color::RGB(50, 50, 50),
        };

        let default_style = ButtonStyle {
            font_color: Color::RGB(128, 128, 128), // GRAY
            background_color: normal_style.background_color,
        };

        let conflict_style = ButtonStyle {
            font_color: Color::RED,
            background_color: normal_style.background_color,
        };

        Self {
            default,
            current: KeyCharAndCode::UNKNOWN,
            mark_conflict: false,
            style,
            on_key_hit,
            saved_focus: None,
            normal_style,
            default_style,
            conflict_style,
        }
    }

    /// Update the button's label based on the current key
    fn update_label(&mut self) {
        let text = if self.current == KeyCharAndCode::BACK {
            "ESC/Back".to_string()
        } else {
            self.current.to_string()
        };
        // TODO: Update the button's label text
        self.update_style();
    }

    /// Update the button's style based on its state
    fn update_style(&mut self) {
        let style = if self.mark_conflict {
            &self.conflict_style
        } else if self.current == self.default {
            &self.default_style
        } else {
            &self.normal_style
        };
        // TODO: Apply the style to the button
    }

    /// Handle a key press
    fn handle_key(&mut self, code: i32, control: bool) {
        self.current = if control {
            KeyCharAndCode::ctrl_from_code(code)
        } else {
            KeyCharAndCode::from_code(code)
        };
        if let Some(callback) = &self.on_key_hit {
            callback(self.current);
        }
    }

    /// Reset the key to the default
    fn reset_key(&mut self) {
        self.current = self.default;
        if let Some(callback) = &self.on_key_hit {
            callback(self.current);
        }
    }

    /// Handle mouse enter event
    pub fn on_enter(&mut self) {
        // TODO: Save current keyboard focus and set focus to this button
    }

    /// Handle mouse exit event
    pub fn on_exit(&mut self) {
        // TODO: Restore saved keyboard focus
    }

    /// Handle key down event
    pub fn on_key_down(&mut self, event: &Event) -> bool {
        if let Event::KeyDown { keycode: Some(keycode), .. } = event {
            if keycode == Keycode::Unknown {
                return false;
            }
            if keycode == Keycode::LCtrl || keycode == Keycode::RCtrl {
                return false;
            }

            let code = keycode as i32;
            let control = false; // TODO: Check if control key is pressed
            self.handle_key(code, control);
            return true;
        }
        false
    }

    /// Handle click event
    pub fn on_click(&mut self, double_click: bool) {
        if double_click {
            self.reset_key();
        }
    }
}

impl Default for KeyCapturingButton {
    fn default() -> Self {
        Self::new(
            KeyCharAndCode::UNKNOWN,
            KeyCapturingButtonStyle::default(),
            None,
        )
    }
}