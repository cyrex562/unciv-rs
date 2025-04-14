use std::sync::atomic::{AtomicReference, Ordering};
use std::time::{Duration, Instant};
use log::debug;

use crate::game::unciv_game::UncivGame;
use crate::game::game_info::GameInfoPreview;
use crate::game::event_bus::EventBus;
use crate::game::files::FileHandle;
use crate::game::audio::SoundPlayer;
use crate::multiplayer::storage::{
    FileStorageRateLimitReached,
    MultiplayerFileNotFoundException,
    MultiplayerServer,
};
use crate::multiplayer::Multiplayer;

/// How often games can be checked for remote updates when using Dropbox.
/// More attempted checks within this time period will do nothing.
const DROPBOX_THROTTLE_PERIOD: u64 = 8;

/// How often games can be checked for remote updates when using a custom server.
/// More attempted checks within this time period will do nothing.
const CUSTOM_SERVER_THROTTLE_PERIOD: u64 = 1;

/// A multiplayer game
pub struct MultiplayerGame {
    /// The file handle for the game
    pub file_handle: FileHandle,
    /// The game preview, if available
    pub preview: Option<GameInfoPreview>,
    /// The last time the game was updated online
    last_online_update: AtomicReference<Option<Instant>>,
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
    /// * `last_online_update` - The last time the game was updated online
    ///
    /// # Returns
    ///
    /// A new MultiplayerGame
    pub fn new(file_handle: &FileHandle, preview: Option<&GameInfoPreview>, last_online_update: Option<Instant>) -> Self {
        let mut game = Self {
            file_handle: file_handle.clone(),
            preview: preview.cloned(),
            last_online_update: AtomicReference::new(last_online_update),
            name: file_handle.name().to_string(),
            error: None,
        };

        // Load preview from file if not provided
        if game.preview.is_none() {
            match game.load_preview_from_file() {
                Ok(_) => {}
                Err(e) => {
                    game.error = Some(Box::new(e));
                }
            }
        }

        game
    }

    /// Get the last time the game was updated
    ///
    /// # Returns
    ///
    /// The last time the game was updated
    pub fn get_last_update(&self) -> Instant {
        let last_file_update_time = Instant::from_epoch_millis(self.file_handle.last_modified());
        let last_online_update_time = self.last_online_update.load(Ordering::Relaxed);

        if last_online_update_time.is_none() || last_file_update_time > last_online_update_time.unwrap() {
            last_file_update_time
        } else {
            last_online_update_time.unwrap()
        }
    }

    /// Load the preview from the file
    ///
    /// # Returns
    ///
    /// The loaded preview
    ///
    /// # Errors
    ///
    /// * `std::io::Error` - If the preview cannot be loaded
    fn load_preview_from_file(&mut self) -> Result<GameInfoPreview, Box<dyn std::error::Error>> {
        let preview_from_file = UncivGame::current().files.load_game_preview_from_file(&self.file_handle)?;
        self.preview = Some(preview_from_file.clone());
        Ok(preview_from_file)
    }

    /// Check if the game needs an update
    ///
    /// # Returns
    ///
    /// `true` if the game needs an update
    fn needs_update(&self) -> bool {
        self.preview.is_none() || self.error.is_some()
    }

