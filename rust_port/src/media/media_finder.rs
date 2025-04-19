// Source: orig_src/core/src/com/unciv/logic/files/IMediaFinder.kt
// Ported to Rust

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::iter::FromIterator;
use std::env;

// TODO: Replace these imports with the actual modules once they are created
// For now, we'll use placeholder types
type UncivGame = ();
type UncivSound = String;
type BaseRuleset = ();
type Ruleset = ();
type RulesetCache = ();
type BaseUnit = ();

use crate::utils::file_chooser::FileHandle;

/// Encapsulate how media files are found and enumerated.
///
/// ## API requirements
/// - Map a name to a FileHandle
/// - Enumerate folders over all active mods
/// - Enumerate files over all active mods
/// - Allow fine-tuning where the list of mods comes from
/// - By default, "active mods" means selected in a running game, or selected as permanent audiovisual mod, and including builtin, in that order.
///
/// ## Instructions
/// - Inherit the trait or one of the specializations [Sounds], [Music], [Images], [Voices]
/// - Simply call [find_media], [list_media_folders] or [list_media_files]
/// - If you need a specialization but your host already has a superclass - use delegation:
///   `struct SoundPlayer : Popup, IMediaFinder = IMediaFinder::Sounds { ... }`
/// - Direct instantiation is fine too when the call is simple.
///
/// ## Usages
/// - OptionsPopup multiplayer notification sounds: [multiplayerTab][com.unciv.ui.popups.options.multiplayerTab]
/// - todo, prepared: [SoundPlayer.get_folders]
/// - todo, prepared: [MusicController.get_all_music_files]
/// - todo, prepared: Leader voices: [MusicController.play_voice]
/// - todo, prepared: ExtraImages: [ImageGetter.find_external_image], [FormattedLine.extra_image]
/// - todo: Particle effect definitions: Open PR
///
/// ## Caveats
/// - FileHandle.list() won't work for internal folders when running a desktop jar (unless the assets in extracted form are actually available)
/// - FileHandle.exists() - doc claims it won't work on Android for internal _folders_. Cannot repro on Android S, but respected here nonetheless.
/// - FileHandle.exists() - doc quote: "Note that this can be very slow for internal files on Android!" (meaning files).
///     I disagree - that's very true if there's an obb, so the zip directory has to be scanned.
///     But otherwise, asking the Android asset manager should not be too bad - better than file access since SAF anyway.
/// - FileHandle.is_directory - won't work for internal folders when running a desktop jar
///   Doc: "On Android, an Files.FileType.Internal handle to an empty directory will return false. On the desktop, an Files.FileType.Internal handle to a directory on the classpath will return false."
pub trait IMediaFinder: Send + Sync {
    /// Get the names of all Unciv sounds
    fn unciv_sound_names(&self) -> Vec<String> {
        vec![
            "notification1".to_string(),
            "notification2".to_string(),
            "coin".to_string(),
            "construction".to_string(),
            "paper".to_string(),
            "policy".to_string(),
            "setup".to_string(),
            "swap".to_string(),
            "whoosh".to_string(),
            "nuke".to_string(),
            "fire".to_string(),
            "slider".to_string(),
        ]
    }

    /// Set of supported extensions **including the leading dot**.
    /// - use `setOf("")` if the [find_media] API should not guess extensions but require the name parameter to only match full names.
    /// - supplying "" ***and*** a list of extensions will make [find_media] accept both an explicit full name and have it guess extensions.
    /// - Don't use emptyset() - that will cause [find_media] to always return `None`.
    fn supported_media_extensions(&self) -> HashSet<String>;

    /// Name of assets subfolder.
    /// - Will be interpreted as a direct child of a Mod folder or the Unciv internal assets folder.
    fn media_sub_folder_name(&self) -> String;

