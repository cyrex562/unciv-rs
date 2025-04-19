// Source: orig_src/desktop/src/com/unciv/app/desktop/ImagePacker.kt
// Ported to Rust

use std::fs;
use std::path::{Path, PathBuf};
use image::{self, ImageBuffer};
use crate::utils::log::Log;

/// Re-packs texture assets into atlas + png File pairs, which will be loaded by the game.
/// With the exception of the ExtraImages folder and the Font system these are the only graphics used.
pub struct ImagePacker;

impl ImagePacker {
    const SUFFIX_USING_LINEAR: &'static str = "Icons";
    const ATLAS_LIST_FILENAME: &'static str = "Atlases.json";
    const IMAGE_EXTENSIONS: &'static [&'static str] = &["png", "jpg", "jpeg"];

    /// Gets default texture packer settings
    fn get_default_settings() -> TexturePackerSettings {
        let mut settings = TexturePackerSettings::default();

        // Apparently some chipsets, like NVIDIA Tegra 3 graphics chipset (used in Asus TF700T tablet),
        // don't support non-power-of-two texture sizes
        settings.max_width = 2048;
        settings.max_height = 2048;

        // Trying to disable the subdirectory combine lead to even worse results. Don't.
        settings.combine_subdirectories = true;
        settings.pot = true; // powers of two only for width/height
        settings.fast = true; // with pot on this just sorts by width

        // Set some additional padding and enable duplicatePadding to prevent image edges from bleeding
        settings.padding_x = 8;
        settings.padding_y = 8;
        settings.duplicate_padding = true;
        settings.filter_min = TextureFilter::MipMapLinearLinear;
        settings.filter_mag = TextureFilter::MipMapLinearLinear;

        settings
    }

    /// Packs images into atlases
    pub fn pack_images(is_run_from_jar: bool, data_directory: &str) {
        let start_time = std::time::Instant::now();
        let default_settings = Self::get_default_settings();

        // Scan for Image folders and build one atlas each
        if !is_run_from_jar {
            Self::pack_images_per_mod(
                BUILTIN_IMAGE_SOURCE_PATH,
                BUILTIN_ATLAS_DESTINATION_PATH,
                &default_settings,
            );
        }

        // Pack for mods
        let mod_directory = Path::new(data_directory).join(MODS_BASE_PATH);
        if mod_directory.exists() {
            if let Ok(entries) = fs::read_dir(&mod_directory) {
                for entry in entries.filter_map(Result::ok) {
                    let mod_path = entry.path();
                    if !Self::is_hidden(&mod_path) {
                        match Self::pack_images_per_mod(
                            &mod_path.to_string_lossy(),
                            &mod_path.to_string_lossy(),
                            &default_settings,
                        ) {
                            Ok(_) => (),
                            Err(e) => {
                                let mod_name = mod_path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                if let Some(inner_error) = e.source() {
                                    Log::error(&format!(
                                        "Exception in ImagePacker for mod {}: {} ({})",
                                        mod_name, e, inner_error
                                    ));
                                } else {
                                    Log::error(&format!(
                                        "Exception in ImagePacker for mod {}: {}",
                                        mod_name, e
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        let texture_packing_time = start_time.elapsed();
        Log::debug(&format!("Packing textures - {}ms", texture_packing_time.as_millis()));
    }

    /// Packs images for a specific mod
    fn pack_images_per_mod(
        input: &str,
        output: &str,
        default_settings: &TexturePackerSettings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let base_dir = Path::new(input);
        if !base_dir.join(IMAGES_PATH_BASE).exists() && !base_dir.join(EXIST_CHECK2).exists() {
            return Ok(());
        }

        let mut atlas_list = Vec::new();
        for (file, pack_file_name) in Self::image_folders(base_dir) {
            atlas_list.push(pack_file_name.clone());
            let mut settings = default_settings.clone();
            settings.filter_mag = if file.ends_with(Self::SUFFIX_USING_LINEAR) {
                TextureFilter::Linear
            } else {
                TextureFilter::MipMapLinearLinear
            };
            Self::pack_images_if_outdated(&settings, &file, output, &pack_file_name)?;
        }

        let list_file = Path::new(output).join(Self::ATLAS_LIST_FILENAME);
        if atlas_list.is_empty() {
            if list_file.exists() {
                fs::remove_file(list_file)?;
            }
        } else {
            atlas_list.sort();
            fs::write(
                list_file,
                format!("[{}]", atlas_list.join(",")),
            )?;
        }

        Ok(())
    }

    /// Checks if a path is hidden
    fn is_hidden(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("."))
            .unwrap_or(false)
    }

    /// Gets image folders and their corresponding atlas names
    fn image_folders(parent: &Path) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path.file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if name != IMAGES_PATH_BASE {
                    continue;
                }
                let atlas_name = if path.file_name().and_then(|n| n.to_str()) == Some(IMAGES_PATH_BASE) {
                    "game".to_string()
                } else {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or_default()
                        .to_string()
                };
                result.push((path.to_string_lossy().into_owned(), atlas_name));
            }
        }
        result
    }

    /// Packs images if the atlas is outdated
    fn pack_images_if_outdated(
        settings: &TexturePackerSettings,
        input: &str,
        output: &str,
        pack_file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let atlas_file = Path::new(output).join(format!("{}.atlas", pack_file_name));
        let png_file = Path::new(output).join(format!("{}.png", pack_file_name));

        if atlas_file.exists() && png_file.exists() {
            let atlas_mod_time = fs::metadata(&atlas_file)?.modified()?;

            let input_path = Path::new(input);
            let mut needs_update = false;

            if let Ok(entries) = fs::read_dir(input_path) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    let metadata = fs::metadata(&path)?;

                    if Self::is_image_file(&path) || path.file_name() == Some("TexturePacker.settings".as_ref()) {
                        if metadata.modified()? > atlas_mod_time {
                            needs_update = true;
                            break;
                        }
                    }
                }
            }

            if !needs_update {
                return Ok(());
            }
        }

        // TODO: Implement actual texture packing using a Rust texture packing library
        // For now, this is a placeholder that copies files without packing
        Log::debug(&format!("Would pack textures from {} to {}", input, output));

        Ok(())
    }

    /// Checks if a path is an image file
    fn is_image_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Self::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }
}

/// Settings for texture packing
#[derive(Clone, Debug)]
struct TexturePackerSettings {
    max_width: u32,
    max_height: u32,
    combine_subdirectories: bool,
    pot: bool,
    fast: bool,
    padding_x: u32,
    padding_y: u32,
    duplicate_padding: bool,
    filter_min: TextureFilter,
    filter_mag: TextureFilter,
}

impl Default for TexturePackerSettings {
    fn default() -> Self {
        Self {
            max_width: 1024,
            max_height: 1024,
            combine_subdirectories: false,
            pot: false,
            fast: false,
            padding_x: 2,
            padding_y: 2,
            duplicate_padding: false,
            filter_min: TextureFilter::Linear,
            filter_mag: TextureFilter::Linear,
        }
    }
}

/// Texture filtering options
#[derive(Clone, Copy, Debug)]
enum TextureFilter {
    Linear,
    MipMapLinearLinear,
}

// Constants
const BUILTIN_IMAGE_SOURCE_PATH: &str = "Images";
const BUILTIN_ATLAS_DESTINATION_PATH: &str = "atlas";
const MODS_BASE_PATH: &str = "mods";
const IMAGES_PATH_BASE: &str = "Images";
const EXIST_CHECK2: &str = "Images.Civ V base ruleset";