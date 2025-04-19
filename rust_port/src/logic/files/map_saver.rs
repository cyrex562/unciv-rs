use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use serde_json;

use crate::utils::gzip::Gzip;
use crate::utils::file_chooser::FileHandle;
use crate::map_parameters::MapParameters;
use crate::logic::map::{self, AssignContinentsMode};

/// Whether to save maps in zipped format
pub static SAVE_ZIPPED: AtomicBool = AtomicBool::new(true);

/// Handles saving and loading map files
pub struct MapSaver;

impl MapSaver {
    /// The maps folder name
    pub const MAPS_FOLDER: &'static str = "maps";

    /// Convert a map from its saved string format
    pub fn map_from_saved_string(map_string: &str) -> io::Result<map::TileMap> {
        let trimmed = map_string.trim();
        let json = Gzip::unzip(trimmed);

        serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Convert a map to its saved string format
    pub fn map_to_saved_string(tile_map: &mut map::TileMap) -> io::Result<String> {
        tile_map.assign_continents(AssignContinentsMode::Reassign);
        let map_json = serde_json::to_string(tile_map)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if SAVE_ZIPPED.load(Ordering::Relaxed) {
            let zipped = Gzip::zip(&map_json);
            Ok(zipped)
        } else {
            Ok(map_json)
        }
    }

    /// Save a map to disk
    pub fn save_map(map_name: &str, tile_map: &mut map::TileMap) -> io::Result<()> {
        let map_string = Self::map_to_saved_string(tile_map)?;
        let file = FileHandle::from_path(format!("{}/{}", Self::MAPS_FOLDER, map_name));
        file.write_contents(map_string.as_bytes())
    }

    /// Save a map to a file
    pub fn save_map_to_file(tile_map: &map::TileMap, file_handle: &FileHandle) -> io::Result<()> {
        let map_string = serde_json::to_string(tile_map)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        file_handle.write_contents(map_string.as_bytes())
    }

    /// Load a map from a file handle
    pub fn load_map(map_file: &FileHandle) -> io::Result<map::TileMap> {
        let contents = map_file.read_contents()?;
        let map_string = String::from_utf8(contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Self::map_from_saved_string(&map_string)
    }

    /// Load map parameters from a file
    pub fn load_map_parameters(map_file: &FileHandle) -> io::Result<MapParameters> {
        Self::load_map_preview(map_file)
            .map(|preview| preview.map_parameters)
    }

    /// Load a map preview from a file
    pub fn load_map_preview(map_file: &FileHandle) -> io::Result<map::Preview> {
        let contents = map_file.read_contents()?;
        let map_string = String::from_utf8(contents)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Self::map_preview_from_saved_string(&map_string)
    }

    /// Convert a saved string to a map preview
    fn map_preview_from_saved_string(map_string: &str) -> io::Result<map::Preview> {
        let trimmed = map_string.trim();
        let json = Gzip::unzip(trimmed);

        serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// List all map files in a folder
    pub fn list_map_files(maps_folder: &FileHandle) -> io::Result<Vec<FileHandle>> {
        maps_folder.list_files()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_map_save_and_load() {
        // TODO: Add tests for map saving and loading
    }
}