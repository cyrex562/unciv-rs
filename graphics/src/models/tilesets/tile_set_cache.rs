use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use lazy_static::lazy_static;
use serde_json;
use crate::models::tilesets::{TileSet, TileSetConfig};
use crate::models::ruleset::RulesetCache;
use crate::game::UncivGame;
use crate::utils::file_utils::{list_files, is_directory, get_file_name, get_file_extension, get_file_name_without_extension};

/// A singleton that manages tile sets for the game
pub struct TileSetCache {
    /// The map of tile set names to tile sets
    tilesets: HashMap<String, TileSet>,
}

impl TileSetCache {
    /// Get the singleton instance
    pub fn instance() -> &'static Mutex<TileSetCache> {
        lazy_static! {
            static ref INSTANCE: Mutex<TileSetCache> = Mutex::new(TileSetCache {
                tilesets: HashMap::new(),
            });
        }
        &INSTANCE
    }

    /// Get the current tile set based on game settings
    pub fn get_current(&self) -> &TileSet {
        let game = UncivGame::current();
        let tile_set_name = &game.settings.tile_set;
        self.get(tile_set_name).expect("Current tile set not found")
    }

    /// Get a tile set by name
    pub fn get(&self, name: &str) -> Option<&TileSet> {
        self.tilesets.get(name)
    }

    /// Insert a tile set
    pub fn insert(&mut self, name: String, tileset: TileSet) -> Option<TileSet> {
        self.tilesets.insert(name, tileset)
    }

    /// Clear all tile sets
    pub fn clear(&mut self) {
        self.tilesets.clear();
    }

    /// Get all tile sets
    pub fn values(&self) -> &HashMap<String, TileSet> {
        &self.tilesets
    }

    /// Combine TileSetConfigs for chosen mods.
    /// Vanilla always active, even with a base ruleset mod active.
    /// Permanent visual mods always included as long as UncivGame.Current is initialized.
    /// Other active mods can be passed in parameter rule_set_mods.
    pub fn assemble_tile_set_configs(&mut self, rule_set_mods: &HashSet<String>) {
        // Needs to be a list and not a set, so subsequent mods override the previous ones
        // Otherwise you rely on hash randomness to determine override order... not good
        let mut mods = Vec::new();
        mods.push(TileSet::default_name().to_string());

        if UncivGame::is_current_initialized() {
            let game = UncivGame::current();
            mods.extend(game.settings.visual_mods.iter().cloned());
        }

        mods.extend(rule_set_mods.iter().cloned());

        // Reset all configs
        for tileset in self.tilesets.values_mut() {
            tileset.reset_config();
        }

        // Apply mods in order
        let unique_mods: Vec<String> = mods.into_iter().collect::<HashSet<String>>().into_iter().collect();
        for mod_name in unique_mods {
            for tileset in self.tilesets.values_mut() {
                tileset.merge_mod_config(&mod_name);
            }
        }
    }

    /// Load the json config files and do an initial assemble_tile_set_configs run without explicit mods.
    /// Runs from UncivGame.create without exception handling, therefore should not be vulnerable to broken mods.
    /// Also runs from ModManagementScreen after downloading a new mod or deleting one.
    pub fn load_tile_set_configs(&mut self, console_mode: bool) {
        self.clear();

        // Load internal TileSets
        let internal_files = if console_mode {
            list_files("jsons/TileSets")
        } else {
            let available_tilesets = UncivGame::current().image_getter.get_available_tilesets();
            available_tilesets
                .iter()
                .map(|name| format!("jsons/TileSets/{}.json", name))
                .filter(|path| Path::new(path).exists())
                .collect()
        };

        self.load_config_files(&internal_files, TileSet::default_name());

        // Load mod TileSets
        let mods_handles = if console_mode {
            list_files("mods")
        } else {
            RulesetCache::instance()
                .lock()
                .unwrap()
                .values()
                .iter()
                .filter_map(|ruleset| ruleset.folder_location.clone())
                .collect()
        };

        for mod_folder in mods_handles {
            let mod_name = get_file_name(&mod_folder);
            if !is_directory(&mod_folder) || mod_name.starts_with('.') {
                continue;
            }

            let mod_files = list_files(&format!("{}/jsons/TileSets", mod_folder));
            self.load_config_files(&mod_files, &mod_name);
        }

        // Set fallbacks
        for tileset in self.tilesets.values_mut() {
            if let Some(fallback_name) = &tileset.config.fallback_tile_set {
                if let Some(fallback) = self.tilesets.get(fallback_name) {
                    tileset.fallback = Some(Box::new(fallback.clone()));
                }
            }
        }

        // Initial setup with no mods
        self.assemble_tile_set_configs(&HashSet::new());
    }

    /// Load configuration files for tile sets
    fn load_config_files(&mut self, files: &[String], config_id: &str) {
        for config_file in files {
            // Skip directories and non-json files
            if is_directory(config_file) || get_file_extension(config_file) != "json" {
                continue;
            }

            // Try to load the config
            let config = match fs::read_to_string(config_file) {
                Ok(content) => match serde_json::from_str::<TileSetConfig>(&content) {
                    Ok(config) => config,
                    Err(_) => continue, // Ignore jsons that don't load
                },
                Err(_) => continue,
            };

            let name = get_file_name_without_extension(config_file)
                .trim_end_matches("Config")
                .to_string();

            let tileset = self.tilesets.entry(name.clone()).or_insert_with(|| TileSet::new(name));
            tileset.cache_config_from_mod(config_id.to_string(), config);
        }
    }

    /// Determines potentially available TileSets - by scanning for TileSet jsons.
    /// Available before initialization finishes.
    pub fn get_available_tilesets(&self, image_getter_tilesets: &[String]) -> HashSet<String> {
        let mut result = HashSet::new();

        // Get mod tileset config files
        let mods_folder = UncivGame::current().files.get_mods_folder();
        let mod_folders = list_files(&mods_folder);

        for mod_folder in mod_folders {
            if !is_directory(&mod_folder) || get_file_name(&mod_folder).starts_with('.') {
                continue;
            }

            let tileset_folder = format!("{}/jsons/TileSets", mod_folder);
            if !is_directory(&tileset_folder) {
                continue;
            }

            let mod_files = list_files(&tileset_folder);
            for file in mod_files {
                if get_file_extension(&file) == "json" {
                    let name = get_file_name_without_extension(&file)
                        .trim_end_matches("Config")
                        .to_string();
                    result.insert(name);
                }
            }
        }

        // Get builtin tileset config files
        for tileset in image_getter_tilesets {
            let path = format!("jsons/TileSets/{}.json", tileset);
            if Path::new(&path).exists() {
                result.insert(tileset.clone());
            }
        }

        result
    }
}

// Helper functions for file operations
mod file_utils {
    use std::path::{Path, PathBuf};
    use std::fs;

    pub fn list_files(path: &str) -> Vec<String> {
        match fs::read_dir(path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path().to_string_lossy().to_string())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn is_directory(path: &str) -> bool {
        Path::new(path).is_dir()
    }

    pub fn get_file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn get_file_extension(path: &str) -> String {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn get_file_name_without_extension(path: &str) -> String {
        Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string()
    }
}