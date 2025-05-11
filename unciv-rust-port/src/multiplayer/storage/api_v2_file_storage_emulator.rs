use std::sync::Arc;
use log::debug;
use uuid::Uuid;
use tokio::runtime::Runtime;

use crate::multiplayer::api_v2::ApiV2;
use crate::multiplayer::storage::file_storage::{FileStorage, FileMetaData};
use crate::files::unciv_files::{UncivFiles, GameInfo};

const PREVIEW_SUFFIX: &str = "_Preview";

/// Transition helper that emulates file storage behavior using the API v2
pub struct ApiV2FileStorageEmulator {
    api: Arc<ApiV2>,
    runtime: Runtime,
}

impl ApiV2FileStorageEmulator {
    pub fn new(api: Arc<ApiV2>) -> Self {
        Self {
            api,
            runtime: Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }

    async fn save_game_data(&self, game_id: &str, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let uuid = Uuid::parse_str(&game_id.to_lowercase())?;
        self.api.game.upload(uuid, data).await?;
        Ok(())
    }

    async fn save_preview_data(&self, _game_id: &str, _data: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Not implemented for this API
        debug!("Call to deprecated API 'savePreviewData'");
        Ok(())
    }

    async fn load_game_data(&self, game_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let uuid = Uuid::parse_str(&game_id.to_lowercase())?;
        let game_state = self.api.game.get(uuid, false).await?;
        Ok(game_state.game_data)
    }

    async fn load_preview_data(&self, game_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Not implemented for this API
        debug!("Call to deprecated API 'loadPreviewData'");
        // TODO: This could be improved, since this consumes more resources than necessary
        let game_data = self.load_game_data(game_id).await?;
        let game_info = UncivFiles::game_info_from_string(&game_data)?;
        let preview = game_info.as_preview();
        Ok(UncivFiles::game_info_to_string(&preview)?)
    }

    async fn delete_game_data(&self, _game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Not yet implemented
        unimplemented!("Not yet implemented");
    }

    async fn delete_preview_data(&self, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Not implemented for this API
        debug!("Call to deprecated API 'deletedPreviewData'");
        self.delete_game_data(game_id).await
    }

    async fn authenticate(&self, user_id: &str, password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.api.auth.login_only(user_id, password).await
    }

    async fn set_password(&self, new_password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.api.account.set_password(new_password, true).await
    }
}

impl FileStorage for ApiV2FileStorageEmulator {
    fn save_file_data(&self, file_name: &str, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.block_on(async {
            if file_name.ends_with(PREVIEW_SUFFIX) {
                let base_name = file_name.strip_suffix(PREVIEW_SUFFIX).unwrap();
                self.save_preview_data(base_name, data).await
            } else {
                self.save_game_data(file_name, data).await
            }
        })
    }

    fn load_file_data(&self, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.runtime.block_on(async {
            if file_name.ends_with(PREVIEW_SUFFIX) {
                let base_name = file_name.strip_suffix(PREVIEW_SUFFIX).unwrap();
                self.load_preview_data(base_name).await
            } else {
                self.load_game_data(file_name).await
            }
        })
    }

    fn get_file_meta_data(&self, _file_name: &str) -> Result<FileMetaData, Box<dyn std::error::Error>> {
        // Not yet implemented
        unimplemented!("Not yet implemented");
    }

    fn delete_file(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.block_on(async {
            if file_name.ends_with(PREVIEW_SUFFIX) {
                let base_name = file_name.strip_suffix(PREVIEW_SUFFIX).unwrap();
                self.delete_preview_data(base_name).await
            } else {
                self.delete_game_data(file_name).await
            }
        })
    }

    fn authenticate(&self, user_id: &str, password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.runtime.block_on(self.authenticate(user_id, password))
    }

    fn set_password(&self, new_password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.runtime.block_on(self.set_password(new_password))
    }
}

/// Workaround to "just get" the file storage handler and the API, but without initializing
///
/// TODO: This wrapper should be replaced by better file storage initialization handling.
///
/// This struct keeps references which are populated during program startup at runtime.
pub struct ApiV2FileStorageWrapper {
    pub api: Option<Arc<ApiV2>>,
    pub storage: Option<Arc<ApiV2FileStorageEmulator>>,
}

impl ApiV2FileStorageWrapper {
    pub fn new() -> Self {
        Self {
            api: None,
            storage: None,
        }
    }
}

// Make it a singleton
impl Default for ApiV2FileStorageWrapper {
    fn default() -> Self {
        Self::new()
    }
}