use bevy::prelude::*;
use bevy_egui::egui::{self, Align, Color32, Frame, Image, Layout, Rect, ScrollArea, Ui, Vec2};
use std::collections::HashMap;
use std::f32::NAN;
use std::sync::Arc;
use regex::Regex;

use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::ruleset::unique::Unique;
use crate::models::ruleset::validation::RulesetValidator;
use crate::ui::components::widgets::{Button, ImageGetter};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::civilopediascreen::civilopedia_categories::CivilopediaCategories;
use crate::ui::widgets::label::{Label, ColorMarkupLabel};
use crate::utils::constants::DEFAULT_FONT_SIZE;
use crate::utils::logging::Log;
use crate::UncivGame;

/// A formatted line of text with various styling options, icons, and links.
/// This is used for displaying text in the Civilopedia and other UI components.
pub struct FormattedLine {
    /// Text to display.
    pub text: String,
    /// Create link: Line gets a 'Link' icon and is linked to either
    /// an Unciv object (format `category/entryname`) or an external URL.
    pub link: String,
    /// Display an Unciv object's icon inline but do not link (format `category/entryname`).
    pub icon: String,
    /// Display an Image instead of text, sized by `image_size`. Can be a path as understood by
    /// `ImageGetter.get_image` or the name of a png or jpg in ExtraImages.
    pub extra_image: String,
    /// Width of the `extra_image`, height is calculated preserving aspect ratio. Defaults to available width.
    pub image_size: f32,
    /// Text size, defaults to `DEFAULT_FONT_SIZE`. Use `size` or `header` but not both.
    pub size: i32,
    /// Header level. 1 means double text size and decreases from there.
    pub header: i32,
    /// Indentation: 0 = text will follow icons with a little padding,
    /// 1 = aligned to a little more than 3 icons, each step above that adds 30f.
    pub indent: i32,
    /// Defines vertical padding between rows, defaults to 5f.
    pub padding: f32,
    /// Sets text color, accepts 6/3-digit web colors (e.g. #FFA040) or names as defined by egui.
    pub color: String,
    /// Renders a separator line instead of text. Can be combined only with `color` and `size` (line width, default 2)
    pub separator: bool,
    /// Decorates text with a star icon - if set, it receives the `color` instead of the text.
    pub starred: bool,
    /// Centers the line (and turns off wrap)
    pub centered: bool,
    /// Paint a red X over the `icon` or `link` image
    pub icon_crossed: bool,
}

/// Link types that can be used for `FormattedLine.link`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    /// No link
    None,
    /// Internal link to another Civilopedia entry
    Internal,
    /// External link to a URL
    External,
}

/// Icon display options for `FormattedLine.render`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconDisplay {
    /// Show all icons
    All,
    /// Show only link icons
    LinkOnly,
    /// Show no icons
    None,
}

impl FormattedLine {
    /// Create a new FormattedLine with default values
    pub fn new() -> Self {
        Self {
            text: String::new(),
            link: String::new(),
            icon: String::new(),
            extra_image: String::new(),
            image_size: NAN,
            size: i32::MIN_VALUE,
            header: 0,
            indent: 0,
            padding: NAN,
            color: String::new(),
            separator: false,
            starred: false,
            centered: false,
            icon_crossed: false,
        }
    }

    /// Create a new FormattedLine with the given text
    pub fn with_text(text: impl Into<String>) -> Self {
        let mut line = Self::new();
        line.text = text.into();
        line
    }

    /// Create a new FormattedLine with the given link
    pub fn with_link(link: impl Into<String>) -> Self {
        let mut line = Self::new();
        line.link = link.into();
        line
    }

    /// Create a new FormattedLine with the given icon
    pub fn with_icon(icon: impl Into<String>) -> Self {
        let mut line = Self::new();
        line.icon = icon.into();
        line
    }

    /// Create a new FormattedLine with the given extra image
    pub fn with_extra_image(extra_image: impl Into<String>) -> Self {
        let mut line = Self::new();
        line.extra_image = extra_image.into();
        line
    }

    /// Create a new FormattedLine with the given image size
    pub fn with_image_size(image_size: f32) -> Self {
        let mut line = Self::new();
        line.image_size = image_size;
        line
    }

