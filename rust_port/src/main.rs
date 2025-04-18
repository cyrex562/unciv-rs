// Main entry point for the Unciv Rust port

mod ui;

use eframe::egui;
use log::info;
use std::rc::Rc;

use ui::{
    popups::{AskTextPopup, AuthPopup},
    screens::basescreen::BaseScreen,
};

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    env_logger::init();
    info!("Starting Unciv Rust port");

    // Create the application
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "Unciv Rust",
        options,
        Box::new(|cc| Box::new(UncivApp::new(cc))),
    )
}

/// Main application struct
struct UncivApp {
    // Application state
    base_screen: Rc<BaseScreen>,
    active_popup: Option<Box<dyn ui::popups::Popup>>,
}

impl UncivApp {
    /// Create a new UncivApp
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create the base screen
        let base_screen = Rc::new(BaseScreen::new(Rc::new(cc.egui_ctx.clone())));

        Self {
            base_screen,
            active_popup: None,
        }
    }

    /// Show the AskTextPopup
    fn show_ask_text_popup(&mut self) {
        let screen = self.base_screen.clone();
        let popup = AskTextPopup::default(screen);
        self.active_popup = Some(Box::new(popup));
    }

    /// Show the AuthPopup
    fn show_auth_popup(&mut self) {
        let screen = self.base_screen.clone();
        let auth_successful = Rc::new(|success: bool| {
            info!("Authentication {}", if success { "successful" } else { "failed" });
        });

        let popup = AuthPopup::new(screen, Some(auth_successful));
        self.active_popup = Some(Box::new(popup));
    }
}

impl eframe::App for UncivApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Main application update loop
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Unciv Rust Port");
            ui.label("Welcome to the Unciv Rust port!");

            ui.add_space(10.0);

            // Add buttons to show popups
            ui.horizontal(|ui| {
                if ui.button("Show AskTextPopup").clicked() {
                    self.show_ask_text_popup();
                }

                if ui.button("Show AuthPopup").clicked() {
                    self.show_auth_popup();
                }
            });
        });

        // Show active popup if any
        if let Some(popup) = &mut self.active_popup {
            egui::Window::new(popup.title())
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if popup.show(ui) {
                        self.active_popup = None;
                    }
                });
        }
    }
}
