use ggez::graphics::{Color, DrawParam, Text};
use ggez::mint::Point2;
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::Constants;
use crate::models::translations::TranslationManager;
use crate::ui::components::fonts::{FontRulesetIcons, Fonts};
use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::screens::basescreen::BaseScreen;

/// A Label allowing Gdx markup
///
/// This constructor does _not_ auto-translate or otherwise preprocess [text]
/// See also [Color Markup Language](https://libgdx.com/wiki/graphics/2d/fonts/color-markup-language)
pub struct ColorMarkupLabel {
    /// The base Label that this ColorMarkupLabel extends
    base: Label,

    /// Only if wrap was turned on, this is the prefWidth before.
    /// Used for getMaxWidth as better estimate than the default 0.
    unwrapped_pref_width: f32,
}

impl ColorMarkupLabel {
    /// Creates a new ColorMarkupLabel with the given text and font size
    fn new_with_font_size(text: String, font_size: i32) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let mut label = Label::new(text, skin);

        // Apply font size if not default
        if font_size != Constants::default_font_size() {
            let mut style = label.style().clone();
            style.font = Fonts::font();
            label.set_style(style);
            label.set_font_scale(font_size as f32 / Fonts::ORIGINAL_FONT_SIZE as f32);
        }

        Self {
            base: label,
            unwrapped_pref_width: 0.0,
        }
    }

    /// Creates a new ColorMarkupLabel with the given text, font size, default color, and hide icons flag
    ///
    /// A Label allowing Gdx markup, auto-translated.
    ///
    /// Since Gdx markup markers are interpreted and removed by translation, use «» instead.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to display
    /// * `font_size` - The font size to use
    /// * `default_color` - The color text starts with - will be converted to markup, not actor tint
    /// * `hide_icons` - Passed to translation to prevent auto-insertion of symbols for gameplay names
    pub fn new(
        text: String,
        font_size: Option<i32>,
        default_color: Option<Color>,
        hide_icons: Option<bool>,
    ) -> Self {
        let font_size = font_size.unwrap_or(Constants::default_font_size());
        let default_color = default_color.unwrap_or(Color::WHITE);
        let hide_icons = hide_icons.unwrap_or(false);

        let processed_text = Self::map_markup(text, default_color, hide_icons);
        Self::new_with_font_size(processed_text, font_size)
    }

    /// Creates a new ColorMarkupLabel with the given text, text color, symbol color, and font size
    ///
    /// A Label automatically applying Gdx markup colors to symbols and rest of text separately -
    /// _**after**_ translating [text].
    ///
    /// Use to easily color text without also coloring the icons which translation inserts as
    /// characters for recognized gameplay names.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to display
    /// * `text_color` - The color for the text
    /// * `symbol_color` - The color for symbols
    /// * `font_size` - The font size to use
    pub fn new_with_colors(
        text: String,
        text_color: Color,
        symbol_color: Option<Color>,
        font_size: Option<i32>,
    ) -> Self {
        let font_size = font_size.unwrap_or(Constants::default_font_size());
        let symbol_color = symbol_color.unwrap_or(Color::WHITE);

        let processed_text = Self::prepare_text(text, text_color, symbol_color);
        Self::new_with_font_size(processed_text, font_size)
    }

    /// Gets the unwrapped preferred width
    pub fn unwrapped_pref_width(&self) -> f32 {
        self.unwrapped_pref_width
    }

    /// Sets the unwrapped preferred width
    pub fn set_unwrapped_pref_width(&mut self, width: f32) {
        self.unwrapped_pref_width = width;
    }

    /// Converts a color to its markup representation
    fn color_to_markup(color: Color) -> String {
        // In a real implementation, this would use a map of colors to their names
        // For now, we'll just use the hex representation
        if color.a < 1.0 {
            format!("#{:02X}{:02X}{:02X}{:02X}",
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                (color.a * 255.0) as u8)
        } else {
            format!("#{:02X}{:02X}{:02X}",
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8)
        }
    }

    /// Maps text with default color to markup
    fn map_markup(text: String, default_color: Color, hide_icons: bool) -> String {
        let translation_manager = TranslationManager::get_instance();
        let translated = if default_color == Color::WHITE {
            translation_manager.tr(&text, hide_icons)
        } else {
            let color_markup = Self::color_to_markup(default_color);
            format!("[{}]{}[]", color_markup, translation_manager.tr(&text, hide_icons))
        };

        // Replace «» with [] for markup
        if translated.contains('«') {
            translated.replace('«', '[').replace('»', ']')
        } else {
            translated
        }
    }

    /// Prepares text with separate colors for text and symbols
    fn prepare_text(text: String, text_color: Color, symbol_color: Color) -> String {
        let translation_manager = TranslationManager::get_instance();
        let translated = translation_manager.tr(&text);

        if (text_color == Color::WHITE && symbol_color == Color::WHITE) || translated.trim().is_empty() {
            return translated;
        }

        let text_color_markup = Self::color_to_markup(text_color);

        if text_color == symbol_color {
            return format!("[{}]{}[]", text_color_markup, translated);
        }

        let symbol_color_markup = Self::color_to_markup(symbol_color);

        let mut result = String::with_capacity(translated.len() + 42);
        let mut current_color = ' ';

        for c in translated.chars() {
            let new_color = if Fonts::all_symbols().contains(&c) || FontRulesetIcons::char_to_ruleset_image_actor().contains_key(&c) {
                'S'
            } else {
                'T'
            };

            if new_color != current_color {
                if current_color != ' ' {
                    result.push_str("[]");
                }
                result.push('[');
                result.push_str(if new_color == 'S' { &symbol_color_markup } else { &text_color_markup });
                result.push(']');
                current_color = new_color;
            }

            result.push(c);
        }

        if current_color != ' ' {
            result.push_str("[]");
        }

        result
    }

    /// Creates a clone of this ColorMarkupLabel
    pub fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            unwrapped_pref_width: self.unwrapped_pref_width,
        }
    }
}

