// Source: orig_src/core/src/com/unciv/ui/screens/LoadingScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use egui::{Ui, Color32, Image, TextureHandle};
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::popups::loading_popup::LoadingPopup;
use crate::ui::images::ImageWithCustomSize;

/// A loading screen that creates a screenshot of the current screen and adds a "Loading..." popup on top of that
pub struct LoadingScreen {
    previous_screen: Option<Rc<RefCell<BaseScreen>>>,
    screenshot: Option<TextureHandle>,
    loading_popup: Option<LoadingPopup>,
    image: Option<ImageWithCustomSize>,
    stage_width: f32,
    stage_height: f32,
    loading_start_time: std::time::Instant,
}

impl LoadingScreen {
    pub fn new(previous_screen: Option<Rc<RefCell<BaseScreen>>>, stage_width: f32, stage_height: f32) -> Self {
        let mut screen = Self {
            previous_screen,
            screenshot: None,
            loading_popup: None,
            image: None,
            stage_width,
            stage_height,
            loading_start_time: std::time::Instant::now(),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Take screenshot of previous screen if available
        self.screenshot = self.take_screenshot();

        if let Some(screenshot) = &self.screenshot {
            // Create image from screenshot
            let mut image = ImageWithCustomSize::new(screenshot.clone());
            image.width = self.stage_width;
            image.height = self.stage_height;
            self.image = Some(image);

            // Schedule loading popup to appear after 1 second
            // This will be handled in the update method
        }
    }

    fn take_screenshot(&self) -> Option<TextureHandle> {
        // TODO: Implement screenshot functionality
        // In Kotlin, this uses Gdx.graphics to capture the screen
        // In Rust, we'll need to use a different approach with egui

        // For now, return None as a placeholder
        None
    }

    pub fn update(&mut self, ui: &mut Ui) {
        // Check if 1 second has passed since loading started
        if self.loading_start_time.elapsed() >= Duration::from_secs(1) && self.loading_popup.is_none() {
            // Create and show loading popup
            self.loading_popup = Some(LoadingPopup::new(self.stage_width, self.stage_height));
        }

        // Draw the screenshot image if available
        if let Some(image) = &self.image {
            image.draw(ui);
        }

        // Draw the loading popup if available
        if let Some(popup) = &mut self.loading_popup {
            popup.show(ui);
        }
    }

    pub fn dispose(&mut self) {
        // Clean up resources
        self.screenshot = None;
        self.image = None;
        self.loading_popup = None;
    }
}