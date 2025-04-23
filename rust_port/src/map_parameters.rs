use crate::hex_math::HexMath;
use crate::map::mapgenerator::map_resource_setting::MapResourceSetting;
use crate::metadata::base_ruleset::BaseRuleset;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Map shape constants
pub struct MapShape;

impl MapShape {
    pub const RECTANGULAR: &'static str = "Rectangular";
    pub const HEXAGONAL: &'static str = "Hexagonal";
    pub const FLAT_EARTH: &'static str = "Flat Earth Hexagonal";
}

/// Map generation main type constants
pub struct MapGeneratedMainType;

impl MapGeneratedMainType {
    pub const GENERATED: &'static str = "Generated";
    // Randomly choose a generated map type
    pub const RANDOM_GENERATED: &'static str = "Random Generated";
    // Non-generated maps
    pub const CUSTOM: &'static str = "Custom";
    pub const SCENARIO: &'static str = "Scenario";
}

/// Map type constants
pub struct MapType;

impl MapType {
    pub const PERLIN: &'static str = "Perlin";
    pub const PANGAEA: &'static str = "Pangaea";
    pub const CONTINENT_AND_ISLANDS: &'static str = "Continent and Islands";
    pub const TWO_CONTINENTS: &'static str = "Two Continents";
    pub const THREE_CONTINENTS: &'static str = "Three Continents";
    pub const FOUR_CORNERS: &'static str = "Four Corners";
    pub const ARCHIPELAGO: &'static str = "Archipelago";
    pub const FRACTAL: &'static str = "Fractal";
    pub const INNER_SEA: &'static str = "Inner Sea";
    pub const LAKES: &'static str = "Lakes";
    pub const SMALL_CONTINENTS: &'static str = "Small Continents";

    // All ocean tiles
    pub const EMPTY: &'static str = "Empty";
}

/// Mirroring type constants
pub struct MirroringType;

impl MirroringType {
    pub const NONE: &'static str = "None";
    pub const AROUND_CENTER_TILE: &'static str = "Around Center Tile";
    pub const FOURWAY: &'static str = "4-way";
    pub const TOPBOTTOM: &'static str = "Top-Bottom";
    pub const LEFTRIGHT: &'static str = "Bottom-Top";
}

/// Map size constants
pub struct MapSize;

impl MapSize {
    pub const TINY: &'static str = "Tiny";
    pub const SMALL: &'static str = "Small";
    pub const MEDIUM: &'static str = "Medium";
    pub const LARGE: &'static str = "Large";
    pub const HUGE: &'static str = "Huge";
    pub const CUSTOM: &'static str = "Custom";
}

/// Map size parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapSizeParams {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub radius: i32,
}

impl MapSizeParams {
    fn new(name: &str, width: i32, height: i32, radius: i32) -> Self {
        Self {
            name: name.to_string(),
            width,
            height,
            radius,
        }
    }
}

impl Default for MapSizeParams {
    fn default() -> Self {
        Self::new(MapSize::MEDIUM, 40, 30, 12)
    }
}

// Define the size constants as functions instead
impl MapSizeParams {
    pub fn tiny() -> Self {
        Self::new(MapSize::TINY, 20, 15, 5)
    }

    pub fn small() -> Self {
        Self::new(MapSize::SMALL, 30, 20, 8)
    }

    pub fn medium() -> Self {
        Self::new(MapSize::MEDIUM, 40, 30, 12)
    }

    pub fn large() -> Self {
        Self::new(MapSize::LARGE, 60, 40, 18)
    }

    pub fn huge() -> Self {
        Self::new(MapSize::HUGE, 80, 60, 25)
    }
}

/// Map parameters for map generation and configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapParameters {
    pub name: String,
    pub map_type: String,
    pub shape: String,
    pub map_size: MapSizeParams,
    pub map_resources: String,
    pub mirroring: String,
    pub no_ruins: bool,
    pub no_natural_wonders: bool,
    pub world_wrap: bool,
    pub strategic_balance: bool,
    pub legendary_start: bool,
    pub mods: HashSet<String>,
    pub base_ruleset: String,
    pub created_with_version: String,
    pub seed: i64,
    pub tiles_per_biome_area: i32,
    pub max_coast_extension: i32,
    pub elevation_exponent: f32,
    pub temperature_intensity: f32,
    pub vegetation_richness: f32,
    pub rare_features_richness: f32,
    pub resource_richness: f32,
    pub water_threshold: f32,
    pub temperature_shift: f32,
}