    /// Request an update for the game
    ///
    /// Fires: `MultiplayerGameUpdateStarted`, `MultiplayerGameUpdated`, `MultiplayerGameUpdateUnchanged`, `MultiplayerGameUpdateFailed`
    ///
    /// # Parameters
    ///
    /// * `force_update` - Whether to force the update
    ///
    /// # Returns
    ///
    /// The result of the update
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    pub async fn request_update(&self, force_update: bool) -> Result<(), Box<dyn std::error::Error>> {
        let on_unchanged = || GameUpdateResult::new(GameUpdateResultType::Unchanged, self.preview.clone().unwrap());
        let on_error = |t: Box<dyn std::error::Error>| {
            let mut game = self.clone();
            game.error = Some(t.clone());
            GameUpdateResult::from_error(t)
        };

        debug!("Starting multiplayer game update for {} with id {:?}", self.name, self.preview.as_ref().map(|p| &p.game_id));

        // Send update started event
        EventBus::send(MultiplayerGameUpdateStarted::new(self.name.clone()));

        let throttle_interval = if force_update { Duration::ZERO } else { get_update_throttle_interval() };

        let update_result = if force_update || self.needs_update() {
            Self::attempt_action(&self.last_online_update, on_unchanged, on_error, || self.update()).await
        } else {
            Self::throttle(&self.last_online_update, throttle_interval, on_unchanged, on_error, || self.update()).await
        };

        // Send appropriate event based on update result
        let update_event = match update_result.type_ {
            GameUpdateResultType::Changed if update_result.status.is_some() => {
                let status = update_result.status.as_ref().unwrap();
                debug!("Game update for {} with id {} had remote change", self.name, status.game_id);
                MultiplayerGameUpdated::new(self.name.clone(), status.clone())
            }
            GameUpdateResultType::Failure if update_result.error.is_some() => {
                debug!("Game update for {} with id {:?} failed: {:?}", self.name, self.preview.as_ref().map(|p| &p.game_id), update_result.error);
                MultiplayerGameUpdateFailed::new(self.name.clone(), update_result.error.unwrap())
            }
            GameUpdateResultType::Unchanged if update_result.status.is_some() => {
                let status = update_result.status.as_ref().unwrap();
                debug!("Game update for {} with id {} had no changes", self.name, status.game_id);
                let mut game = self.clone();
                game.error = None;
                MultiplayerGameUpdateUnchanged::new(self.name.clone(), status.clone())
            }
            _ => panic!("Unknown update event"),
        };

        EventBus::send(update_event);
        Ok(())
    }

    /// Update the game
    ///
    /// # Returns
    ///
    /// The result of the update
    ///
    /// # Errors
    ///
    /// * `FileStorageRateLimitReached` - If the file storage backend can't handle any additional actions for a time
    /// * `MultiplayerFileNotFoundException` - If the file can't be found
    async fn update(&self) -> Result<GameUpdateResult, Box<dyn std::error::Error>> {
        let cur_preview = if let Some(preview) = &self.preview {
            preview.clone()
        } else {
            self.load_preview_from_file()?
        };

        let server_identifier = &cur_preview.game_parameters.multiplayer_server_url;
        let new_preview = MultiplayerServer::new(server_identifier).try_download_game_preview(&cur_preview.game_id).await?;

        if new_preview.turns == cur_preview.turns && new_preview.current_player == cur_preview.current_player {
            return Ok(GameUpdateResult::new(GameUpdateResultType::Unchanged, new_preview));
        }

        UncivGame::current().files.save_game(&new_preview, &self.file_handle)?;
        let mut game = self.clone();
        game.preview = Some(new_preview.clone());

        Ok(GameUpdateResult::new(GameUpdateResultType::Changed, new_preview))
    }

    /// Do a manual update for the game
    ///
    /// # Parameters
    ///
    /// * `game_info` - The new game info
    pub fn do_manual_update(&mut self, game_info: GameInfoPreview) {
        debug!("Doing manual update of game {}", game_info.game_id);
        self.last_online_update.store(Some(Instant::now()), Ordering::Relaxed);
        self.error = None;
        self.preview = Some(game_info.clone());

        EventBus::send(MultiplayerGameUpdated::new(self.name.clone(), game_info.clone()));
        self.play_multiplayer_turn_notification(&game_info);
    }

    /// Play a notification sound for a multiplayer turn
    ///
    /// # Parameters
    ///
    /// * `game_info_preview` - The game info preview
    fn play_multiplayer_turn_notification(&self, game_info_preview: &GameInfoPreview) {
        if !game_info_preview.is_users_turn() {
            return;
        }

        if UncivGame::is_deep_linked_game_loading() {
            return; // This means we already arrived here through a turn notification, no need to notify again
        }

        let sound = if UncivGame::is_current_game(&game_info_preview.game_id) {
            UncivGame::current().settings.multiplayer.current_game_turn_notification_sound.clone()
        } else {
            UncivGame::current().settings.multiplayer.other_game_turn_notification_sound.clone()
        };

        SoundPlayer::play(&sound);
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
    async fn throttle<T, F, G, A>(
        last_successful_execution: &AtomicReference<Option<Instant>>,
        throttle_interval: Duration,
        on_no_execution: F,
        on_failed: G,
        action: A,
    ) -> T
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
            on_no_execution()
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
    ) -> T
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
                Ok(result) => result,
                Err(e) => {
                    last_successful_execution.compare_exchange(
                        Some(now),
                        last_execution,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ).ok();
                    on_failed(e)
                }
            }
        } else {
            on_no_execution()
        }
    }
}

