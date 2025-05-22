use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::constants::DEFAULT_FALLBACK_TILESET;
use crate::ui::images::ImageGetter;

/// Configuration for a tile set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSetConfig {
    /// Whether to use color as base terrain
    pub use_color_as_base_terrain: bool,
    /// Whether to use summary images
    pub use_summary_images: bool,
    /// Color for unexplored tiles
    pub unexplored_tile_color: Color,
    /// Color for fog of war
    pub fog_of_war_color: Color,
    /// Name of the tileset to use when this one is missing images. None to disable.
    pub fallback_tile_set: Option<String>,
    /// Scale factor for hex images, with hex center as origin.
    pub tile_scale: f32,
    /// Map of tile strings to scale factors
    pub tile_scales: HashMap<String, f32>,
    /// Map of tile set strings to render orders
    pub rule_variants: HashMap<String, Vec<String>>,
}

impl Default for TileSetConfig {
    fn default() -> Self {
        Self {
            use_color_as_base_terrain: false,
            use_summary_images: false,
            unexplored_tile_color: Color::DARK_GRAY,
            fog_of_war_color: ImageGetter::CHARCOAL,
            fallback_tile_set: Some(DEFAULT_FALLBACK_TILESET.to_string()),
            tile_scale: 1.0,
            tile_scales: HashMap::new(),
            rule_variants: HashMap::new(),
        }
    }
}

impl TileSetConfig {
    /// Create a new TileSetConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a deep copy of this config
    pub fn clone(&self) -> Self {
        Self {
            use_color_as_base_terrain: self.use_color_as_base_terrain,
            use_summary_images: self.use_summary_images,
            unexplored_tile_color: self.unexplored_tile_color,
            fog_of_war_color: self.fog_of_war_color,
            fallback_tile_set: self.fallback_tile_set.clone(),
            tile_scale: self.tile_scale,
            tile_scales: self.tile_scales.clone(),
            rule_variants: self.rule_variants.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }

    /// Update this config with values from another config
    pub fn update_config(&mut self, other: &TileSetConfig) {
        self.use_color_as_base_terrain = other.use_color_as_base_terrain;
        self.use_summary_images = other.use_summary_images;
        self.unexplored_tile_color = other.unexplored_tile_color;
        self.fog_of_war_color = other.fog_of_war_color;
        self.fallback_tile_set = other.fallback_tile_set.clone();
        self.tile_scale = other.tile_scale;

        // Update tile scales
        for (tile_string, scale) in &other.tile_scales {
            self.tile_scales.insert(tile_string.clone(), *scale);
        }

        // Update rule variants
        for (tile_set_string, render_order) in &other.rule_variants {
            self.rule_variants.insert(tile_set_string.clone(), render_order.clone());
        }
    }
}

/// Color representation for the game
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const DARK_GRAY: Color = Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 };
}

impl ImageGetter {
    pub const CHARCOAL: Color = Color { r: 0.21, g: 0.27, b: 0.31, a: 1.0 };
}