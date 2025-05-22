use std::collections::HashMap;
use std::sync::Arc;
use reqwest::header::{HeaderMap, HeaderValue};
use log::debug;

use crate::constants::Constants;
use crate::game::unciv_game::UncivGame;
use crate::game::game_info::{GameInfo, GameInfoPreview};
use crate::files::unciv_files::UncivFiles;
use crate::multiplayer::server_feature_set::ServerFeatureSet;
use crate::multiplayer::storage::file_storage::{FileStorage, FileStorageRateLimitReached, MultiplayerFileNotFoundException, MultiplayerAuthException};
use crate::multiplayer::storage::dropbox::DropBox;
use crate::multiplayer::storage::unciv_server_file_storage::UncivServerFileStorage;
use crate::utils::simple_http::SimpleHttp;

/// Allows access to games stored on a server for multiplayer purposes.
/// Defaults to using UncivGame.Current.settings.multiplayerServer if fileStorageIdentifier is not given.
///
/// For low-level access only, use [UncivGame.onlineMultiplayer] on [UncivGame.Current] if you're looking to load/save a game.
///
/// # Parameters
///
/// * `file_storage_identifier` - must be given if UncivGame.Current might not be initialized
/// * `authentication_header` - optional authentication header for the server
///
/// # See also
///
/// * [FileStorage]
/// * [UncivGame.settings.multiplayer.server]
pub struct MultiplayerServer {
    file_storage_identifier: Option<String>,
    authentication_header: Option<HashMap<String, String>>,
    pub feature_set: ServerFeatureSet,
}