impl Clone for MultiplayerGame {
    fn clone(&self) -> Self {
        Self {
            file_handle: self.file_handle.clone(),
            preview: self.preview.clone(),
            last_online_update: AtomicReference::new(self.last_online_update.load(Ordering::Relaxed)),
            name: self.name.clone(),
            error: self.error.as_ref().map(|e| Box::new(e.to_string()) as Box<dyn std::error::Error>),
        }
    }
}

impl PartialEq for MultiplayerGame {
    fn eq(&self, other: &Self) -> bool {
        self.file_handle == other.file_handle
    }
}

impl Eq for MultiplayerGame {}

impl std::hash::Hash for MultiplayerGame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_handle.hash(state);
    }
}

/// The result of a game update
pub struct GameUpdateResult {
    /// The type of the update result
    pub type_: GameUpdateResultType,
    /// The status of the update, if successful
    pub status: Option<GameInfoPreview>,
    /// The error that occurred, if any
    pub error: Option<Box<dyn std::error::Error>>,
}

impl GameUpdateResult {
    /// Create a new GameUpdateResult
    ///
    /// # Parameters
    ///
    /// * `type_` - The type of the update result
    /// * `status` - The status of the update, if successful
    ///
    /// # Returns
    ///
    /// A new GameUpdateResult
    pub fn new(type_: GameUpdateResultType, status: GameInfoPreview) -> Self {
        Self {
            type_,
            status: Some(status),
            error: None,
        }
    }

    /// Create a new GameUpdateResult from an error
    ///
    /// # Parameters
    ///
    /// * `error` - The error that occurred
    ///
    /// # Returns
    ///
    /// A new GameUpdateResult
    pub fn from_error(error: Box<dyn std::error::Error>) -> Self {
        Self {
            type_: GameUpdateResultType::Failure,
            status: None,
            error: Some(error),
        }
    }
}

/// The type of a game update result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameUpdateResultType {
    /// The game was changed
    Changed,
    /// The game was unchanged
    Unchanged,
    /// The update failed
    Failure,
}

/// Event fired when a multiplayer game update is started
pub struct MultiplayerGameUpdateStarted {
    /// The name of the game
    pub name: String,
}

impl MultiplayerGameUpdateStarted {
    /// Create a new MultiplayerGameUpdateStarted event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateStarted event
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

/// Event fired when a multiplayer game is updated
pub struct MultiplayerGameUpdated {
    /// The name of the game
    pub name: String,
    /// The updated game info
    pub game_info: GameInfoPreview,
}

impl MultiplayerGameUpdated {
    /// Create a new MultiplayerGameUpdated event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `game_info` - The updated game info
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdated event
    pub fn new(name: String, game_info: GameInfoPreview) -> Self {
        Self { name, game_info }
    }
}

/// Event fired when a multiplayer game update is unchanged
pub struct MultiplayerGameUpdateUnchanged {
    /// The name of the game
    pub name: String,
    /// The game info
    pub game_info: GameInfoPreview,
}

impl MultiplayerGameUpdateUnchanged {
    /// Create a new MultiplayerGameUpdateUnchanged event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `game_info` - The game info
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateUnchanged event
    pub fn new(name: String, game_info: GameInfoPreview) -> Self {
        Self { name, game_info }
    }
}

/// Event fired when a multiplayer game update fails
pub struct MultiplayerGameUpdateFailed {
    /// The name of the game
    pub name: String,
    /// The error that occurred
    pub error: Box<dyn std::error::Error>,
}

impl MultiplayerGameUpdateFailed {
    /// Create a new MultiplayerGameUpdateFailed event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `error` - The error that occurred
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateFailed event
    pub fn new(name: String, error: Box<dyn std::error::Error>) -> Self {
        Self { name, error }
    }
}

/// Get the throttle interval for game updates
///
/// # Returns
///
/// The throttle interval
fn get_update_throttle_interval() -> Duration {
    Duration::from_secs(if Multiplayer::uses_custom_server() {
        CUSTOM_SERVER_THROTTLE_PERIOD
    } else {
        DROPBOX_THROTTLE_PERIOD
    })
}