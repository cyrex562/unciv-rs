use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::multiplayer::json_serializers::{serialize_uuid, deserialize_uuid, serialize_instant, deserialize_instant};

/// Collection of API response structs in a single file for simplicity

/// The account data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountResponse {
    pub username: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
}

/// The Response that is returned in case of an error
///
/// For client errors the HTTP status code will be 400, for server errors the 500 will be used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub message: String,
    #[serde(rename = "status_code")]
    #[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_api_status_code")]
    #[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_api_status_code")]
    pub status_code: ApiStatusCode,
}

impl ApiErrorResponse {
    /// Convert the ApiErrorResponse to an ApiException for throwing and showing to users
    pub fn to_exception(&self) -> ApiException {
        ApiException::new(self.clone())
    }
}

/// API status code enum for mapping integer codes to names
///
/// The status code represents a unique identifier for an error.
/// Error codes in the range of 1000..2000 represent client errors that could be handled
/// by the client. Error codes in the range of 2000..3000 represent server errors.
/// The message is a user-showable default string for every possible status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_api_status_code")]
#[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_api_status_code")]
pub enum ApiStatusCode {
    Unauthenticated,
    NotFound,
    InvalidContentType,
    InvalidJson,
    PayloadOverflow,
    LoginFailed,
    UsernameAlreadyOccupied,
    InvalidPassword,
    EmptyJson,
    InvalidUsername,
    InvalidDisplayName,
    FriendshipAlreadyRequested,
    AlreadyFriends,
    MissingPrivileges,
    InvalidMaxPlayersCount,
    AlreadyInALobby,
    InvalidUuid,
    InvalidLobbyUuid,
    InvalidFriendUuid,
    GameNotFound,
    InvalidMessage,
    WsNotConnected,
    LobbyFull,
    InvalidPlayerUUID,
    InternalServerError,
    DatabaseError,
    SessionError,
}

impl ApiStatusCode {
    pub fn value(&self) -> i32 {
        match self {
            ApiStatusCode::Unauthenticated => 1000,
            ApiStatusCode::NotFound => 1001,
            ApiStatusCode::InvalidContentType => 1002,
            ApiStatusCode::InvalidJson => 1003,
            ApiStatusCode::PayloadOverflow => 1004,
            ApiStatusCode::LoginFailed => 1005,
            ApiStatusCode::UsernameAlreadyOccupied => 1006,
            ApiStatusCode::InvalidPassword => 1007,
            ApiStatusCode::EmptyJson => 1008,
            ApiStatusCode::InvalidUsername => 1009,
            ApiStatusCode::InvalidDisplayName => 1010,
            ApiStatusCode::FriendshipAlreadyRequested => 1011,
            ApiStatusCode::AlreadyFriends => 1012,
            ApiStatusCode::MissingPrivileges => 1013,
            ApiStatusCode::InvalidMaxPlayersCount => 1014,
            ApiStatusCode::AlreadyInALobby => 1015,
            ApiStatusCode::InvalidUuid => 1016,
            ApiStatusCode::InvalidLobbyUuid => 1017,
            ApiStatusCode::InvalidFriendUuid => 1018,
            ApiStatusCode::GameNotFound => 1019,
            ApiStatusCode::InvalidMessage => 1020,
            ApiStatusCode::WsNotConnected => 1021,
            ApiStatusCode::LobbyFull => 1022,
            ApiStatusCode::InvalidPlayerUUID => 1023,
            ApiStatusCode::InternalServerError => 2000,
            ApiStatusCode::DatabaseError => 2001,
            ApiStatusCode::SessionError => 2002,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            ApiStatusCode::Unauthenticated => "You are not logged in. Please login first.",
            ApiStatusCode::NotFound => "The operation couldn't be completed, since the resource was not found.",
            ApiStatusCode::InvalidContentType => "The media content type was invalid. Please report this as a bug.",
            ApiStatusCode::InvalidJson => "The server didn't understand the sent data. Please report this as a bug.",
            ApiStatusCode::PayloadOverflow => "The amount of data sent to the server was too large. Please report this as a bug.",
            ApiStatusCode::LoginFailed => "The login failed. Is the username and password correct?",
            ApiStatusCode::UsernameAlreadyOccupied => "The selected username is already taken. Please choose another name.",
            ApiStatusCode::InvalidPassword => "This password is not valid. Please choose another password.",
            ApiStatusCode::EmptyJson => "The server encountered an empty JSON problem. Please report this as a bug.",
            ApiStatusCode::InvalidUsername => "The username is not valid. Please choose another one.",
            ApiStatusCode::InvalidDisplayName => "The display name is not valid. Please choose another one.",
            ApiStatusCode::FriendshipAlreadyRequested => "You have already requested friendship with this player. Please wait until the request is accepted.",
            ApiStatusCode::AlreadyFriends => "You are already friends, you can't request it again.",
            ApiStatusCode::MissingPrivileges => "You don't have the required privileges to perform this operation.",
            ApiStatusCode::InvalidMaxPlayersCount => "The maximum number of players for this lobby is out of the supported range for this server. Please adjust the number. Two players should always work.",
            ApiStatusCode::AlreadyInALobby => "You are already in another lobby. You need to close or leave the other lobby before.",
            ApiStatusCode::InvalidUuid => "The operation could not be completed, since an invalid UUID was given. Please retry later or restart the game. If the problem persists, please report this as a bug.",
            ApiStatusCode::InvalidLobbyUuid => "The lobby was not found. Maybe it has already been closed?",
            ApiStatusCode::InvalidFriendUuid => "You must be friends with the other player before this action can be completed. Try again later.",
            ApiStatusCode::GameNotFound => "The game was not found on the server. Try again later. If the problem persists, the game was probably already removed from the server, sorry.",
            ApiStatusCode::InvalidMessage => "This message could not be sent, since it was invalid. Remove any invalid characters and try again.",
            ApiStatusCode::WsNotConnected => "The WebSocket is not available. Please restart the game and try again. If the problem persists, please report this as a bug.",
            ApiStatusCode::LobbyFull => "The lobby is currently full. You can't join right now.",
            ApiStatusCode::InvalidPlayerUUID => "The ID of the player was invalid. Does the player exist? Please try again. If the problem persists, please report this as a bug.",
            ApiStatusCode::InternalServerError => "Internal server error. Please report this as a bug.",
            ApiStatusCode::DatabaseError => "Internal server database error. Please report this as a bug.",
            ApiStatusCode::SessionError => "Internal session error. Please report this as a bug.",
        }
    }

