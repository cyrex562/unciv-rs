use gdx::graphics::{Pixmap, g2d::BitmapFont};
use crate::ui::components::fonts::font_family_data::FontFamilyData;
use crate::ui::components::fonts::font_metrics_common::FontMetricsCommon;

/// Trait for font implementation that provides font-related functionality
///
/// This trait defines methods for managing font families, generating character pixmaps,
/// and creating bitmap fonts for rendering text in the game.
pub trait FontImplementation {
    /// Sets the font family and size
    fn set_font_family(&mut self, font_family_data: &FontFamilyData, size: i32);

    /// Gets the current font size
    fn get_font_size(&self) -> i32;

    /// Gets a pixmap for a single character
    ///
    /// This is a convenience method that calls `get_char_pixmap_string` with the character
    /// converted to a string.
    fn get_char_pixmap(&self, char: char) -> Pixmap {
        self.get_char_pixmap_string(&char.to_string())
    }

    /// Gets a pixmap for a string (used for diacritic support)
    ///
    /// # Notes
    /// - This method is used for diacritic support to handle both single characters
    ///   and short combinations of diacritics with their target characters
    /// - It still is meant to give one glyph per input
    /// - The desktop implementation currently uses different metrics for char vs string width
    /// - This method was added to ensure nothing changes for non-diacritic languages
    fn get_char_pixmap_string(&self, symbol_string: &str) -> Pixmap;

    /// Gets a list of system fonts available
    fn get_system_fonts(&self) -> Vec<FontFamilyData>;

    /// Gets a bitmap font for rendering
    fn get_bitmap_font(&self) -> BitmapFont {
        let font_data = NativeBitmapFontData::new(self);
        let font = BitmapFont::new(&font_data, &font_data.regions, false);
        font.set_owns_texture(true);
        font
    }

    /// Gets font metrics
    fn get_metrics(&self) -> FontMetricsCommon;
}

/// Native bitmap font data implementation
pub struct NativeBitmapFontData<'a> {
    font_impl: &'a dyn FontImplementation,
    pub regions: Vec<gdx::graphics::g2d::TextureRegion>,
}

impl<'a> NativeBitmapFontData<'a> {
    /// Creates a new NativeBitmapFontData instance
    pub fn new(font_impl: &'a dyn FontImplementation) -> Self {
        Self {
            font_impl,
            regions: Vec::new(),
        }
    }
}