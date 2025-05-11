use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use ggez::graphics::{self, Image};
use ggez::Context;
use ggez::GameResult;
use serde_json;
use log::debug;

use crate::models::ruleset::Ruleset;
use crate::models::ruleset::validation::{RulesetErrorList, RulesetErrorSeverity};

/// A class that extracts all texture names from all atlases of a Ruleset.
///
/// Weak points:
/// - For combined rulesets, this always loads the builtin assets
/// - Used by RulesetValidator to check texture names without relying on ImageGetter
/// - Doubles as integrity checker and detects:
///   - Atlases.json names an atlas that does not exist
///   - Existing atlas is empty
///   - If Atlases.json names an atlas that does not exist, but the corresponding Images folder exists:
///     - Non-png files in the Images folders
///     - Any png files in the Images folders that can't be loaded
pub struct AtlasPreview {
    region_names: HashSet<String>,
}

impl AtlasPreview {
    /// Creates a new AtlasPreview for the given ruleset.
    pub fn new(ruleset: &Ruleset, error_list: &mut RulesetErrorList) -> Self {
        let mut preview = Self {
            region_names: HashSet::new(),
        };

        preview.initialize(ruleset, error_list);
        preview
    }

    /// Initializes the AtlasPreview by loading and validating texture atlases.
    fn initialize(&mut self, ruleset: &Ruleset, error_list: &mut RulesetErrorList) {
        // For builtin rulesets, the Atlases.json is right in internal root
        let folder = ruleset.folder();
        let control_file = folder.join("Atlases.json");
        let control_file_exists = control_file.exists();

        let mut file_names = if control_file_exists {
            let content = fs::read_to_string(&control_file).unwrap_or_default();
            serde_json::from_str::<Vec<String>>(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        let backwards_compatibility = !ruleset.name.is_empty() && !file_names.contains(&"game".to_string());
        if backwards_compatibility {
            file_names.push("game".to_string()); // Backwards compatibility - when packed by 4.9.15+ this is already in the control file
        }

        for file_name in file_names {
            let file = folder.join(format!("{}.atlas", file_name));
            if !file.exists() {
                if control_file_exists && (file_name != "game" || !backwards_compatibility) {
                    self.log_missing_atlas(&file_name, ruleset, error_list);
                }
                continue;
            }

            // Load the atlas file and extract region names
            match self.load_atlas(&file, error_list) {
                Ok(regions) => {
                    for region in regions {
                        self.region_names.insert(region);
                    }
                }
                Err(_) => {
                    // Error already logged in load_atlas
                }
            }
        }

        debug!("Atlas preview for {}: {} entries.", ruleset.name, self.region_names.len());
    }

    /// Loads an atlas file and extracts region names.
    fn load_atlas(&self, file: &Path, error_list: &mut RulesetErrorList) -> Result<Vec<String>, ()> {
        // In Rust, we'll use ggez's Image loading capabilities to validate the atlas
        // This is a simplified approach compared to the Kotlin version which uses TextureAtlasData
        let ctx = &mut Context::default();

        // Try to load the atlas as an image to validate it
        match Image::new(ctx, file) {
            Ok(_) => {
                // In a real implementation, we would parse the atlas file to extract region names
                // For now, we'll just return an empty vector as a placeholder
                Ok(Vec::new())
            }
            Err(e) => {
                error_list.add(&format!("{} contains no textures: {}", file.display(), e));
                Err(())
            }
        }
    }

    /// Logs errors for missing atlas files and validates image files in the corresponding Images folder.
    fn log_missing_atlas(&self, name: &str, ruleset: &Ruleset, error_list: &mut RulesetErrorList) {
        error_list.add(&format!("Atlases.json contains \"{}\" but there is no corresponding atlas file.", name));

        let images_folder_name = if name == "game" { "Images" } else { &format!("Images.{}", name) };
        let images_folder = ruleset.folder().join(images_folder_name);

        if !images_folder.exists() || !images_folder.is_dir() {
            return;
        }

        // Walk through the images folder and validate PNG files
        if let Ok(entries) = fs::read_dir(&images_folder) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() || path.file_name().map_or(true, |n| n.to_string_lossy().starts_with('.')) {
                    continue;
                }

                if path.extension().map_or(true, |ext| ext != "png") {
                    let relative_path = self.get_relative_path(&path, ruleset);
                    error_list.add_with_severity(
                        &format!("{} contains {} which does not have the png extension",
                                images_folder_name, relative_path),
                        RulesetErrorSeverity::WarningOptionsOnly
                    );
                    continue;
                }

                // Try to load the image to validate it
                let ctx = &mut Context::default();
                match Image::new(ctx, &path) {
                    Ok(_) => {
                        // Image loaded successfully
                    }
                    Err(e) => {
                        let relative_path = self.get_relative_path(&path, ruleset);
                        error_list.add(&format!("Cannot load {}: {}", relative_path, e));
                    }
                }
            }
        }
    }

    /// Gets the relative path of a file within the ruleset folder.
    fn get_relative_path(&self, file: &Path, ruleset: &Ruleset) -> String {
        let ruleset_path = ruleset.folder();
        let file_path = file.to_string_lossy();
        let ruleset_path_str = ruleset_path.to_string_lossy();

        file_path
            .strip_prefix(&ruleset_path_str)
            .unwrap_or(&file_path)
            .trim_start_matches('/')
            .to_string()
    }

    /// Checks if an image with the given name exists in the atlas.
    pub fn image_exists(&self, name: &str) -> bool {
        self.region_names.contains(name)
    }
}

impl IntoIterator for AtlasPreview {
    type Item = String;
    type IntoIter = std::collections::hash_set::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.region_names.into_iter()
    }
}