    pub fn from_value(value: i32) -> Option<Self> {
        match value {
            1000 => Some(ApiStatusCode::Unauthenticated),
            1001 => Some(ApiStatusCode::NotFound),
            1002 => Some(ApiStatusCode::InvalidContentType),
            1003 => Some(ApiStatusCode::InvalidJson),
            1004 => Some(ApiStatusCode::PayloadOverflow),
            1005 => Some(ApiStatusCode::LoginFailed),
            1006 => Some(ApiStatusCode::UsernameAlreadyOccupied),
            1007 => Some(ApiStatusCode::InvalidPassword),
            1008 => Some(ApiStatusCode::EmptyJson),
            1009 => Some(ApiStatusCode::InvalidUsername),
            1010 => Some(ApiStatusCode::InvalidDisplayName),
            1011 => Some(ApiStatusCode::FriendshipAlreadyRequested),
            1012 => Some(ApiStatusCode::AlreadyFriends),
            1013 => Some(ApiStatusCode::MissingPrivileges),
            1014 => Some(ApiStatusCode::InvalidMaxPlayersCount),
            1015 => Some(ApiStatusCode::AlreadyInALobby),
            1016 => Some(ApiStatusCode::InvalidUuid),
            1017 => Some(ApiStatusCode::InvalidLobbyUuid),
            1018 => Some(ApiStatusCode::InvalidFriendUuid),
            1019 => Some(ApiStatusCode::GameNotFound),
            1020 => Some(ApiStatusCode::InvalidMessage),
            1021 => Some(ApiStatusCode::WsNotConnected),
            1022 => Some(ApiStatusCode::LobbyFull),
            1023 => Some(ApiStatusCode::InvalidPlayerUUID),
            2000 => Some(ApiStatusCode::InternalServerError),
            2001 => Some(ApiStatusCode::DatabaseError),
            2002 => Some(ApiStatusCode::SessionError),
            _ => None,
        }
    }
}

/// A member of a chatroom
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMember {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub username: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(rename = "joined_at")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub joined_at: SystemTime,
}

/// The message of a chatroom
///
/// The parameter `uuid` should be used to uniquely identify a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub sender: AccountResponse,
    pub message: String,
    #[serde(rename = "created_at")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub created_at: SystemTime,
}

/// The small representation of a chatroom
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSmall {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    #[serde(rename = "last_message_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub last_message_uuid: Option<Uuid>,
}

/// The response of a create lobby request, which contains the `lobby_uuid` and `lobby_chat_room_uuid`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLobbyResponse {
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
    #[serde(rename = "lobby_chat_room_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_chat_room_uuid: Uuid,
}

/// A single friend (the relationship is identified by the `uuid`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendResponse {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    #[serde(rename = "chat_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_uuid: Uuid,
    pub friend: OnlineAccountResponse,
}

/// A single friend request
///
/// Use `from` and `to` comparing with "myself" to determine if it's incoming or outgoing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestResponse {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub from: AccountResponse,
    pub to: AccountResponse,
}

