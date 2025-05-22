use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Client, ClientBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::models::multiplayer::api_error_response::ApiErrorResponse;
use crate::models::multiplayer::api_status_code::ApiStatusCode;
use crate::models::multiplayer::api_v2_file_storage_emulator::ApiV2FileStorageEmulator;
use crate::models::multiplayer::api_v2_file_storage_wrapper::ApiV2FileStorageWrapper;
use crate::models::multiplayer::api_version::ApiVersion;
use crate::models::multiplayer::multiplayer_file_not_found_exception::MultiplayerFileNotFoundException;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;
use crate::models::multiplayer::update_game_data::UpdateGameData;
use crate::models::multiplayer::version_response::VersionResponse;
use crate::models::multiplayer::websocket_message_serializer::WebSocketMessageSerializer;
use crate::models::multiplayer::websocket_message_type::WebSocketMessageType;
use crate::models::multiplayer::websocket_message_with_content::WebSocketMessageWithContent;
use crate::utils::concurrency::Concurrency;

use super::accounts_api::AccountsApi;
use super::auth_api::AuthApi;
use super::auth_helper::AuthHelper;
use super::chat_api::ChatApi;
use super::friend_api::FriendApi;
use super::game_api::GameApi;
use super::invite_api::InviteApi;
use super::lobby_api::LobbyApi;

/// Default request timeout in milliseconds
const DEFAULT_REQUEST_TIMEOUT: u64 = 30000;
/// Default connection timeout in milliseconds
const DEFAULT_CONNECT_TIMEOUT: u64 = 10000;

/// API wrapper around the newly implemented REST API for multiplayer game handling
///
/// Note that this class does not include the handling of messages via the
/// WebSocket connection, but rather only the pure HTTP-based API.
/// Almost any method may throw certain OS or network errors as well as the
/// [ApiErrorResponse] for invalid requests (4xx) or server failures (5xx).
///
/// This class should be considered implementation detail, since it just
/// abstracts HTTP endpoint names from other modules in this package.
/// Use the [ApiV2] class for public methods to interact with the server.
pub struct ApiV2Wrapper {
    /// Base URL for the API
    base_url: String,

    /// Base URL implementation (ensures it ends with a slash)
    base_url_impl: String,

    /// Base server URL
    base_server: String,

    /// HTTP client to handle the server connections, logging, content parsing and cookies
    client: Client,

    /// Helper that replaces library cookie storages to fix cookie serialization problems and keeps
    /// track of user-supplied credentials to be able to refresh expired sessions on the fly
    auth_helper: Arc<AuthHelper>,

    /// Queue to keep references to all opened WebSocket handler jobs
    websocket_jobs: Vec<tokio::task::JoinHandle<()>>,

    /// API for account management
    pub account: AccountsApi,

    /// API for authentication management
    pub auth: AuthApi,

    /// API for chat management
    pub chat: ChatApi,

    /// API for friendship management
    pub friend: FriendApi,

    /// API for game management
    pub game: GameApi,

    /// API for invite management
    pub invite: InviteApi,

    /// API for lobby management
    pub lobby: LobbyApi,
}

