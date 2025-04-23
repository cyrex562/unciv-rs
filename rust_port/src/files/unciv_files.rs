// Source: orig_src/core/src/com/unciv/logic/files/UncivFiles.kt
// Ported to Rust

use std::path::{Path, PathBuf};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use serde::{Serialize, Deserialize};
use serde_json;

use crate::unciv_game::UncivGame;
use crate::utils::gzip::Gzip;
use crate::utils::file_chooser::FileHandle;
use crate::models::metadata::{GameSettings, ModUIData};
use crate::models::ruleset::RulesetCache;
use crate::logic::game_info::{GameInfo, GameInfoPreview, GameInfoSerializationVersion, HasGameInfoSerializationVersion};
use crate::logic::compatibility_version::CompatibilityVersion;
use crate::utils::concurrency::Concurrency;
use crate::utils::log::Log;
use crate::utils::debug;

/// Constants for file paths and names
const SAVE_FILES_FOLDER: &str = "SaveFiles";
const MULTIPLAYER_FILES_FOLDER: &str = "MultiplayerGames";
const AUTOSAVE_FILE_NAME: &str = "Autosave";
const SETTINGS_FILE_NAME: &str = "GameSettings.json";
const MOD_LIST_CACHE_FILE_NAME: &str = "ModListCache.json";

/// Main file handling struct for Unciv
pub struct UncivFiles {
    /// The filesystem interface
    files: Arc<dyn Files>,
    /// Custom data directory path, if any
    custom_data_directory: Option<String>,
    /// Autosaves manager
    pub autosaves: Autosaves,
}

/// Trait for filesystem operations
pub trait Files: Send + Sync {
    /// Get the local storage path
    fn local_storage_path(&self) -> &str;
    /// Get the external storage path
    fn external_storage_path(&self) -> &str;
    /// Check if external storage is available
    fn is_external_storage_available(&self) -> bool;
    /// Get a local file handle
    fn local(&self, path: &str) -> FileHandle;
    /// Get an external file handle
    fn external(&self, path: &str) -> FileHandle;
    /// Get an absolute file handle
    fn absolute(&self, path: &str) -> FileHandle;
}

impl UncivFiles {
    /// Create a new UncivFiles instance
    pub fn new(files: Arc<dyn Files>, custom_data_directory: Option<String>) -> Self {
        debug!("Creating UncivFiles, localStoragePath: {}, externalStoragePath: {}",
            files.local_storage_path(), files.external_storage_path());

        let unciv_files = UncivFiles {
            files,
            custom_data_directory,
            autosaves: Autosaves::new(Arc::new(UncivFiles::new(
                Arc::new(DefaultFiles::new()),
                None
            ))),
        };

        let autosaves = Autosaves::new(Arc::new(unciv_files.clone()));

        UncivFiles {
            files: unciv_files.files,
            custom_data_directory: unciv_files.custom_data_directory,
            autosaves,
        }
    }

    /// Get a local file handle
    pub fn get_local_file(&self, file_name: &str) -> FileHandle {
        if let Some(ref custom_dir) = self.custom_data_directory {
            let path = format!("{}{}{}", custom_dir, std::path::MAIN_SEPARATOR, file_name);
            self.files.absolute(&path)
        } else {
            self.files.local(file_name)
        }
    }

    /// Get the mods folder
    pub fn get_mods_folder(&self) -> FileHandle {
        self.get_local_file("mods")
    }

    /// Get a specific mod folder
    pub fn get_mod_folder(&self, mod_name: &str) -> FileHandle {
        self.get_mods_folder().child(mod_name)
    }

    /// Get the data folder
    pub fn get_data_folder(&self) -> FileHandle {
        self.get_local_file("")
    }

    /// Get a save file handle
    pub fn get_save(&self, game_name: &str) -> FileHandle {
        self.get_save_internal(SAVE_FILES_FOLDER, game_name)
    }

    /// Get a multiplayer save file handle
    pub fn get_multiplayer_save(&self, game_name: &str) -> FileHandle {
        self.get_save_internal(MULTIPLAYER_FILES_FOLDER, game_name)
    }

