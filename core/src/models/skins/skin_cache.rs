use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use log::{debug, error};
use crate::models::ruleset::RulesetCache;
use crate::ui::images::ImageGetter;
use crate::unciv_game::UncivGame;
use crate::json::{from_json_file, json};

/// Represents a skin and its associated mod
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SkinAndMod {
    /// The name of the skin
    skin: String,
    /// The name of the mod (empty string for built-in skins)
    mod_name: String,
}

/// Represents a skin configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkinConfig {
    // Add fields based on the actual SkinConfig structure
    // This is a placeholder - you'll need to add the actual fields
    pub name: String,
}

impl SkinConfig {
    /// Create a new SkinConfig instance
    pub fn new() -> Self {
        Self {
            name: String::new(),
        }
    }

    /// Update this config with values from another config
    pub fn update_config(&mut self, other: &Self) {
        // Update fields based on the actual SkinConfig structure
        // This is a placeholder - you'll need to implement the actual update logic
        self.name = other.name.clone();
    }
}

/// Cache for skin configurations
pub struct SkinCache {
    /// All loaded skin configurations
    all_configs: HashMap<SkinAndMod, SkinConfig>,
    /// Currently active skin configurations
    active_configs: HashMap<String, SkinConfig>,
}

impl SkinCache {
    /// Create a new SkinCache instance
    pub fn new() -> Self {
        Self {
            all_configs: HashMap::new(),
            active_configs: HashMap::new(),
        }
    }

    /// Combine SkinConfigs for chosen mods.
    /// Vanilla always active, even with a base ruleset mod active.
    /// Permanent visual mods always included as long as UncivGame.Current is initialized.
    /// Other active mods can be passed in parameter rule_set_mods.
    pub fn assemble_skin_configs(&mut self, rule_set_mods: &HashSet<String>) {
        // Needs to be a list and not a set, so subsequent mods override the previous ones
        // Otherwise you rely on hash randomness to determine override order... not good
        let mut mods = vec![String::new()]; // Not an empty list - placeholder for built-in skin

        if UncivGame::is_current_initialized() {
            mods.extend(UncivGame::current().settings.visual_mods.iter().cloned());
        }

        mods.extend(rule_set_mods.iter().cloned());

        self.active_configs.clear();

        // Get unique mods
        let unique_mods: Vec<String> = mods.into_iter().collect::<HashSet<_>>().into_iter().collect();

        for mod_name in unique_mods {
            // Filter configs for this mod
            let configs: Vec<_> = self.all_configs.iter()
                .filter(|(key, _)| key.mod_name == mod_name)
                .collect();

            for (key, config) in configs {
                let skin_name = &key.skin;

                if let Some(existing_config) = self.active_configs.get_mut(skin_name) {
                    // Update existing config
                    existing_config.update_config(config);
                } else {
                    // Add new config
                    self.active_configs.insert(skin_name.clone(), config.clone());
                }
            }
        }
    }

    /// Load skin configurations from files
    pub fn load_skin_configs(&mut self, console_mode: bool) {
        self.all_configs.clear();
        let mut skin_name = String::new();

        // Load internal skins
        let file_handles: Vec<PathBuf> = if console_mode {
            // In console mode, list files directly
            let skins_dir = Path::new("jsons/Skins");
            if skins_dir.exists() {
                fs::read_dir(skins_dir)
                    .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", skins_dir))
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            // In normal mode, use ImageGetter
            ImageGetter::get_available_skins()
                .iter()
                .map(|skin| PathBuf::from(format!("jsons/Skins/{}.json", skin)))
                .filter(|path| path.exists())
                .collect()
        };

        for config_file in file_handles {
            if let Some(file_name) = config_file.file_stem() {
                skin_name = file_name.to_string_lossy().replace("Config", "");

                match from_json_file::<SkinConfig>(&config_file) {
                    Ok(config) => {
                        let key = SkinAndMod {
                            skin: skin_name.clone(),
                            mod_name: String::new(),
                        };

                        if !self.all_configs.contains_key(&key) {
                            self.all_configs.insert(key, config);
                            debug!("SkinConfig loaded successfully: {:?}", config_file);
                        } else {
                            error!("Duplicate skin config: {:?}", config_file);
                        }
                    },
                    Err(ex) => {
                        error!("Exception loading SkinConfig '{:?}':", config_file);
                        error!("  {}", ex);
                        if let Some(source) = ex.source() {
                            error!("  Source: {}", source);
                        }
                    }
                }
            }
        }

        // Load mod skins
        let mods_handles: Vec<PathBuf> = if console_mode {
            // In console mode, list mods directly
            let mods_dir = Path::new("mods");
            if mods_dir.exists() {
                fs::read_dir(mods_dir)
                    .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", mods_dir))
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir() && !path.file_name().map_or(false, |name| name.to_string_lossy().starts_with('.')))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            // In normal mode, use RulesetCache
            RulesetCache::values()
                .iter()
                .filter_map(|ruleset| ruleset.folder_location.clone())
                .collect()
        };

        for mod_folder in mods_handles {
            let mod_name = mod_folder.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();

            let skins_dir = mod_folder.join("jsons/Skins");
            if !skins_dir.exists() || !skins_dir.is_dir() {
                continue;
            }

            match fs::read_dir(&skins_dir) {
                Ok(entries) => {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let config_file = entry.path();
                        if !config_file.is_file() || config_file.extension().map_or(true, |ext| ext != "json") {
                            continue;
                        }

                        if let Some(file_name) = config_file.file_stem() {
                            skin_name = file_name.to_string_lossy().replace("Config", "");

                            match from_json_file::<SkinConfig>(&config_file) {
                                Ok(config) => {
                                    let key = SkinAndMod {
                                        skin: skin_name.clone(),
                                        mod_name: mod_name.clone(),
                                    };

                                    if !self.all_configs.contains_key(&key) {
                                        self.all_configs.insert(key, config);
                                        debug!("Skin loaded successfully: {:?}", config_file);
                                    } else {
                                        error!("Duplicate skin config: {:?}", config_file);
                                    }
                                },
                                Err(ex) => {
                                    error!("Exception loading Skin '{}/jsons/Skins/{}':", mod_name, skin_name);
                                    error!("  {}", ex);
                                    if let Some(source) = ex.source() {
                                        error!("  Source: {}", source);
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to read directory {:?}: {}", skins_dir, e);
                }
            }
        }

        // No game is loaded, this is just the initial game setup
        self.assemble_skin_configs(&HashSet::new());
    }

    /// Get a skin configuration by name
    pub fn get(&self, name: &str) -> Option<&SkinConfig> {
        self.active_configs.get(name)
    }

    /// Get all active skin configurations
    pub fn get_all(&self) -> &HashMap<String, SkinConfig> {
        &self.active_configs
    }
}

// Singleton instance
lazy_static::lazy_static! {
    pub static ref SKIN_CACHE: std::sync::Mutex<SkinCache> = std::sync::Mutex::new(SkinCache::new());
}