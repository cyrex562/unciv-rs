// Source: orig_src/core/src/com/unciv/ui/components/widgets/UncivTextField.kt

use eframe::egui::{self, Color32, Response, TextEdit, Ui};
use std::rc::Rc;

/// A custom text field component for Unciv
pub struct UncivTextField {
    label: String,
    text: String,
    max_length: usize,
    filter: Option<Box<dyn Fn(&str, char) -> bool>>,
    hint: Option<String>,
    password: bool,
}

impl UncivTextField {
    /// Create a new UncivTextField with the given label and initial text
    pub fn new(label: &str, initial_text: &str) -> Self {
        Self {
            label: label.to_string(),
            text: initial_text.to_string(),
            max_length: 0, // No limit by default
            filter: None,
            hint: None,
            password: false,
        }
    }

    /// Set the maximum length of the text field
    pub fn set_max_length(&mut self, max_length: usize) {
        self.max_length = max_length;
    }

    /// Set a filter function for the text field
    pub fn set_filter<F>(&mut self, filter: Box<F>)
    where
        F: Fn(&str, char) -> bool + 'static,
    {
        self.filter = Some(filter);
    }

    /// Set a hint text for the text field
    pub fn set_hint(&mut self, hint: String) {
        self.hint = Some(hint);
    }

    /// Set whether the text field should display as a password field
    pub fn set_password(&mut self, password: bool) {
        self.password = password;
    }

    /// Get the current text in the text field
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text in the text field
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    /// Render the text field in the given UI
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        let mut text_edit = TextEdit::singleline(&mut self.text)
            .desired_width(f32::INFINITY);

        // Apply max length if set
        if self.max_length > 0 {
            text_edit = text_edit.max_length(self.max_length);
        }

        // Apply password mode if set
        if self.password {
            text_edit = text_edit.password(true);
        }

        // Apply hint if set
        if let Some(hint) = &self.hint {
            text_edit = text_edit.hint_text(hint);
        }

        // Apply filter if set
        if let Some(filter) = &self.filter {
            let filter_clone = filter.clone();
            text_edit = text_edit.filter(move |text, c| filter_clone(text, c));
        }

        ui.add(text_edit)
    }
}