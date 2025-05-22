use std::path::Path;
use std::fs;
use crate::models::{
    ruleset::{Ruleset, RulesetCache},
    ruleset::unique::UniqueType,
};

/// Helper collection dealing with declarative Mod compatibility
///
/// Implements:
/// - UniqueType::ModRequires
/// - UniqueType::ModIncompatibleWith
/// - UniqueType::ModIsAudioVisual
/// - UniqueType::ModIsNotAudioVisual
/// - UniqueType::ModIsAudioVisualOnly
///
/// Methods:
/// - meets_base_requirements - to build a checkbox list of Extension mods
/// - meets_all_requirements - to see if a mod is allowed in the context of a complete mod selection
pub struct ModCompatibility;

impl ModCompatibility {
    /// Should the "Permanent Audiovisual Mod" checkbox be shown for the given mod?
    ///
    /// Note: The guessing part may potentially be deprecated and removed if we get our Modders to complete declarative coverage.
    pub fn is_audio_visual_mod(mod_: &Ruleset) -> bool {
        Self::is_audio_visual_declared(mod_).unwrap_or_else(|| Self::is_audio_visual_guessed(mod_))
    }

    /// Checks if the mod is declared as audiovisual
    fn is_audio_visual_declared(mod_: &Ruleset) -> Option<bool> {
        if mod_.mod_options.has_unique(UniqueType::ModIsAudioVisualOnly) {
            return Some(true);
        }
        if mod_.mod_options.has_unique(UniqueType::ModIsAudioVisual) {
            return Some(true);
        }
        if mod_.mod_options.has_unique(UniqueType::ModIsNotAudioVisual) {
            return Some(false);
        }
        None
    }

    /// Guesses if the mod is audiovisual based on its contents
    ///
    /// If there's media (audio folders or any atlas), show the PAV choice...
    fn is_audio_visual_guessed(mod_: &Ruleset) -> bool {
        let folder_location = mod_.folder_location.as_ref()?;

        // Helper function to check if a subfolder exists and is not empty
        fn is_sub_folder_not_empty(mod_folder: &Path, name: &str) -> bool {
            let path = mod_folder.join(name);
            if !path.exists() || !path.is_dir() {
                return false;
            }

            match fs::read_dir(path) {
                Ok(entries) => entries.count() > 0,
                Err(_) => false,
            }
        }

        // Check for audio folders
        if is_sub_folder_not_empty(folder_location, "music") {
            return true;
        }
        if is_sub_folder_not_empty(folder_location, "sounds") {
            return true;
        }
        if is_sub_folder_not_empty(folder_location, "voices") {
            return true;
        }

        // Check for atlas files
        match fs::read_dir(folder_location) {
            Ok(entries) => entries.any(|entry| {
                if let Ok(entry) = entry {
                    entry.file_name().to_string_lossy().contains("atlas")
                } else {
                    false
                }
            }),
            Err(_) => false,
        }
    }

    /// Checks if the mod is an extension mod
    pub fn is_extension_mod(mod_: &Ruleset) -> bool {
        !mod_.mod_options.is_base_ruleset
            && !mod_.name.is_empty()
            && !mod_.mod_options.has_unique(UniqueType::ModIsAudioVisualOnly)
    }

    /// Checks if a mod name matches a filter
    pub fn mod_name_filter(mod_name: &str, filter: &str) -> bool {
        if mod_name == filter {
            return true;
        }

        if filter.len() < 3 || !filter.starts_with('*') || !filter.ends_with('*') {
            return false;
        }

        let partial_name = &filter[1..filter.len() - 1].to_lowercase();
        mod_name.to_lowercase().contains(partial_name)
    }

    /// Checks if a mod is incompatible with another mod
    fn is_incompatible_with(mod_: &Ruleset, other_mod: &Ruleset) -> bool {
        mod_.mod_options.get_matching_uniques(UniqueType::ModIncompatibleWith)
            .iter()
            .any(|unique| Self::mod_name_filter(&other_mod.name, &unique.params[0]))
    }

