use ggez::graphics::{Color, DrawParam};
use ggez::mint::Point2;
use std::sync::Arc;

use crate::constants::Constants;
use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::stack::Stack;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;

/// A widget containing two Labels superimposed with an offset to create a shadow effect.
///
/// Reported pref_width, pref_height, min_width and min_height are always those of the Label plus `shadow_offset`.
///
/// If not sized by a parent Layout hierarchy, this starts pre-"pack"ed at its preferred size.
pub struct ShadowedLabel {
    /// The base Stack that this ShadowedLabel extends
    base: Stack,
    /// The width including the shadow offset
    width_with_shadow: f32,
    /// The height including the shadow offset
    height_with_shadow: f32,
    /// The main label (top layer)
    label: Arc<Label>,
    /// The shadow label (bottom layer)
    shadow: Arc<Label>,
}

impl ShadowedLabel {
    /// Creates a new ShadowedLabel with the given parameters
    ///
    /// # Arguments
    ///
    /// * `text` - The label text, autotranslated
    /// * `font_size` - The font size
    /// * `label_color` - The color of the main label
    /// * `shadow_color` - The color of the shadow
    /// * `hide_icons` - Whether to hide icons in the text
    /// * `shadow_offset` - Displacement distance of the shadow to right and to bottom
    pub fn new(
        text: &str,
        font_size: i32,
        label_color: Color,
        shadow_color: Color,
        hide_icons: bool,
        shadow_offset: f32,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        // Create the shadow label (bottom layer)
        let mut shadow = Label::new(
            text,
            font_size,
            shadow_color,
            hide_icons,
            skin.clone(),
        );
        shadow.set_align(Point2 { x: 1.0, y: 0.0 }); // bottomRight
        shadow.set_touchable(false);

        // Create the main label (top layer)
        let mut label = Label::new(
            text,
            font_size,
            label_color,
            hide_icons,
            skin.clone(),
        );
        label.set_align(Point2 { x: 0.0, y: 1.0 }); // topLeft
        label.set_touchable(false);

        // Calculate dimensions including shadow
        let width_with_shadow = label.get_pref_width() + shadow_offset;
        let height_with_shadow = label.get_pref_height() + shadow_offset;

        // Create the stack and add the labels
        let mut stack = Stack::new();
        stack.set_touchable(false);

        let shadow_arc = Arc::new(shadow);
        let label_arc = Arc::new(label);

        stack.add_child(shadow_arc.clone());
        stack.add_child(label_arc.clone());

        // Set the size of the stack
        stack.set_size(width_with_shadow, height_with_shadow);

        Self {
            base: stack,
            width_with_shadow,
            height_with_shadow,
            label: label_arc,
            shadow: shadow_arc,
        }
    }

    /// Creates a new ShadowedLabel with default parameters
    ///
    /// # Arguments
    ///
    /// * `text` - The label text, autotranslated
    pub fn new_default(text: &str) -> Self {
        Self::new(
            text,
            Constants::default_font_size(),
            Color::WHITE,
            ImageGetter::charcoal(),
            true,
            1.0,
        )
    }
}

// Implement the necessary traits for ShadowedLabel
impl std::ops::Deref for ShadowedLabel {
    type Target = Stack;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for ShadowedLabel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Widget for ShadowedLabel {
    fn get_pref_width(&self) -> f32 {
        self.width_with_shadow
    }

    fn get_pref_height(&self) -> f32 {
        self.height_with_shadow
    }

    fn get_min_width(&self) -> f32 {
        self.width_with_shadow
    }

    fn get_min_height(&self) -> f32 {
        self.height_with_shadow
    }
}

impl Clone for ShadowedLabel {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            width_with_shadow: self.width_with_shadow,
            height_with_shadow: self.height_with_shadow,
            label: self.label.clone(),
            shadow: self.shadow.clone(),
        }
    }
}