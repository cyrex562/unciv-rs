use std::rc::Rc;
use std::time::{Duration, Instant};
use eframe::egui::{self, Ui, Color32, Response, Rect, Vec2, Align, TextBuffer};
use log::info;

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::popups::Popup;
use crate::utils::concurrency::Concurrency;

/// An unobtrusive popup which will close itself after a given amount of time.
///
/// - Will show on top of other Popups, but not on top of other ToastPopups
/// - Several calls in a short time will be shown sequentially, each "waiting their turn"
/// - The user can close a Toast by clicking it
/// - Supports color markup using «» instead of standard brackets
pub struct ToastPopup {
    /// The message to display
    message: String,

    /// The screen to show the popup on
    screen: Rc<BaseScreen>,

    /// The duration in milliseconds before the popup automatically closes
    duration_ms: u64,

    /// When the popup was created
    created_at: Option<Instant>,

    /// Whether the popup is visible
    visible: bool,

    /// Whether the popup should be closed
    should_close: bool,

    /// The width of the popup
    width: f32,

    /// The height of the popup
    height: f32,

    /// The position of the popup
    position: Vec2,
}

impl ToastPopup {
    /// Creates a new ToastPopup with the given message and screen
    ///
    /// # Arguments
    ///
    /// * `message` - The message to display
    /// * `screen` - The screen to show the popup on
    /// * `duration_ms` - The duration in milliseconds before the popup automatically closes (default: 2000)
    pub fn new(message: String, screen: &Rc<BaseScreen>, duration_ms: u64) -> Self {
        let screen_width = screen.width();
        let width = screen_width / 2.0;

        Self {
            message,
            screen: Rc::clone(screen),
            duration_ms,
            created_at: None,
            visible: false,
            should_close: false,
            width,
            height: 0.0, // Will be calculated when shown
            position: Vec2::ZERO, // Will be set when shown
        }
    }

    /// Creates a new ToastPopup with the default duration of 2000ms
    pub fn new_default(message: String, screen: &Rc<BaseScreen>) -> Self {
        Self::new(message, screen, 2000)
    }

    /// Shows the popup
    pub fn show(&mut self) {
        // Check if there are any other toast popups visible
        let has_other_toasts = self.screen.has_toast_popups();

        if !has_other_toasts {
            self.visible = true;
            self.created_at = Some(Instant::now());

            // Position the popup at the top of the screen
            let screen_height = self.screen.height();
            self.position = Vec2::new(
                (self.screen.width() - self.width) / 2.0,
                screen_height - (self.height + 20.0)
            );

            // Start the timer to close the popup
            self.start_timer();
        }
    }

    /// Starts the timer to close the popup
    fn start_timer(&mut self) {
        let duration = Duration::from_millis(self.duration_ms);
        let screen = Rc::clone(&self.screen);

        Concurrency::run("ToastPopup", move || {
            std::thread::sleep(duration);
            Concurrency::run_on_gl_thread(move || {
                // Find and close this toast popup
                if let Some(toast) = screen.find_toast_popup() {
                    toast.should_close = true;
                }
            });
        });
    }

    /// Closes the popup
    pub fn close(&mut self) {
        self.should_close = true;
    }

    /// Returns whether the popup is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns whether the popup should be closed
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// Returns the message of the popup
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the width of the popup
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns the height of the popup
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Returns the position of the popup
    pub fn position(&self) -> Vec2 {
        self.position
    }
}

impl Popup for ToastPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        if !self.visible {
            return false;
        }

        // Create a frame for the popup
        let response = egui::Frame::none()
            .fill(Color32::from_black_alpha(200))
            .rounding(5.0.into())
            .show(ui, |ui| {
                // Set the position of the popup
                let rect = Rect::from_min_size(self.position, Vec2::new(self.width, self.height));
                ui.set_clip_rect(rect);

                // Add the message with color markup
                let mut text = self.message.clone();
                // Replace «» with color markup
                text = text.replace("«", "[");
                text = text.replace("»", "]");

                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    ui.label(egui::RichText::new(text).size(14.0));
                });

                // Check if the popup was clicked
                if ui.rect_contains_pointer(rect) && ui.input(|i| i.pointer.primary_clicked()) {
                    self.close();
                }

                // Return true if the popup should be closed
                self.should_close
            });

        // Update the height of the popup
        self.height = response.response.rect.height();

        // Return true if the popup should be closed
        self.should_close
    }

    fn title(&self) -> String {
        String::new() // Toast popups don't have titles
    }

    fn screen(&self) -> &Rc<BaseScreen> {
        &self.screen
    }

    fn max_size_percentage(&self) -> f32 {
        0.5 // Toast popups are half the screen width
    }

    fn scrollability(&self) -> crate::ui::popups::Scrollability {
        crate::ui::popups::Scrollability::None // Toast popups don't scroll
    }
}