// Implement the necessary traits for ColorMarkupLabel
impl std::ops::Deref for ColorMarkupLabel {
    type Target = Label;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ColorMarkupLabel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Implement the Widget trait for ColorMarkupLabel
impl Widget for ColorMarkupLabel {
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        // Ensure markup is enabled for drawing
        let original_markup_enabled = self.base.style().font.data.markup_enabled;
        self.base.style_mut().font.data.markup_enabled = true;

        let result = self.base.draw(ctx, param);

        // Restore original markup setting
        self.base.style_mut().font.data.markup_enabled = original_markup_enabled;

        result
    }

    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        self.base.update(ctx)
    }

    fn contains_point(&self, point: Point2<f32>) -> bool {
        self.base.contains_point(point)
    }

    fn get_stage(&self) -> Option<Arc<dyn crate::ui::components::widgets::stage::Stage>> {
        self.base.get_stage()
    }

    fn set_stage(&mut self, stage: Option<Arc<dyn crate::ui::components::widgets::stage::Stage>>) {
        self.base.set_stage(stage);
    }

    fn get_parent(&self) -> Option<Arc<dyn Widget>> {
        self.base.get_parent()
    }

    fn set_parent(&mut self, parent: Option<Arc<dyn Widget>>) {
        self.base.set_parent(parent);
    }

    fn get_position(&self) -> Point2<f32> {
        self.base.get_position()
    }

    fn set_position(&mut self, position: Point2<f32>) {
        self.base.set_position(position);
    }

    fn get_size(&self) -> Point2<f32> {
        self.base.get_size()
    }

    fn set_size(&mut self, size: Point2<f32>) {
        self.base.set_size(size);
    }

    fn get_visible(&self) -> bool {
        self.base.get_visible()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn get_enabled(&self) -> bool {
        self.base.get_enabled()
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.base.set_enabled(enabled);
    }

    fn get_name(&self) -> &str {
        self.base.get_name()
    }

    fn set_name(&mut self, name: String) {
        self.base.set_name(name);
    }

    fn get_user_data(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.base.get_user_data()
    }

    fn set_user_data(&mut self, user_data: Option<Arc<dyn std::any::Any + Send + Sync>>) {
        self.base.set_user_data(user_data);
    }

    fn get_children(&self) -> &[Arc<dyn Widget>] {
        self.base.get_children()
    }

    fn add_child(&mut self, child: Arc<dyn Widget>) {
        self.base.add_child(child);
    }

    fn remove_child(&mut self, child: &Arc<dyn Widget>) -> bool {
        self.base.remove_child(child)
    }

    fn clear_children(&mut self) {
        self.base.clear_children();
    }

    fn get_color(&self) -> Color {
        self.base.get_color()
    }

    fn set_color(&mut self, color: Color) {
        self.base.set_color(color);
    }

    fn get_pref_width(&self) -> f32 {
        if !self.base.wrap() {
            return self.base.get_pref_width();
        }

        // Label has a Quirk that together with bad choices in Table become a bug:
        // Label.getPrefWidth will always return 0 if wrap is on, and Table will NOT
        // interpret that as "unknown" like it should but as "I want to be 0 wide".
        self.base.get_pref_height(); // Ensure scaleAndComputePrefSize has been run

        // In Rust, we can't access private fields directly like in Kotlin
        // Instead, we'll use the unwrapped_pref_width field we maintain
        let result = self.unwrapped_pref_width;

        // That prefWidth we got still might have to be wrapped in some background metrics
        if self.base.style().background.is_none() {
            return result;
        }

        let background = self.base.style().background.as_ref().unwrap();
        (result + background.left_width() + background.right_width()).max(self.base.get_min_width())
    }

    fn get_min_width(&self) -> f32 {
        48.0
    }

    fn get_max_width(&self) -> f32 {
        self.unwrapped_pref_width // If unwrapped, we return 0 same as super
    }

    fn set_wrap(&mut self, wrap: bool) {
        if !self.base.wrap() {
            self.unwrapped_pref_width = self.base.get_pref_width();
        }
        self.base.set_wrap(wrap);
    }

    fn handle_mouse_button_down_event(&mut self, button: ggez::event::MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_down_event(button, position)
    }

    fn handle_mouse_button_up_event(&mut self, button: ggez::event::MouseButton, position: Point2<f32>) -> bool {
        self.base.handle_mouse_button_up_event(button, position)
    }

    fn handle_mouse_motion_event(&mut self, delta: Point2<f32>, position: Point2<f32>) -> bool {
        self.base.handle_mouse_motion_event(delta, position)
    }

    fn handle_mouse_wheel_event(&mut self, delta: ggez::event::MouseWheel, position: Point2<f32>) -> bool {
        self.base.handle_mouse_wheel_event(delta, position)
    }

    fn handle_key_down_event(&mut self, keycode: ggez::event::KeyCode, keymods: ggez::event::KeyMods, repeat: bool) -> bool {
        self.base.handle_key_down_event(keycode, keymods, repeat)
    }

    fn handle_key_up_event(&mut self, keycode: ggez::event::KeyCode, keymods: ggez::event::KeyMods) -> bool {
        self.base.handle_key_up_event(keycode, keymods)
    }

    fn handle_text_input_event(&mut self, character: char) -> bool {
        self.base.handle_text_input_event(character)
    }

    fn handle_focus_event(&mut self, focused: bool) -> bool {
        self.base.handle_focus_event(focused)
    }

    fn handle_scroll_focus_event(&mut self, focused: bool) -> bool {
        self.base.handle_scroll_focus_event(focused)
    }

    fn handle_cursor_enter_event(&mut self, entered: bool) -> bool {
        self.base.handle_cursor_enter_event(entered)
    }

    fn handle_cursor_leave_event(&mut self, entered: bool) -> bool {
        self.base.handle_cursor_leave_event(entered)
    }

    fn handle_cursor_motion_event(&mut self, position: Point2<f32>) -> bool {
        self.base.handle_cursor_motion_event(position)
    }

    fn handle_touch_event(&mut self, phase: ggez::event::TouchPhase, position: Point2<f32>, id: i64) -> bool {
        self.base.handle_touch_event(phase, position, id)
    }

    fn handle_resize_event(&mut self, width: f32, height: f32) -> bool {
        self.base.handle_resize_event(width, height)
    }

    fn handle_draw_event(&mut self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        self.base.handle_draw_event(ctx, param)
    }

    fn handle_update_event(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        self.base.handle_update_event(ctx)
    }

    fn layout(&mut self) {
        // Ensure markup is enabled for layout
        let original_markup_enabled = self.base.style().font.data.markup_enabled;
        self.base.style_mut().font.data.markup_enabled = true;

        self.base.layout();

        // Restore original markup setting
        self.base.style_mut().font.data.markup_enabled = original_markup_enabled;
    }

    fn compute_pref_size(&mut self, layout: Option<&mut Text>) {
        // Ensure markup is enabled for computing preferred size
        let original_markup_enabled = self.base.style().font.data.markup_enabled;
        self.base.style_mut().font.data.markup_enabled = true;

        self.base.compute_pref_size(layout);

        // Restore original markup setting
        self.base.style_mut().font.data.markup_enabled = original_markup_enabled;
    }
}