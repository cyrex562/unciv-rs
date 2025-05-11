use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::constants::Constants;

/// Represents a color in RGBA format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0-1)
    pub r: f32,
    /// Green component (0-1)
    pub g: f32,
    /// Blue component (0-1)
    pub b: f32,
    /// Alpha component (0-1)
    pub a: f32,
}

impl Color {
    /// Create a new color from RGBA components (0-1)
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new color from a hex value (0xAARRGGBB format)
    pub fn from_hex(hex: u32) -> Self {
        let a = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a }
    }

    /// Create a copy of this color
    pub fn clone(&self) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

/// Skin element, read from UI Skin json
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkinElement {
    /// The image path
    pub image: Option<String>,
    /// The tint color
    pub tint: Option<Color>,
    /// The alpha value
    pub alpha: Option<f32>,
    /// The foreground color
    pub foreground_color: Option<Color>,
    /// The icon color
    pub icon_color: Option<Color>,
}

impl SkinElement {
    /// Create a new SkinElement instance
    pub fn new() -> Self {
        Self {
            image: None,
            tint: None,
            alpha: None,
            foreground_color: None,
            icon_color: None,
        }
    }
}

/// Represents a skin configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkinConfig {
    /// The base color for the skin
    pub base_color: Color,
    /// The clear color for the skin
    pub clear_color: Color,
    /// The default variant tint
    pub default_variant_tint: Option<Color>,
    /// The fallback skin name
    pub fallback_skin: Option<String>,
    /// The skin variants
    pub skin_variants: HashMap<String, SkinElement>,
}

impl SkinConfig {
    /// Create a new SkinConfig instance with the specified initial capacity
    pub fn with_capacity(initial_capacity: usize) -> Self {
        Self {
            base_color: Color::from_hex(0x004085bf),
            clear_color: Color::from_hex(0x000033ff),
            default_variant_tint: None,
            fallback_skin: Some(Constants::default_fallback_skin().to_string()),
            skin_variants: HashMap::with_capacity(initial_capacity),
        }
    }

    /// Create a new SkinConfig instance with default capacity
    pub fn new() -> Self {
        // 16 = HashMap::DEFAULT_INITIAL_CAPACITY
        Self::with_capacity(16)
    }

    /// Create a clone of this SkinConfig
    pub fn clone(&self) -> Self {
        let mut new_config = Self::with_capacity(self.skin_variants.len());
        new_config.update_config(self);
        new_config
    }

    /// 'Merges' other into this
    ///
    /// base_color, clear_color, and default_variant_tint are overwritten with clones from other.
    /// fallback_skin is overwritten with other's value.
    /// skin_variants with the same key are copied and overwritten, new skin_variants are added.
    pub fn update_config(&mut self, other: &SkinConfig) {
        self.base_color = other.base_color.clone();
        self.clear_color = other.clear_color.clone();
        self.default_variant_tint = other.default_variant_tint.clone();
        self.fallback_skin = other.fallback_skin.clone();

        // Add or update all skin variants from other
        for (key, value) in &other.skin_variants {
            self.skin_variants.insert(key.clone(), value.clone());
        }
    }
}