    /// Create a new FormattedLine with the given size
    pub fn with_size(size: i32) -> Self {
        let mut line = Self::new();
        line.size = size;
        line
    }

    /// Create a new FormattedLine with the given header level
    pub fn with_header(header: i32) -> Self {
        let mut line = Self::new();
        line.header = header;
        line
    }

    /// Create a new FormattedLine with the given indent level
    pub fn with_indent(indent: i32) -> Self {
        let mut line = Self::new();
        line.indent = indent;
        line
    }

    /// Create a new FormattedLine with the given padding
    pub fn with_padding(padding: f32) -> Self {
        let mut line = Self::new();
        line.padding = padding;
        line
    }

    /// Create a new FormattedLine with the given color
    pub fn with_color(color: impl Into<String>) -> Self {
        let mut line = Self::new();
        line.color = color.into();
        line
    }

    /// Create a new FormattedLine with separator
    pub fn with_separator() -> Self {
        let mut line = Self::new();
        line.separator = true;
        line
    }

    /// Create a new FormattedLine with starred
    pub fn with_starred() -> Self {
        let mut line = Self::new();
        line.starred = true;
        line
    }

    /// Create a new FormattedLine with centered
    pub fn with_centered() -> Self {
        let mut line = Self::new();
        line.centered = true;
        line
    }

    /// Create a new FormattedLine with icon crossed
    pub fn with_icon_crossed() -> Self {
        let mut line = Self::new();
        line.icon_crossed = true;
        line
    }

    /// Create a new FormattedLine from a Unique
    pub fn from_unique(unique: &Unique, indent: i32) -> Self {
        let text = unique.get_display_text();
        let link = Self::get_unique_link(unique);
        Self::new()
            .with_text(text)
            .with_link(link)
            .with_indent(indent)
    }

    /// Get the link type for this FormattedLine
    pub fn link_type(&self) -> LinkType {
        if self.link.starts_with("http://") || self.link.starts_with("https://") || self.link.starts_with("mailto:") {
            LinkType::External
        } else if !self.link.is_empty() {
            LinkType::Internal
        } else {
            LinkType::None
        }
    }

    /// Get the alignment for this FormattedLine
    pub fn align(&self) -> Align {
        if self.centered {
            Align::Center
        } else {
            Align::Left
        }
    }

    /// Get the icon to display for this FormattedLine
    fn icon_to_display(&self) -> String {
        if !self.icon.is_empty() {
            self.icon.clone()
        } else if self.link_type() == LinkType::Internal {
            self.link.clone()
        } else {
            String::new()
        }
    }

    /// Get the text to display for this FormattedLine
    fn text_to_display(&self) -> String {
        if self.text.is_empty() && self.link_type() == LinkType::External {
            self.link.clone()
        } else {
            self.text.clone()
        }
    }

    /// Get the display color for this FormattedLine
    fn display_color(&self) -> Color32 {
        self.parse_color().unwrap_or(Color32::WHITE)
    }

