use std::collections::HashMap;
use std::f32;
use gdx::graphics::{Pixmap, g2d::{BitmapFont, TextureRegion}};
use crate::gui::GUI;
use crate::UncivGame;
use crate::ui::components::MayaCalendar;
use crate::ui::components::extensions::get_readonly_pixmap;
use crate::ui::components::fonts::font_implementation::FontImplementation;
use crate::ui::components::fonts::font_family_data::FontFamilyData;

/// The "Font manager"
///
/// * We have one global `font`, held by this module.
/// * Platform-dependent code is linked through `font_implementation`.
/// * Most of the work happens in `get_glyph` in `NativeBitmapFontData`. It dispatches to one of three handlers:
///   - Normal text goes to the platform specific implementation to fetch a glyph as pixels from the system.
///   - A set of "symbols" (for strength, movement, death, war, gold and some others)
///     comes from the texture atlas and is implemented in `extract_pixmap_from_texture_region`.
///     They use Unicode code points which normally hold related symbols.
///   - Icons for Ruleset objects are pre-build as Actors then drawn as pixels in `FontRulesetIcons`.
///     They use code points from the 'Private use' range - see comments over there.
///
/// @see NativeBitmapFontData
/// @see DesktopFont
/// @see AndroidFont
/// @see extract_pixmap_from_texture_region
/// @see FontRulesetIcons
pub struct Fonts {
    /// All text is originally rendered in 50px (set in AndroidLauncher and DesktopLauncher), and then scaled to fit the size of the text we need now.
    /// This has several advantages: It means we only render each character once (good for both runtime and RAM),
    /// AND it means that our 'custom' emojis only need to be once size (50px) and they'll be rescaled for what's needed.
    pub const ORIGINAL_FONT_SIZE: f32 = 50.0;
    pub const DEFAULT_FONT_FAMILY: &'static str = "";

    /// The font implementation for the current platform
    pub font_implementation: Box<dyn FontImplementation>,
    /// The bitmap font used for rendering
    pub font: BitmapFont,
}

impl Fonts {
    /// Creates a new Fonts instance
    pub fn new(font_implementation: Box<dyn FontImplementation>) -> Self {
        Self {
            font_implementation,
            font: BitmapFont::new(),
        }
    }

    /// This resets all cached font data in the Fonts struct.
    /// Do not call from normal code - reset the Skin instead: `BaseScreen.set_skin()`
    pub fn reset_font(&mut self) {
        let settings = GUI::get_settings();
        self.font_implementation.set_font_family(&settings.font_family_data, settings.get_font_size());
        self.font = self.font_implementation.get_bitmap_font();
        self.font.data.markup_enabled = true;
    }

    /// Reduce the font list returned by platform-specific code to font families (plain variant if possible)
    pub fn get_system_fonts(&self) -> Vec<FontFamilyData> {
        let mut fonts = self.font_implementation.get_system_fonts();
        fonts.sort_by(|a, b| {
            UncivGame::current().settings.get_collator_from_locale()
                .compare(&a.local_name, &b.local_name)
        });
        fonts
    }

    /// Helper for v-centering the text of Icon – Label -type components:
    ///
    /// Normal vertical centering uses the entire font height. In reality,
    /// it is customary to align the centre from the baseline to the ascent
    /// with the centre of the other element. This function estimates the
    /// correct amount to shift the text element.
    pub fn get_descender_height(&self, font_size: i32) -> f32 {
        let metrics = self.font_implementation.get_metrics();
        let ratio = metrics.descent / metrics.height;
        // For whatever reason, undershooting the adjustment slightly
        // causes rounding to work better
        ratio * font_size as f32 + 2.25
    }

