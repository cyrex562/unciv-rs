use std::collections::HashMap;
use std::f32;
use gdx::graphics::{Color, Pixmap, Texture, g2d::{BitmapFont, GlyphLayout, PixmapPacker, TextureRegion}};
use gdx::utils::{Array, Disposable};
use crate::constants::Constants;
use crate::ui::images::ImageGetter;
use crate::ui::components::fonts::fonts::{Fonts, get_all_symbols};
use crate::ui::components::fonts::font_implementation::FontImplementation;
use crate::ui::components::fonts::font_ruleset_icons::FontRulesetIcons;
use crate::ui::components::fonts::diacritic_support::DiacriticSupport;

/// This class is loosely based on libgdx's FreeTypeBitmapFontData
pub struct NativeBitmapFontData {
    /// The font implementation for the current platform
    font_implementation: Box<dyn FontImplementation>,
    /// The texture regions for the font
    pub regions: Array<TextureRegion>,
    /// Whether the font data is dirty and needs to be updated
    dirty: bool,
    /// The pixmap packer for the font
    packer: PixmapPacker,
    /// The texture filter for the font
    filter: Texture::TextureFilter,
}

impl NativeBitmapFontData {
    /// How to get the alpha channel in a Pixmap.getPixel return value (Int) - it's the LSB */
    const ALPHA_CHANNEL_MASK: i32 = 255;
    /// Where to test circle for transparency */
    /// The center of a squared circle's corner wedge would be at (1-PI/4)/2 ≈ 0.1073 */
    const NEAR_CORNER_RELATIVE_OFFSET: f32 = 0.1;
    /// Where to test circle for opacity */
    /// arbitrary choice just off-center */
    const NEAR_CENTER_RELATIVE_OFFSET: f32 = 0.4;
    /// Width multiplier to get extra advance after a ruleset icon, empiric */
    const RELATIVE_ADVANCE_EXTRA: f32 = 0.039;
    /// Multiplier to get default kerning between a ruleset icon and 'open' characters */
    const RELATIVE_KERNING: f32 = -0.055;
    /// Which follower characters receive how much kerning relative to [relativeKerning] */
    const KERNING_MAP: &'static [(char, f32)] = &[
        ('A', 1.0), ('T', 0.6), ('V', 1.0), ('Y', 1.2)
    ];

    /// Creates a new NativeBitmapFontData instance
    pub fn new(font_implementation: Box<dyn FontImplementation>) -> Self {
        let mut data = Self {
            font_implementation,
            regions: Array::new(),
            dirty: false,
            packer: PixmapPacker::new(1024, 1024, Pixmap::Format::RGBA8888, 1, false, PixmapPacker::GuillotineStrategy::new()),
            filter: Texture::TextureFilter::Linear,
        };

        // set general font data
        data.flipped = false;
        data.line_height = data.font_implementation.get_font_size() as f32;
        data.cap_height = data.line_height;
        data.ascent = -data.line_height;
        data.down = -data.line_height;

        // Set transparent color
        data.packer.transparent_color = Color::WHITE;
        data.packer.transparent_color.a = 0.0;

        // Generate texture regions
        data.packer.update_texture_regions(&mut data.regions, data.filter, data.filter, false);

        // Set space glyph
        let space_glyph = data.get_glyph(' ');
        data.space_xadvance = space_glyph.xadvance as f32;

        data.set_scale(Constants::DEFAULT_FONT_SIZE as f32 / Fonts::ORIGINAL_FONT_SIZE);

        data
    }

    /// Gets a glyph for a character, creating and caching it if necessary
    pub fn get_glyph(&mut self, ch: char) -> &mut BitmapFont::Glyph {
        if let Some(glyph) = self.get_glyph_opt(ch) {
            glyph
        } else {
            self.create_and_cache_glyph(ch)
        }
    }

    /// Gets a glyph for a character if it exists
    fn get_glyph_opt(&mut self, ch: char) -> Option<&mut BitmapFont::Glyph> {
        // This would be implemented to check if the glyph exists in the cache
        // For now, we'll just return None to force creation
        None
    }

    /// Creates and caches a glyph for a character
    fn create_and_cache_glyph(&mut self, ch: char) -> &mut BitmapFont::Glyph {
        let char_pixmap = self.get_pixmap_from_char(ch);

        let mut glyph = BitmapFont::Glyph::new();
        glyph.id = ch as i32;
        glyph.width = char_pixmap.width;
        glyph.height = char_pixmap.height;
        glyph.xadvance = glyph.width;

        // Check alpha to guess whether this is a round icon
        // Needs to be done before disposing charPixmap, and we want to do that soon
        let is_font_ruleset_icon = ch as i32 >= FontRulesetIcons::UNUSED_CHARACTER_CODES_START &&
                                   ch as i32 <= DiacriticSupport::get_current_free_code();
        let assume_round_icon = is_font_ruleset_icon && self.guess_is_round_surrounded_by_transparency(&char_pixmap);

        let rect = self.packer.pack(&char_pixmap);
        char_pixmap.dispose();
        glyph.page = self.packer.pages.len() - 1; // Glyph is always packed into the last page for now.
        glyph.src_x = rect.x as i32;
        glyph.src_y = rect.y as i32;

        if is_font_ruleset_icon {
            self.set_ruleset_icon_geometry(&mut glyph, assume_round_icon);
        }

        // If a page was added, create a new texture region for the incrementally added glyph.
        if self.regions.len() <= glyph.page {
            self.packer.update_texture_regions(&mut self.regions, self.filter, self.filter, false);
        }

        self.set_glyph_region(&mut glyph, &self.regions[glyph.page]);
        self.set_glyph(ch as i32, glyph);
        self.dirty = true;

        // Return the glyph from the cache
        self.get_glyph_opt(ch).unwrap()
    }

    /// Guesses if a pixmap is a round icon surrounded by transparency
    fn guess_is_round_surrounded_by_transparency(&self, pixmap: &Pixmap) -> bool {
        // If a pixel near the center is opaque...
        let near_center_offset = (pixmap.width as f32 * Self::NEAR_CENTER_RELATIVE_OFFSET) as i32;
        if (pixmap.get_pixel(near_center_offset, near_center_offset) & Self::ALPHA_CHANNEL_MASK) == 0 {
            return false;
        }
        // ... and one near a corner is transparent ...
        let near_corner_offset = (pixmap.width as f32 * Self::NEAR_CORNER_RELATIVE_OFFSET) as i32;
        (pixmap.get_pixel(near_corner_offset, near_corner_offset) & Self::ALPHA_CHANNEL_MASK) == 0
        // ... then assume it's a circular icon surrounded by transparency - for kerning
    }

    /// Sets the geometry for a ruleset icon glyph
    fn set_ruleset_icon_geometry(&self, glyph: &mut BitmapFont::Glyph, assume_round_icon: bool) {
        // This is a Ruleset object icon - first avoid "glue"'ing them to the next char..
        // ends up 2px for default font scale, 1px for min, 3px for max
        glyph.xadvance += (glyph.width as f32 * Self::RELATIVE_ADVANCE_EXTRA).round() as i32;

        if !assume_round_icon {
            return;
        }

        // Now, if we guessed it's round, do some kerning, only for the most conspicuous combos.
        // Will look ugly for very unusual Fonts - should we limit this to only default fonts?

        // Kerning is a sparse 2D array of up to 2^16 hints, each stored as byte, so this is
        // costly: kerningMap.size * Fonts.charToRulesetImageActor.size * 512 bytes
        // Which is 1.76MB for vanilla G&K rules.

        // Ends up -3px for default font scale, -2px for minimum, -4px for max
        let default_kerning = (glyph.width as f32 * Self::RELATIVE_KERNING).round() as i32;
        for (ch, kerning) in Self::KERNING_MAP {
            glyph.set_kerning(*ch as i32, (default_kerning as f32 * kerning).round() as i32);
        }
    }

    /// Gets a pixmap for a texture name
    fn get_pixmap_for_texture_name(&self, region_name: &str) -> Pixmap {
        Fonts::extract_pixmap_from_texture_region(&ImageGetter::get_drawable(region_name).region)
    }

    /// Gets a pixmap for a character
    fn get_pixmap_from_char(&self, ch: char) -> Pixmap {
        let symbols = get_all_symbols();
        if let Some(texture_name) = symbols.get(&ch) {
            if ImageGetter::image_exists(texture_name) {
                return self.get_pixmap_for_texture_name(texture_name);
            }
        }

        if let Some(actor) = FontRulesetIcons::get_char_to_ruleset_image_actor().get(&ch) {
            return match FontRulesetIcons::get_pixmap_from_actor(actor) {
                Ok(pixmap) => pixmap,
                Err(_) => {
                    // This sometimes fails with a "Frame buffer couldn't be constructed: incomplete attachment" error, unclear why
                    Pixmap::new(0, 0, Pixmap::Format::RGBA8888) // Empty space
                }
            };
        }

        if DiacriticSupport::is_empty() {
            return self.font_implementation.get_char_pixmap(ch);
        }

        self.font_implementation.get_char_pixmap(DiacriticSupport::get_string_for(ch))
    }

    /// Gets glyphs for a text run
    pub fn get_glyphs(&mut self, run: &mut GlyphLayout::GlyphRun, str: &str, start: usize, end: usize, last_glyph: Option<&mut BitmapFont::Glyph>) {
        self.packer.pack_to_texture = true; // All glyphs added after this are packed directly to the texture.
        self.get_glyphs_impl(run, str, start, end, last_glyph);
        if self.dirty {
            self.dirty = false;
            self.packer.update_texture_regions(&mut self.regions, self.filter, self.filter, false);
        }
    }

    /// Implementation of get_glyphs
    fn get_glyphs_impl(&mut self, run: &mut GlyphLayout::GlyphRun, str: &str, start: usize, end: usize, last_glyph: Option<&mut BitmapFont::Glyph>) {
        // This would be implemented to get glyphs for the text run
        // For now, we'll just leave it empty
    }

    /// Sets a glyph region
    fn set_glyph_region(&mut self, glyph: &mut BitmapFont::Glyph, region: &TextureRegion) {
        // This would be implemented to set the glyph region
        // For now, we'll just leave it empty
    }

    /// Sets a glyph
    fn set_glyph(&mut self, id: i32, glyph: BitmapFont::Glyph) {
        // This would be implemented to set the glyph in the cache
        // For now, we'll just leave it empty
    }
}

impl Disposable for NativeBitmapFontData {
    fn dispose(&mut self) {
        self.packer.dispose();
    }
}