    /// Check if this FormattedLine is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.extra_image.is_empty() &&
            !self.starred && self.icon.is_empty() && self.link.is_empty() && !self.separator
    }

    /// Check if this FormattedLine has normal content (not empty, separator, or extra image)
    fn has_normal_content(&self) -> bool {
        !self.text.is_empty() || !self.link.is_empty() || !self.icon.is_empty() ||
        !self.color.is_empty() || self.size != i32::MIN_VALUE || self.header != 0 || self.starred
    }

    /// Check if the given link is a valid internal link
    fn is_valid_internal_link(link: &str) -> bool {
        let re = Regex::new(r"^[^/]+/[^/]+$").unwrap();
        re.is_match(link)
    }

    /// Check if a string has a protocol (http://, https://, mailto:)
    fn has_protocol(s: &str) -> bool {
        s.starts_with("http://") || s.starts_with("https://") || s.starts_with("mailto:")
    }

    /// Check if a section of a string is composed entirely of hex digits
    fn is_hex(s: &str, start: usize, length: usize) -> bool {
        if length == 0 || start + length > s.len() {
            return false;
        }

        let substring = &s[start..start + length];
        substring.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Parse a color string to a Color32
    fn parse_color(&self) -> Option<Color32> {
        if self.color.is_empty() {
            return None;
        }

        if self.color.starts_with('#') && Self::is_hex(&self.color, 1, 3) {
            if Self::is_hex(&self.color, 1, 6) {
                return Some(self.parse_hex_color(&self.color));
            }

            // Convert 3-digit hex to 6-digit hex
            let hex6 = format!(
                "#{}{}{}{}{}{}",
                self.color.chars().nth(1).unwrap(),
                self.color.chars().nth(1).unwrap(),
                self.color.chars().nth(2).unwrap(),
                self.color.chars().nth(2).unwrap(),
                self.color.chars().nth(3).unwrap(),
                self.color.chars().nth(3).unwrap()
            );

            return Some(self.parse_hex_color(&hex6));
        }

        // Try to parse named colors
        match self.color.to_uppercase().as_str() {
            "WHITE" => Some(Color32::WHITE),
            "BLACK" => Some(Color32::BLACK),
            "RED" => Some(Color32::RED),
            "GREEN" => Some(Color32::GREEN),
            "BLUE" => Some(Color32::BLUE),
            "YELLOW" => Some(Color32::YELLOW),
            "GRAY" => Some(Color32::GRAY),
            "LIGHT_GRAY" => Some(Color32::LIGHT_GRAY),
            "DARK_GRAY" => Some(Color32::DARK_GRAY),
            _ => None,
        }
    }

    /// Parse a hex color string to a Color32
    fn parse_hex_color(&self, hex: &str) -> Color32 {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        Color32::from_rgb(r, g, b)
    }

    /// Get the unique link for a Unique
    fn get_unique_link(unique: &Unique) -> String {
        let ruleset = Self::get_current_ruleset();
        let all_object_names_category_map = Self::init_names_category_map(&ruleset);

        for parameter in unique.params.iter().chain(unique.modifiers.iter().flat_map(|m| m.params.iter())) {
            if let Some(category) = all_object_names_category_map.get(parameter) {
                return format!("{}/{}", category.name(), parameter);
            }
        }

        String::new()
    }

    /// Get the current ruleset
    fn get_current_ruleset() -> Ruleset {
        if !UncivGame::is_current_initialized() {
            Ruleset::new()
        } else if UncivGame::current().game_info.is_none() {
            RulesetCache::get("Civ V - Vanilla").unwrap().clone()
        } else {
            UncivGame::current().game_info.as_ref().unwrap().ruleset.clone()
        }
    }

    /// Initialize the names category map
    fn init_names_category_map(ruleset: &Ruleset) -> HashMap<String, CivilopediaCategories> {
        let mut result = HashMap::new();

        // Add entries for each category
        Self::all_object_maps_sequence(ruleset)
            .flat_map(|(category, map)| {
                map.keys().map(move |key| (category, key.clone()))
            })
            .for_each(|(category, key)| {
                result.insert(key, category);
            });

        // Add special entries
        result.insert("Maya Long Count calendar cycle".to_string(), CivilopediaCategories::Tutorial);

        result
    }

    /// Get all object maps sequence
    fn all_object_maps_sequence(ruleset: &Ruleset) -> impl Iterator<Item = (CivilopediaCategories, &HashMap<String, Box<dyn INamed>>)> {
        let mut sequence = Vec::new();

        sequence.push((CivilopediaCategories::Belief, &ruleset.beliefs));
        sequence.push((CivilopediaCategories::Difficulty, &ruleset.difficulties));
        sequence.push((CivilopediaCategories::Promotion, &ruleset.unit_promotions));
        sequence.push((CivilopediaCategories::Policy, &ruleset.policies));
        sequence.push((CivilopediaCategories::Terrain, &ruleset.terrains));
        sequence.push((CivilopediaCategories::Improvement, &ruleset.tile_improvements));
        sequence.push((CivilopediaCategories::Resource, &ruleset.tile_resources));
        sequence.push((CivilopediaCategories::Nation, &ruleset.nations));
        sequence.push((CivilopediaCategories::UnitType, &ruleset.unit_types));
        sequence.push((CivilopediaCategories::Unit, &ruleset.units));
        sequence.push((CivilopediaCategories::Technology, &ruleset.technologies));
        sequence.push((CivilopediaCategories::Building, &ruleset.buildings.iter().filter(|(_, b)| !b.is_any_wonder()).collect()));
        sequence.push((CivilopediaCategories::Wonder, &ruleset.buildings.iter().filter(|(_, b)| b.is_any_wonder()).collect()));

        sequence.into_iter()
    }

    /// Render this FormattedLine to a UI element
    pub fn render(&self, label_width: f32, icon_display: IconDisplay) -> egui::Frame {
        if !self.extra_image.is_empty() {
            return self.render_extra_image(label_width);
        }

        let font_size = if self.header > 0 && self.header < Self::HEADER_SIZES.len() as i32 {
            Self::HEADER_SIZES[self.header as usize]
        } else if self.size == i32::MIN_VALUE {
            DEFAULT_FONT_SIZE
        } else {
            self.size as f32
        };

        let label_color = if self.starred { Color32::WHITE } else { self.display_color() };

        let mut frame = egui::Frame::new();
        let mut icon_count = 0;
        let icon_size = f32::max(Self::MIN_ICON_SIZE, font_size * 1.5);

        if self.link_type() != LinkType::None && icon_display == IconDisplay::All {
            frame.add(ImageGetter::get_image(Self::LINK_IMAGE).size(icon_size).pad_right(Self::ICON_PAD));
            icon_count += 1;
        }

        if icon_display != IconDisplay::None {
            icon_count += self.render_icon(&mut frame, &self.icon_to_display(), icon_size);
        }

        if self.starred {
            let mut image = ImageGetter::get_image(Self::STAR_IMAGE);
            image.color = self.display_color();
            frame.add(image).size(icon_size).pad_right(Self::ICON_PAD);
            icon_count += 1;
        }

        if !self.text_to_display().is_empty() {
            let used_width = icon_count as f32 * (icon_size + Self::ICON_PAD);
            let indent_width = if self.centered {
                -used_width
            } else if self.indent == 0 && icon_count == 0 {
                0.0
            } else if self.indent == 0 {
                Self::ICON_PAD
            } else if icon_count == 0 {
                self.indent as f32 * Self::INDENT_PAD - used_width
            } else {
                (self.indent - 1) as f32 * Self::INDENT_PAD +
                    Self::INDENT_ONE_AT_NUM_ICONS as f32 * (Self::MIN_ICON_SIZE + Self::ICON_PAD) + Self::ICON_PAD - used_width
            };

            let label = if self.text_to_display().contains('«') {
                ColorMarkupLabel::new(&self.text_to_display(), font_size, icon_count != 0)
            } else {
                Label::new(&self.text_to_display()).with_color(label_color).with_font_size(font_size).with_hide_icons(icon_count != 0)
            };

            if label_width == 0.0 {
                frame.add(label)
                    .pad_left(f32::max(0.0, indent_width))
                    .pad_right(f32::max(0.0, -indent_width))
                    .align(self.align());
            } else {
                frame.add(label)
                    .width(label_width - used_width - indent_width)
                    .pad_left(indent_width)
                    .align(self.align());
            }
        }

        frame
    }

    /// Render an extra image
    fn render_extra_image(&self, label_width: f32) -> egui::Frame {
        let mut frame = egui::Frame::new();

        if let Some(image) = self.get_extra_image() {
            // Limit larger coordinate to a given max size
            let max_size = if self.image_size.is_nan() { label_width } else { self.image_size };

            let (width, height) = if image.width() > image.height() {
                (max_size, max_size * image.height() as f32 / image.width() as f32)
            } else {
                (max_size * image.width() as f32 / image.height() as f32, max_size)
            };

            frame.add(image).size(width, height);
        }

        frame
    }

    /// Get the extra image
    fn get_extra_image(&self) -> Option<egui::Image> {
        if ImageGetter::image_exists(&self.extra_image) {
            if self.centered {
                Some(ImageGetter::get_drawable(&self.extra_image).crop_to_content())
            } else {
                Some(ImageGetter::get_image(&self.extra_image))
            }
        } else if let Some(external_image) = ImageGetter::find_external_image(&self.extra_image) {
            Some(ImageGetter::get_external_image(&external_image))
        } else {
            None
        }
    }

    /// Render an icon
    fn render_icon(&self, frame: &mut egui::Frame, icon_to_display: &str, icon_size: f32) -> i32 {
        // prerequisites: iconToDisplay has form "category/name", category can be mapped to
        // a `CivilopediaCategories`, and that knows how to get an Image.
        if icon_to_display.is_empty() {
            return 0;
        }

        let parts: Vec<&str> = icon_to_display.split('/').collect();
        if parts.len() != 2 {
            return 0;
        }

        let category = CivilopediaCategories::from_link(parts[0])?;
        let image = category.get_image(parts[1], icon_size)?;

        if self.icon_crossed {
            frame.add(ImageGetter::get_crossed_image(image, icon_size)).size(icon_size).pad_right(Self::ICON_PAD);
        } else {
            frame.add(image).size(icon_size).pad_right(Self::ICON_PAD);
        }

        1
    }

    /// Convert to string for debugging
    pub fn to_string(&self) -> String {
        if self.is_empty() {
            "(empty)".to_string()
        } else if self.separator {
            "(separator)".to_string()
        } else if !self.extra_image.is_empty() {
            format!("(extraImage='{}')", self.extra_image)
        } else if self.header > 0 {
            format!("(header={})'{}'", self.header, self.text)
        } else if self.link_type() == LinkType::None {
            format!("'{}'", self.text)
        } else {
            format!("'{}'->{}", self.text, self.link)
        }
    }

    /// Constants used by FormattedLine
    pub const HEADER_SIZES: [f32; 9] = [DEFAULT_FONT_SIZE, 36.0, 32.0, 27.0, 24.0, 21.0, 15.0, 12.0, 9.0];
    pub const MIN_ICON_SIZE: f32 = 15.0;
    pub const ICON_PAD: f32 = 5.0;
    pub const INDENT_PAD: f32 = 30.0;
    pub const INDENT_ONE_AT_NUM_ICONS: i32 = 3;
    pub const LINK_IMAGE: &str = "OtherIcons/Link";
    pub const STAR_IMAGE: &str = "OtherIcons/Star";
}