    /// Internal method to get a save file handle
    fn get_save_internal(&self, save_folder: &str, game_name: &str) -> FileHandle {
        debug!("Getting save {} from folder {}, preferExternal: {}, externalStoragePath: {}",
            game_name, save_folder, SAVE_ZIPPED.load(Ordering::Relaxed), self.files.external_storage_path());

        let location = format!("{}/{}", save_folder, game_name);
        let local_file = self.get_local_file(&location);
        let external_file = self.files.external(&location);

        let to_return = if self.files.is_external_storage_available() && (
            external_file.exists() && !local_file.exists() || // external file is only valid choice
            PREFER_EXTERNAL_STORAGE.load(Ordering::Relaxed) && (external_file.exists() || !local_file.exists()) // unless local file is only valid choice, choose external
        ) {
            external_file
        } else {
            local_file
        };

        debug!("Save found: {}", to_return.path());
        to_return
    }

    /// Get a file writer
    pub fn file_writer(&self, path: &str, append: bool) -> io::Result<File> {
        let file = self.path_to_file_handler(path);
        file.writer(append)
    }

    /// Convert a path to a file handle
    pub fn path_to_file_handler(&self, path: &str) -> FileHandle {
        if PREFER_EXTERNAL_STORAGE.load(Ordering::Relaxed) && self.files.is_external_storage_available() {
            self.files.external(path)
        } else {
            self.get_local_file(path)
        }
    }

    /// Get multiplayer saves
    pub fn get_multiplayer_saves(&self) -> Vec<FileHandle> {
        self.get_saves_internal(MULTIPLAYER_FILES_FOLDER)
    }

    /// Get all saves
    pub fn get_saves(&self, auto_saves: bool) -> Vec<FileHandle> {
        let saves = self.get_saves_internal(SAVE_FILES_FOLDER);

        if auto_saves {
            saves
        } else {
            saves.into_iter()
                .filter(|file| !file.name().starts_with(AUTOSAVE_FILE_NAME))
                .collect()
        }
    }

    /// Internal method to get saves from a folder
    fn get_saves_internal(&self, save_folder: &str) -> Vec<FileHandle> {
        debug!("Getting saves from folder {}, externalStoragePath: {}",
            save_folder, self.files.external_storage_path());

        let local_files = self.get_local_file(save_folder).list().unwrap_or_default();

        let external_files = if self.files.is_external_storage_available() &&
            self.get_data_folder().path() != self.files.external("").path() {
            self.files.external(save_folder).list().unwrap_or_default()
        } else {
            Vec::new()
        };

        debug!("Local files: {:?}, external files: {:?}",
            local_files.iter().map(|f| f.path()).collect::<Vec<_>>(),
            external_files.iter().map(|f| f.path()).collect::<Vec<_>>());

        let mut result = local_files;
        result.extend(external_files);
        result
    }

    /// Delete a save by name
    pub fn delete_save(&self, game_name: &str) -> io::Result<bool> {
        self.delete_save_file(self.get_save(game_name))
    }

    /// Delete a save file
    pub fn delete_save_file(&self, file: FileHandle) -> io::Result<bool> {
        debug!("Deleting save {}", file.path());
        file.delete()
    }

    /// Save a game by name
    pub fn save_game(&self, game: &GameInfo, game_name: &str,
                    save_completion_callback: impl FnOnce(Option<&dyn Error>) + Send + 'static) -> FileHandle {
        let file = self.get_save(game_name);
        self.save_game_to_file(game, &file, save_completion_callback);
        file
    }

    /// Save a game to a file
    pub fn save_game_to_file(&self, game: &GameInfo, file: &FileHandle,
                            save_completion_callback: impl FnOnce(Option<&dyn Error>) + Send + 'static) {
        match self.save_game_internal(game, file) {
            Ok(_) => save_completion_callback(None),
            Err(e) => save_completion_callback(Some(&e)),
        }
    }

