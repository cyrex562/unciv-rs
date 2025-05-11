use std::rc::Rc;
use eframe::egui::{self, Ui, Color32, Response, Rect, Vec2, ScrollArea};
use log::info;

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::popups::animated_menu_popup::AnimatedMenuPopup;
use crate::ui::popups::Popup;

/// A popup that extends AnimatedMenuPopup to add scrollability to its content.
///
/// This popup separates content into a scrollable upper part and a fixed lower part.
/// The fixed part typically contains buttons or other controls that should always be visible.
pub struct ScrollableAnimatedMenuPopup {
    /// The base animated menu popup
    base: AnimatedMenuPopup,

    /// The maximum width of the popup as a percentage of the screen width
    max_width_percentage: f32,

    /// The maximum height of the popup as a percentage of the screen height
    max_height_percentage: f32,

    /// The scrollable content
    scrollable_content: Option<Box<dyn FnMut(&mut Ui)>>,

    /// The fixed content (typically buttons)
    fixed_content: Option<Box<dyn FnMut(&mut Ui)>>,
}

impl ScrollableAnimatedMenuPopup {
    /// Creates a new ScrollableAnimatedMenuPopup
    pub fn new(screen: &Rc<BaseScreen>, position: Vec2) -> Self {
        let base = AnimatedMenuPopup::new(screen, position);

        Self {
            base,
            max_width_percentage: 0.95,
            max_height_percentage: 0.95,
            scrollable_content: None,
            fixed_content: None,
        }
    }

    /// Sets the scrollable content of the popup
    pub fn set_scrollable_content<F>(&mut self, content: F)
    where
        F: FnMut(&mut Ui) + 'static,
    {
        self.scrollable_content = Some(Box::new(content));
    }

    /// Sets the fixed content of the popup (typically buttons)
    pub fn set_fixed_content<F>(&mut self, content: F)
    where
        F: FnMut(&mut Ui) + 'static,
    {
        self.fixed_content = Some(Box::new(content));
    }

    /// Sets the maximum width percentage of the popup
    pub fn set_max_width_percentage(&mut self, percentage: f32) {
        self.max_width_percentage = percentage;
    }

    /// Sets the maximum height percentage of the popup
    pub fn set_max_height_percentage(&mut self, percentage: f32) {
        self.max_height_percentage = percentage;
    }

    /// Gets the maximum width of the popup
    pub fn max_popup_width(&self) -> f32 {
        self.max_width_percentage * self.base.screen().width() - 5.0
    }

    /// Gets the maximum height of the popup
    pub fn max_popup_height(&self) -> f32 {
        self.max_height_percentage * self.base.screen().height() - 5.0
    }

    /// Shows the popup
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        // Create a frame for the popup
        let max_width = self.max_popup_width();
        let max_height = self.max_popup_height();

        // Create a centered frame for the popup
        let response = egui::Frame::none()
            .fill(Color32::from_black_alpha(200))
            .show(ui.ctx(), |ui| {
                ui.allocate_space(Vec2::new(ui.available_width(), ui.available_height()));

                // Center the popup
                let popup_rect = Rect::from_min_size(
                    ui.available_rect_before_wrap().center() - Vec2::new(max_width / 2.0, max_height / 2.0),
                    Vec2::new(max_width, max_height)
                );

                egui::Frame::none()
                    .fill(Color32::from_gray(40))
                    .rounding(5.0.into())
                    .show(ui, |ui| {
                        ui.set_clip_rect(popup_rect);

                        // Add title
                        ui.heading(self.base.title());
                        ui.add_space(10.0);

                        // Add scrollable content
                        if let Some(content) = &mut self.scrollable_content {
                            ScrollArea::vertical().show(ui, |ui| {
                                content(ui);
                            });
                        }

                        ui.add_space(10.0);

                        // Add fixed content (buttons)
                        if let Some(content) = &mut self.fixed_content {
                            ui.horizontal(|ui| {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    content(ui);
                                });
                            });
                        }

                        // Return true if the popup should be closed
                        false
                    });

                // Return true if the popup should be closed
                false
            });

        // Return true if the popup should be closed
        false
    }

    /// Closes the popup
    pub fn close(&mut self) {
        self.base.close();
    }

    /// Returns a reference to the base animated menu popup
    pub fn base(&self) -> &AnimatedMenuPopup {
        &self.base
    }

    /// Returns a mutable reference to the base animated menu popup
    pub fn base_mut(&mut self) -> &mut AnimatedMenuPopup {
        &mut self.base
    }
}

impl Popup for ScrollableAnimatedMenuPopup {
    fn show(&mut self, ui: &mut Ui) -> bool {
        self.show(ui)
    }

    fn title(&self) -> String {
        self.base.title()
    }

    fn screen(&self) -> &Rc<BaseScreen> {
        self.base.screen()
    }

    fn max_size_percentage(&self) -> f32 {
        self.max_width_percentage
    }

    fn scrollability(&self) -> crate::ui::popups::Scrollability {
        crate::ui::popups::Scrollability::WithoutButtons
    }
}