impl Default for MapParameters {
    fn default() -> Self {
        Self {
            name: String::new(),
            map_type: MapType::PANGAEA.to_string(),
            shape: MapShape::HEXAGONAL.to_string(),
            map_size: MapSizeParams::default(),
            map_resources: MapResourceSetting::Default.to_string(),
            mirroring: MirroringType::NONE.to_string(),
            no_ruins: false,
            no_natural_wonders: false,
            world_wrap: false,
            strategic_balance: false,
            legendary_start: false,
            mods: HashSet::new(),
            base_ruleset: BaseRuleset::CivVGnK.to_string(),
            created_with_version: String::new(),
            seed: 0,
            tiles_per_biome_area: 6,
            max_coast_extension: 2,
            elevation_exponent: 0.7,
            temperature_intensity: 0.6,
            vegetation_richness: 0.4,
            rare_features_richness: 0.05,
            resource_richness: 0.1,
            water_threshold: 0.0,
            temperature_shift: 0.0,
        }
    }
}

impl MapParameters {
    /// Creates a new MapParameters with default values
    pub fn new() -> Self {
        Self {
            name: String::new(),
            map_type: MapType::PANGAEA.to_string(),
            shape: MapShape::HEXAGONAL.to_string(),
            map_size: MapSizeParams::medium(),
            map_resources: MapResourceSetting::Default.to_string(),
            mirroring: MirroringType::NONE.to_string(),
            no_ruins: false,
            no_natural_wonders: false,
            world_wrap: false,
            strategic_balance: false,
            legendary_start: false,
            mods: HashSet::new(),
            base_ruleset: BaseRuleset::CivVGnK.to_string(),
            created_with_version: String::new(),
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            tiles_per_biome_area: 6,
            max_coast_extension: 2,
            elevation_exponent: 0.7,
            temperature_intensity: 0.6,
            vegetation_richness: 0.4,
            rare_features_richness: 0.05,
            resource_richness: 0.1,
            water_threshold: 0.0,
            temperature_shift: 0.0,
        }
    }

    /// Creates a clone of this MapParameters
    pub fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            map_type: self.map_type.clone(),
            shape: self.shape.clone(),
            map_size: self.map_size.clone(),
            map_resources: self.map_resources.clone(),
            mirroring: self.mirroring.clone(),
            no_ruins: self.no_ruins,
            no_natural_wonders: self.no_natural_wonders,
            world_wrap: self.world_wrap,
            strategic_balance: self.strategic_balance,
            legendary_start: self.legendary_start,
            mods: self.mods.clone(),
            base_ruleset: self.base_ruleset.clone(),
            created_with_version: self.created_with_version.clone(),
            seed: self.seed,
            tiles_per_biome_area: self.tiles_per_biome_area,
            max_coast_extension: self.max_coast_extension,
            elevation_exponent: self.elevation_exponent,
            temperature_intensity: self.temperature_intensity,
            vegetation_richness: self.vegetation_richness,
            rare_features_richness: self.rare_features_richness,
            resource_richness: self.resource_richness,
            water_threshold: self.water_threshold,
            temperature_shift: self.temperature_shift,
        }
    }

    /// Reseeds the map with a new random seed
    pub fn reseed(&mut self) {
        self.seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
    }

    /// Resets advanced settings to their default values
    pub fn reset_advanced_settings(&mut self) {
        self.reseed();
        self.tiles_per_biome_area = 6;
        self.max_coast_extension = 2;
        self.elevation_exponent = 0.7;
        self.temperature_intensity = 0.6;
        self.temperature_shift = 0.0;
        self.vegetation_richness = 0.4;
        self.rare_features_richness = 0.05;
        self.resource_richness = 0.1;
        self.water_threshold = 0.0;
    }

    /// Gets the map resources setting
    pub fn get_map_resources(&self) -> MapResourceSetting {
        MapResourceSetting::from_str(&self.map_resources)
    }

    /// Gets whether strategic balance is enabled
    pub fn get_strategic_balance(&self) -> bool {
        self.strategic_balance
            || self.map_resources == MapResourceSetting::StrategicBalance.to_string()
    }

    /// Gets whether legendary start is enabled
    pub fn get_legendary_start(&self) -> bool {
        self.legendary_start || self.map_resources == MapResourceSetting::LegendaryStart.to_string()
    }

    /// Gets the area of the map
    pub fn get_area(&self) -> i32 {
        if self.shape == MapShape::HEXAGONAL || self.shape == MapShape::FLAT_EARTH {
            HexMath::get_number_of_tiles_in_hexagon(self.map_size.radius)
        } else if self.world_wrap && self.map_size.width % 2 != 0 {
            (self.map_size.width - 1) * self.map_size.height
        } else {
            self.map_size.width * self.map_size.height
        }
    }

    /// Displays the map dimensions
    fn display_map_dimensions(&self) -> String {
        let dimensions = if self.shape == MapShape::HEXAGONAL || self.shape == MapShape::FLAT_EARTH
        {
            format!("R{}", self.map_size.radius)
        } else {
            format!("{}x{}", self.map_size.width, self.map_size.height)
        };

        if self.world_wrap {
            format!("{}w", dimensions)
        } else {
            dimensions
        }
    }

    /// Formats a float to a string with a maximum precision, removing trailing zeros
    fn nice_to_string(value: f32, max_precision: i32) -> String {
        let formatted = format!("{:.1$}", value, max_precision as usize);
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }

    /// Gets the number of tiles in the map
    pub fn number_of_tiles(&self) -> i32 {
        if self.shape == MapShape::HEXAGONAL || self.shape == MapShape::FLAT_EARTH {
            1 + 3 * self.map_size.radius * (self.map_size.radius - 1)
        } else {
            self.map_size.width * self.map_size.height
        }
    }
}

