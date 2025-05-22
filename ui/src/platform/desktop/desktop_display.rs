// Source: orig_src/desktop/src/com/unciv/app/desktop/DesktopDisplay.kt
// Ported to Rust

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Context, Visuals};
use crate::ui::platform::PlatformDisplay;
use crate::ui::theme::Theme;
use crate::ui::theme::ThemeManager;

/// Display implementation for desktop platform
pub struct DesktopDisplay {
    context: Rc<RefCell<Context>>,
    theme_manager: Rc<RefCell<ThemeManager>>,
}

impl DesktopDisplay {
    /// Creates a new desktop display
    pub fn new(context: Rc<RefCell<Context>>) -> Self {
        Self {
            context,
            theme_manager: Rc::new(RefCell::new(ThemeManager::new())),
        }
    }
}

impl PlatformDisplay for DesktopDisplay {
    /// Gets the egui context
    fn context(&self) -> Rc<RefCell<Context>> {
        self.context.clone()
    }

    /// Gets the theme manager
    fn theme_manager(&self) -> Rc<RefCell<ThemeManager>> {
        self.theme_manager.clone()
    }

    /// Sets the current theme
    fn set_theme(&self, theme: Theme) {
        let mut ctx = self.context.borrow_mut();
        match theme {
            Theme::Light => ctx.set_visuals(Visuals::light()),
            Theme::Dark => ctx.set_visuals(Visuals::dark()),
            Theme::System => {
                // TODO: Implement system theme detection
                ctx.set_visuals(Visuals::light())
            }
        }
    }

    /// Gets the current theme
    fn get_theme(&self) -> Theme {
        // TODO: Implement theme detection from egui context
        Theme::Light
    }
}