use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Collection of API request structs in a single file for simplicity

/// The content to register a new account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRegistrationRequest {
    pub username: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    pub password: String,
}

/// The request of a new friendship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFriendRequest {
    #[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_uuid")]
    #[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_uuid")]
    pub uuid: Uuid,
}

/// The request to invite a friend into a lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    #[serde(rename = "friend_uuid")]
    #[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_uuid")]
    #[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_uuid")]
    pub friend_uuid: Uuid,
    #[serde(rename = "lobby_uuid")]
    #[serde(serialize_with = "crate::multiplayer::json_serializers::serialize_uuid")]
    #[serde(deserialize_with = "crate::multiplayer::json_serializers::deserialize_uuid")]
    pub lobby_uuid: Uuid,
}

/// The parameters to create a lobby
///
/// The parameter `max_players` must be greater or equals 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLobbyRequest {
    pub name: String,
    pub password: Option<String>,
    #[serde(rename = "max_players")]
    pub max_players: i32,
}

/// The request a user sends to the server to upload a new game state (non-WebSocket API)
///
/// The game's UUID has to be set via the path argument of the endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUploadRequest {
    #[serde(rename = "game_data")]
    pub game_data: String,
}

/// The request to join a lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinLobbyRequest {
    pub password: Option<String>,
}

/// The request data of a login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// The request to lookup an account by its username
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupAccountUsernameRequest {
    pub username: String,
}

/// The request for sending a message to a chatroom
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
}

/// The set password request data
///
/// The parameter `new_password` must not be empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPasswordRequest {
    #[serde(rename = "old_password")]
    pub old_password: String,
    #[serde(rename = "new_password")]
    pub new_password: String,
}

/// Update account request data
///
/// All parameters are optional, but at least one of them is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccountRequest {
    pub username: Option<String>,
    #[serde(rename = "display_name")]
    pub display_name: Option<String>,
}