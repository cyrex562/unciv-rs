use ggez::graphics::{Color, DrawParam, Text};
use ggez::input::keyboard::KeyCode;
use std::sync::Arc;

use crate::ui::components::input::{KeyCharAndCode, KeyShortcutDispatcher};
use crate::ui::components::widgets::text_field::TextField;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::screens::basescreen::BaseScreen;

/// A text field with two deviations from the default TextField:
/// - Turns off color markup support while drawing, so [] in the text display properly
/// - If this TextField handles the Tab key, its focus navigation feature is disabled
pub struct TextFieldWithFixes {
    /// The base TextField that this TextFieldWithFixes extends
    base: TextField,

    /// The text content
    text: String,

    /// The message text (placeholder)
    message_text: String,

    /// Whether there is a text selection
    has_selection: bool,

    /// The cursor position
    cursor: usize,

    /// The selection start position
    selection_start: usize,

    /// The text alignment
    alignment: TextAlignment,

    /// Whether the field is in password mode
    is_password_mode: bool,

    /// Whether to show the onscreen keyboard
    onscreen_keyboard: bool,

    /// The text field filter
    text_field_filter: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl TextFieldWithFixes {
    /// Creates a new TextFieldWithFixes with the given text and style
    pub fn new(text: String, style: TextFieldStyle) -> Self {
        Self {
            base: TextField::new(text.clone(), style),
            text,
            message_text: String::new(),
            has_selection: false,
            cursor: 0,
            selection_start: 0,
            alignment: TextAlignment::Left,
            is_password_mode: false,
            onscreen_keyboard: false,
            text_field_filter: None,
        }
    }

    /// Creates a new TextFieldWithFixes from an existing one
    pub fn from_text_field(text_field: &TextFieldWithFixes) -> Self {
        Self {
            base: text_field.base.clone(),
            text: text_field.text.clone(),
            message_text: text_field.message_text.clone(),
            has_selection: text_field.has_selection,
            cursor: text_field.cursor,
            selection_start: text_field.selection_start,
            alignment: text_field.alignment,
            is_password_mode: text_field.is_password_mode,
            onscreen_keyboard: text_field.onscreen_keyboard,
            text_field_filter: text_field.text_field_filter.clone(),
        }
    }

    /// Copies text and selection from another TextFieldWithFixes
    pub fn copy_text_and_selection(&mut self, text_field: &TextFieldWithFixes) {
        self.text = text_field.text.clone();
        self.has_selection = text_field.has_selection;
        self.cursor = text_field.cursor;
        self.selection_start = text_field.selection_start;
    }

    /// Overrides the next method to handle tab key differently
    pub fn next(&mut self, up: bool) {
        // If this TextField handles the Tab key, disable focus navigation
        if KeyCharAndCode::TAB.is_in_shortcuts() {
            return;
        }
        self.base.next(up);
    }

    /// Overrides the layout method to disable markup during layout
    pub fn layout(&mut self) {
        // Store the old markup enabled state
        let old_enable = self.base.style().font.data.markup_enabled;

        // Disable markup during layout
        self.base.style_mut().font.data.markup_enabled = false;

        // Call the base layout method
        self.base.layout();

        // Restore the markup enabled state
        self.base.style_mut().font.data.markup_enabled = old_enable;
    }

    /// Overrides the draw_text method to disable markup during drawing
    pub fn draw_text(&self, ctx: &mut ggez::Context, x: f32, y: f32) {
        // Store the old markup enabled state
        let old_enable = self.base.style().font.data.markup_enabled;

        // Disable markup during drawing
        self.base.style_mut().font.data.markup_enabled = false;

        // Call the base draw_text method
        self.base.draw_text(ctx, x, y);

        // Restore the markup enabled state
        self.base.style_mut().font.data.markup_enabled = old_enable;
    }
}

// Implement the necessary traits for TextFieldWithFixes
impl std::ops::Deref for TextFieldWithFixes {
    type Target = TextField;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for TextFieldWithFixes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for TextFieldWithFixes {
    fn clone(&self) -> Self {
        Self::from_text_field(self)
    }
}

/// Style for TextField
pub struct TextFieldStyle {
    /// The font to use
    pub font: Font,

    /// The font color
    pub font_color: Color,

    /// The background color
    pub background_color: Color,

    /// The cursor color
    pub cursor_color: Color,

    /// The selection color
    pub selection_color: Color,

    /// The message font
    pub message_font: Font,

    /// The message font color
    pub message_font_color: Color,

    /// The background image
    pub background: Option<String>,
}

/// Font data
pub struct Font {
    /// The font data
    pub data: FontData,
}

/// Font data
pub struct FontData {
    /// Whether markup is enabled
    pub markup_enabled: bool,
}