    /// Turn a TextureRegion into a Pixmap.
    ///
    /// `.dispose()` must be called on the returned Pixmap when it is no longer needed, or else it will leave a memory leak behind.
    ///
    /// @return New Pixmap with all the size and pixel data from this TextureRegion copied into it.
    // From https://stackoverflow.com/questions/29451787/libgdx-textureregion-to-pixmap
    pub fn extract_pixmap_from_texture_region(&self, texture_region: &TextureRegion) -> Pixmap {
        let metrics = self.font_implementation.get_metrics();
        let box_height = f32::ceil(metrics.height) as i32;
        let box_width = f32::ceil(metrics.ascent * texture_region.region_width as f32 / texture_region.region_height as f32) as i32;

        // In case the region's aspect isn't 1:1, scale the rounded-up width back to a height with unrounded aspect ratio
        // (using integer math only, do the equivalent of float math rounded to closest integer)
        let draw_height = (2 * texture_region.region_height * box_width + 1) / texture_region.region_width / 2;

        // place region from top of bounding box down
        // Adding half the descent is empiric - should theoretically be leading only
        let draw_y = f32::ceil(metrics.leading + metrics.descent * 0.5) as i32;

        let texture_data = &texture_region.texture.texture_data;
        let texture_data_pixmap = texture_data.get_readonly_pixmap();

        let pixmap = Pixmap::new(box_width, box_height, texture_data.format);

        // We're using the scaling draw_pixmap so pixmap.filter is relevant - it defaults to BiLinear
        pixmap.draw_pixmap(
            texture_data_pixmap,              // The source Pixmap
            texture_region.region_x,          // The source x-coordinate (top left corner)
            texture_region.region_y,          // The source y-coordinate (top left corner)
            texture_region.region_width,      // The width of the area from the other Pixmap in pixels
            texture_region.region_height,     // The height of the area from the other Pixmap in pixels
            0,                                // The target x-coordinate (top left corner)
            draw_y,                          // The target y-coordinate (top left corner)
            box_width,                       // The target width
            draw_height,                     // The target height
        );

        pixmap
    }
}

// Symbols added to font from atlas textures
pub const TURN: char = '⏳';               // U+23F3 'hourglass'
pub const STRENGTH: char = '†';            // U+2020 'dagger'
pub const RANGED_STRENGTH: char = '‡';      // U+2021 'double dagger'
pub const MOVEMENT: char = '➡';            // U+27A1 'black rightwards arrow'
pub const RANGE: char = '…';               // U+2026 'horizontal ellipsis'
pub const HEALTH: char = '♡';              // U+2661 'white heart suit'
pub const PRODUCTION: char = '⚙';          // U+2699 'gear'
pub const GOLD: char = '¤';                // U+00A4 'currency sign'
pub const FOOD: char = '⁂';                // U+2042 'asterism' (to avoid 🍏 U+1F34F 'green apple' needing 2 symbols in utf-16 and 4 in utf-8)
pub const SCIENCE: char = '⍾';             // U+237E 'bell symbol' (🧪 U+1F9EA 'test tube', 🔬 U+1F52C 'microscope')
pub const CULTURE: char = '♪';             // U+266A 'eighth note' (🎵 U+1F3B5 'musical note')
pub const HAPPINESS: char = '⌣';           // U+2323 'smile' (😀 U+1F600 'grinning face')
pub const FAITH: char = '☮';               // U+262E 'peace symbol' (🕊 U+1F54A 'dove of peace')
pub const GREAT_ARTIST: char = '♬';         // U+266C 'sixteenth note'
pub const GREAT_ENGINEER: char = '⚒';       // U+2692 'hammer'
pub const GREAT_GENERAL: char = '⛤';        // U+26E4 'pentagram'
pub const GREAT_MERCHANT: char = '⚖';       // U+2696 'scale'
pub const GREAT_SCIENTIST: char = '⚛';      // U+269B 'atom'
pub const DEATH: char = '☠';               // U+2620 'skull and crossbones'
pub const AUTOMATE: char = '⛏';            // U+26CF 'pick'

