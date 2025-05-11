use std::collections::HashMap;
use std::sync::atomic::{AtomicReference, Ordering};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio::task::JoinHandle;
use reqwest::Client;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use rand::Rng;
use log::{debug, error, info};

use crate::models::game_info::GameInfo;
use crate::models::event::Event;
use crate::models::event_bus::EventBus;
use crate::models::settings::Settings;
use crate::models::multiplayer::api_version::ApiVersion;
use crate::models::multiplayer::api_v2_wrapper::ApiV2Wrapper;
use crate::models::multiplayer::api_v2_file_storage_emulator::ApiV2FileStorageEmulator;
use crate::models::multiplayer::api_v2_file_storage_wrapper::ApiV2FileStorageWrapper;
use crate::models::multiplayer::multiplayer_file_not_found_exception::MultiplayerFileNotFoundException;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;
use crate::models::multiplayer::api_status_code::ApiStatusCode;
use crate::models::multiplayer::version_response::VersionResponse;
use crate::models::multiplayer::api_error_response::ApiErrorResponse;
use crate::models::multiplayer::websocket_message_serializer::WebSocketMessageSerializer;
use crate::models::multiplayer::websocket_message_type::WebSocketMessageType;
use crate::models::multiplayer::websocket_message_with_content::WebSocketMessageWithContent;
use crate::models::multiplayer::update_game_data::UpdateGameData;
use crate::utils::concurrency::Concurrency;

/// Main class to interact with multiplayer servers implementing ApiVersion::ApiV2
pub struct ApiV2 {
    /// Base URL for the API
    base_url: String,

    /// Cache the result of the last server API compatibility check
    compatibility_check: Option<bool>,

    /// Channel to send frames via WebSocket to the server, may be null
    /// for unsupported servers or unauthenticated/uninitialized clients
    send_channel: Option<Sender<Message>>,

    /// Info whether this class is fully initialized and ready to use
    initialized: bool,

    /// Switch to enable auto-reconnect attempts for the WebSocket connection
    reconnect_websocket: bool,

    /// Timestamp of the last successful login
    last_successful_authentication: AtomicReference<Option<Instant>>,

    /// Cache for the game details to make certain lookups faster
    game_details: HashMap<Uuid, TimedGameDetails>,

    /// List of channels that extend the usage of the EventBus system
    event_channel_list: Vec<Sender<Event>>,

    /// Map of waiting receivers of pongs (answers to pings) via a channel that gets null
    /// or any thrown exception; access is synchronized on the ApiV2 instance
    pong_receivers: HashMap<String, oneshot::Sender<Option<UncivNetworkException>>>,

    /// WebSocket jobs
    websocket_jobs: Vec<JoinHandle<()>>,

    /// HTTP client
    client: Client,

    /// API wrapper
    wrapper: ApiV2Wrapper,
}

/// Small struct to store the most relevant details about a game, useful for caching
#[derive(Debug, Clone)]
pub struct GameDetails {
    pub game_uuid: Uuid,
    pub chat_room_uuid: Uuid,
    pub data_id: i64,
    pub name: String,
}

/// Holding the same values as GameDetails, but with an instant determining the last refresh
#[derive(Debug, Clone)]
struct TimedGameDetails {
    refreshed: Instant,
    game_uuid: Uuid,
    chat_room_uuid: Uuid,
    data_id: i64,
    name: String,
}

impl TimedGameDetails {
    fn to(&self) -> GameDetails {
        GameDetails {
            game_uuid: self.game_uuid,
            chat_room_uuid: self.chat_room_uuid,
            data_id: self.data_id,
            name: self.name.clone(),
        }
    }
}

impl ApiV2 {
    /// Creates a new ApiV2 instance
    pub fn new(base_url: String) -> Self {
        ApiV2 {
            base_url,
            compatibility_check: None,
            send_channel: None,
            initialized: false,
            reconnect_websocket: true,
            last_successful_authentication: AtomicReference::new(None),
            game_details: HashMap::new(),
            event_channel_list: Vec::new(),
            pong_receivers: HashMap::new(),
            websocket_jobs: Vec::new(),
            client: Client::new(),
            wrapper: ApiV2Wrapper::new(base_url.clone()),
        }
    }

    /// Get a receiver channel for WebSocket Events that is decoupled from the EventBus system
    pub fn get_websocket_event_channel(&mut self) -> Receiver<Event> {
        // We're using a channel with capacity 1 to avoid usage of possibly huge amounts of memory
        let (tx, rx) = channel::<Event>(1);
        self.event_channel_list.push(tx);
        rx
    }

