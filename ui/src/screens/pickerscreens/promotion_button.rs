// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PromotionButton.kt

use std::rc::Rc;
use egui::{Ui, Color32, Align, Response, Button, Image, Label, LabelStyle};
use crate::models::ruleset::unit::Promotion;
use crate::ui::components::widgets::bordered_table::BorderedTable;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use super::promotion_tree::PromotionTree;
use super::promotion_screen_colors::PromotionScreenColors;

pub struct PromotionButton {
    node: Rc<PromotionTree::PromotionNode>,
    is_pickable: bool,
    adopted_label_style: LabelStyle,
    max_width: f32,
    label: Label,
    default_label_style: LabelStyle,
    colors: Rc<PromotionScreenColors>,
    bg_color: Color32,
}

impl PromotionButton {
    pub fn new(
        node: Rc<PromotionTree::PromotionNode>,
        is_pickable: bool,
        adopted_label_style: LabelStyle,
        max_width: f32,
    ) -> Self {
        let mut button = Self {
            node: Rc::clone(&node),
            is_pickable,
            adopted_label_style: adopted_label_style.clone(),
            max_width,
            label: Label::new(node.promotion.name.clone()),
            default_label_style: LabelStyle::default(),
            colors: Rc::new(PromotionScreenColors::default()),
            bg_color: Color32::TRANSPARENT,
        };

        button.init();
        button
    }

    fn init(&mut self) {
        // Set up the button
        self.set_touchable(true);
        self.set_border_size(5.0);
        self.pad(5.0);
        self.set_align(Align::LEFT);

        // Add the promotion portrait
        let portrait = ImageGetter::get_promotion_portrait(&self.node.promotion.name);
        self.add_image(portrait).pad_right(10.0);

        // Set up the label
        self.label.set_ellipsis(true);
        self.add_label(&self.label).left().max_width(self.max_width);

        // Initialize with default colors
        self.update_color(false, &Vec::new(), &Vec::new());
    }

    pub fn update_color(
        &mut self,
        is_selected: bool,
        path_to_selection: &Vec<Promotion>,
        prerequisites: &Vec<PromotionTree::PromotionNode>,
    ) {
        // Update background color based on state
        self.bg_color = if is_selected {
            self.colors.selected
        } else if self.node.is_adopted {
            self.colors.promoted
        } else if path_to_selection.contains(&self.node.promotion) {
            self.colors.path_to_selection
        } else if prerequisites.contains(&self.node) {
            self.colors.prerequisite
        } else if self.is_pickable {
            self.colors.pickable
        } else {
            self.colors.default
        };

        // Update label style
        if !is_selected && self.node.is_adopted {
            self.label.set_style(self.adopted_label_style.clone());
        } else {
            self.label.set_style(self.default_label_style.clone());
        }
    }

    // Helper methods for UI layout
    fn set_touchable(&mut self, touchable: bool) {
        // In Rust with egui, we'll need to implement this differently
    }

    fn set_border_size(&mut self, size: f32) {
        // In Rust with egui, we'll need to implement this differently
    }

    fn pad(&mut self, size: f32) {
        // In Rust with egui, we'll need to implement this differently
    }

    fn set_align(&mut self, align: Align) {
        // In Rust with egui, we'll need to implement this differently
    }

    fn add_image(&mut self, image: Image) -> &mut Self {
        // In Rust with egui, we'll need to implement this differently
        self
    }

    fn pad_right(&mut self, size: f32) -> &mut Self {
        // In Rust with egui, we'll need to implement this differently
        self
    }

    fn add_label(&mut self, label: &Label) -> &mut Self {
        // In Rust with egui, we'll need to implement this differently
        self
    }

    fn left(&mut self) -> &mut Self {
        // In Rust with egui, we'll need to implement this differently
        self
    }

    fn max_width(&mut self, width: f32) -> &mut Self {
        // In Rust with egui, we'll need to implement this differently
        self
    }
}

// Define the PromotionScreenColors struct
pub struct PromotionScreenColors {
    pub selected: Color32,
    pub promoted: Color32,
    pub path_to_selection: Color32,
    pub prerequisite: Color32,
    pub pickable: Color32,
    pub default: Color32,
}

impl Default for PromotionScreenColors {
    fn default() -> Self {
        Self {
            selected: Color32::from_rgba_premultiplied(0, 100, 255, 255),
            promoted: Color32::from_rgba_premultiplied(0, 200, 0, 255),
            path_to_selection: Color32::from_rgba_premultiplied(200, 200, 0, 255),
            prerequisite: Color32::from_rgba_premultiplied(150, 150, 150, 255),
            pickable: Color32::from_rgba_premultiplied(100, 100, 255, 255),
            default: Color32::from_rgba_premultiplied(50, 50, 50, 255),
        }
    }
}