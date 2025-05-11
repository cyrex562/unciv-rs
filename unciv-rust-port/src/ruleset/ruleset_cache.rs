use std::collections::{HashMap, HashSet, LinkedHashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::models::ruleset::Ruleset;
use crate::models::metadata::BaseRuleset;
use crate::models::metadata::GameParameters;
use crate::models::ruleset::validation::{RulesetError, RulesetErrorList, RulesetErrorSeverity};
use crate::models::ruleset::validation::text_similarity::TextSimilarity;
use crate::utils::log::Log;
use crate::utils::debug;

/// Similarity below which an untyped unique can be considered a potential misspelling.
/// Roughly corresponds to the fraction of the Unique placeholder text that can be different/misspelled,
/// but with some extra room for text similarity idiosyncrasies.
const UNIQUE_MISSPELLING_THRESHOLD: f64 = 0.15;

/// A cache for loaded rulesets to avoid reloading them multiple times.
/// This is a singleton that holds all loaded rulesets.
pub struct RulesetCache {
    /// The map of ruleset names to rulesets
    rulesets: Mutex<HashMap<String, Ruleset>>,
}

impl RulesetCache {
    /// Get the singleton instance of RulesetCache
    pub fn instance() -> &'static RulesetCache {
        static INSTANCE: Lazy<RulesetCache> = Lazy::new(|| RulesetCache {
            rulesets: Mutex::new(HashMap::new()),
        });
        &INSTANCE
    }

    /// Load all rulesets from the filesystem
    ///
    /// # Arguments
    ///
    /// * `console_mode` - Whether to run in console mode (affects file loading)
    /// * `no_mods` - Whether to skip loading mods
    ///
    /// # Returns
    ///
    /// A list of error messages that occurred during loading
    pub fn load_rulesets(&self, console_mode: bool, no_mods: bool) -> Vec<String> {
        let mut new_rulesets = HashMap::new();
        let mut error_lines = Vec::new();

        // Load base rulesets
        for ruleset in BaseRuleset::entries() {
            let file_name = format!("jsons/{}", ruleset.full_name());
            let file_path = if console_mode {
                PathBuf::from(&file_name)
            } else {
                // In non-console mode, we would use the game's internal file system
                // This is a placeholder - you'll need to implement the actual file loading
                PathBuf::from(&file_name)
            };

            let mut new_ruleset = Ruleset::new();
            new_ruleset.name = ruleset.full_name().to_string();

            match new_ruleset.load(&file_path) {
                Ok(_) => {
                    new_rulesets.insert(ruleset.full_name().to_string(), new_ruleset);
                }
                Err(e) => {
                    error_lines.push(format!("Error loading base ruleset '{}': {}", ruleset.full_name(), e));
                }
            }
        }

        // Load mods if requested
        if !no_mods {
            let mods_path = if console_mode {
                PathBuf::from("mods")
            } else {
                // In non-console mode, we would use the game's mods folder
                // This is a placeholder - you'll need to implement the actual path resolution
                PathBuf::from("mods")
            };

            if let Ok(mod_folders) = fs::read_dir(&mods_path) {
                for entry in mod_folders {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                        // Skip hidden folders
                        if name.starts_with('.') || !path.is_dir() {
                            continue;
                        }

                        let mut mod_ruleset = Ruleset::new();
                        mod_ruleset.name = name.clone();

                        let jsons_path = path.join("jsons");
                        match mod_ruleset.load(&jsons_path) {
                            Ok(_) => {
                                mod_ruleset.folder_location = Some(path.to_string_lossy().to_string());
                                new_rulesets.insert(name.clone(), mod_ruleset);
                                debug!("Mod loaded successfully: {}", name);

                                if Log::should_log() {
                                    let mod_links_errors = mod_ruleset.get_error_list(false);

                                    // For extension mods which use references to base ruleset objects, the parameter type
                                    // errors are irrelevant - the checker ran without a base ruleset
                                    let log_filter = if mod_ruleset.mod_options.is_base_ruleset {
                                        |error: &RulesetError| error.error_severity_to_report > RulesetErrorSeverity::WarningOptionsOnly
                                    } else {
                                        |error: &RulesetError| {
                                            error.error_severity_to_report > RulesetErrorSeverity::WarningOptionsOnly
                                            && !error.text.contains("does not fit parameter type")
                                        }
                                    };

                                    if mod_links_errors.iter().any(log_filter) {
                                        debug!(
                                            "checkModLinks errors: {}",
                                            mod_links_errors.get_error_text(log_filter)
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error_lines.push(format!("Exception loading mod '{}':", name));
                                error_lines.push(format!("  {}", e));
                                if let Some(cause) = e.source() {
                                    error_lines.push(format!("  {}", cause));
                                }
                            }
                        }
                    }
                }
            }

            if Log::should_log() {
                for line in &error_lines {
                    debug!("{}", line);
                }
            }
        }

        // We save the 'old' cache values until we're ready to replace everything, so that the cache isn't empty while we try to load ruleset files
        // - this previously lead to "can't find Vanilla ruleset" if the user had a lot of mods and downloaded a new one
        let mut rulesets = self.rulesets.lock().unwrap();
        rulesets.clear();
        for (name, ruleset) in new_rulesets {
            rulesets.insert(name, ruleset);
        }

        error_lines
    }

    /// Get the vanilla ruleset (a clone to prevent accidental modification)
    pub fn get_vanilla_ruleset(&self) -> Ruleset {
        let rulesets = self.rulesets.lock().unwrap();
        rulesets.get(&BaseRuleset::Civ_V_Vanilla.full_name().to_string())
            .expect("Vanilla ruleset not found")
            .clone()
    }

    /// Get a sorted list of base ruleset names
    pub fn get_sorted_base_rulesets(&self) -> Vec<String> {
        let rulesets = self.rulesets.lock().unwrap();
        let mut base_rulesets: Vec<String> = rulesets.values()
            .filter(|r| r.mod_options.is_base_ruleset)
            .map(|r| r.name.clone())
            .collect();

        base_rulesets.sort_unstable_by(|a, b| {
            // We sort the base rulesets such that the ones unciv provides are on the top,
            // and the rest is alphabetically ordered.
            let a_ordinal = BaseRuleset::entries()
                .iter()
                .find(|br| br.full_name() == *a)
                .map(|br| br.ordinal())
                .unwrap_or(BaseRuleset::entries().len());

            let b_ordinal = BaseRuleset::entries()
                .iter()
                .find(|br| br.full_name() == *b)
                .map(|br| br.ordinal())
                .unwrap_or(BaseRuleset::entries().len());

            a_ordinal.cmp(&b_ordinal).then(a.cmp(b))
        });

        base_rulesets
    }

    /// Create a combined ruleset from map parameters
    pub fn get_complex_ruleset_from_map_params(&self, params: &GameParameters) -> Ruleset {
        self.get_complex_ruleset(&params.mods, Some(&params.base_ruleset))
    }

    /// Create a combined ruleset from game parameters
    pub fn get_complex_ruleset_from_game_params(&self, params: &GameParameters) -> Ruleset {
        self.get_complex_ruleset(&params.mods, Some(&params.base_ruleset))
    }

    /// Create a combined ruleset from a list of mods
    ///
    /// # Arguments
    ///
    /// * `mods` - The list of mod names to include
    /// * `optional_base_ruleset` - Optional base ruleset name. If None or not found, vanilla is used.
    ///
    /// # Returns
    ///
    /// A new combined ruleset
    pub fn get_complex_ruleset(&self, mods: &LinkedHashSet<String>, optional_base_ruleset: Option<&str>) -> Ruleset {
        let rulesets = self.rulesets.lock().unwrap();

        // Get the base ruleset
        let base_ruleset = if let Some(base_name) = optional_base_ruleset {
            if let Some(ruleset) = rulesets.get(base_name) {
                if ruleset.mod_options.is_base_ruleset {
                    ruleset.clone()
                } else {
                    self.get_vanilla_ruleset()
                }
            } else {
                self.get_vanilla_ruleset()
            }
        } else {
            self.get_vanilla_ruleset()
        };

        // Get the extension mods
        let loaded_mods: Vec<Ruleset> = mods.iter()
            .filter_map(|mod_name| rulesets.get(mod_name).cloned())
            .filter(|ruleset| !ruleset.mod_options.is_base_ruleset)
            .collect();

        self.get_complex_ruleset_from_base_and_extensions(&base_ruleset, &loaded_mods)
    }

    /// Create a combined ruleset from a base ruleset and extension rulesets
    ///
    /// # Arguments
    ///
    /// * `base_ruleset` - The base ruleset to use
    /// * `extension_rulesets` - The extension rulesets to add
    ///
    /// # Returns
    ///
    /// A new combined ruleset
    pub fn get_complex_ruleset_from_base_and_extensions(&self, base_ruleset: &Ruleset, extension_rulesets: &[Ruleset]) -> Ruleset {
        let mut new_ruleset = Ruleset::new();

        // Combine all mods, with base ruleset first
        let mut all_mods = Vec::with_capacity(extension_rulesets.len() + 1);
        all_mods.extend(extension_rulesets.iter().cloned());
        all_mods.push(base_ruleset.clone());

        // Sort by base ruleset status (base rulesets first)
        all_mods.sort_by(|a, b| {
            b.mod_options.is_base_ruleset.cmp(&a.mod_options.is_base_ruleset)
        });

        // Add each mod to the new ruleset
        for mod_ruleset in all_mods {
            if mod_ruleset.mod_options.is_base_ruleset {
                // This is so we don't keep using the base ruleset's uniques *by reference* and add to in ad infinitum
                new_ruleset.mod_options.uniques = Vec::new();
                new_ruleset.mod_options.is_base_ruleset = true;
                // Default tileset and unitset are according to base ruleset
                new_ruleset.mod_options.tileset = mod_ruleset.mod_options.tileset.clone();
                new_ruleset.mod_options.unitset = mod_ruleset.mod_options.unitset.clone();
            }

            new_ruleset.add(&mod_ruleset);
            new_ruleset.mods.insert(mod_ruleset.name.clone());
        }

        // Only after we've added all the mods can we calculate the building costs
        new_ruleset.update_building_costs();
        new_ruleset.update_resource_transients();

        new_ruleset
    }

    /// Check for errors in a combined ruleset
    ///
    /// # Arguments
    ///
    /// * `mods` - The list of mod names to include
    /// * `base_ruleset` - Optional base ruleset name
    /// * `try_fix_unknown_uniques` - Whether to try to fix unknown uniques
    ///
    /// # Returns
    ///
    /// A list of errors found in the combined ruleset
    pub fn check_combined_mod_links(&self, mods: &LinkedHashSet<String>, base_ruleset: Option<&str>, try_fix_unknown_uniques: bool) -> RulesetErrorList {
        match self.get_complex_ruleset(mods, base_ruleset) {
            Ok(mut new_ruleset) => {
                new_ruleset.mod_options.is_base_ruleset = true; // This is so the checkModLinks finds all connections
                new_ruleset.get_error_list(try_fix_unknown_uniques)
            }
            Err(e) => {
                // This happens if a building is dependent on a tech not in the base ruleset
                // because new_ruleset.update_building_costs() in get_complex_ruleset() throws an error
                RulesetErrorList::of(&e.to_string(), RulesetErrorSeverity::Error)
            }
        }
    }
}

impl std::ops::Deref for RulesetCache {
    type Target = Mutex<HashMap<String, Ruleset>>;

    fn deref(&self) -> &Self::Target {
        &self.rulesets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_vanilla_ruleset() {
        let cache = RulesetCache::instance();
        let vanilla = cache.get_vanilla_ruleset();
        assert_eq!(vanilla.name, BaseRuleset::Civ_V_Vanilla.full_name());
    }

    #[test]
    fn test_get_sorted_base_rulesets() {
        let cache = RulesetCache::instance();
        let base_rulesets = cache.get_sorted_base_rulesets();
        assert!(!base_rulesets.is_empty());
        assert!(base_rulesets.contains(&BaseRuleset::Civ_V_Vanilla.full_name().to_string()));
    }
}