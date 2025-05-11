// Source: orig_src/desktop/src/com/unciv/app/desktop/DesktopLauncher.kt
// Ported to Rust

use std::rc::Rc;
use std::cell::RefCell;
use eframe::{egui, NativeOptions, run_native};
use crate::ui::platform::desktop::desktop_display::DesktopDisplay;
use crate::ui::screens::main_menu::MainMenuScreen;
use crate::ui::theme::Theme;

/// Main launcher for desktop platform
pub struct DesktopLauncher {
    display: Rc<RefCell<DesktopDisplay>>,
    main_menu: Rc<RefCell<MainMenuScreen>>,
}

impl DesktopLauncher {
    /// Creates a new desktop launcher
    pub fn new() -> Self {
        let options = NativeOptions {
            initial_window_size: Some(egui::vec2(1280.0, 720.0)),
            ..Default::default()
        };

        let app = Self {
            display: Rc::new(RefCell::new(DesktopDisplay::new(
                Rc::new(RefCell::new(egui::Context::default()))
            ))),
            main_menu: Rc::new(RefCell::new(MainMenuScreen::new())),
        };

        run_native(
            "Unciv",
            options,
            Box::new(|_cc| Box::new(app)),
        );

        app
    }
}

impl eframe::App for DesktopLauncher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update display context
        *self.display.borrow_mut().context() = Rc::new(RefCell::new(ctx.clone()));

        // Set initial theme
        self.display.borrow().set_theme(Theme::Light);

        // Draw main menu
        self.main_menu.borrow_mut().draw(&self.display);
    }
}