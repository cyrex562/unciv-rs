use std::path::PathBuf;
use crate::models::skins::{SkinConfig, Color, SkinCache};
use crate::ui::images::ImageGetter;
use crate::game::UncivGame;

/// Represents a nine-patch drawable for UI elements
pub struct NinePatchDrawable {
    // TODO: Implement nine-patch drawable functionality
    // This is a placeholder for the actual implementation
}

/// Handles UI skin strings and background elements
pub struct SkinStrings {
    /// The skin location path
    skin_location: String,
    /// The skin configuration
    skin_config: SkinConfig,
    /// The fallback skin location path
    fallback_skin_location: Option<String>,
    /// The fallback skin configuration
    fallback_skin_config: Option<SkinConfig>,
}

impl SkinStrings {
    /// Create a new SkinStrings instance
    pub fn new(skin: Option<String>) -> Self {
        let skin = skin.unwrap_or_else(|| UncivGame::current().settings().skin().to_string());
        let skin_location = format!("Skins/{}/", skin);
        let skin_config = SkinCache::get(&skin).unwrap_or_else(SkinConfig::new);
        let fallback_skin_location = skin_config.fallback_skin.as_ref()
            .map(|fallback| format!("Skins/{}/", fallback));
        let fallback_skin_config = skin_config.fallback_skin.as_ref()
            .and_then(|fallback| SkinCache::get(fallback));

        Self {
            skin_location,
            skin_config,
            fallback_skin_location,
            fallback_skin_config,
        }
    }

    /// Default shapes must always end with "Shape" so the UiElementDocsWriter can identify them
    pub const ROUNDED_EDGE_RECTANGLE_SMALL_SHAPE: &'static str = "roundedEdgeRectangle-small";
    pub const ROUNDED_TOP_EDGE_RECTANGLE_SMALL_SHAPE: &'static str = "roundedTopEdgeRectangle-small";
    pub const ROUNDED_TOP_EDGE_RECTANGLE_SMALL_BORDER_SHAPE: &'static str = "roundedTopEdgeRectangle-small-border";
    pub const ROUNDED_EDGE_RECTANGLE_MID_SHAPE: &'static str = "roundedEdgeRectangle-mid";
    pub const ROUNDED_EDGE_RECTANGLE_MID_BORDER_SHAPE: &'static str = "roundedEdgeRectangle-mid-border";
    pub const ROUNDED_EDGE_RECTANGLE_SHAPE: &'static str = "roundedEdgeRectangle";
    pub const RECTANGLE_WITH_OUTLINE_SHAPE: &'static str = "rectangleWithOutline";
    pub const SELECT_BOX_SHAPE: &'static str = "select-box";
    pub const SELECT_BOX_PRESSED_SHAPE: &'static str = "select-box-pressed";
    pub const CHECKBOX_SHAPE: &'static str = "checkbox";
    pub const CHECKBOX_PRESSED_SHAPE: &'static str = "checkbox-pressed";