// Symbols that can be optionally added to the font from atlas textures
// (a mod can override these, otherwise the font supplies the glyph)
pub const INFINITY: char = '∞';            // U+221E
pub const CLOCK: char = '⌚';               // U+231A 'watch'
pub const STAR: char = '✯';                // U+272F 'pinwheel star'
pub const STATUS: char = '◉';              // U+25C9 'fisheye'
// The following two are used for sort visualization.
// They may disappear (show as placeholder box) on Linux if you clean out asian fonts.
// Alternatives: "↑" U+2191, "↓" U+2193 - much wider and weird spacing in some fonts (e.g. Verdana).
// These are possibly the highest codepoints in use in Unciv -
// Taken into account when limiting FontRulesetIcons codepoints (it respects the private area ending at U+F8FF)
pub const SORT_UP_ARROW: char = '￪';         // U+FFEA 'half wide upward arrow'
pub const SORT_DOWN_ARROW: char = '￬';       // U+FFEC 'half wide downward arrow'
pub const RIGHT_ARROW: char = '→';          // U+2192, e.g. Battle table or event-based tutorials

/// Map of all symbols to their texture paths
pub fn get_all_symbols() -> HashMap<char, String> {
    let mut symbols = HashMap::new();

    symbols.insert(TURN, "EmojiIcons/Turn".to_string());
    symbols.insert(STRENGTH, "StatIcons/Strength".to_string());
    symbols.insert(RANGED_STRENGTH, "StatIcons/RangedStrength".to_string());
    symbols.insert(RANGE, "StatIcons/Range".to_string());
    symbols.insert(MOVEMENT, "StatIcons/Movement".to_string());
    symbols.insert(PRODUCTION, "EmojiIcons/Production".to_string());
    symbols.insert(GOLD, "EmojiIcons/Gold".to_string());
    symbols.insert(FOOD, "EmojiIcons/Food".to_string());
    symbols.insert(SCIENCE, "EmojiIcons/Science".to_string());
    symbols.insert(CULTURE, "EmojiIcons/Culture".to_string());
    symbols.insert(HAPPINESS, "EmojiIcons/Happiness".to_string());
    symbols.insert(FAITH, "EmojiIcons/Faith".to_string());
    symbols.insert(GREAT_ARTIST, "EmojiIcons/Great Artist".to_string());
    symbols.insert(GREAT_ENGINEER, "EmojiIcons/Great Engineer".to_string());
    symbols.insert(GREAT_GENERAL, "EmojiIcons/Great General".to_string());
    symbols.insert(GREAT_MERCHANT, "EmojiIcons/Great Merchant".to_string());
    symbols.insert(GREAT_SCIENTIST, "EmojiIcons/Great Scientist".to_string());
    symbols.insert(DEATH, "EmojiIcons/Death".to_string());
    symbols.insert(AUTOMATE, "EmojiIcons/Automate".to_string());
    symbols.insert(INFINITY, "EmojiIcons/Infinity".to_string());
    symbols.insert(CLOCK, "EmojiIcons/SortedByTime".to_string());
    symbols.insert(STAR, "EmojiIcons/Star".to_string());
    symbols.insert(STATUS, "EmojiIcons/SortedByStatus".to_string());
    symbols.insert(SORT_UP_ARROW, "EmojiIcons/SortedAscending".to_string());
    symbols.insert(SORT_DOWN_ARROW, "EmojiIcons/SortedDescending".to_string());
    symbols.insert(RIGHT_ARROW, "EmojiIcons/RightArrow".to_string());

    // Add Maya calendar symbols
    for (symbol, path) in MayaCalendar::get_all_symbols() {
        symbols.insert(symbol, path);
    }

    symbols
}

// Implement singleton pattern
lazy_static::lazy_static! {
    pub static ref FONTS: std::sync::Mutex<Fonts> = std::sync::Mutex::new(Fonts::new(
        Box::new(crate::ui::components::fonts::desktop_font::DesktopFont::new())
    ));
}