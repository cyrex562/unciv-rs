// Source: orig_src/core/src/com/unciv/ui/popups/ConfirmPopup.kt

use std::rc::Rc;
use eframe::egui::{self, Ui, Color32, Response};
use log::info;

use crate::ui::{
    popups::Popup,
    screens::basescreen::BaseScreen,
    components::input::KeyboardBinding,
};
use crate::constants::CANCEL;

/// A variant of Popup pre-populated with one label, plus confirm and cancel buttons
pub struct ConfirmPopup {
    screen: Rc<BaseScreen>,
    question: String,
    confirm_text: String,
    is_confirm_positive: bool,
    restore_default: Option<Box<dyn FnOnce()>>,
    action: Box<dyn FnOnce()>,
    should_close: bool,
}

impl ConfirmPopup {
    /// Create a new ConfirmPopup
    pub fn new(
        screen: Rc<BaseScreen>,
        question: String,
        confirm_text: String,
        is_confirm_positive: bool,
        restore_default: Option<Box<dyn FnOnce()>>,
        action: Box<dyn FnOnce()>,
    ) -> Self {
        Self {
            screen,
            question,
            confirm_text,
            is_confirm_positive,
            restore_default,
            action,
            should_close: false,
        }
    }

    /// Get the color for the confirm button based on whether it's positive or negative
    fn get_confirm_color(&self) -> Color32 {
        if self.is_confirm_positive {
            Color32::from_rgb(50, 200, 50) // Green for positive actions
        } else {
            Color32::from_rgb(200, 50, 50) // Red for negative actions
        }
    }
}

impl Popup for ConfirmPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        // Create a frame for the popup
        egui::Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(300.0);

                // Add title
                ui.heading("Confirmation");

                ui.add_space(10.0);

                // Add the question text
                ui.label(&self.question);

                ui.add_space(20.0);

                // Add buttons in a horizontal layout
                ui.horizontal(|ui| {
                    // Cancel button (with negative style)
                    if ui.add(egui::Button::new(CANCEL).fill(Color32::from_rgb(150, 150, 150))).clicked() {
                        if let Some(restore) = self.restore_default.take() {
                            restore();
                        }
                        self.should_close = true;
                    }

                    ui.add_space(10.0);

                    // Confirm button (with appropriate style)
                    if ui.add(egui::Button::new(&self.confirm_text).fill(self.get_confirm_color())).clicked() {
                        let action = self.action.take().unwrap();
                        action();
                        self.should_close = true;
                    }
                });
            });

        self.should_close
    }

    fn title(&self) -> String {
        String::from("Confirmation")
    }
}