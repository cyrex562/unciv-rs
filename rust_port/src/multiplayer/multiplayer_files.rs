use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use log::debug;

use crate::game::unciv_game::UncivGame;
use crate::game::game_info::{GameInfo, GameInfoPreview};
use crate::game::event_bus::EventBus;
use crate::game::files::FileHandle;

/// Files that are stored locally
pub struct MultiplayerFiles {
    /// The files manager
    pub(crate) files: Arc<UncivGame>,
    /// The saved games, keyed by file handle
    pub(crate) saved_games: Arc<Mutex<HashMap<FileHandle, MultiplayerGame>>>,
}

impl MultiplayerFiles {
    /// Create a new MultiplayerFiles instance
    pub fn new() -> Self {
        Self {
            files: UncivGame::current(),
            saved_games: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Update the saved games from the files
    pub(crate) fn update_saves_from_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        let saves = self.files.get_multiplayer_saves()?;
        let mut saved_games = self.saved_games.lock().unwrap();

        // Remove games that no longer exist
        let removed_saves: Vec<_> = saved_games
            .keys()
            .filter(|save_file| !saves.contains(save_file))
            .cloned()
            .collect();

        for save_file in removed_saves {
            self.delete_game_internal(&save_file)?;
        }

        // Add new games
        let new_saves: Vec<_> = saves
            .iter()
            .filter(|save_file| !saved_games.contains_key(save_file))
            .cloned()
            .collect();

        for save_file in new_saves {
            self.add_game_internal(&save_file)?;
        }

        Ok(())
    }

    /// Deletes the game from disk, does not delete it remotely.
    ///
    /// # Parameters
    ///
    /// * `multiplayer_game` - The game to delete
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the file cannot be deleted
    pub fn delete_game(&self, multiplayer_game: &MultiplayerGame) -> Result<(), Box<dyn std::error::Error>> {
        self.delete_game_internal(&multiplayer_game.file_handle)
    }

    /// Delete a game by its file handle
    ///
    /// # Parameters
    ///
    /// * `file_handle` - The file handle of the game to delete
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the file cannot be deleted
    fn delete_game_internal(&self, file_handle: &FileHandle) -> Result<(), Box<dyn std::error::Error>> {
        self.files.delete_save(file_handle)?;

        let mut saved_games = self.saved_games.lock().unwrap();
        if let Some(game) = saved_games.get(file_handle) {
            debug!("Deleting game {} with id {:?}", file_handle.name(), game.preview.as_ref().map(|p| &p.game_id));
            saved_games.remove(&game.file_handle);
        }

        Ok(())
    }

    /// Add a new game
    ///
    /// # Parameters
    ///
    /// * `new_game` - The game to add
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the game cannot be saved
    pub(crate) fn add_game(&self, new_game: &GameInfo) -> Result<(), Box<dyn std::error::Error>> {
        let new_game_preview = new_game.as_preview();
        self.add_game_with_preview(&new_game_preview, &new_game_preview.game_id)
    }

    /// Add a game with a preview
    ///
    /// # Parameters
    ///
    /// * `preview` - The game preview
    /// * `save_file_name` - The name to save the game as
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the game cannot be saved
    pub(crate) fn add_game_with_preview(&self, preview: &GameInfoPreview, save_file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_handle = self.files.save_game(preview, save_file_name)?;
        self.add_game_internal(&file_handle)
    }

    /// Add a game by its file handle
    ///
    /// # Parameters
    ///
    /// * `file_handle` - The file handle of the game to add
    /// * `preview` - The game preview, if available
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the game cannot be added
    fn add_game_internal(&self, file_handle: &FileHandle, preview: Option<&GameInfoPreview> = None) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Adding game {}", file_handle.name());
        let game = MultiplayerGame::new(
            file_handle.clone(),
            preview.cloned(),
            if preview.is_some() { Some(Instant::now()) } else { None },
        );
        let mut saved_games = self.saved_games.lock().unwrap();
        saved_games.insert(file_handle.clone(), game);
        Ok(())
    }

    /// Get a game by its name
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game to get
    ///
    /// # Returns
    ///
    /// The game, if found
    pub fn get_game_by_name(&self, name: &str) -> Option<MultiplayerGame> {
        let saved_games = self.saved_games.lock().unwrap();
        saved_games.values().find(|game| game.name == name).cloned()
    }

