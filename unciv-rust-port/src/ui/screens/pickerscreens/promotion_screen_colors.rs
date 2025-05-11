// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PromotionScreenColors.kt

use egui::Color32;
use crate::ui::images::ImageGetter;

/// Colors used in the promotion picker screen
pub struct PromotionScreenColors {
    pub default: Color32,
    pub selected: Color32,
    pub path_to_selection: Color32,
    pub promoted: Color32,
    pub promoted_text: Color32,
    pub pickable: Color32,
    pub prerequisite: Color32,
    pub group_lines: Color32,
    pub other_lines: Color32,
}

impl Default for PromotionScreenColors {
    fn default() -> Self {
        Self {
            default: ImageGetter::CHARCOAL,
            // colorFromRGB(72, 147, 175)
            selected: Color32::from_rgba_premultiplied(72, 147, 175, 255),
            // selected.darken(0.33f)
            path_to_selection: Color32::from_rgba_premultiplied(48, 98, 117, 255),
            // colorFromRGB(255, 215, 0).darken(0.2f)
            promoted: Color32::from_rgba_premultiplied(204, 172, 0, 255),
            // promoted.darken(0.8f)
            promoted_text: Color32::from_rgba_premultiplied(41, 34, 0, 255),
            // colorFromRGB(28, 80, 0)
            pickable: Color32::from_rgba_premultiplied(28, 80, 0, 255),
            // HSV(225,50,80): muted Royal
            prerequisite: Color32::from_rgba_premultiplied(102, 128, 204, 255),
            group_lines: Color32::WHITE,
            other_lines: Color32::TRANSPARENT,
        }
    }
}