    /// Save a game preview by name
    pub fn save_game_preview(&self, game: &GameInfoPreview, game_name: &str,
                           save_completion_callback: impl FnOnce(Option<&dyn Error>) + Send + 'static) -> FileHandle {
        let file = self.get_multiplayer_save(game_name);
        self.save_game_preview_to_file(game, &file, save_completion_callback);
        file
    }

    /// Save a game preview to a file
    pub fn save_game_preview_to_file(&self, game: &GameInfoPreview, file: &FileHandle,
                                   save_completion_callback: impl FnOnce(Option<&dyn Error>) + Send + 'static) {
        match self.save_game_preview_internal(game, file) {
            Ok(_) => save_completion_callback(None),
            Err(e) => save_completion_callback(Some(&e)),
        }
    }

    /// Save a game to a custom location
    pub fn save_game_to_custom_location(&self, game: &mut GameInfo, game_name: &str,
                                      on_saved: impl FnOnce() + Send + 'static,
                                      on_error: impl FnOnce(&dyn Error) + Send + 'static) {
        let save_location = game.custom_save_location.clone().unwrap_or_else(|| {
            UncivGame::current().files().get_local_file(game_name).path()
        });

        match Self::game_info_to_string(game, None, false) {
            Ok(data) => {
                debug!("Initiating UI to save GameInfo {} to custom location {}", game.game_id, save_location);

                SAVER_LOADER.load(Ordering::Relaxed).save_game(
                    &data,
                    &save_location,
                    move |location| {
                        game.custom_save_location = Some(location.to_string());
                        Concurrency::run_on_main_thread(on_saved);
                    },
                    move |error| {
                        Concurrency::run_on_main_thread(move || on_error(error));
                    }
                );
            },
            Err(e) => {
                Concurrency::run_on_main_thread(move || on_error(&e));
            }
        }
    }

    /// Load a game by name
    pub fn load_game_by_name(&self, game_name: &str) -> io::Result<GameInfo> {
        self.load_game_from_file(&self.get_save(game_name))
    }

    /// Load a game from a file
    pub fn load_game_from_file(&self, game_file: &FileHandle) -> io::Result<GameInfo> {
        let game_data = game_file.read_string()?;

        if game_data.trim().is_empty() {
            return Err(self.empty_file(game_file));
        }

        Self::game_info_from_string(&game_data)
    }

    /// Load a game preview from a file
    pub fn load_game_preview_from_file(&self, game_file: &FileHandle) -> io::Result<GameInfoPreview> {
        match serde_json::from_reader(game_file.reader()?) {
            Ok(preview) => Ok(preview),
            Err(_) => Err(self.empty_file(game_file)),
        }
    }