/// Extension trait for TextureRegionDrawable
pub trait TextureRegionDrawableExt {
    /// Crop to content
    fn crop_to_content(&self) -> egui::Image;

    /// Get content size
    fn get_content_size(&self) -> IntRectangle;
}

/// Extension trait for Pixmap
pub trait PixmapExt {
    /// Check if a row is empty
    fn is_row_empty(&self, bounds: &IntRectangle, relative_y: i32) -> bool;

    /// Check if a column is empty
    fn is_column_empty(&self, bounds: &IntRectangle, relative_x: i32) -> bool;
}

/// Rectangle with integer coordinates
#[derive(Debug, Clone, Copy)]
pub struct IntRectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl IntRectangle {
    /// Create a new IntRectangle
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a copy of this IntRectangle
    pub fn copy(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Grow this IntRectangle by the given amount
    pub fn grow(&mut self, h: i32, v: i32) {
        self.x -= h;
        self.width += h + h;
        self.y -= v;
        self.height += v + v;
    }

    /// Get the intersection of this IntRectangle and another
    pub fn intersection(&self, r: &IntRectangle) -> IntRectangle {
        let tx1 = i32::max(self.x, r.x);
        let ty1 = i32::max(self.y, r.y);
        let tx2 = i32::min(self.x + self.width, r.x + r.width);
        let ty2 = i32::min(self.y + self.height, r.y + r.height);
        IntRectangle::new(tx1, ty1, tx2 - tx1, ty2 - ty1)
    }
}

/// Trait for objects that can be named
pub trait INamed {
    /// Get the name of this object
    fn name(&self) -> &str;

    /// Get the sort group of this object
    fn sort_group(&self, ruleset: &Ruleset) -> i32;

    /// Check if this object is named
    fn is_named(&self) -> bool;
}

/// Trait for objects that can be displayed in the Civilopedia
pub trait ICivilopediaText {
    /// Get the civilopedia text header
    fn get_civilopedia_text_header(&self) -> Option<FormattedLine>;

    /// Get the civilopedia text
    fn civilopedia_text(&self) -> Vec<FormattedLine>;

    /// Get the civilopedia text lines
    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine>;

    /// Get the icon name
    fn get_icon_name(&self) -> String;

    /// Make a link
    fn make_link(&self) -> String;
}