    /// Checks if two mods are incompatible with each other
    fn is_incompatible(mod_: &Ruleset, other_mod: &Ruleset) -> bool {
        Self::is_incompatible_with(mod_, other_mod) || Self::is_incompatible_with(other_mod, mod_)
    }

    /// Implement UniqueType::ModRequires and UniqueType::ModIncompatibleWith
    /// for selecting extension mods to show - after a base_ruleset was chosen.
    ///
    /// - Extension mod is incompatible with base_ruleset -> Nope
    /// - Extension mod has no ModRequires unique -> OK
    /// - For each ModRequires: Not (base_ruleset meets filter OR any other cached _extension_ mod meets filter) -> Nope
    /// - All ModRequires tested -> OK
    pub fn meets_base_requirements(mod_: &Ruleset, base_ruleset: &Ruleset) -> bool {
        if Self::is_incompatible(mod_, base_ruleset) {
            return false;
        }

        let all_other_extension_mod_names: Vec<String> = RulesetCache::values()
            .iter()
            .filter(|&it| it != mod_ && !it.mod_options.is_base_ruleset && !it.name.is_empty())
            .map(|it| it.name.clone())
            .collect();

        for unique in mod_.mod_options.get_matching_uniques(UniqueType::ModRequires) {
            let filter = &unique.params[0];
            if Self::mod_name_filter(&base_ruleset.name, filter) {
                continue;
            }
            if !all_other_extension_mod_names.iter().any(|name| Self::mod_name_filter(name, filter)) {
                return false;
            }
        }

        true
    }

    /// Implement UniqueType::ModRequires and UniqueType::ModIncompatibleWith
    /// for _enabling_ shown extension mods depending on other extension choices
    ///
    /// # Arguments
    ///
    /// * `selected_extension_mods` - all "active" mods for the compatibility tests - including the testee mod_ itself in this is allowed, it will be ignored. Will be iterated only once.
    ///
    /// - No need to test: Extension mod is incompatible with base_ruleset - we expect meets_base_requirements did exclude it from the UI entirely
    /// - Extension mod is incompatible with any _other_ **selected** extension mod -> Nope
    /// - Extension mod has no ModRequires unique -> OK
    /// - For each ModRequires: Not(base_ruleset meets filter OR any other **selected** extension mod meets filter) -> Nope
    /// - All ModRequires tested -> OK
    pub fn meets_all_requirements(mod_: &Ruleset, base_ruleset: &Ruleset, selected_extension_mods: &[&Ruleset]) -> bool {
        let other_selected_extension_mods: Vec<&Ruleset> = selected_extension_mods.iter()
            .filter(|&&it| it != mod_)
            .copied()
            .collect();

        if other_selected_extension_mods.iter().any(|&other_mod| Self::is_incompatible(mod_, other_mod)) {
            return false;
        }

        for unique in mod_.mod_options.get_matching_uniques(UniqueType::ModRequires) {
            let filter = &unique.params[0];
            if Self::mod_name_filter(&base_ruleset.name, filter) {
                continue;
            }
            if !other_selected_extension_mods.iter().any(|other_mod| Self::mod_name_filter(&other_mod.name, filter)) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ruleset::unique::Unique;

    #[test]
    fn test_mod_name_filter() {
        assert!(ModCompatibility::mod_name_filter("MyMod", "MyMod"));
        assert!(ModCompatibility::mod_name_filter("MyMod", "*Mod*"));
        assert!(ModCompatibility::mod_name_filter("MyMod", "*my*"));
        assert!(!ModCompatibility::mod_name_filter("MyMod", "OtherMod"));
        assert!(!ModCompatibility::mod_name_filter("MyMod", "*Other*"));
        assert!(!ModCompatibility::mod_name_filter("MyMod", "Mod")); // Too short
        assert!(!ModCompatibility::mod_name_filter("MyMod", "Mod*")); // Missing prefix
    }
}