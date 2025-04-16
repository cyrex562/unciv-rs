// Source: orig_src/core/src/com/unciv/logic/files/MapSaver.kt
// Ported to Rust

use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use std::fs;
use std::io::{self, Read, Write};
use serde_json;

use crate::constants::Constants;
use crate::unciv_game::UncivGame;
use crate::utils::file_chooser::FileHandle;
use crate::logic::map::{TileMap, MapParameters};
use crate::utils::gzip::Gzip;

/// Handles saving and loading map files
pub struct MapSaver;

impl MapSaver {
    /// The maps folder name
    pub const MAPS_FOLDER: &'static str = "maps";

    /// Whether to save maps in zipped format
    pub static SAVE_ZIPPED: AtomicBool = AtomicBool::new(true);

    /// Get the map file handle for a given map name
    fn get_map(map_name: &str) -> FileHandle {
        UncivGame::current().files().get_local_file(&format!("{}/{}", Self::MAPS_FOLDER, map_name))
    }

    /// Convert a map from its saved string format
    pub fn map_from_saved_string(map_string: &str) -> io::Result<TileMap> {
        let unzipped_json = match Gzip::unzip(map_string.trim()) {
            Ok(json) => json,
            Err(_) => map_string.to_string(),
        };
        Self::map_from_json(&unzipped_json)
    }

    /// Convert a map to its saved string format
    pub fn map_to_saved_string(tile_map: &mut TileMap) -> io::Result<String> {
        tile_map.assign_continents(TileMap::AssignContinentsMode::Reassign);
        let map_json = serde_json::to_string(tile_map)?;

        if Self::SAVE_ZIPPED.load(Ordering::Relaxed) {
            Ok(Gzip::zip(&map_json)?)
        } else {
            Ok(map_json)
        }
    }

    /// Save a map to disk
    pub fn save_map(map_name: &str, tile_map: &mut TileMap) -> io::Result<()> {
        let map_string = Self::map_to_saved_string(tile_map)?;
        let file_handle = Self::get_map(map_name);
        file_handle.write_string(&map_string)
    }

    /// Load a map from a file handle
    pub fn load_map(map_file: &FileHandle) -> io::Result<TileMap> {
        let map_string = map_file.read_string()?;
        Self::map_from_saved_string(&map_string)
    }

    /// Get all available map files
    pub fn get_maps() -> io::Result<Vec<FileHandle>> {
        let maps_folder = UncivGame::current().files().get_local_file(Self::MAPS_FOLDER);
        maps_folder.list()
    }

    /// Convert a JSON string to a TileMap
    fn map_from_json(json: &str) -> io::Result<TileMap> {
        serde_json::from_str(json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Load map parameters from a file
    pub fn load_map_parameters(map_file: &FileHandle) -> io::Result<MapParameters> {
        let preview = Self::load_map_preview(map_file)?;
        Ok(preview.map_parameters)
    }

    /// Load a map preview from a file
    pub fn load_map_preview(map_file: &FileHandle) -> io::Result<TileMap::Preview> {
        let map_string = map_file.read_string()?;
        Self::map_preview_from_saved_string(&map_string)
    }

    /// Convert a saved string to a map preview
    fn map_preview_from_saved_string(map_string: &str) -> io::Result<TileMap::Preview> {
        let unzipped_json = match Gzip::unzip(map_string.trim()) {
            Ok(json) => json,
            Err(_) => map_string.to_string(),
        };
        serde_json::from_str(&unzipped_json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_save_and_load() {
        // TODO: Add tests for map saving and loading
    }
}