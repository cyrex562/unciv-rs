use std::collections::HashMap;
use crate::models::metadata::GameSettings;
use crate::models::translations::tr;

/// Screen orientation options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenOrientation {
    /// Landscape (fixed)
    Landscape,
    /// Portrait (fixed)
    Portrait,
    /// Auto (sensor adjusted)
    Auto,
}

impl ScreenOrientation {
    /// Get the description of the orientation
    pub fn description(&self) -> String {
        match self {
            ScreenOrientation::Landscape => tr("Landscape (fixed)"),
            ScreenOrientation::Portrait => tr("Portrait (fixed)"),
            ScreenOrientation::Auto => tr("Auto (sensor adjusted)"),
        }
    }
}

impl ToString for ScreenOrientation {
    fn to_string(&self) -> String {
        self.description()
    }
}

/// Interface for screen modes
pub trait ScreenMode {
    /// Get the ID of the screen mode
    fn get_id(&self) -> i32;

    /// Check if the screen mode has user selectable size
    fn has_user_selectable_size(&self) -> bool {
        false
    }
}

/// Interface for platform display functionality
pub trait PlatformDisplay {
    /// Set the screen mode
    fn set_screen_mode(&self, id: i32, settings: &GameSettings) {}

    /// Get available screen modes
    fn get_screen_modes(&self) -> HashMap<i32, Box<dyn ScreenMode>> {
        HashMap::new()
    }

    /// Check if the platform has cutout support
    fn has_cutout(&self) -> bool {
        false
    }

    /// Set cutout enabled/disabled
    fn set_cutout(&self, enabled: bool) {}

    /// Check if the platform has orientation support
    fn has_orientation(&self) -> bool {
        false
    }

    /// Set the screen orientation
    fn set_orientation(&self, orientation: ScreenOrientation) {}

    /// Check if the screen mode has user selectable size
    fn has_user_selectable_size(&self, id: i32) -> bool {
        false
    }

    /// Check if the platform has system UI visibility support
    fn has_system_ui_visibility(&self) -> bool {
        false
    }

    /// Set system UI visibility
    fn set_system_ui_visibility(&self, hide: bool) {}
}

/// Display utility for managing screen settings
pub struct Display {
    platform: Box<dyn PlatformDisplay>,
}

impl Display {
    /// Create a new Display instance
    pub fn new(platform: Box<dyn PlatformDisplay>) -> Self {
        Self { platform }
    }

    /// Check if the platform has orientation support
    pub fn has_orientation(&self) -> bool {
        self.platform.has_orientation()
    }

    /// Set the screen orientation
    pub fn set_orientation(&self, orientation: ScreenOrientation) {
        self.platform.set_orientation(orientation);
    }

    /// Check if the platform has cutout support
    pub fn has_cutout(&self) -> bool {
        self.platform.has_cutout()
    }

    /// Set cutout enabled/disabled
    pub fn set_cutout(&self, enabled: bool) {
        self.platform.set_cutout(enabled);
    }

    /// Get available screen modes
    pub fn get_screen_modes(&self) -> HashMap<i32, Box<dyn ScreenMode>> {
        self.platform.get_screen_modes()
    }

    /// Set the screen mode
    pub fn set_screen_mode(&self, id: i32, settings: &GameSettings) {
        self.platform.set_screen_mode(id, settings);
    }

    /// Check if the screen mode has user selectable size
    pub fn has_user_selectable_size(&self, id: i32) -> bool {
        self.platform.has_user_selectable_size(id)
    }

    /// Check if the platform has system UI visibility support
    pub fn has_system_ui_visibility(&self) -> bool {
        self.platform.has_system_ui_visibility()
    }

    /// Set system UI visibility
    pub fn set_system_ui_visibility(&self, hide: bool) {
        self.platform.set_system_ui_visibility(hide);
    }
}

// Global instance
lazy_static::lazy_static! {
    static ref DISPLAY: std::sync::Mutex<Option<Display>> = std::sync::Mutex::new(None);
}

/// Initialize the global Display instance
pub fn init_display(platform: Box<dyn PlatformDisplay>) {
    let mut display = DISPLAY.lock().unwrap();
    *display = Some(Display::new(platform));
}

/// Get the global Display instance
pub fn get_display() -> std::sync::MutexGuard<'static, Option<Display>> {
    DISPLAY.lock().unwrap()
}