    /// Access the current Ruleset.
    /// - Defaults to [UncivGame.get_game_info_or_null]`()?.ruleset`.
    /// - Override to provide a direct source to enumerate game mods.
    /// @return `None` if no game is loaded - only permanent audiovisual mods and builtin are valid sources
    fn get_ruleset(&self) -> Option<Arc<Ruleset>> {
        // TODO: Implement this once UncivGame is properly defined
        None
    }

    /// Supply a list of possible file names for the builtin folder.
    /// - The children of builtin folders cannot be enumerated.
    /// - Thus [list_media_files] will throw unless this is overridden.
    fn get_internal_media_names(&self, folder: &FileHandle) -> Vec<String> {
        panic!("Using IMediaFinder.list_media_files from a jar requires overriding get_internal_media_names")
    }

    /// Find a specific asset by name.
    /// - Calls [list_media_folders] and looks in each candidate folder.
    /// - [supported_media_extensions] are the extensions searched.
    fn find_media(&self, name: &str) -> Option<FileHandle> {
        self.list_media_folders()
            .iter()
            .flat_map(|folder| {
                self.supported_media_extensions()
                    .iter()
                    .map(|ext| folder.child(format!("{}{}", name, ext)))
            })
            .find(|file| file.exists())
    }

    /// Enumerate all candidate media folders according to current Ruleset mod choice, Permanent audiovisual mods, and builtin sources.
    /// - Remember builtin are under Gdx.internal and will not support listing children on most build types.
    fn list_media_folders(&self) -> Vec<FileHandle> {
        let mut folders = self.list_mod_folders();

        // Add the internal folder
        let internal_folder = FileHandle::internal(&self.media_sub_folder_name());
        folders.push(internal_folder);

        // Filter to only include directories that exist
        folders.into_iter()
            .filter(|folder| self.directory_exists(folder))
            .collect()
    }

    /// Enumerate all media files.
    /// - Existence is ensured.
    fn list_media_files(&self) -> Vec<FileHandle> {
        self.list_media_folders()
            .iter()
            .flat_map(|folder| {
                if folder.is_internal() && Self::is_run_from_jar() {
                    self.get_internal_media_names(folder)
                        .iter()
                        .flat_map(|name| {
                            self.supported_media_extensions()
                                .iter()
                                .map(|ext| folder.child(format!("{}{}", name, ext)))
                        })
                        .filter(|file| file.exists())
                        .collect::<Vec<_>>()
                } else {
                    folder.list().unwrap_or_default()
                }
            })
            .collect()
    }

    /// Get the mod media folder for a given mod name
    fn get_mod_media_folder(&self, mod_name: &str) -> FileHandle {
        // TODO: Implement this once UncivGame is properly defined
        FileHandle::from_path(format!("mods/{}", mod_name))
    }

    /// Check if a file handle is a directory that exists
    fn directory_exists(&self, file: &FileHandle) -> bool {
        if !file.is_internal() {
            file.exists() && file.is_directory()
        } else if Self::is_android() {
            file.is_directory() // We accept that an empty folder is no folder in this case
        } else if Self::is_run_from_jar() {
            file.exists()
        } else {
            file.exists() && file.is_directory()
        }
    }

    /// List all mod folders
    fn list_mod_folders(&self) -> Vec<FileHandle> {
        let mut folders = Vec::new();

        // Order determines winner if several sources contain the same asset!
        // todo: Can there be a deterministic priority/ordering within game mods or visualMods?

        // Mods chosen in the running game go first
        // - including BaseRuleset (which can be Vanilla/G&K but those don't have folders under local/mods), which always is first in Ruleset.mods
        if let Some(ruleset) = self.get_ruleset() {
            // TODO: Implement this once Ruleset is properly defined
            // for mod_name in &ruleset.mods {
            //     folders.push(self.get_mod_media_folder(mod_name));
            // }
        }

        // Permanent audiovisual mods next
        // TODO: Implement this once UncivGame is properly defined
        // if UncivGame::is_current_initialized() {
        //     for mod_name in &UncivGame::current().settings.visual_mods {
        //         folders.push(self.get_mod_media_folder(mod_name));
        //     }
        // }

        // Our caller will append the one builtin folder candidate (not here, as it's internal instead of local)

        folders
    }