/// A shortened game state identified by its ID and state identifier
///
/// If the state (`game_data_id`) of a known game differs from the last known
/// identifier, the server has a newer state of the game. The `last_activity`
/// field is a convenience attribute and shouldn't be used for update checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOverviewResponse {
    #[serde(rename = "chat_room_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_room_uuid: Uuid,
    #[serde(rename = "game_data_id")]
    pub game_data_id: i64,
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
    #[serde(rename = "last_activity")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub last_activity: SystemTime,
    #[serde(rename = "last_player")]
    pub last_player: AccountResponse,
    #[serde(rename = "max_players")]
    pub max_players: i32,
    pub name: String,
}

/// A single game state identified by its ID and state identifier; see `game_data`
///
/// If the state (`game_data_id`) of a known game differs from the last known
/// identifier, the server has a newer state of the game. The `last_activity`
/// field is a convenience attribute and shouldn't be used for update checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateResponse {
    #[serde(rename = "chat_room_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_room_uuid: Uuid,
    #[serde(rename = "game_data")]
    pub game_data: String,
    #[serde(rename = "game_data_id")]
    pub game_data_id: i64,
    #[serde(rename = "last_activity")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub last_activity: SystemTime,
    #[serde(rename = "last_player")]
    pub last_player: AccountResponse,
    #[serde(rename = "max_players")]
    pub max_players: i32,
    pub name: String,
}

/// The response a user receives after uploading a new game state successfully
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUploadResponse {
    #[serde(rename = "game_data_id")]
    pub game_data_id: i64,
}

/// All chat rooms your user has access to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAllChatsResponse {
    #[serde(rename = "friend_chat_rooms")]
    pub friend_chat_rooms: Vec<ChatSmall>,
    #[serde(rename = "game_chat_rooms")]
    pub game_chat_rooms: Vec<ChatSmall>,
    #[serde(rename = "lobby_chat_rooms")]
    pub lobby_chat_rooms: Vec<ChatSmall>,
}

/// The response to a get chat
///
/// `messages` should be sorted by the datetime of message.created_at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetChatResponse {
    pub members: Vec<ChatMember>,
    pub messages: Vec<ChatMessage>,
}

/// A list of your friends and friend requests
///
/// `friends` is a list of already established friendships
/// `friend_requests` is a list of friend requests (incoming and outgoing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFriendResponse {
    pub friends: Vec<FriendResponse>,
    #[serde(rename = "friend_requests")]
    pub friend_requests: Vec<FriendRequestResponse>,
}

/// An overview of games a player participates in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameOverviewResponse {
    pub games: Vec<GameOverviewResponse>,
}

/// A single invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvite {
    #[serde(rename = "created_at")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub created_at: SystemTime,
    pub from: AccountResponse,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub lobby_uuid: Uuid,
}

/// The invites that an account has received
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvitesResponse {
    pub invites: Vec<GetInvite>,
}

/// The lobbies that are open
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLobbiesResponse {
    pub lobbies: Vec<LobbyResponse>,
}

/// A single lobby (in contrast to `LobbyResponse`, this is fetched by its own)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLobbyResponse {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub name: String,
    #[serde(rename = "max_players")]
    pub max_players: i32,
    #[serde(rename = "current_players")]
    pub current_players: Vec<AccountResponse>,
    #[serde(rename = "chat_room_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_room_uuid: Uuid,
    #[serde(rename = "created_at")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub created_at: SystemTime,
    #[serde(rename = "password")]
    pub has_password: bool,
    pub owner: AccountResponse,
}

/// A single lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyResponse {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub name: String,
    #[serde(rename = "max_players")]
    pub max_players: i32,
    #[serde(rename = "current_players")]
    pub current_players: i32,
    #[serde(rename = "chat_room_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub chat_room_uuid: Uuid,
    #[serde(rename = "created_at")]
    #[serde(serialize_with = "serialize_instant")]
    #[serde(deserialize_with = "deserialize_instant")]
    pub created_at: SystemTime,
    #[serde(rename = "password")]
    pub has_password: bool,
    pub owner: AccountResponse,
}

/// The account data
///
/// It provides the extra field `online` indicating whether the account has any connected client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineAccountResponse {
    pub online: bool,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub uuid: Uuid,
    pub username: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
}

impl OnlineAccountResponse {
    pub fn to_account_response(&self) -> AccountResponse {
        AccountResponse {
            uuid: self.uuid,
            username: self.username.clone(),
            display_name: self.display_name.clone(),
        }
    }
}

/// The response when starting a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGameResponse {
    #[serde(rename = "game_chat_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_chat_uuid: Uuid,
    #[serde(rename = "game_uuid")]
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    pub game_uuid: Uuid,
}

/// The version data for clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: i32,
}

/// Exception thrown when an API error occurs
#[derive(Debug, Clone)]
pub struct ApiException {
    pub error: ApiErrorResponse,
}

impl ApiException {
    pub fn new(error: ApiErrorResponse) -> Self {
        Self { error }
    }
}

impl std::fmt::Display for ApiException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "API Error: {} (Code: {})", self.error.message, self.error.status_code.value())
    }
}

impl std::error::Error for ApiException {}