    /// Initialize this class (performing actual networking connectivity)
    pub async fn initialize(&mut self, credentials: Option<(String, String)>) -> Result<(), UncivNetworkException> {
        if self.compatibility_check.is_none() {
            self.is_compatible().await?;
        }

        if !self.is_compatible().await? {
            error!("Incompatible API detected at '{}'! Further APIv2 usage will most likely break!", self.base_url);
        }

        if let Some((username, password)) = credentials {
            if !self.wrapper.auth.login(&username, &password, true).await? {
                debug!("Login failed using provided credentials (username '{}')", username);
            } else {
                self.last_successful_authentication.store(Some(Instant::now()), Ordering::SeqCst);
                Concurrency::run(|| {
                    self.refresh_game_details().await;
                });
            }
        }

        ApiV2FileStorageWrapper::storage = Some(Box::new(ApiV2FileStorageEmulator::new(self)));
        ApiV2FileStorageWrapper::api = Some(Box::new(self.clone()));
        self.initialized = true;

        Ok(())
    }

    /// Determine if the user is authenticated by comparing timestamps
    pub fn is_authenticated(&self) -> bool {
        if let Some(last_auth) = self.last_successful_authentication.load(Ordering::SeqCst) {
            if let Some(timeout) = last_auth.checked_add(Duration::from_secs(3600)) {
                return timeout > Instant::now();
            }
        }
        false
    }