    /// Check if the application is running from a jar
    fn is_run_from_jar() -> bool {
        Self::is_desktop() && env::var("JAVA_SPECIFICATION_VERSION").is_ok()
    }

    /// Check if the application is running on Android
    fn is_android() -> bool {
        cfg!(target_os = "android")
    }

    /// Check if the application is running on desktop
    fn is_desktop() -> bool {
        cfg!(target_os = "windows") || cfg!(target_os = "linux") || cfg!(target_os = "macos")
    }

    /// Get unit attack sounds
    fn unit_attack_sounds(&self) -> Vec<(BaseUnit, String)> {
        Vec::new()
    }

    /// Get the supported audio extensions
    fn supported_audio_extensions() -> HashSet<String> {
        HashSet::from_iter(vec![
            ".mp3".to_string(),
            ".ogg".to_string(),
            ".wav".to_string(),
        ]) // Per Gdx docs, no aac/m4a
    }

    /// Get the supported image extensions
    fn supported_image_extensions() -> HashSet<String> {
        HashSet::from_iter(vec![
            ".png".to_string(),
            ".jpg".to_string(),
            ".jpeg".to_string(),
        ])
    }
}

/// Specialization for sounds
pub struct Sounds;

impl IMediaFinder for Sounds {
    fn supported_media_extensions(&self) -> HashSet<String> {
        IMediaFinder::supported_audio_extensions()
    }

    fn media_sub_folder_name(&self) -> String {
        "sounds".to_string()
    }

    fn unciv_sound_names(&self) -> Vec<String> {
        // In a real implementation, this would use reflection to get all UncivSound values
        // For now, we'll just return a hardcoded list
        vec![
            "notification1".to_string(),
            "notification2".to_string(),
            "coin".to_string(),
            "construction".to_string(),
            "paper".to_string(),
            "policy".to_string(),
            "setup".to_string(),
            "swap".to_string(),
            "whoosh".to_string(),
            "nuke".to_string(),
            "fire".to_string(),
            "slider".to_string(),
        ]
    }

    fn unit_attack_sounds(&self) -> Vec<(BaseUnit, String)> {
        // TODO: Implement this once RulesetCache and BaseUnit are properly defined
        Vec::new()
    }
}

/// Specialization for music
pub struct Music;

impl IMediaFinder for Music {
    fn supported_media_extensions(&self) -> HashSet<String> {
        IMediaFinder::supported_audio_extensions()
    }

    fn media_sub_folder_name(&self) -> String {
        "music".to_string()
    }

    fn get_internal_media_names(&self, _folder: &FileHandle) -> Vec<String> {
        vec!["Thatched Villagers - Ambient".to_string()]
    }
}

/// Specialization for voices
pub struct Voices;

impl IMediaFinder for Voices {
    fn supported_media_extensions(&self) -> HashSet<String> {
        IMediaFinder::supported_audio_extensions()
    }

    fn media_sub_folder_name(&self) -> String {
        "voices".to_string()
    }
}

/// Specialization for images
pub struct Images;

impl IMediaFinder for Images {
    fn supported_media_extensions(&self) -> HashSet<String> {
        IMediaFinder::supported_image_extensions()
    }

    fn media_sub_folder_name(&self) -> String {
        "ExtraImages".to_string()
    }
    // no get_internal_media_names - no list_media_files() for internal assets needed
}

/// Specialized subclass to provide all accessible sounds with a human-readable label.
/// - API: Use [get_labeled_sounds] only.
/// - Note: Redesign if UncivSound should ever be made into or use an Enum, to store the label there.
pub struct LabeledSounds {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl LabeledSounds {
    /// Create a new labeled sounds finder
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get all labeled sounds
    pub fn get_labeled_sounds(&self) -> Vec<(String, UncivSound)> {
        self.fill_cache();

        let cache = self.cache.lock().unwrap();
        cache.iter()
            .map(|(key, value)| (value.clone(), key.clone()))
            .collect()
    }

