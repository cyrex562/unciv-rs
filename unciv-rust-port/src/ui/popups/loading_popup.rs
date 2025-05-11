// Source: orig_src/core/src/com/unciv/ui/popups/LoadingPopup.kt

use std::rc::Rc;
use eframe::egui::{self, Ui, Color32, Response};
use log::info;

use crate::ui::{
    popups::Popup,
    screens::basescreen::BaseScreen,
};
use crate::constants::LOADING;

/// A mini popup that just displays "Loading..." and opens itself.
///
/// Not to be confused with LoadingScreen, which tries to preserve background as screenshot.
/// That screen will use this once the screenshot is on-screen, though.
pub struct LoadingPopup {
    screen: Rc<BaseScreen>,
    should_close: bool,
}

impl LoadingPopup {
    /// Create a new LoadingPopup
    pub fn new(screen: Rc<BaseScreen>) -> Self {
        Self {
            screen,
            should_close: false,
        }
    }

    /// Open the popup
    pub fn open(&mut self) {
        // In Rust, we don't need to explicitly open the popup
        // It will be shown when the show method is called
    }
}

impl Popup for LoadingPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        // Create a frame for the popup
        egui::Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(200.0);

                // Center the loading text
                ui.allocate_space(egui::vec2(ui.available_width(), 20.0));

                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    ui.heading(LOADING);
                });

                // Add a spinner or progress indicator
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    ui.spinner();
                });
            });

        // Loading popup should not close automatically
        self.should_close
    }

    fn title(&self) -> String {
        String::from("Loading")
    }
}