impl ApiV2Wrapper {
    /// Creates a new ApiV2Wrapper instance
    pub fn new(base_url: String) -> Self {
        let base_url_impl = if base_url.ends_with("/") {
            base_url.clone()
        } else {
            format!("{}/", base_url)
        };

        let base_server = base_url.clone();

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Unciv/4.10.14-GNU-Terry-Pratchett"),
        );

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(DEFAULT_REQUEST_TIMEOUT))
            .connect_timeout(Duration::from_millis(DEFAULT_CONNECT_TIMEOUT))
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        let auth_helper = Arc::new(AuthHelper::new());

        let account = AccountsApi::new(client.clone(), auth_helper.clone());
        let auth = AuthApi::new(client.clone(), auth_helper.clone());
        let chat = ChatApi::new(client.clone(), auth_helper.clone());
        let friend = FriendApi::new(client.clone(), auth_helper.clone());
        let game = GameApi::new(client.clone(), auth_helper.clone());
        let invite = InviteApi::new(client.clone(), auth_helper.clone());
        let lobby = LobbyApi::new(client.clone(), auth_helper.clone());

        Self {
            base_url,
            base_url_impl,
            base_server,
            client,
            auth_helper,
            websocket_jobs: Vec::new(),
            account,
            auth,
            chat,
            friend,
            game,
            invite,
            lobby,
        }
    }

    /// Start a new WebSocket connection
    ///
    /// The parameter `handler` is a function that will be fed the established
    /// WebSocket stream on success at a later point. Note that this
    /// method does instantly return, detaching the creation of the WebSocket.
    /// The `handler` function might not get called, if opening the WS fails.
    /// Use `job_callback` to receive the newly created job handling the WS connection.
    pub async fn websocket<F, C>(
        &mut self,
        handler: F,
        job_callback: Option<C>,
    ) -> Result<bool, UncivNetworkException>
    where
        F: FnOnce(WebSocketStream<MaybeTlsStream<reqwest::Client>>) + Send + 'static,
        C: FnOnce(tokio::task::JoinHandle<()>) + Send + 'static,
    {
        debug!("Starting a new WebSocket connection ...");

        let url = if self.base_server.starts_with("https://") {
            format!(
                "wss://{}/api/v2/ws",
                self.base_server
                    .strip_prefix("https://")
                    .unwrap_or(&self.base_server)
            )
        } else {
            format!(
                "ws://{}/api/v2/ws",
                self.base_server
                    .strip_prefix("http://")
                    .unwrap_or(&self.base_server)
            )
        };

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                let job = tokio::spawn(async move {
                    handler(ws_stream);
                });

                self.websocket_jobs.push(job.clone());

                debug!("A new WebSocket has been created, running in job {:?}", job);

                if let Some(callback) = job_callback {
                    callback(job);
                }

                Ok(true)
            }
            Err(e) => {
                debug!("Failed to establish WebSocket connection: {}", e);
                Ok(false)
            }
        }
    }

    /// Retrieve the currently available API version of the connected server
    ///
    /// Unlike other API endpoint implementations, this function does not handle
    /// any errors or retries on failure. You must wrap any call in a try-except
    /// clause expecting any type of error. The error may not be appropriate to
    /// be shown to end users, i.e. it's definitively no [UncivShowableException].
    pub async fn version(&self) -> Result<VersionResponse, UncivNetworkException> {
        let response = self
            .client
            .get(format!("{}/api/version", self.base_url))
            .send()
            .await
            .map_err(|e| UncivNetworkException::new(&e.to_string(), None))?;

        if response.status().is_success() {
            let version_response = response
                .json::<VersionResponse>()
                .await
                .map_err(|e| UncivNetworkException::new(&e.to_string(), None))?;

            Ok(version_response)
        } else {
            let error_response = response
                .json::<ApiErrorResponse>()
                .await
                .unwrap_or_else(|_| ApiErrorResponse {
                    status_code: ApiStatusCode::Unknown,
                    message: "Failed to parse error response".to_string(),
                });

            Err(UncivNetworkException::new(
                &error_response.message,
                Some(error_response.status_code),
            ))
        }
    }

    /// Performs post-login hooks and updates
    pub async fn after_login(&self) -> Result<(), UncivNetworkException> {
        // Implementation would depend on what needs to be done after login
        Ok(())
    }

    /// Performs post-logout hooks and updates
    pub async fn after_logout(&self, success: bool) -> Result<(), UncivNetworkException> {
        // Implementation would depend on what needs to be done after logout
        Ok(())
    }
}

impl Clone for ApiV2Wrapper {
    fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            base_url_impl: self.base_url_impl.clone(),
            base_server: self.base_server.clone(),
            client: self.client.clone(),
            auth_helper: self.auth_helper.clone(),
            websocket_jobs: Vec::new(), // Don't clone the jobs
            account: self.account.clone(),
            auth: self.auth.clone(),
            chat: self.chat.clone(),
            friend: self.friend.clone(),
            game: self.game.clone(),
            invite: self.invite.clone(),
            lobby: self.lobby.clone(),
        }
    }
}
