use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};

/// Represents a set of tiles for the game map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSet {
    /// The name of the tile set
    pub name: String,

    /// The configuration for this tile set
    pub config: TileSetConfig,

    /// Optional fallback tile set if this one is incomplete
    pub fallback: Option<Box<TileSet>>,

    /// Map of mod names to their configurations
    #[serde(skip)]
    mod_name_to_config: HashMap<String, TileSetConfig>,
}

impl TileSet {
    /// Create a new TileSet with the given name
    pub fn new(name: String) -> Self {
        TileSet {
            name,
            config: TileSetConfig::new(),
            fallback: None,
            mod_name_to_config: HashMap::new(),
        }
    }

    /// Cache a configuration from a mod
    pub fn cache_config_from_mod(&mut self, mod_name: String, config: TileSetConfig) {
        self.mod_name_to_config.insert(mod_name, config);
    }

    /// Merge a mod's configuration into the current configuration
    pub fn merge_mod_config(&mut self, mod_name: &str) {
        if let Some(config_to_merge) = self.mod_name_to_config.get(mod_name) {
            self.config.update_config(config_to_merge);
        }
    }

    /// Reset the configuration to default
    pub fn reset_config(&mut self) {
        self.config = TileSetConfig::new();
    }

    /// Get the default tile set name
    pub fn default_name() -> &'static str {
        "INTERNAL"
    }
}

impl fmt::Display for TileSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Default implementation for TileSet
impl Default for TileSet {
    fn default() -> Self {
        TileSet::new(TileSet::default_name().to_string())
    }
}

/// Configuration for a tile set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileSetConfig {
    // Add fields as needed based on the actual TileSetConfig implementation
}

impl TileSetConfig {
    /// Create a new TileSetConfig with default values
    pub fn new() -> Self {
        TileSetConfig {
            // Initialize fields with default values
        }
    }

    /// Update this configuration with values from another configuration
    pub fn update_config(&mut self, other: &TileSetConfig) {
        // Implement the update logic based on the actual TileSetConfig implementation
    }
}

impl Default for TileSetConfig {
    fn default() -> Self {
        TileSetConfig::new()
    }
}