use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::multiplayer::json_serializers::{serialize_uuid, deserialize_uuid, serialize_instant, deserialize_instant};
use crate::multiplayer::response_structs::{AccountResponse, ChatMessage};

/// Enum of all events that can happen in a friendship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_friendship_event")]
#[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_friendship_event")]
pub enum FriendshipEvent {
    Accepted,
    Rejected,
    Deleted,
}

impl FriendshipEvent {
    pub fn to_string(&self) -> &'static str {
        match self {
            FriendshipEvent::Accepted => "accepted",
            FriendshipEvent::Rejected => "rejected",
            FriendshipEvent::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(FriendshipEvent::Accepted),
            "rejected" => Some(FriendshipEvent::Rejected),
            "deleted" => Some(FriendshipEvent::Deleted),
            _ => None,
        }
    }
}

/// The notification for the clients that a new game has started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStarted {
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
    #[serde(rename = "game_chat_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_chat_uuid: Uuid,
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
    #[serde(rename = "lobby_chat_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_chat_uuid: Uuid,
}

/// An update of the game data
///
/// This variant is sent from the server to all accounts that are in the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGameData {
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
    #[serde(rename = "game_data")]
    pub game_data: String,  // base64-encoded, gzipped game state
    /// A counter that is incremented every time a new game states has been uploaded for the same game_uuid via HTTP API.
    #[serde(rename = "game_data_id")]
    pub game_data_id: i64,
}

/// Notification for clients if a client in their game disconnected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDisconnected {
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,  // client identifier
}

/// Notification for clients if a client in their game reconnected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientReconnected {
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,  // client identifier
}

/// A new chat message is sent to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingChatMessage {
    #[serde(rename = "chat_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_uuid: Uuid,
    pub message: ChatMessage,
}

/// An invite to a lobby is sent to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingInvite {
    #[serde(rename = "invite_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub invite_uuid: Uuid,
    pub from: AccountResponse,
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
}

/// A friend request is sent to a client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingFriendRequest {
    pub from: AccountResponse,
}

/// A friendship was modified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendshipChanged {
    pub friend: AccountResponse,
    pub event: FriendshipEvent,
}

/// A new player joined the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyJoin {
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
    pub player: AccountResponse,
}

/// A lobby closed in which the client was part of
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyClosed {
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
}

/// A player has left the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyLeave {
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
    pub player: AccountResponse,
}

/// A player was kicked out of the lobby.
///
/// Make sure to check the player if you were kicked ^^
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyKick {
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
    pub player: AccountResponse,
}

/// The user account was updated
///
/// This might be especially useful for reflecting changes in the username, etc. in the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdated {
    pub account: AccountResponse,
}

/// The base WebSocket message, encapsulating only the type of the message
pub trait WebSocketMessage {
    fn message_type(&self) -> WebSocketMessageType;
}

/// The useful base WebSocket message, encapsulating only the type of the message and the content
pub trait WebSocketMessageWithContent: WebSocketMessage {
    fn content(&self) -> &dyn Event;
}

/// Trait for all event types
pub trait Event {}

// Implement Event for all event types
impl Event for GameStarted {}
impl Event for UpdateGameData {}
impl Event for ClientDisconnected {}
impl Event for ClientReconnected {}
impl Event for IncomingChatMessage {}
impl Event for IncomingInvite {}
impl Event for IncomingFriendRequest {}
impl Event for FriendshipChanged {}
impl Event for LobbyJoin {}
impl Event for LobbyClosed {}
impl Event for LobbyLeave {}
impl Event for LobbyKick {}
impl Event for AccountUpdated {}

/// Message when a previously sent WebSocket frame a received frame is invalid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
}

impl WebSocketMessage for InvalidMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

/// Message to indicate that a game started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: GameStarted,
}

impl WebSocketMessage for GameStartedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for GameStartedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to publish the new game state from the server to all clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGameDataMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: UpdateGameData,
}

impl WebSocketMessage for UpdateGameDataMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for UpdateGameDataMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client disconnected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDisconnectedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: ClientDisconnected,
}

impl WebSocketMessage for ClientDisconnectedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for ClientDisconnectedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client, who previously disconnected, reconnected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientReconnectedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: ClientReconnected,
}

impl WebSocketMessage for ClientReconnectedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for ClientReconnectedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a user received a new text message via the chat feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingChatMessageMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: IncomingChatMessage,
}