impl ToString for MapParameters {
    fn to_string(&self) -> String {
        let mut result = String::new();

        if !self.name.is_empty() {
            result.push_str(&format!("\"{}\" ", self.name));
        }

        result.push('(');

        if self.map_size.name != MapSize::CUSTOM {
            result.push_str(&format!("{{{}}} ", self.map_size.name));
        }

        if self.world_wrap {
            result.push_str("{World Wrap} ");
        }

        result.push_str(&format!("{{{}}}", self.shape));
        result.push_str(&format!(" {} )", self.display_map_dimensions()));

        if self.map_resources != MapResourceSetting::Default.to_string() {
            result.push_str(&format!(
                " {{Resource Setting}}: {{{}}}",
                self.map_resources
            ));
        }

        if self.strategic_balance {
            result.push_str(" {Strategic Balance}");
        }

        if self.legendary_start {
            result.push_str(" {Legendary Start}");
        }

        if self.name.is_empty() {
            return result;
        }

        result.push('\n');

        if self.map_type != MapGeneratedMainType::CUSTOM && self.map_type != MapType::EMPTY {
            result.push_str(&format!("{{Map Generation Type}}: {{{}}}, ", self.map_type));
        }

        result.push_str(&format!("{{RNG Seed}} {}", self.seed));
        result.push_str(&format!(
            ", {{Map Elevation}}={}",
            Self::nice_to_string(self.elevation_exponent, 2)
        ));
        result.push_str(&format!(
            ", {{Temperature intensity}}={}",
            Self::nice_to_string(self.temperature_intensity, 2)
        ));
        result.push_str(&format!(
            ", {{Resource richness}}={}",
            Self::nice_to_string(self.resource_richness, 3)
        ));
        result.push_str(&format!(
            ", {{Vegetation richness}}={}",
            Self::nice_to_string(self.vegetation_richness, 2)
        ));
        result.push_str(&format!(
            ", {{Rare features richness}}={}",
            Self::nice_to_string(self.rare_features_richness, 3)
        ));
        result.push_str(&format!(
            ", {{Max Coast extension}}={}",
            self.max_coast_extension
        ));
        result.push_str(&format!(
            ", {{Biome areas extension}}={}",
            self.tiles_per_biome_area
        ));
        result.push_str(&format!(
            ", {{Water level}}={}",
            Self::nice_to_string(self.water_threshold, 2)
        ));

        result
    }
}