    /// Create an error for an empty file
    fn empty_file(&self, game_file: &FileHandle) -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("The file for the game {} is empty", game_file.name())
        )
    }

    /// Load a game from a custom location
    pub fn load_game_from_custom_location(&self,
                                        on_loaded: impl FnOnce(&GameInfo) + Send + 'static,
                                        on_error: impl FnOnce(&dyn Error) + Send + 'static) {
        SAVER_LOADER.load(Ordering::Relaxed).load_game(
            move |data, location| {
                match Self::game_info_from_string(data) {
                    Ok(mut game) => {
                        game.custom_save_location = Some(location.to_string());
                        Concurrency::run_on_main_thread(move || on_loaded(&game));
                    },
                    Err(e) => {
                        Concurrency::run_on_main_thread(move || on_error(&e));
                    }
                }
            },
            move |error| {
                Concurrency::run_on_main_thread(move || on_error(error));
            }
        );
    }

    /// Get the general settings file
    fn get_general_settings_file(&self) -> FileHandle {
        if UncivGame::current().is_console_mode() {
            FileHandle::new(SETTINGS_FILE_NAME)
        } else {
            self.get_local_file(SETTINGS_FILE_NAME)
        }
    }

    /// Get the general settings
    pub fn get_general_settings(&self) -> GameSettings {
        let settings_file = self.get_general_settings_file();

        if settings_file.exists() {
            match serde_json::from_reader(settings_file.reader().unwrap()) {
                Ok(mut settings) => {
                    if settings.is_migration_necessary() {
                        // TODO: Implement migrations
                        // settings.do_migrations(JsonReader().parse(settings_file))
                    }
                    settings
                },
                Err(e) => {
                    Log::error("Error reading settings file", &e);
                    GameSettings::default()
                }
            }
        } else {
            GameSettings::default()
        }
    }

    /// Set the general settings
    pub fn set_general_settings(&self, game_settings: &GameSettings) -> io::Result<()> {
        let json = serde_json::to_string(game_settings)?;
        self.get_general_settings_file().write_string(&json, false)
    }

    /// Get scenario files
    pub fn get_scenario_files(&self) -> Vec<(FileHandle, String)> {
        let mut result = Vec::new();
        let scenario_folder = "scenarios";

        for (mod_name, mod_data) in RulesetCache::values() {
            if let Some(folder_location) = mod_data.folder_location() {
                let mod_folder = FileHandle::new(&folder_location);
                let scenario_folder_handle = mod_folder.child(scenario_folder);

                if scenario_folder_handle.exists() {
                    if let Ok(files) = scenario_folder_handle.list() {
                        for file in files {
                            result.push((file, mod_name.to_string()));
                        }
                    }
                }
            }
        }

        result
    }

    /// Save mod cache
    pub fn save_mod_cache(&self, mod_data_list: &[ModUIData]) -> io::Result<()> {
        let file = self.get_local_file(MOD_LIST_CACHE_FILE_NAME);
        match serde_json::to_writer(file.writer(false)?, mod_data_list) {
            Ok(_) => Ok(()),
            Err(e) => {
                Log::error("Error saving mod cache", &e);
                Err(io::Error::new(io::ErrorKind::Other, e))
            }
        }
    }

    /// Load mod cache
    pub fn load_mod_cache(&self) -> Vec<ModUIData> {
        let file = self.get_local_file(MOD_LIST_CACHE_FILE_NAME);

        if !file.exists() {
            return Vec::new();
        }

        match serde_json::from_reader(file.reader().unwrap()) {
            Ok(data) => data,
            Err(e) => {
                Log::error("Error loading mod cache", &e);
                Vec::new()
            }
        }
    }

    /// Convert a game info to a string
    pub fn game_info_to_string(game: &GameInfo, force_zip: Option<bool>, update_checksum: bool) -> io::Result<String> {
        let mut game_clone = game.clone();
        game_clone.version = GameInfo::CURRENT_COMPATIBILITY_VERSION;

        if update_checksum {
            game_clone.checksum = game_clone.calculate_checksum();
        }

        let plain_json = serde_json::to_string(&game_clone)?;

        if force_zip.unwrap_or(SAVE_ZIPPED.load(Ordering::Relaxed)) {
            Ok(Gzip::zip(&plain_json)?)
        } else {
            Ok(plain_json)
        }
    }

    /// Convert a game info preview to a string
    pub fn game_info_preview_to_string(game: &GameInfoPreview) -> io::Result<String> {
        let json = serde_json::to_string(game)?;
        Gzip::zip(&json)
    }

    /// Convert a string to a game info
    pub fn game_info_from_string(game_data: &str) -> io::Result<GameInfo> {
        let fixed_data = game_data.trim().replace("\r", "").replace("\n", "");

        let unzipped_json = match Gzip::unzip(&fixed_data) {
            Ok(json) => json,
            Err(_) => fixed_data,
        };

        match serde_json::from_str::<GameInfo>(&unzipped_json) {
            Ok(mut game_info) => {
                if game_info.version > GameInfo::CURRENT_COMPATIBILITY_VERSION {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        IncompatibleGameInfoVersionException::new(game_info.version, None)
                    ));
                }

                game_info.set_transients();
                Ok(game_info)
            },
            Err(e) => {
                Log::error("Exception while deserializing GameInfo JSON", &e);

                match serde_json::from_str::<GameInfoSerializationVersion>(&unzipped_json) {
                    Ok(version) => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        IncompatibleGameInfoVersionException::new(version.version, Some(Box::new(e)))
                    )),
                    Err(_) => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "The file data seems to be corrupted."
                    )),
                }
            }
        }
    }

    /// Convert a string to a game info preview
    pub fn game_info_preview_from_string(game_data: &str) -> io::Result<GameInfoPreview> {
        let unzipped = Gzip::unzip(game_data)?;
        serde_json::from_str(&unzipped).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Get settings for platform launchers
    pub fn get_settings_for_platform_launchers(base_directory: &str) -> GameSettings {
        let file = FileHandle::new(&format!("{}{}{}", base_directory, std::path::MAIN_SEPARATOR, SETTINGS_FILE_NAME));

        if file.exists() {
            match serde_json::from_reader(file.reader().unwrap()) {
                Ok(settings) => settings,
                Err(e) => {
                    Log::error("Exception while deserializing GameSettings JSON", &e);
                    GameSettings::default()
                }
            }
        } else {
            GameSettings::default()
        }
    }
}