    /// Get a game by its ID
    ///
    /// # Parameters
    ///
    /// * `game_id` - The ID of the game to get
    ///
    /// # Returns
    ///
    /// The game, if found
    pub fn get_game_by_game_id(&self, game_id: &str) -> Option<MultiplayerGame> {
        let saved_games = self.saved_games.lock().unwrap();
        saved_games
            .values()
            .find(|game| game.preview.as_ref().map_or(false, |p| p.game_id == game_id))
            .cloned()
    }

    /// Change the name of a game
    ///
    /// Fires `MultiplayerGameNameChanged`
    ///
    /// # Parameters
    ///
    /// * `game` - The game to rename
    /// * `new_name` - The new name for the game
    /// * `on_exception` - Callback for handling exceptions
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the game cannot be renamed
    pub fn change_game_name(&self, game: &MultiplayerGame, new_name: &str, on_exception: impl FnOnce(Option<Box<dyn std::error::Error>>)) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Changing name of game {} to {}", game.name, new_name);
        let old_preview = game.preview.as_ref().ok_or_else(|| game.error.clone().unwrap_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "No preview available"))))?;
        let old_last_update = game.get_last_update();
        let old_name = game.name.clone();

        let new_file_handle = self.files.save_game(old_preview, new_name)?;
        let new_game = MultiplayerGame::new(&new_file_handle, Some(old_preview), old_last_update);

        let mut saved_games = self.saved_games.lock().unwrap();
        saved_games.insert(new_file_handle.clone(), new_game);
        saved_games.remove(&game.file_handle);

        self.files.delete_save(&game.file_handle)?;
        EventBus::send(MultiplayerGameNameChanged::new(old_name, new_name.to_string()));

        Ok(())
    }
}

/// Event fired when a multiplayer game's name is changed
pub struct MultiplayerGameNameChanged {
    /// The old name of the game
    pub old_name: String,
    /// The new name of the game
    pub new_name: String,
}

impl MultiplayerGameNameChanged {
    /// Create a new MultiplayerGameNameChanged event
    ///
    /// # Parameters
    ///
    /// * `old_name` - The old name of the game
    /// * `new_name` - The new name of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameNameChanged event
    pub fn new(old_name: String, new_name: String) -> Self {
        Self { old_name, new_name }
    }
}

/// A multiplayer game
pub struct MultiplayerGame {
    /// The file handle for the game
    pub file_handle: FileHandle,
    /// The game preview, if available
    pub preview: Option<GameInfoPreview>,
    /// The last time the game was updated
    pub last_update: Option<Instant>,
    /// The name of the game
    pub name: String,
    /// Any error that occurred while loading the game
    pub error: Option<Box<dyn std::error::Error>>,
}

impl MultiplayerGame {
    /// Create a new MultiplayerGame
    ///
    /// # Parameters
    ///
    /// * `file_handle` - The file handle for the game
    /// * `preview` - The game preview, if available
    /// * `last_update` - The last time the game was updated
    ///
    /// # Returns
    ///
    /// A new MultiplayerGame
    pub fn new(file_handle: &FileHandle, preview: Option<&GameInfoPreview>, last_update: Option<Instant>) -> Self {
        Self {
            file_handle: file_handle.clone(),
            preview: preview.cloned(),
            last_update,
            name: file_handle.name().to_string(),
            error: None,
        }
    }

    /// Get the last time the game was updated
    ///
    /// # Returns
    ///
    /// The last time the game was updated
    pub fn get_last_update(&self) -> Option<Instant> {
        self.last_update
    }

    /// Request an update for the game
    ///
    /// # Parameters
    ///
    /// * `force_update` - Whether to force the update
    ///
    /// # Returns
    ///
    /// The result of the update
    pub async fn request_update(&self, force_update: bool) -> Result<(), Box<dyn std::error::Error>> {
        // This is a placeholder for the actual implementation
        // The actual implementation would be in the MultiplayerGame struct
        Ok(())
    }

    /// Do a manual update for the game
    ///
    /// # Parameters
    ///
    /// * `preview` - The new preview for the game
    pub fn do_manual_update(&mut self, preview: GameInfoPreview) {
        self.preview = Some(preview);
        self.last_update = Some(Instant::now());
        self.error = None;
    }
}

impl Clone for MultiplayerGame {
    fn clone(&self) -> Self {
        Self {
            file_handle: self.file_handle.clone(),
            preview: self.preview.clone(),
            last_update: self.last_update,
            name: self.name.clone(),
            error: self.error.as_ref().map(|e| Box::new(e.to_string()) as Box<dyn std::error::Error>),
        }
    }
}