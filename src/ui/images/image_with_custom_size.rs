use ggez::graphics::{self, DrawParam, Image, Rect, Point2, Context, GameResult};
use std::path::Path;

/// A custom image implementation that provides proper handling of preferred width and height.
///
/// The standard Image class in ggez has limitations where the drawable's texture width/height
/// is always used as the minimum width/height, and get_pref_width/height always returns
/// the drawable's minWidth/minHeight. This class provides a custom implementation that
/// properly respects the width/height set by set_size.
pub struct ImageWithCustomSize {
    /// The underlying image
    image: Image,
    /// Custom width (if set)
    width: f32,
    /// Custom height (if set)
    height: f32,
}

impl ImageWithCustomSize {
    /// Creates a new ImageWithCustomSize with the given image
    pub fn new(image: Image) -> Self {
        Self {
            image,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Creates a new ImageWithCustomSize from a file path
    pub fn from_file(ctx: &mut Context, path: impl AsRef<Path>) -> GameResult<Self> {
        let image = Image::new(ctx, path)?;
        Ok(Self::new(image))
    }

    /// Sets the custom size for this image
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// Gets the preferred width of this image
    pub fn get_pref_width(&self) -> f32 {
        if self.width > 0.0 {
            self.width
        } else if let Some(drawable) = &self.image {
            drawable.width()
        } else {
            0.0
        }
    }

    /// Gets the preferred height of this image
    pub fn get_pref_height(&self) -> f32 {
        if self.height > 0.0 {
            self.height
        } else if let Some(drawable) = &self.image {
            drawable.height()
        } else {
            0.0
        }
    }

    /// Draws the image with the custom size
    pub fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        if self.width > 0.0 || self.height > 0.0 {
            // Calculate scaling factors based on custom size
            let scale_x = if self.width > 0.0 {
                self.width / self.image.width()
            } else {
                1.0
            };

            let scale_y = if self.height > 0.0 {
                self.height / self.image.height()
            } else {
                1.0
            };

            // Apply scaling to the draw parameter
            let mut scaled_param = param;
            scaled_param.scale = Point2::new(scale_x, scale_y);

            self.image.draw(ctx, scaled_param)
        } else {
            // If no custom size is set, draw normally
            self.image.draw(ctx, param)
        }
    }
}

impl Default for ImageWithCustomSize {
    fn default() -> Self {
        Self {
            image: Image::default(),
            width: 0.0,
            height: 0.0,
        }
    }
}