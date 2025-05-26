mod version;

use tracing::{info, warn, error, debug, trace}


use ::log::info;
use clap::Parser;
use eframe::egui;

use std::rc::Rc;

use crate::ui::popups::ask_text_popup::AskTextPopup;
use crate::ui::popups::auth_popup::AuthPopup;
use crate::ui::screens::base_screen::basescreen::BaseScreen;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run the multiplayer server instead of the game
    #[arg(short, long)]
    server: bool,
    // ... existing arguments ...
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let file_appender = tracing_appender::rolling::hourly("logs", "unciv_rust.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout)) // Console
        .with(fmt::layer().with_writer(non_blocking)) // File
        .with(EnvFilter::from_default_env())
        .init();
    
    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Check if server mode is requested
    if args.server {
        info!("Starting Unciv Server");
        // Run server
        let config = server::ServerConfig::parse();
        tokio::runtime::Runtime::new()?.block_on(server::UncivServer::run(config))?;
        return Ok(());
    }

    // Run game
    info!("Starting Unciv Rust port");
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280.0, 720.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Unciv Rust",
        options,
        Box::new(|_cc| Box::new(UncivApp::new())),
    )?;

    Ok(())
}

/// Main application struct
struct UncivApp {
    // Application state
    base_screen: Rc<BaseScreen>,
    active_popup: Option<Box<dyn ui::popups::Popup>>,
}

impl UncivApp {
    /// Create a new UncivApp
    fn new() -> Self {
        // Create the base screen
        let base_screen = Rc::new(BaseScreen::new(Rc::new(eframe::egui::Context::default())));

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
            info!(
                "Authentication {}",
                if success { "successful" } else { "failed" }
            );
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
