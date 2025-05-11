use std::rc::Rc;
use eframe::egui::{self, Ui, Color32, Response, Rect, Vec2};
use log::info;

use crate::ui::screens::basescreen::BaseScreen;
use crate::constants::{CLOSE, OK};

/// Controls how content may scroll in a popup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scrollability {
    /// No scrolling
    None,
    /// Entire content can scroll if larger than maximum dimensions
    All,
    /// Content separated into scrollable upper part and static lower part containing the buttons
    WithoutButtons,
}

/// Base trait for all Popups, i.e. dialogs that get rendered in the middle of a screen and on top of everything else
pub trait Popup {
    /// Show the popup in the given UI
    /// Returns true if the popup should be closed
    fn show(&mut self, ui: &mut Ui) -> bool;

    /// Get the title of the popup
    fn title(&self) -> String;

    /// Get the screen this popup is associated with
    fn screen(&self) -> &Rc<BaseScreen>;

    /// Get the maximum width of the popup as a percentage of the screen width
    fn max_size_percentage(&self) -> f32 {
        0.9
    }

    /// Get the scrollability of the popup
    fn scrollability(&self) -> Scrollability {
        Scrollability::WithoutButtons
    }

    /// Add a good sized label to the popup
    fn add_good_sized_label(&mut self, ui: &mut Ui, text: &str, size: f32, hide_icons: bool) {
        ui.add_space(10.0);
        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
            ui.heading(text);
        });
        ui.add_space(10.0);
    }

    /// Add a button to the popup
    fn add_button<F>(&mut self, ui: &mut Ui, text: &str, action: F) -> Response
    where
        F: FnOnce() + 'static,
    {
        ui.add_space(10.0);
        let button = ui.button(text);
        if button.clicked() {
            action();
        }
        button
    }

    /// Add a close button to the popup
    fn add_close_button<F>(&mut self, ui: &mut Ui, text: &str, action: Option<F>) -> Response
    where
        F: FnOnce() + 'static,
    {
        let button = ui.button(text);
        if button.clicked() {
            if let Some(action) = action {
                action();
            }
        }
        button
    }

    /// Add an OK button to the popup
    fn add_ok_button<F, V>(&mut self, ui: &mut Ui, text: &str, validate: V, action: F) -> Response
    where
        F: FnOnce() + 'static,
        V: FnOnce() -> bool + 'static,
    {
        let button = ui.button(text);
        if button.clicked() && validate() {
            action();
        }
        button
    }

    /// Equalize the width of the last two buttons
    fn equalize_last_two_button_widths(&mut self, ui: &mut Ui) {
        // In egui, this is handled automatically by the layout system
    }

    /// Reuse this popup as an error/info popup with a new message
    fn reuse_with(&mut self, ui: &mut Ui, new_text: &str, with_close_button: bool) {
        ui.clear();
        self.add_good_sized_label(ui, new_text, 16.0, false);
        if with_close_button {
            self.add_close_button(ui, CLOSE, None::<fn()>);
        }
    }

    /// Get the color for a confirm button based on the action type
    fn get_confirm_color(&self) -> Color32 {
        Color32::from_rgb(0, 200, 0) // Green for confirm actions
    }
}

/// Extension trait for BaseScreen to manage popups
pub trait PopupExt {
    /// Get all active popups
    fn popups(&self) -> Vec<Box<dyn Popup>>;

    /// Get the currently active popup
    fn active_popup(&self) -> Option<&Box<dyn Popup>>;

    /// Check if there are any open popups
    fn has_open_popups(&self) -> bool;

    /// Close all popups
    fn close_all_popups(&mut self);
}

impl PopupExt for BaseScreen {
    fn popups(&self) -> Vec<Box<dyn Popup>> {
        // This would be implemented to return all active popups
        Vec::new()
    }

    fn active_popup(&self) -> Option<&Box<dyn Popup>> {
        // This would be implemented to return the currently active popup
        None
    }

    fn has_open_popups(&self) -> bool {
        // This would be implemented to check if there are any open popups
        false
    }

    fn close_all_popups(&mut self) {
        // This would be implemented to close all popups
    }
}