impl MultiplayerServer {
    /// Create a new MultiplayerServer with the given file storage identifier and authentication header
    pub fn new(
        file_storage_identifier: Option<String>,
        authentication_header: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            file_storage_identifier,
            authentication_header,
            feature_set: ServerFeatureSet::new(),
        }
    }

    /// Get the server URL
    pub fn get_server_url(&self) -> String {
        self.file_storage_identifier.clone().unwrap_or_else(|| {
            UncivGame::current().settings.multiplayer.server.clone()
        })
    }

    /// Get the file storage implementation for the server
    pub fn file_storage(&self) -> Arc<dyn FileStorage> {
        let auth_header = if self.authentication_header.is_none() {
            let settings = &UncivGame::current().settings.multiplayer;
            let mut header = HashMap::new();
            header.insert(
                "Authorization".to_string(),
                settings.get_auth_header(),
            );
            Some(header)
        } else {
            self.authentication_header.clone()
        };

        if self.get_server_url() == Constants::DROPBOX_MULTIPLAYER_SERVER {
            Arc::new(DropBox::new())
        } else {
            let mut storage = UncivServerFileStorage::new();
            storage.server_url = self.get_server_url();
            storage.auth_header = auth_header;
            Arc::new(storage)
        }
    }

    /// Checks if the server is alive and sets the [serverFeatureSet] accordingly.
    /// Returns true if the server is alive, false otherwise
    pub fn check_server_status(&mut self) -> bool {
        let mut status_ok = false;
        let url = format!("{}/isalive", self.get_server_url());

        SimpleHttp::send_get_request(&url, |success, result, _| {
            status_ok = success;
            if !result.is_empty() {
                self.feature_set = match serde_json::from_str::<ServerFeatureSet>(&result) {
                    Ok(feature_set) => feature_set,
                    Err(_) => {
                        // The server does not support server feature set - not an error!
                        ServerFeatureSet::new()
                    }
                };
            }
        });

        status_ok
    }

    /// Authenticate with the server
    ///
    /// # Returns
    ///
    /// true if the authentication was successful or the server does not support authentication.
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerAuthException] - if the authentication failed
    pub fn authenticate(&self, password: Option<&str>) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if self.feature_set.auth_version == 0 {
            return Ok(true);
        }

        let settings = &UncivGame::current().settings.multiplayer;
        let storage = self.file_storage();

        let user_id = &settings.user_id;
        let password = password.unwrap_or_else(|| {
            settings.passwords.get(&settings.server)
                .map(|s| s.as_str())
                .unwrap_or("")
        });

        let success = storage.authenticate(user_id, password)?;

        if success && password.is_some() {
            let mut settings = UncivGame::current().settings.multiplayer;
            settings.passwords.insert(settings.server.clone(), password.unwrap().to_string());
        }

        Ok(success)
    }

    /// Set a new password for the server
    ///
    /// # Returns
    ///
    /// true if setting the password was successful, false otherwise.
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerAuthException] - if the authentication failed
    pub fn set_password(&self, password: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if self.feature_set.auth_version > 0 {
            let storage = self.file_storage();
            if storage.set_password(password)? {
                let mut settings = UncivGame::current().settings.multiplayer;
                settings.passwords.insert(settings.server.clone(), password.to_string());
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Upload a game to the server
    ///
    /// # Parameters
    ///
    /// * `game_info` - the game info to upload
    /// * `with_preview` - whether to also upload a preview of the game
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerAuthException] - if the authentication failed
    pub async fn try_upload_game(&self, game_info: &GameInfo, with_preview: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let zipped_game_info = UncivFiles::game_info_to_string(game_info, true, true)?;
        self.file_storage().save_file_data(&game_info.game_id, &zipped_game_info)?;

        // We upload the preview after the game because otherwise the following race condition will happen:
        // Current player ends turn -> Uploads Game Preview
        // Other player checks for updates -> Downloads Game Preview
        // Current player starts game upload
        // Other player sees update in preview -> Downloads game, gets old state
        // Current player finishes uploading game
        if with_preview {
            self.try_upload_game_preview(&game_info.as_preview()).await?;
        }

        Ok(())
    }

    /// Upload a game preview to the server
    ///
    /// Used to upload only the preview of a game. If the preview is uploaded together with (before/after)
    /// the gameInfo, it is recommended to use try_upload_game(gameInfo, withPreview = true)
    ///
    /// # Parameters
    ///
    /// * `game_info` - the game preview to upload
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerAuthException] - if the authentication failed
    ///
    /// # See also
    ///
    /// * [try_upload_game]
    /// * [GameInfo.as_preview]
    pub async fn try_upload_game_preview(&self, game_info: &GameInfoPreview) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let zipped_game_info = UncivFiles::game_info_to_string(game_info)?;
        self.file_storage().save_file_data(&format!("{}_Preview", game_info.game_id), &zipped_game_info)?;

        Ok(())
    }

    /// Try to download a game from the server
    ///
    /// # Parameters
    ///
    /// * `game_id` - the ID of the game to download
    ///
    /// # Returns
    ///
    /// The downloaded game info
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerFileNotFoundException] - if the file can't be found
    pub async fn try_download_game(&self, game_id: &str) -> Result<GameInfo, Box<dyn std::error::Error + Send + Sync>> {
        let zipped_game_info = self.file_storage().load_file_data(game_id)?;
        let mut game_info = UncivFiles::game_info_from_string(&zipped_game_info)?;
        game_info.game_parameters.multiplayer_server_url = Some(UncivGame::current().settings.multiplayer.server.clone());

        Ok(game_info)
    }

    /// Download a game from the server
    ///
    /// # Parameters
    ///
    /// * `game_id` - the ID of the game to download
    ///
    /// # Returns
    ///
    /// The downloaded game info
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerFileNotFoundException] - if the file can't be found
    pub async fn download_game(&self, game_id: &str) -> Result<GameInfo, Box<dyn std::error::Error + Send + Sync>> {
        let mut latest_game = self.try_download_game(game_id).await?;
        latest_game.is_up_to_date = true;

        Ok(latest_game)
    }

    /// Try to download a game preview from the server
    ///
    /// # Parameters
    ///
    /// * `game_id` - the ID of the game to download
    ///
    /// # Returns
    ///
    /// The downloaded game preview
    ///
    /// # Errors
    ///
    /// * [FileStorageRateLimitReached] - if the file storage backend can't handle any additional actions for a time
    /// * [MultiplayerFileNotFoundException] - if the file can't be found
    pub async fn try_download_game_preview(&self, game_id: &str) -> Result<GameInfoPreview, Box<dyn std::error::Error + Send + Sync>> {
        let zipped_game_info = self.file_storage().load_file_data(&format!("{}_Preview", game_id))?;
        let game_preview = UncivFiles::game_info_preview_from_string(&zipped_game_info)?;

        Ok(game_preview)
    }
}