impl WebSocketMessage for IncomingChatMessageMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for IncomingChatMessageMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client gets invited to a lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingInviteMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: IncomingInvite,
}

impl WebSocketMessage for IncomingInviteMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for IncomingInviteMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client received a friend request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingFriendRequestMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: IncomingFriendRequest,
}

impl WebSocketMessage for IncomingFriendRequestMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for IncomingFriendRequestMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a friendship has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendshipChangedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: FriendshipChanged,
}

impl WebSocketMessage for FriendshipChangedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for FriendshipChangedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client joined the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyJoinMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: LobbyJoin,
}

impl WebSocketMessage for LobbyJoinMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for LobbyJoinMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that the current lobby got closed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyClosedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: LobbyClosed,
}

impl WebSocketMessage for LobbyClosedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for LobbyClosedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client left the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyLeaveMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: LobbyLeave,
}

impl WebSocketMessage for LobbyLeaveMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for LobbyLeaveMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that a client got kicked out of the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyKickMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: LobbyKick,
}

impl WebSocketMessage for LobbyKickMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for LobbyKickMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Message to indicate that the current user account's data have been changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdatedMessage {
    #[serde(rename = "type")]
    pub message_type: WebSocketMessageType,
    pub content: AccountUpdated,
}

impl WebSocketMessage for AccountUpdatedMessage {
    fn message_type(&self) -> WebSocketMessageType {
        self.message_type
    }
}

impl WebSocketMessageWithContent for AccountUpdatedMessage {
    fn content(&self) -> &dyn Event {
        &self.content
    }
}

/// Type enum of all known WebSocket messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_websocket_message_type")]
#[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_websocket_message_type")]
pub enum WebSocketMessageType {
    InvalidMessage,
    GameStarted,
    UpdateGameData,
    ClientDisconnected,
    ClientReconnected,
    IncomingChatMessage,
    IncomingInvite,
    IncomingFriendRequest,
    FriendshipChanged,
    LobbyJoin,
    LobbyClosed,
    LobbyLeave,
    LobbyKick,
    AccountUpdated,
}

impl WebSocketMessageType {
    pub fn to_string(&self) -> &'static str {
        match self {
            WebSocketMessageType::InvalidMessage => "invalidMessage",
            WebSocketMessageType::GameStarted => "gameStarted",
            WebSocketMessageType::UpdateGameData => "updateGameData",
            WebSocketMessageType::ClientDisconnected => "clientDisconnected",
            WebSocketMessageType::ClientReconnected => "clientReconnected",
            WebSocketMessageType::IncomingChatMessage => "incomingChatMessage",
            WebSocketMessageType::IncomingInvite => "incomingInvite",
            WebSocketMessageType::IncomingFriendRequest => "incomingFriendRequest",
            WebSocketMessageType::FriendshipChanged => "friendshipChanged",
            WebSocketMessageType::LobbyJoin => "lobbyJoin",
            WebSocketMessageType::LobbyClosed => "lobbyClosed",
            WebSocketMessageType::LobbyLeave => "lobbyLeave",
            WebSocketMessageType::LobbyKick => "lobbyKick",
            WebSocketMessageType::AccountUpdated => "accountUpdated",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "invalidMessage" => Some(WebSocketMessageType::InvalidMessage),
            "gameStarted" => Some(WebSocketMessageType::GameStarted),
            "updateGameData" => Some(WebSocketMessageType::UpdateGameData),
            "clientDisconnected" => Some(WebSocketMessageType::ClientDisconnected),
            "clientReconnected" => Some(WebSocketMessageType::ClientReconnected),
            "incomingChatMessage" => Some(WebSocketMessageType::IncomingChatMessage),
            "incomingInvite" => Some(WebSocketMessageType::IncomingInvite),
            "incomingFriendRequest" => Some(WebSocketMessageType::IncomingFriendRequest),
            "friendshipChanged" => Some(WebSocketMessageType::FriendshipChanged),
            "lobbyJoin" => Some(WebSocketMessageType::LobbyJoin),
            "lobbyClosed" => Some(WebSocketMessageType::LobbyClosed),
            "lobbyLeave" => Some(WebSocketMessageType::LobbyLeave),
            "lobbyKick" => Some(WebSocketMessageType::LobbyKick),
            "accountUpdated" => Some(WebSocketMessageType::AccountUpdated),
            _ => None,
        }
    }
}