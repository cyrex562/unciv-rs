use std::f32;
use std::regex::Regex;

use ggez::graphics::Color;
use ggez::graphics::Text;

use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::constants::Constants;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::translations::TranslationManager;
use crate::ui::fonts::Fonts;

/// A Label that unlike the original participates correctly in layout
///
/// Major feature: Distribute wrapping points so the overall width is minimized without triggering additional breaks.
/// Caveat: You still need to turn wrap on _after_ instantiation, doing it here in init leads to hell.
pub struct WrappableLabel {
    /// The base Label that this WrappableLabel extends
    base: Label,

    /// The expected width
    expected_width: f32,

    /// The font size
    font_size: i32,

    /// The measured width
    measured_width: f32,

    /// The optimized width
    optimized_width: f32,
}

impl WrappableLabel {
    /// Creates a new WrappableLabel
    ///
    /// # Arguments
    ///
    /// * `text` - Automatically translated text
    /// * `expected_width` - Upper limit for the preferred width the Label will report
    /// * `font_color` - The color of the font
    /// * `font_size` - The size of the font
    /// * `hide_icons` - Whether to hide icons in the text
    pub fn new(
        text: String,
        expected_width: f32,
        font_color: Color,
        font_size: i32,
        hide_icons: bool,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let translated_text = text.tr(hide_icons, hide_icons);

        let mut label = Self {
            base: Label::new(translated_text, skin),
            expected_width,
            font_size,
            measured_width: 0.0,
            optimized_width: f32::MAX,
        };

        // Set up the label style
        if font_color != Color::WHITE || font_size != Constants::default_font_size() {
            let mut style = label.base.style().clone();
            style.font_color = font_color;

            if font_size != Constants::default_font_size() {
                style.font = Fonts::font();
                label.base.set_font_scale(font_size as f32 / Fonts::ORIGINAL_FONT_SIZE as f32);
            }

            label.base.set_style(style);
        }

        label
    }

    /// Sets whether the label should wrap text
    pub fn set_wrap(&mut self, wrap: bool) {
        self.measured_width = self.base.pref_width();
        self.base.set_wrap(wrap);
    }

    /// Gets the measured width
    fn get_measured_width(&self) -> f32 {
        if self.base.wrap() {
            self.measured_width
        } else {
            self.base.pref_width()
        }
    }

    /// Gets the minimum width
    pub fn min_width(&self) -> f32 {
        48.0 // ~ 2 chars
    }

    /// Gets the preferred width
    pub fn pref_width(&self) -> f32 {
        let measured_width = self.get_measured_width();
        measured_width.min(self.expected_width).min(self.optimized_width)
    }

    /// Gets the maximum width
    pub fn max_width(&self) -> f32 {
        self.get_measured_width()
    }

    /// If the label can wrap and needs to, try to determine the minimum width that will still wrap
    /// to the least number of lines possible. Return that value, and set as new prefWidth.
    pub fn optimize_pref_width(&mut self) -> f32 {
        if !self.base.wrap() {
            return self.measured_width;
        }

        let label_rows = (self.measured_width / self.expected_width).floor() + 1.0;
        let mut optimized_width = self.measured_width / label_rows;
        let mut line_width = 0.0;

        let regex = Regex::new(r"\b").unwrap();
        let words = regex.split(&self.base.text());

        for word in words {
            if word.is_empty() {
                continue;
            }

            let word_width = WrappableLabel::new(
                word.to_string(),
                f32::MAX,
                Color::WHITE,
                self.font_size,
                false,
            ).pref_width();

            line_width += word_width;

            if line_width > optimized_width {
                if !word.trim().is_empty() {
                    optimized_width = line_width;
                }
                line_width = 0.0;
            }
        }

        self.optimized_width = optimized_width.min(self.expected_width);
        optimized_width
    }
}

// Implement the necessary traits for WrappableLabel
impl std::ops::Deref for WrappableLabel {
    type Target = Label;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for WrappableLabel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for WrappableLabel {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            expected_width: self.expected_width,
            font_size: self.font_size,
            measured_width: self.measured_width,
            optimized_width: self.optimized_width,
        }
    }
}