    /// Fill the cache with all available sounds
    fn fill_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        if !cache.is_empty() {
            return;
        }

        self.cache_builtins(&mut cache);
        self.cache_mods(&mut cache);

        // Remove excluded sounds
        for sound in &["nuke", "fire", "slider"] {
            cache.remove(*sound);
        }
    }

    /// Cache builtin sounds
    fn cache_builtins(&self, cache: &mut HashMap<String, String>) {
        // Also determines display order
        let prettify_unciv_sound_names = [
            ("", "None"),
            ("notification1", "Notification [1]"),
            ("notification2", "Notification [2]"),
            ("coin", "Buy"),
            ("construction", "Create"),
            ("paper", "Pick a tech"),
            ("policy", "Adopt policy"),
            ("setup", "Set up"),
            ("swap", "Swap units"),
        ];

        for (key, value) in prettify_unciv_sound_names.iter() {
            cache.insert(key.to_string(), value.to_string());
        }

        // Add other Unciv sounds
        let sounds = Sounds;
        for sound in sounds.unciv_sound_names() {
            if !cache.contains_key(&sound) {
                let mut chars: Vec<char> = sound.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap();
                }
                let title_case = chars.into_iter().collect::<String>();
                cache.insert(sound, title_case);
            }
        }

        // Add unit attack sounds
        for (unit, sound) in sounds.unit_attack_sounds() {
            if !cache.contains_key(&sound) {
                cache.insert(sound, format!("[{}] Attack Sound", unit.name));
            }
        }
    }

    /// Cache mod sounds
    fn cache_mods(&self, cache: &mut HashMap<String, String>) {
        // TODO: Implement this once UncivGame is properly defined
        // if !UncivGame::is_current_initialized() {
        //     return;
        // }

        let sounds = Sounds;
        for folder in sounds.list_media_folders() {
            if folder.is_internal() {
                continue;
            }

            let mod_name = folder.parent().map(|p| p.name()).unwrap_or_default();
            // TODO: Implement this once RulesetCache is properly defined
            // let ruleset = RulesetCache::get(&mod_name);

            // if let Some(ruleset) = ruleset {
            //     for unit in ruleset.units.values() {
            //         if let Some(sound) = &unit.attack_sound {
            //             if !cache.contains_key(sound) {
            //                 let mod_prefix = if mod_name.len() > 32 {
            //                     mod_name[..32].to_string()
            //                 } else {
            //                     mod_name
            //                 };
            //                 cache.insert(sound.clone(), format!("{}: [{}] Attack Sound", mod_prefix, unit.name));
            //             }
            //         }
            //     }
            // }

            if let Ok(files) = folder.list() {
                for file in files {
                    let sound = file.name_without_extension();
                    if !cache.contains_key(&sound) {
                        let mod_prefix = if mod_name.len() > 32 {
                            mod_name[..32].to_string()
                        } else {
                            mod_name
                        };
                        let mut chars: Vec<char> = sound.chars().collect();
                        if !chars.is_empty() {
                            chars[0] = chars[0].to_uppercase().next().unwrap();
                        }
                        let title_case = chars.into_iter().collect::<String>();
                        cache.insert(sound, format!("{}: {} {}", mod_prefix, title_case, "{}"));
                    }
                }
            }
        }
    }
}

// Extension methods for FileHandle
pub trait FileHandleExt {
    /// Check if this file handle is internal
    fn is_internal(&self) -> bool;

    /// Get the name without extension
    fn name_without_extension(&self) -> String;
}

impl FileHandleExt for FileHandle {
    fn is_internal(&self) -> bool {
        FileHandle::is_internal(self)
    }

    fn name_without_extension(&self) -> String {
        let name = self.name();
        if let Some(dot_pos) = name.rfind('.') {
            name[..dot_pos].to_string()
        } else {
            name
        }
    }
}