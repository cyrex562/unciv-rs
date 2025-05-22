use ggez::graphics::{self, DrawParam, Image, Rect};
use ggez::mint::Point2;
use ggez::Context;
use ggez::GameResult;

/// A class that draws multiple images layered on top of each other.
pub struct ImageLayer {
    images: Vec<(Image, Point2<f32>)>,
    width: f32,
    height: f32,
}

impl ImageLayer {
    /// Creates a new ImageLayer.
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            width: 0.0,
            height: 0.0,
        }
    }

    /// Adds an image to the layer at the specified position.
    pub fn add_image(&mut self, image: Image, x: f32, y: f32) {
        self.images.push((image, Point2::from([x, y])));
        self.update_dimensions();
    }

    /// Clears all images from the layer.
    pub fn clear(&mut self) {
        self.images.clear();
        self.width = 0.0;
        self.height = 0.0;
    }

    /// Gets the width of the layer.
    pub fn get_width(&self) -> f32 {
        self.width
    }

    /// Gets the height of the layer.
    pub fn get_height(&self) -> f32 {
        self.height
    }

    /// Updates the dimensions of the layer based on the images it contains.
    fn update_dimensions(&mut self) {
        self.width = 0.0;
        self.height = 0.0;

        for (image, pos) in &self.images {
            let image_width = image.width() as f32;
            let image_height = image.height() as f32;

            self.width = self.width.max(pos.x + image_width);
            self.height = self.height.max(pos.y + image_height);
        }
    }

    /// Draws all images in the layer.
    pub fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult {
        for (image, pos) in &self.images {
            image.draw(
                ctx,
                DrawParam::new()
                    .dest(*pos)
                    .color([1.0, 1.0, 1.0, parent_alpha]),
            )?;
        }
        Ok(())
    }
}

impl Default for ImageLayer {
    fn default() -> Self {
        Self::new()
    }
}