    /// Gets either a drawable which was defined inside skinConfig for the given path or the drawable
    /// found at path itself or the default drawable to be applied as the background for an UI element.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the UI background in UpperCamelCase. Should be the location of the
    ///           UI element inside the UI tree e.g. WorldScreen/TopBar/StatsTable.
    ///
    ///           If the UI element is used in multiple Screens start the path with General
    ///           e.g. General/Tooltip
    ///
    ///           If the UI element has multiple states with different tints use a distinct
    ///           name for every state e.g.
    ///           - CityScreen/CityConstructionTable/QueueEntry
    ///           - CityScreen/CityConstructionTable/QueueEntrySelected
    ///
    /// * `default` - The path to the background which should be used if path is not available.
    ///              Should be one of the predefined ones inside SkinStrings or None to get a
    ///              solid background.
    ///
    /// * `tint_color` - Default tint color if the UI Skin doesn't specify one. If both not specified,
    ///                 the returned background will not be tinted. If the UI Skin specifies a
    ///                 separate alpha value, it will be applied to a clone of either color.
    pub fn get_ui_background(&self, path: &str, default: Option<&str>, tint_color: Option<&Color>) -> NinePatchDrawable {
        let location_for_default = default.map(|d| format!("{}{}", self.skin_location, d));
        let location_by_name = format!("{}{}", self.skin_location, path);
        let skin_variant = self.skin_config.skin_variants.get(path);
        let location_by_config_variant = skin_variant.and_then(|variant| {
            variant.image.as_ref().map(|image| format!("{}{}", self.skin_location, image))
        });

        let tint = skin_variant.and_then(|variant| variant.tint.as_ref())
            .or_else(|| self.skin_config.default_variant_tint.as_ref())
            .or(tint_color)
            .map(|color| {
                if let Some(variant) = skin_variant {
                    if let Some(alpha) = variant.alpha {
                        let mut color_clone = color.clone();
                        color_clone.a = alpha;
                        color_clone
                    } else {
                        color.clone()
                    }
                } else {
                    color.clone()
                }
            });

        let location = if let Some(loc) = location_by_config_variant {
            if ImageGetter::nine_patch_image_exists(&loc) {
                Some(loc)
            } else if ImageGetter::nine_patch_image_exists(&location_by_name) {
                Some(location_by_name)
            } else if let Some(def_loc) = location_for_default {
                if ImageGetter::nine_patch_image_exists(&def_loc) {
                    Some(def_loc)
                } else {
                    None
                }
            } else {
                None
            }
        } else if ImageGetter::nine_patch_image_exists(&location_by_name) {
            Some(location_by_name)
        } else if let Some(def_loc) = location_for_default {
            if ImageGetter::nine_patch_image_exists(&def_loc) {
                Some(def_loc)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(loc) = location {
            return ImageGetter::get_nine_patch(&loc, tint.as_ref());
        }

        // Try fallback skin
        if let Some(fallback_location) = &self.fallback_skin_location {
            let fallback_location_for_default = default.map(|d| format!("{}{}", fallback_location, d));
            let fallback_location_by_name = format!("{}{}", fallback_location, path);
            let fallback_skin_variant = self.fallback_skin_config.as_ref()
                .and_then(|config| config.skin_variants.get(path));
            let fallback_location_by_config_variant = fallback_skin_variant.and_then(|variant| {
                variant.image.as_ref().map(|image| format!("{}{}", fallback_location, image))
            });

            let fallback_tint = fallback_skin_variant.and_then(|variant| variant.tint.as_ref())
                .or(tint_color)
                .map(|color| {
                    if let Some(variant) = fallback_skin_variant {
                        if let Some(alpha) = variant.alpha {
                            let mut color_clone = color.clone();
                            color_clone.a = alpha;
                            color_clone
                        } else {
                            color.clone()
                        }
                    } else {
                        color.clone()
                    }
                });

            let fallback_location = if let Some(loc) = fallback_location_by_config_variant {
                if ImageGetter::nine_patch_image_exists(&loc) {
                    Some(loc)
                } else if ImageGetter::nine_patch_image_exists(&fallback_location_by_name) {
                    Some(fallback_location_by_name)
                } else if let Some(def_loc) = fallback_location_for_default {
                    if ImageGetter::nine_patch_image_exists(&def_loc) {
                        Some(def_loc)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if ImageGetter::nine_patch_image_exists(&fallback_location_by_name) {
                Some(fallback_location_by_name)
            } else if let Some(def_loc) = fallback_location_for_default {
                if ImageGetter::nine_patch_image_exists(&def_loc) {
                    Some(def_loc)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(loc) = fallback_location {
                return ImageGetter::get_nine_patch(&loc, fallback_tint.as_ref());
            }
        }

        // Return a default nine-patch if nothing else worked
        ImageGetter::get_nine_patch("", None)
    }

    /// Get the UI color for a path
    pub fn get_ui_color(&self, path: &str, default: Option<&Color>) -> Color {
        self.skin_config.skin_variants.get(path)
            .and_then(|variant| variant.tint.as_ref())
            .or(default)
            .unwrap_or(&self.skin_config.clear_color)
            .clone()
    }

    /// Get the UI font color for a path
    pub fn get_ui_font_color(&self, path: &str) -> Option<Color> {
        self.skin_config.skin_variants.get(path)
            .and_then(|variant| variant.foreground_color.clone())
    }

    /// Get the UI icon color for a path
    pub fn get_ui_icon_color(&self, path: &str) -> Option<Color> {
        self.skin_config.skin_variants.get(path)
            .and_then(|variant| variant.icon_color.as_ref().or(variant.foreground_color.as_ref()))
            .cloned()
    }
}