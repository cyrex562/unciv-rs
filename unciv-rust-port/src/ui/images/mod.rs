pub mod atlas_preview;
pub mod clipping_image;
pub mod icon_circle_group;
pub mod icon_text_button;
pub mod image_attempter;
pub mod image_getter;
pub mod image_layer;
pub mod image_with_custom_size;
pub mod portrait;



// Source: orig_src/core/src/com/unciv/ui/images/ImageGetter.kt

use eframe::egui::{self, Color32, Image, TextureHandle, Vec2};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use lazy_static::lazy_static;

/// Utility for loading and manipulating images
pub struct ImageGetter {
    // Cache of loaded textures
    texture_cache: HashMap<String, TextureHandle>,
}

// Default colors
impl ImageGetter {
    pub const CHARCOAL: Color32 = Color32::from_rgb(54, 69, 79);
}

// Singleton instance
lazy_static! {
    static ref INSTANCE: Mutex<ImageGetter> = Mutex::new(ImageGetter {
        texture_cache: HashMap::new(),
    });
}

impl ImageGetter {
    /// Get the singleton instance
    pub fn instance() -> &'static Mutex<ImageGetter> {
        &INSTANCE
    }

    /// Initialize the ImageGetter
    pub fn init(&mut self) {
        // Load default textures
        // This would be expanded in a real implementation
    }

    /// Get an image by name
    pub fn get_image(name: &str) -> Image {
        // In a real implementation, this would load the image from the cache or load it from disk
        // For now, we'll return a placeholder
        Image::new(egui::TextureId::default(), Vec2::new(80.0, 80.0))
    }

    /// Create a new image with the given color
    pub fn with_color(self, color: Color32) -> Image {
        // In a real implementation, this would tint the image
        // For now, we'll just return the image
        self
    }

    /// Create a new image with a circle background
    pub fn with_circle(self, size: f32) -> Image {
        // In a real implementation, this would add a circle background
        // For now, we'll just return the image
        self
    }
}