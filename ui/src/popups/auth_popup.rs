// Source: orig_src/core/src/com/unciv/ui/popups/AuthPopup.kt

use eframe::egui::{self, Color32, Response, Ui};
use std::rc::Rc;

use crate::ui::{
    components::text_field::UncivTextField,
    screens::basescreen::BaseScreen,
    popups::Popup,
};

/// A popup for authenticating with a server
///
/// # Arguments
///
/// * `screen` - The screen to show the popup on
/// * `auth_successful` - Callback function that will be called with the authentication result
pub struct AuthPopup {
    screen: Rc<BaseScreen>,
    password_field: UncivTextField,
    auth_successful: Option<Rc<dyn Fn(bool)>>,
    show_error: bool,
}

impl AuthPopup {
    /// Create a new AuthPopup with the given screen and callback
    pub fn new(
        screen: Rc<BaseScreen>,
        auth_successful: Option<Rc<dyn Fn(bool)>>,
    ) -> Self {
        let mut password_field = UncivTextField::new("Password", "");
        password_field.set_password(true);

        Self {
            screen,
            password_field,
            auth_successful,
            show_error: false,
        }
    }

    /// Authenticate with the server
    fn authenticate(&mut self) -> Result<(), String> {
        // In a real implementation, this would call the server's authenticate method
        // For now, we'll just simulate a successful authentication
        let password = self.password_field.text().to_string();

        // TODO: Implement actual authentication with the server
        // This is a placeholder that always succeeds
        if password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }

        Ok(())
    }
}

impl Popup for AuthPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        let mut should_close = false;

        // Create a frame for the popup
        egui::Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(300.0);

                // Add title
                ui.heading("Server Authentication");

                ui.add_space(10.0);

                // Show error message if authentication failed
                if self.show_error {
                    ui.label(egui::RichText::new("Authentication failed").color(Color32::RED));
                    ui.add_space(5.0);
                } else {
                    ui.label("Please enter your server password");
                }

                ui.add_space(10.0);

                // Add password field
                self.password_field.ui(ui);

                ui.add_space(10.0);

                // Add buttons in a horizontal layout
                ui.horizontal(|ui| {
                    // Close button (with negative style)
                    if ui.add(egui::Button::new("Cancel").fill(Color32::from_rgb(200, 50, 50))).clicked() {
                        if let Some(callback) = &self.auth_successful {
                            callback(false);
                        }
                        should_close = true;
                    }

                    ui.add_space(10.0);

                    // Authenticate button
                    if ui.button("Authenticate").clicked() {
                        match self.authenticate() {
                            Ok(_) => {
                                if let Some(callback) = &self.auth_successful {
                                    callback(true);
                                }
                                should_close = true;
                            },
                            Err(_) => {
                                self.show_error = true;
                            }
                        }
                    }
                });
            });

        should_close
    }

    fn title(&self) -> String {
        String::from("Server Authentication")
    }
}