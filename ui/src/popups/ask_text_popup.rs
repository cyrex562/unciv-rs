// Source: orig_src/core/src/com/unciv/ui/popups/AskTextPopup.kt

use eframe::egui::{self, Color32, Layout, Response, Ui, Vec2};
use std::rc::Rc;

use crate::ui::{
    components::text_field::UncivTextField,
    images::ImageGetter,
    screens::basescreen::BaseScreen,
    popups::Popup,
};

/// Simple class for showing a prompt for a string to the user
///
/// # Arguments
///
/// * `screen` - The previous screen the user was on
/// * `label` - A line of text shown to the user
/// * `icon` - Icon at the top, should have size 80.0
/// * `default_text` - The text that should be in the prompt at the start
/// * `error_text` - Text that will be shown when an error is detected
/// * `max_length` - The maximal amount of characters the user may input
/// * `validate` - Function that should return `true` when a valid input is entered, false otherwise
/// * `action_on_ok` - Lambda that will be executed after pressing 'OK'.
///   Gets the text the user inputted as a parameter.
pub struct AskTextPopup {
    screen: Rc<BaseScreen>,
    label: String,
    icon: egui::Image,
    default_text: String,
    error_text: String,
    max_length: usize,
    validate: Rc<dyn Fn(&str) -> bool>,
    action_on_ok: Rc<dyn Fn(String)>,
    text_field: UncivTextField,
    show_error: bool,
    illegal_chars: String,
}

impl AskTextPopup {
    pub fn new(
        screen: Rc<BaseScreen>,
        label: String,
        icon: egui::Image,
        default_text: String,
        error_text: String,
        max_length: usize,
        validate: Rc<dyn Fn(&str) -> bool>,
        action_on_ok: Rc<dyn Fn(String)>,
    ) -> Self {
        let illegal_chars = String::from("[]{}\"\\<>");

        let mut text_field = UncivTextField::new(&label, &default_text);
        text_field.set_max_length(max_length);

        // Set up text filter to prevent illegal characters
        text_field.set_filter(Box::new(move |_, c| !illegal_chars.contains(c)));

        Self {
            screen,
            label,
            icon,
            default_text,
            error_text,
            max_length,
            validate,
            action_on_ok,
            text_field,
            show_error: false,
            illegal_chars,
        }
    }

    pub fn default(
        screen: Rc<BaseScreen>,
    ) -> Self {
        let label = String::from("Please enter some text");
        let icon = ImageGetter::get_image("OtherIcons/Pencil")
            .with_color(ImageGetter::CHARCOAL)
            .with_circle(80.0);
        let default_text = String::new();
        let error_text = String::from("Invalid input! Please enter a different string.");
        let max_length = 32;
        let validate = Rc::new(|_: &str| true);
        let action_on_ok = Rc::new(|_: String| {});

        Self::new(
            screen,
            label,
            icon,
            default_text,
            error_text,
            max_length,
            validate,
            action_on_ok,
        )
    }
}

impl Popup for AskTextPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        let mut should_close = false;

        // Create a frame for the popup
        egui::Frame::popup(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(300.0);

                // Add icon and label in a horizontal layout
                ui.horizontal(|ui| {
                    ui.add(self.icon.clone());
                    ui.add_space(10.0);
                    ui.label(&self.label);
                });

                ui.add_space(10.0);

                // Add text field
                let text_response = self.text_field.ui(ui);

                // Show error if validation fails
                if self.show_error {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new(&self.error_text).color(Color32::RED));
                }

                ui.add_space(10.0);

                // Add buttons in a horizontal layout
                ui.horizontal(|ui| {
                    // Close button
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }

                    ui.add_space(10.0);

                    // OK button
                    if ui.button("OK").clicked() {
                        let text = self.text_field.text().to_string();

                        // Validate input
                        if (self.validate)(&text) {
                            // Execute action and close
                            (self.action_on_ok)(text);
                            should_close = true;
                        } else {
                            // Show error
                            self.show_error = true;
                        }
                    }
                });
            });

        should_close
    }

    fn title(&self) -> String {
        String::from("Enter Text")
    }
}