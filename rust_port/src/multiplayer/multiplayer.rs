use std::sync::atomic::{AtomicReference, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use log::debug;

use crate::constants::Constants;
use crate::game::unciv_game::UncivGame;
use crate::game::game_info::{GameInfo, GameInfoPreview};
use crate::game::automation::civilization::NextTurnAutomation;
use crate::game::civilization::{NotificationCategory, PlayerType};
use crate::game::event_bus::EventBus;
use crate::multiplayer::storage::{
    FileStorageRateLimitReached,
    MultiplayerAuthException,
    MultiplayerFileNotFoundException,
    MultiplayerServer,
};
use crate::models::metadata::GameSettings;
use crate::utils::dispatcher::Dispatcher;

/// How often files can be checked for new multiplayer games (could be that the user modified their file system directly).
/// More checks within this time period will do nothing.
const FILE_UPDATE_THROTTLE_PERIOD: Duration = Duration::from_secs(60);

/// Provides *online* multiplayer functionality to the rest of the game.
/// Multiplayer data is a mix of local files (`multiplayer_files`) and server data (`multiplayer_server`).
/// This class handles functions that require a mix of both.
///
/// See the file of `HasMultiplayerGameName` for all available `EventBus` events.
pub struct Multiplayer {
    /// Handles SERVER DATA only
    pub multiplayer_server: MultiplayerServer,
    /// Handles LOCAL FILES only
    pub multiplayer_files: MultiplayerFiles,
    /// Last time files were updated
    last_file_update: AtomicReference<Option<Instant>>,
    /// Last time all games were refreshed
    last_all_games_refresh: AtomicReference<Option<Instant>>,
    /// Last time current game was refreshed
    last_cur_game_refresh: AtomicReference<Option<Instant>>,
}

impl Multiplayer {
    /// Create a new Multiplayer instance
    pub fn new() -> Self {
        let multiplayer_server = MultiplayerServer::new();
        let multiplayer_files = MultiplayerFiles::new();

        let last_file_update = AtomicReference::new(None);
        let last_all_games_refresh = AtomicReference::new(None);
        let last_cur_game_refresh = AtomicReference::new(None);

        let multiplayer = Self {
            multiplayer_server,
            multiplayer_files,
            last_file_update,
            last_all_games_refresh,
            last_cur_game_refresh,
        };

        // Start the multiplayer game updater
        multiplayer.start_multiplayer_game_updater();

        multiplayer
    }

    /// Get the current game
    fn get_current_game(&self) -> Option<&MultiplayerGame> {
        let game_info = UncivGame::current().game_info.as_ref()?;
        if game_info.game_parameters.is_online_multiplayer {
            self.multiplayer_files.get_game_by_game_id(&game_info.game_id)
        } else {
            None
        }
    }

    /// Start the multiplayer game updater
    fn start_multiplayer_game_updater(&self) {
        let multiplayer_server = self.multiplayer_server.clone();
        let multiplayer_files = self.multiplayer_files.clone();
        let last_cur_game_refresh = self.last_cur_game_refresh.clone();
        let last_all_games_refresh = self.last_all_games_refresh.clone();

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(500)).await;

                // Get multiplayer settings
                let multiplayer_settings = match UncivGame::current().settings.multiplayer.clone() {
                    Some(settings) => settings,
                    None => continue, // Skip if settings can't be accessed
                };

                // Get current game
                let current_game = multiplayer_files.get_current_game();
                let preview = current_game.as_ref().and_then(|game| game.preview.as_ref());

                // Update current game if needed
                if let Some(game) = current_game {
                    if Self::uses_custom_server() || preview.is_none() || !preview.unwrap().is_users_turn() {
                        Self::throttle(
                            &last_cur_game_refresh,
                            multiplayer_settings.current_game_refresh_delay,
                            || {},
                            || game.request_update(),
                        ).await;
                    }
                }

                // Update all games except current game
                let do_not_update = if current_game.is_some() {
                    vec![current_game.unwrap().clone()]
                } else {
                    vec![]
                };

                Self::throttle(
                    &last_all_games_refresh,
                    multiplayer_settings.all_game_refresh_delay,
                    || {},
                    || multiplayer_files.request_update(false, do_not_update),
                ).await;
            }
        });
    }

    /// Requests an update of all multiplayer game state. Does automatic throttling to try to prevent hitting rate limits.
    ///
    /// Use `force_update` = true to circumvent this throttling.
    ///
    /// Fires: `MultiplayerGameUpdateStarted`, `MultiplayerGameUpdated`, `MultiplayerGameUpdateUnchanged`, `MultiplayerGameUpdateFailed`
    pub async fn request_update(&self, force_update: bool, do_not_update: Vec<MultiplayerGame>) -> Result<(), Box<dyn std::error::Error>> {
        let file_throttle_interval = if force_update { Duration::ZERO } else { FILE_UPDATE_THROTTLE_PERIOD };

        // Update saves from files
        Self::throttle(
            &self.last_file_update,
            file_throttle_interval,
            || {},
            || self.multiplayer_files.update_saves_from_files(),
        ).await?;

        // Update each game
        for game in self.multiplayer_files.saved_games.values().cloned().collect::<Vec<_>>() {
            if do_not_update.contains(&game) {
                continue;
            }

            // Skip inactive games (not updated in 2 weeks)
            let last_modified = game.file_handle.last_modified();
            let now = Instant::now();
            if now.duration_since(last_modified) > Duration::from_days(14) {
                continue;
            }

            // Request update
            game.request_update(force_update).await?;
        }

        Ok(())
    }

    /// Create a new multiplayer game
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    pub async fn create_game(&self, new_game: GameInfo) -> Result<(), Box<dyn std::error::Error>> {
        self.multiplayer_server.try_upload_game(&new_game, true).await?;
        self.multiplayer_files.add_game(&new_game).await?;
        Ok(())
    }

    /// Add a game to the multiplayer list
    ///
    /// # Parameters
    ///
    /// * `game_id` - The ID of the game to add
    /// * `game_name` - The name of the game (if None or empty, will use the gameId as the game name)
    ///
    /// # Returns
    ///
    /// The final name the game was added under
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    pub async fn add_game(&self, game_id: &str, game_name: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        let save_file_name = game_name.unwrap_or("").trim();
        let save_file_name = if save_file_name.is_empty() { game_id } else { save_file_name };

        // Try to download game preview
        let game_preview = match self.multiplayer_server.try_download_game_preview(game_id).await {
            Ok(preview) => preview,
            Err(e) => {
                if e.is::<MultiplayerFileNotFoundException>() {
                    // Game is so old that a preview could not be found, try the real gameInfo instead
                    let game_info = self.multiplayer_server.try_download_game(game_id).await?;
                    game_info.as_preview()
                } else {
                    return Err(e.into());
                }
            }
        };

        self.multiplayer_files.add_game_with_preview(&game_preview, save_file_name).await?;
        Ok(save_file_name.to_string())
    }

    /// Resigns from the given multiplayer game. Can only resign if it's currently the user's turn,
    /// to ensure that no one else can upload the game in the meantime.
    ///
    /// Fires `MultiplayerGameUpdated`
    ///
    /// # Parameters
    ///
    /// * `game` - The game to resign from
    ///
    /// # Returns
    ///
    /// `false` if it's not the user's turn and thus resigning did not happen
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    /// * `MultiplayerAuthException` - If the authentication failed
    pub async fn resign_current_player(&self, game: &mut MultiplayerGame) -> Result<bool, Box<dyn std::error::Error>> {
        let preview = game.preview.as_ref().ok_or_else(|| game.error.clone().unwrap_or_else(|| Box::new(MultiplayerFileNotFoundException::new("No preview available"))))?;

        // Download to work with the latest game state
        let mut game_info = self.multiplayer_server.try_download_game(&preview.game_id).await?;

        if game_info.current_player != preview.current_player {
            game.do_manual_update(game_info.as_preview());
            return Ok(false);
        }

        let player_civ = game_info.get_current_player_civilization();

        // Set civ info to AI
        player_civ.player_type = PlayerType::AI;
        player_civ.player_id = String::new();

        // Call next turn so turn gets simulated by AI
        game_info.next_turn();

        // Add notification so everyone knows what happened
        // Call for every civ cause AI players are skipped anyway
        for civ in &mut game_info.civilizations {
            civ.add_notification(
                &format!("[{}] resigned and is now controlled by AI", player_civ.civ_name),
                NotificationCategory::General,
                &player_civ.civ_name,
            );
        }

        let new_preview = game_info.as_preview();
        self.multiplayer_files.files.save_game(&new_preview, &game.file_handle).await?;
        self.multiplayer_server.try_upload_game(&game_info, true).await?;
        game.do_manual_update(new_preview);

        Ok(true)
    }

    /// Skip the current player's turn
    ///
    /// # Parameters
    ///
    /// * `game` - The game to skip the turn in
    ///
    /// # Returns
    ///
    /// `None` if successful, or an error message if not
    pub async fn skip_current_player_turn(&self, game: &mut MultiplayerGame) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let preview = game.preview.as_ref().ok_or_else(|| game.error.clone().unwrap_or_else(|| Box::new(MultiplayerFileNotFoundException::new("No preview available"))))?;

        // Download to work with the latest game state
        let mut game_info = match self.multiplayer_server.try_download_game(&preview.game_id).await {
            Ok(info) => info,
            Err(e) => return Ok(Some(e.to_string())),
        };

        if game_info.current_player != preview.current_player {
            game.do_manual_update(game_info.as_preview());
            return Ok(Some("Could not pass turn - current player has been updated!".to_string()));
        }

        let player_civ = game_info.get_current_player_civilization();
        NextTurnAutomation::automate_civ_moves(player_civ, false);
        game_info.next_turn();

        let new_preview = game_info.as_preview();
        self.multiplayer_files.files.save_game(&new_preview, &game.file_handle).await?;
        self.multiplayer_server.try_upload_game(&game_info, true).await?;
        game.do_manual_update(new_preview);

        Ok(None)
    }

    /// Load a multiplayer game
    ///
    /// # Parameters
    ///
    /// * `game` - The game to load
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    pub async fn load_game_from_multiplayer_game(&self, game: &MultiplayerGame) -> Result<(), Box<dyn std::error::Error>> {
        let preview = game.preview.as_ref().ok_or_else(|| game.error.clone().unwrap_or_else(|| Box::new(MultiplayerFileNotFoundException::new("No preview available"))))?;
        self.load_game(&preview.game_id).await
    }

    /// Download game, and update it locally
    ///
    /// # Parameters
    ///
    /// * `game_id` - The ID of the game to load
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    pub async fn load_game(&self, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let game_info = self.multiplayer_server.download_game(game_id).await?;
        let preview = game_info.as_preview();
        let online_game = self.multiplayer_files.get_game_by_game_id(game_id);
        let online_preview = online_game.as_ref().and_then(|game| game.preview.as_ref());

        if online_game.is_none() {
            self.create_game(game_info.clone()).await?;
        } else if let Some(online_preview) = online_preview {
            if self.has_newer_game_state(&preview, online_preview) {
                online_game.unwrap().do_manual_update(preview);
            }
        }

        UncivGame::current().load_game(game_info);
        Ok(())
    }

    /// Checks if the given game is current and loads it, otherwise loads the game from the server
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info to load
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    pub async fn load_game_from_game_info(&self, game_info: GameInfo) -> Result<(), Box<dyn std::error::Error>> {
        let game_id = &game_info.game_id;
        let preview = self.multiplayer_server.try_download_game_preview(game_id).await?;

        if self.has_latest_game_state(&game_info, &preview) {
            let mut game_info = game_info;
            game_info.is_up_to_date = true;
            UncivGame::current().load_game(game_info);
            Ok(())
        } else {
            self.load_game(game_id).await
        }
    }

    /// Update a game on the server
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info to update
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    /// * `MultiplayerAuthException` - If the authentication failed
    pub async fn update_game(&self, game_info: &GameInfo) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Updating remote game {}", game_info.game_id);
        self.multiplayer_server.try_upload_game(game_info, true).await?;

        let game = self.multiplayer_files.get_game_by_game_id(&game_info.game_id);
        debug!("Existing OnlineMultiplayerGame: {:?}", game);

        if game.is_none() {
            self.multiplayer_files.add_game(game_info).await?;
        } else {
            game.unwrap().do_manual_update(game_info.as_preview());
        }

        Ok(())
    }

    /// Checks if `game_info` and `preview` are up-to-date with each other.
    ///
    /// # Parameters
    ///
    /// * `game_info` - The game info to check
    /// * `preview` - The preview to check against
    ///
    /// # Returns
    ///
    /// `true` if the game info and preview are up-to-date with each other
    pub fn has_latest_game_state(&self, game_info: &GameInfo, preview: &GameInfoPreview) -> bool {
        game_info.current_player == preview.current_player && game_info.turns == preview.turns
    }

    /// Checks if `preview1` has a more recent game state than `preview2`
    ///
    /// # Parameters
    ///
    /// * `preview1` - The first preview to check
    /// * `preview2` - The second preview to check against
    ///
    /// # Returns
    ///
    /// `true` if `preview1` has a more recent game state than `preview2`
    fn has_newer_game_state(&self, preview1: &GameInfoPreview, preview2: &GameInfoPreview) -> bool {
        preview1.turns > preview2.turns
    }

    /// Check if the game is using a custom server
    ///
    /// # Returns
    ///
    /// `true` if the game is using a custom server
    pub fn uses_custom_server() -> bool {
        UncivGame::current().settings.multiplayer.server != Some(Constants::dropbox_multiplayer_server())
    }

    /// Check if the game is using Dropbox
    ///
    /// # Returns
    ///
    /// `true` if the game is using Dropbox
    pub fn uses_dropbox() -> bool {
        !Self::uses_custom_server()
    }

    /// Calls the given `action` when `last_successful_execution` lies further in the past than `throttle_interval`.
    ///
    /// Also updates `last_successful_execution` to `Instant::now()`, but only when `action` did not result in an exception.
    ///
    /// Any exception thrown by `action` is propagated.
    ///
    /// # Parameters
    ///
    /// * `last_successful_execution` - The last time the action was executed
    /// * `throttle_interval` - The minimum time between executions
    /// * `on_no_execution` - The function to call if the action is not executed
    /// * `on_failed` - The function to call if the action fails
    /// * `action` - The action to execute
    ///
    /// # Returns
    ///
    /// The result of the action or `on_no_execution`
    async fn throttle<T, F, G, H, A>(
        last_successful_execution: &AtomicReference<Option<Instant>>,
        throttle_interval: Duration,
        on_no_execution: F,
        on_failed: G,
        action: A,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> T,
        G: FnOnce(Box<dyn std::error::Error>) -> T,
        A: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
    {
        let last_execution = last_successful_execution.load(Ordering::Relaxed);
        let now = Instant::now();
        let should_run_action = last_execution.is_none() || now.duration_since(last_execution.unwrap()) > throttle_interval;

        if should_run_action {
            Self::attempt_action(last_successful_execution, on_no_execution, on_failed, action).await
        } else {
            Ok(on_no_execution())
        }
    }

    /// Attempts to run the `action`, changing `last_successful_execution`, but only if no other thread changed `last_successful_execution` in the meantime
    /// and `action` did not throw an exception.
    ///
    /// # Parameters
    ///
    /// * `last_successful_execution` - The last time the action was executed
    /// * `on_no_execution` - The function to call if the action is not executed
    /// * `on_failed` - The function to call if the action fails
    /// * `action` - The action to execute
    ///
    /// # Returns
    ///
    /// The result of the action or `on_no_execution`
    async fn attempt_action<T, F, G, A>(
        last_successful_execution: &AtomicReference<Option<Instant>>,
        on_no_execution: F,
        on_failed: G,
        action: A,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> T,
        G: FnOnce(Box<dyn std::error::Error>) -> T,
        A: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
    {
        let last_execution = last_successful_execution.load(Ordering::Relaxed);
        let now = Instant::now();

        if last_successful_execution.compare_exchange(
            last_execution,
            Some(now),
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_ok() {
            match action() {
                Ok(result) => Ok(result),
                Err(e) => {
                    last_successful_execution.compare_exchange(
                        Some(now),
                        last_execution,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ).ok();
                    Err(on_failed(e))
                }
            }
        } else {
            Ok(on_no_execution())
        }
    }
}

/// Extension trait for GameInfoPreview to check if it's the user's turn
pub trait GameInfoPreviewExt {
    /// Check if it's the user's turn
    fn is_users_turn(&self) -> bool;
}

impl GameInfoPreviewExt for GameInfoPreview {
    fn is_users_turn(&self) -> bool {
        self.get_civilization(&self.current_player).player_id == UncivGame::current().settings.multiplayer.user_id
    }
}

/// Extension trait for GameInfo to check if it's the user's turn
pub trait GameInfoExt {
    /// Check if it's the user's turn
    fn is_users_turn(&self) -> bool;
}

impl GameInfoExt for GameInfo {
    fn is_users_turn(&self) -> bool {
        !self.current_player.is_empty() && self.get_civilization(&self.current_player).player_id == UncivGame::current().settings.multiplayer.user_id
    }
}