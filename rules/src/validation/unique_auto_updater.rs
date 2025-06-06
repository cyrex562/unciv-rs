use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::models::ruleset::{Ruleset, RulesetFile};
use crate::models::ruleset::unique::Unique;
use crate::utils::log::Log;

/// Module for automatically updating deprecated uniques in the ruleset.
pub struct UniqueAutoUpdater;

impl UniqueAutoUpdater {
    /// Automatically updates deprecated uniques in the ruleset files.
    ///
    /// # Arguments
    ///
    /// * `mod_ruleset` - The ruleset to update
    /// * `replaceable_uniques` - Optional map of original uniques to their replacements.
    ///                          If not provided, it will be generated by calling `get_deprecated_replaceable_uniques`.
    pub fn autoupdate_uniques(
        mod_ruleset: &Ruleset,
        replaceable_uniques: Option<HashMap<String, String>>
    ) -> Result<(), String> {
        let replaceable_uniques = replaceable_uniques.unwrap_or_else(|| {
            Self::get_deprecated_replaceable_uniques(mod_ruleset)
        });

        let files_to_replace: Vec<String> = RulesetFile::iter()
            .map(|file| file.filename().to_string())
            .collect();

        let json_folder = mod_ruleset.folder_location.as_ref()
            .ok_or_else(|| "Ruleset must have a folder location".to_string())?
            .child("jsons");

        for file_name in files_to_replace {
            let file_path = json_folder.child(&file_name);

            // Skip if file doesn't exist or is a directory
            if !file_path.exists() || file_path.is_dir() {
                continue;
            }

            // Read the file content
            let file_content = fs::read_to_string(file_path.path())
                .map_err(|e| format!("Failed to read file {}: {}", file_name, e))?;

            // Replace deprecated uniques
            let mut new_file_text = file_content;
            for (original, replacement) in &replaceable_uniques {
                new_file_text = new_file_text.replace(&format!("\"{}\"", original), &format!("\"{}\"", replacement));
                new_file_text = new_file_text.replace(&format!("<{}>", original), &format!("<{}>", replacement));
            }

            // Write the updated content back to the file
            fs::write(file_path.path(), new_file_text)
                .map_err(|e| format!("Failed to write file {}: {}", file_name, e))?;
        }

        Ok(())
    }

    /// Gets a map of deprecated uniques to their replacements.
    ///
    /// # Arguments
    ///
    /// * `mod_ruleset` - The ruleset to analyze
    ///
    /// # Returns
    ///
    /// A map of original unique texts to their replacement texts
    pub fn get_deprecated_replaceable_uniques(mod_ruleset: &Ruleset) -> HashMap<String, String> {
        let all_uniques = mod_ruleset.all_uniques();
        let mut all_deprecated_uniques = HashSet::new();
        let mut deprecated_uniques_to_replacement_text = HashMap::new();

        // Get all uniques with deprecation annotations
        let deprecated_uniques: Vec<&Unique> = all_uniques.iter()
            .filter(|unique| unique.get_deprecation_annotation().is_some())
            .collect();

        // Get all modifiers with deprecation annotations
        let deprecated_conditionals: Vec<&Unique> = all_uniques.iter()
            .flat_map(|unique| unique.modifiers.iter())
            .filter(|modifier| modifier.get_deprecation_annotation().is_some())
            .collect();

        // Process all deprecated uniques and conditionals
        for deprecated_unique in deprecated_uniques.iter().chain(deprecated_conditionals.iter()) {
            // Skip if we've already processed this unique
            if all_deprecated_uniques.contains(deprecated_unique.text()) {
                continue;
            }

            all_deprecated_uniques.insert(deprecated_unique.text().to_string());

            // Get the replacement text, handling nested deprecations
            let mut unique_replacement_text = deprecated_unique.get_replacement_text(mod_ruleset);
            while Unique::new(&unique_replacement_text).get_deprecation_annotation().is_some() {
                unique_replacement_text = Unique::new(&unique_replacement_text).get_replacement_text(mod_ruleset);
            }

            // Add any modifiers to the replacement text
            for conditional in deprecated_unique.modifiers.iter() {
                unique_replacement_text.push_str(&format!(" <{}>", conditional.text()));
            }

            let replacement_unique = Unique::new(&unique_replacement_text);

            // Check for mod-invariant errors
            let mod_invariant_errors = UniqueValidator::new(mod_ruleset).check_unique(
                &replacement_unique,
                false,
                None,
                true
            );

            // Log errors and skip if there are any
            for error in &mod_invariant_errors {
                Log::error("ModInvariantError: {} - {:?}", error.text, error.error_severity_to_report);
            }

            if !mod_invariant_errors.is_empty() {
                continue;
            }

            // For base rulesets, also check for mod-specific errors
            if mod_ruleset.mod_options.is_base_ruleset {
                let mod_specific_errors = UniqueValidator::new(mod_ruleset).check_unique(
                    &replacement_unique,
                    false,
                    None,
                    true
                );

                // Log errors and skip if there are any
                for error in &mod_specific_errors {
                    Log::error("ModSpecificError: {} - {:?}", error.text, error.error_severity_to_report);
                }

                if !mod_specific_errors.is_empty() {
                    continue;
                }
            }

            // Add the replacement to the map
            deprecated_uniques_to_replacement_text.insert(
                deprecated_unique.text().to_string(),
                unique_replacement_text
            );

            Log::debug("Replace \"{}\" with \"{}\"", deprecated_unique.text(), unique_replacement_text);
        }

        deprecated_uniques_to_replacement_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_deprecated_replaceable_uniques() {
        // This test would require a mock Ruleset with some deprecated uniques
        // For now, we'll just test that the function returns a HashMap
        let ruleset = Ruleset::new();
        let result = UniqueAutoUpdater::get_deprecated_replaceable_uniques(&ruleset);
        assert!(result.is_empty());
    }

    #[test]
    fn test_autoupdate_uniques() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let jsons_dir = temp_dir.path().join("jsons");
        fs::create_dir(&jsons_dir).unwrap();

        // Create a test file with some uniques
        let test_file_path = jsons_dir.join("test.json");
        fs::write(&test_file_path, r#"{"uniques": ["old_unique", "<old_modifier>"]}"#).unwrap();

        // Create a mock ruleset with the temp directory
        let mut ruleset = Ruleset::new();
        ruleset.folder_location = Some(FileHandle::new(temp_dir.path().to_path_buf()));

        // Create a map of replacements
        let mut replacements = HashMap::new();
        replacements.insert("old_unique".to_string(), "new_unique".to_string());
        replacements.insert("old_modifier".to_string(), "new_modifier".to_string());

        // Run the autoupdate
        let result = UniqueAutoUpdater::autoupdate_uniques(&ruleset, Some(replacements));
        assert!(result.is_ok());

        // Check that the file was updated
        let updated_content = fs::read_to_string(&test_file_path).unwrap();
        assert!(updated_content.contains("\"new_unique\""));
        assert!(updated_content.contains("<new_modifier>"));
    }
}