/// Default implementation of the Files trait
pub struct DefaultFiles {
    local_storage_path: String,
    external_storage_path: String,
    external_storage_available: bool,
}

impl DefaultFiles {
    pub fn new() -> Self {
        DefaultFiles {
            local_storage_path: ".".to_string(),
            external_storage_path: ".".to_string(),
            external_storage_available: false,
        }
    }
}

impl Files for DefaultFiles {
    fn local_storage_path(&self) -> &str {
        &self.local_storage_path
    }

    fn external_storage_path(&self) -> &str {
        &self.external_storage_path
    }

    fn is_external_storage_available(&self) -> bool {
        self.external_storage_available
    }

    fn local(&self, path: &str) -> FileHandle {
        FileHandle::new(path)
    }

    fn external(&self, path: &str) -> FileHandle {
        FileHandle::new(path)
    }

    fn absolute(&self, path: &str) -> FileHandle {
        FileHandle::new(path)
    }
}

/// Autosaves manager
pub struct Autosaves {
    files: Arc<UncivFiles>,
    auto_save_job: Option<Arc<Concurrency::Job>>,
}

impl Autosaves {
    /// Create a new Autosaves instance
    pub fn new(files: Arc<UncivFiles>) -> Self {
        Autosaves {
            files,
            auto_save_job: None,
        }
    }

    /// Request an autosave
    pub fn request_auto_save(&mut self, game_info: &GameInfo, next_turn: bool) -> Arc<Concurrency::Job> {
        // Clone the game info to avoid concurrent modification issues
        let game_clone = game_info.clone();
        self.request_auto_save_uncloned(&game_clone, next_turn)
    }

    /// Request an autosave without cloning
    pub fn request_auto_save_uncloned(&mut self, game_info: &GameInfo, next_turn: bool) -> Arc<Concurrency::Job> {
        let files = Arc::clone(&self.files);
        let game_clone = game_info.clone();

        let job = Concurrency::run("autoSaveUnCloned", move || {
            files.autosaves.auto_save(&game_clone, next_turn);
        });

        self.auto_save_job = Some(Arc::clone(&job));
        job
    }