    /// Determine if this class has been fully initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Dispose this class and its children and jobs
    pub async fn dispose(&mut self) {
        self.disable_reconnecting();

        if let Some(sender) = self.send_channel.take() {
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Disposing".into(),
            }))).await;
        }

        for channel in &self.event_channel_list {
            let _ = channel.try_send(Event::Dispose);
        }

        for job in &self.websocket_jobs {
            job.abort();
        }

        for job in &self.websocket_jobs {
            let _ = job.await;
        }

        self.client = Client::new();
    }

    /// Determine if the remote server is compatible with this API implementation
    pub async fn is_compatible(&mut self, update: bool) -> Result<bool, UncivNetworkException> {
        if self.compatibility_check.is_some() && !update {
            return Ok(self.compatibility_check.unwrap());
        }

        let version_info = match self.client.get(format!("{}/api/version", self.base_url)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<VersionResponse>().await {
                        Ok(body) => body.version == 2,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            },
            Err(e) => {
                error!("Unexpected exception calling version endpoint for '{}': {}", self.base_url, e);
                false
            }
        };

        if !version_info {
            self.compatibility_check = Some(false);
            return Ok(false);
        }

        let websocket_support = match self.client.get(format!("{}/api/v2/ws", self.base_url)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    error!("Websocket endpoint from '{}' accepted unauthenticated request", self.base_url);
                    false
                } else {
                    match response.json::<ApiErrorResponse>().await {
                        Ok(body) => body.status_code == ApiStatusCode::Unauthenticated,
                        Err(_) => false,
                    }
                }
            },
            Err(e) => {
                error!("Unexpected exception calling WebSocket endpoint for '{}': {}", self.base_url, e);
                false
            }
        };

        self.compatibility_check = Some(websocket_support);
        Ok(websocket_support)
    }

    /// Fetch server's details about a game based on its game ID
    pub async fn get_game_details(&mut self, game_id: Uuid) -> Result<GameDetails, MultiplayerFileNotFoundException> {
        if let Some(result) = self.game_details.get(&game_id) {
            if result.refreshed + Duration::from_secs(300) > Instant::now() {
                return Ok(result.to());
            }
        }

        self.refresh_game_details().await;

        if let Some(result) = self.game_details.get(&game_id) {
            Ok(result.to())
        } else {
            Err(MultiplayerFileNotFoundException::new(None))
        }
    }

    /// Refresh the cache of known multiplayer games
    async fn refresh_game_details(&mut self) {
        match self.wrapper.game.list().await {
            Ok(current_games) => {
                // Remove games that are no longer in the list
                let current_game_uuids: Vec<Uuid> = current_games.iter().map(|g| g.game_uuid).collect();
                self.game_details.retain(|uuid, _| current_game_uuids.contains(uuid));

                // Add or update games
                for game in current_games {
                    self.game_details.insert(
                        game.game_uuid,
                        TimedGameDetails {
                            refreshed: Instant::now(),
                            game_uuid: game.game_uuid,
                            chat_room_uuid: game.chat_room_uuid,
                            data_id: game.data_id,
                            name: game.name,
                        }
                    );
                }
            },
            Err(_) => {
                // Handle error
            }
        }
    }

    /// Send text as a TEXT frame to the server via WebSocket (fire & forget)
    pub async fn send_text(&self, text: &str, suppress: bool) -> Result<bool, UncivNetworkException> {
        if let Some(channel) = &self.send_channel {
            match channel.send(Message::Text(text.to_string())).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    debug!("Sending text via WebSocket failed: {}\n{}", e, e);
                    if !suppress {
                        Err(UncivNetworkException::new(&e.to_string(), None))
                    } else {
                        Ok(false)
                    }
                }
            }
        } else {
            debug!("No WebSocket connection, can't send text frame to server: '{}'", text);
            if suppress {
                Ok(false)
            } else {
                Err(UncivNetworkException::new("WebSocket not connected", None))
            }
        }
    }

    /// Send a PING frame to the server, without awaiting a response
    async fn send_ping(&self, size: usize) -> Result<bool, UncivNetworkException> {
        let mut rng = rand::thread_rng();
        let mut body = vec![0u8; size];
        rng.fill_bytes(&mut body);
        self.send_ping_with_content(&body).await
    }

    /// Send a PING frame with the specified content to the server, without awaiting a response
    async fn send_ping_with_content(&self, content: &[u8]) -> Result<bool, UncivNetworkException> {
        if let Some(channel) = &self.send_channel {
            match channel.send(Message::Ping(content.to_vec())).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    /// Send a PONG frame with the specified content to the server
    async fn send_pong(&self, content: &[u8]) -> Result<bool, UncivNetworkException> {
        if let Some(channel) = &self.send_channel {
            match channel.send(Message::Pong(content.to_vec())).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    /// Send a PING and await the response of a PONG
    pub async fn await_ping(&mut self, size: usize, timeout: Option<Duration>) -> Result<Option<f64>, UncivNetworkException> {
        if size < 2 {
            return Err(UncivNetworkException::new("Size too small to identify ping responses uniquely", None));
        }

        let mut rng = rand::thread_rng();
        let mut body = vec![0u8; size];
        rng.fill_bytes(&mut body);

        let key = body.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let (tx, rx) = oneshot::channel();

        {
            let mut pong_receivers = self.pong_receivers.clone();
            pong_receivers.insert(key.clone(), tx);
            self.pong_receivers = pong_receivers;
        }

        let timeout_handle = if let Some(timeout) = timeout {
            let tx_clone = tx.clone();
            Some(tokio::spawn(async move {
                sleep(timeout).await;
                let _ = tx_clone.send(None);
            }))
        } else {
            None
        };

        let start = Instant::now();

        if !self.send_ping_with_content(&body).await? {
            return Ok(None);
        }

        match rx.await {
            Ok(Some(exception)) => Err(exception),
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }?;

        if let Some(handle) = timeout_handle {
            handle.abort();
        }

        let duration = start.elapsed();
        Ok(Some(duration.as_secs_f64() * 1000.0))
    }

    /// Handler for incoming PONG frames to make await_ping work properly
    async fn on_pong(&mut self, content: &[u8]) {
        let key = content.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        if let Some(sender) = self.pong_receivers.remove(&key) {
            let _ = sender.send(None);
        }
    }

    /// Handle a newly established WebSocket connection
    async fn handle_websocket(&mut self, stream: WebSocketStream<MaybeTlsStream<reqwest::Client>>) {
        if let Some(sender) = self.send_channel.take() {
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Replacing connection".into(),
            }))).await;
        }

        let (mut ws_sender, mut ws_receiver) = stream.split();
        let (tx, mut rx) = channel::<Message>(32);
        self.send_channel = Some(tx);

        // Spawn a task to handle outgoing messages
        let sender_handle = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    debug!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
        });

        self.websocket_jobs.push(sender_handle);

        // Spawn a task to send periodic pings
        let ping_handle = {
            let tx = tx.clone();
            tokio::spawn(async move {
                while tx.send(Message::Ping(vec![])).await.is_ok() {
                    sleep(Duration::from_secs(30)).await;
                }
            })
        };

        self.websocket_jobs.push(ping_handle);

        // Handle incoming messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Ping(content)) => {
                    let _ = self.send_pong(&content).await;
                },
                Ok(Message::Pong(content)) => {
                    self.on_pong(&content).await;
                },
                Ok(Message::Close(frame)) => {
                    debug!("Received CLOSE frame via WebSocket: {:?}", frame);
                    break;
                },
                Ok(Message::Binary(data)) => {
                    debug!("Received binary packet of size {} which can't be parsed at the moment", data.len());
                },
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<WebSocketMessageWithContent>(&text) {
                        Ok(msg) => {
                            debug!("Incoming WebSocket message {}: {:?}", std::any::type_name_of_val(&msg), msg);

                            match msg.message_type {
                                WebSocketMessageType::InvalidMessage => {
                                    debug!("Received 'InvalidMessage' from WebSocket connection");
                                },
                                _ => {
                                    // Send the event to the EventBus
                                    EventBus::send(msg.content.clone());

                                    // Send the event to all event channels
                                    for channel in &self.event_channel_list {
                                        if let Err(e) = channel.try_send(msg.content.clone()) {
                                            debug!("Failed to send event to channel: {}", e);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            error!("{}\n{}", e, e);
                        }
                    }
                },
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Clean up
        if let Some(sender) = self.send_channel.take() {
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Connection closed".into(),
            }))).await;
        }

        if self.reconnect_websocket {
            self.websocket().await;
        }
    }

    /// Ensure that the WebSocket is connected (send a PING and build a new connection on failure)
    pub async fn ensure_connected_websocket(&mut self, timeout: Option<Duration>) -> Result<Option<f64>, UncivNetworkException> {
        let ping_measurement = match self.await_ping(2, timeout).await {
            Ok(measurement) => measurement,
            Err(e) => {
                debug!("Error {} while ensuring connected WebSocket: {}", e, e);
                None
            }
        };

        if ping_measurement.is_none() {
            self.websocket().await?;
        }

        Ok(ping_measurement)
    }

    /// Perform post-login hooks and updates
    pub async fn after_login(&mut self) -> Result<(), UncivNetworkException> {
        self.enable_reconnecting();

        match self.wrapper.account.get(false, true).await {
            Ok(me) => {
                error!(
                    "Updating user ID from {} to {}. This is no error. But you may need the old ID to be able to access your old multiplayer saves.",
                    Settings::get().multiplayer.user_id,
                    me.uuid
                );

                Settings::get_mut().multiplayer.user_id = me.uuid.to_string();
                Settings::get().save();

                self.ensure_connected_websocket(Some(Duration::from_secs(5))).await?;
            },
            Err(_) => {
                // Handle error
            }
        }

        self.wrapper.after_login().await
    }

    /// Perform the post-logout hook, cancelling all WebSocket jobs and event channels
    pub async fn after_logout(&mut self, success: bool) -> Result<(), UncivNetworkException> {
        self.disable_reconnecting();

        if let Some(sender) = self.send_channel.take() {
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Logging out".into(),
            }))).await;
        }

        if success {
            for channel in &self.event_channel_list {
                let _ = channel.try_send(Event::Dispose);
            }

            for job in &self.websocket_jobs {
                job.abort();
            }
        }

        self.wrapper.after_logout(success).await
    }

    /// Refresh the currently used session by logging in with username and password stored in the game settings
    pub async fn refresh_session(&mut self, ignore_last_credentials: bool) -> Result<bool, UncivNetworkException> {
        if !ignore_last_credentials {
            return Ok(false);
        }

        let success = self.wrapper.auth.login(
            &Settings::get().multiplayer.user_name,
            &Settings::get().multiplayer.passwords.get(&Settings::get().online_multiplayer.multiplayer_server.get_server_url()).cloned().unwrap_or_default(),
            true
        ).await?;

        if success {
            self.last_successful_authentication.store(Some(Instant::now()), Ordering::SeqCst);
        }

        Ok(success)
    }

    /// Enable auto re-connect attempts for the WebSocket connection
    pub fn enable_reconnecting(&mut self) {
        self.reconnect_websocket = true;
    }

    /// Disable auto re-connect attempts for the WebSocket connection
    pub fn disable_reconnecting(&mut self) {
        self.reconnect_websocket = false;
    }

    /// Connect to the WebSocket
    async fn websocket(&mut self) -> Result<(), UncivNetworkException> {
        let url = format!("{}/api/v2/ws", self.base_url);
        let (ws_stream, _) = connect_async(url).await?;
        self.handle_websocket(ws_stream).await;
        Ok(())
    }
}

impl Clone for ApiV2 {
    fn clone(&self) -> Self {
        ApiV2 {
            base_url: self.base_url.clone(),
            compatibility_check: self.compatibility_check,
            send_channel: None, // Don't clone the channel
            initialized: self.initialized,
            reconnect_websocket: self.reconnect_websocket,
            last_successful_authentication: AtomicReference::new(self.last_successful_authentication.load(Ordering::SeqCst)),
            game_details: self.game_details.clone(),
            event_channel_list: Vec::new(), // Don't clone the channels
            pong_receivers: HashMap::new(), // Don't clone the receivers
            websocket_jobs: Vec::new(), // Don't clone the jobs
            client: self.client.clone(),
            wrapper: self.wrapper.clone(),
        }
    }
}