    /// Perform an autosave
    pub fn auto_save(&self, game_info: &GameInfo, next_turn: bool) {
        // Get GameSettings to check the maxAutosavesStored
        let settings = self.files.get_general_settings();

        match self.files.save_game(game_info, AUTOSAVE_FILE_NAME, |_| {}) {
            Ok(_) => {},
            Err(e) => {
                Log::error("Ran out of memory during autosave", &e);
                return; // not much we can do here
            }
        }

        // Keep auto-saves for the last N turns for debugging purposes
        if next_turn {
            let new_autosave_filename = format!("{}{}{}-{}-{}",
                SAVE_FILES_FOLDER, std::path::MAIN_SEPARATOR,
                AUTOSAVE_FILE_NAME, game_info.current_player, game_info.turns);

            let file = self.files.path_to_file_handler(&new_autosave_filename);

            if let Ok(autosave) = self.files.get_save(AUTOSAVE_FILE_NAME) {
                if let Err(e) = autosave.copy_to(&file) {
                    Log::error("Failed to copy autosave", &e);
                }
            }

            // Clean up old autosaves
            let autosaves = self.get_autosaves();

            // Add 1 to avoid player choosing 6,11,21,51,101, etc.. in options
            while autosaves.len() > settings.max_autosaves_stored + 1 {
                if let Some(save_to_delete) = autosaves.iter()
                    .min_by_key(|f| f.last_modified()) {
                    if let Err(e) = self.files.delete_save(save_to_delete.name()) {
                        Log::error("Failed to delete old autosave", &e);
                    }
                }
            }
        }
    }

    /// Get all autosaves
    fn get_autosaves(&self) -> Vec<FileHandle> {
        self.files.get_saves(true)
            .into_iter()
            .filter(|f| f.name().starts_with(AUTOSAVE_FILE_NAME))
            .collect()
    }

    /// Load the latest autosave
    pub fn load_latest_autosave(&self) -> io::Result<GameInfo> {
        match self.files.load_game_by_name(AUTOSAVE_FILE_NAME) {
            Ok(game) => Ok(game),
            Err(_) => {
                // Silent fail if we can't read the autosave for any reason
                // Try to load the last autosave by turn number
                let autosaves = self.files.get_saves(true)
                    .into_iter()
                    .filter(|f| f.name() != AUTOSAVE_FILE_NAME && f.name().starts_with(AUTOSAVE_FILE_NAME))
                    .collect::<Vec<_>>();

                if let Some(latest) = autosaves.iter()
                    .max_by_key(|f| f.last_modified()) {
                    self.files.load_game_from_file(latest)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "No autosaves found"))
                }
            }
        }
    }

    /// Check if an autosave exists
    pub fn autosave_exists(&self) -> bool {
        self.files.get_save(AUTOSAVE_FILE_NAME).exists()
    }
}

/// Exception for incompatible game info versions
#[derive(Debug)]
pub struct IncompatibleGameInfoVersionException {
    pub version: CompatibilityVersion,
    pub cause: Option<Box<dyn Error + Send + Sync>>,
}

impl IncompatibleGameInfoVersionException {
    /// Create a new IncompatibleGameInfoVersionException
    pub fn new(version: CompatibilityVersion, cause: Option<Box<dyn Error + Send + Sync>>) -> Self {
        IncompatibleGameInfoVersionException { version, cause }
    }
}

impl fmt::Display for IncompatibleGameInfoVersionException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The save was created with an incompatible version of Unciv: [{}]. Please update Unciv to this version or later and try again.",
            self.version.created_with.to_string())
    }
}

impl Error for IncompatibleGameInfoVersionException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &dyn Error)
    }
}

impl HasGameInfoSerializationVersion for IncompatibleGameInfoVersionException {
    fn version(&self) -> CompatibilityVersion {
        self.version
    }
}

// Static variables
lazy_static::lazy_static! {
    /// Whether to save games in zipped format
    pub static ref SAVE_ZIPPED: AtomicBool = AtomicBool::new(false);

    /// Whether to prefer external storage
    pub static ref PREFER_EXTERNAL_STORAGE: AtomicBool = AtomicBool::new(false);

    /// Platform dependent saver-loader to custom system locations
    pub static ref SAVER_LOADER: Arc<Mutex<Box<dyn PlatformSaverLoader>>> =
        Arc::new(Mutex::new(Box::new(PlatformSaverLoaderNone::new())));
}

// Implement Clone for UncivFiles
impl Clone for UncivFiles {
    fn clone(&self) -> Self {
        UncivFiles {
            files: Arc::clone(&self.files),
            custom_data_directory: self.custom_data_directory.clone(),
            autosaves: Autosaves::new(Arc::new(self.clone())),
        }
    }
}