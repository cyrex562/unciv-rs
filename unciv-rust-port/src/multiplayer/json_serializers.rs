use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::models::multiplayer::api_status_code::ApiStatusCode;
use crate::models::multiplayer::websocket_message::WebSocketMessage;
use crate::models::multiplayer::websocket_message_type::WebSocketMessageType;
use crate::models::multiplayer::friendship_event::FriendshipEvent;

/// Serializer for the ApiStatusCode enum to make encoding/decoding as integer work
pub fn serialize_api_status_code<S>(status_code: &ApiStatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i32(status_code.value())
}

/// Deserializer for the ApiStatusCode enum from integer
pub fn deserialize_api_status_code<'de, D>(deserializer: D) -> Result<ApiStatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;
    Ok(ApiStatusCode::from_value(value))
}

/// Serializer for instants (date times) from/to strings in ISO 8601 format
pub fn serialize_instant<S>(instant: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let duration = instant.duration_since(UNIX_EPOCH)
        .map_err(serde::ser::Error::custom)?;
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let iso_string = format!("{}.{:09}Z", secs, nanos);
    serializer.serialize_str(&iso_string)
}

/// Deserializer for instants from ISO 8601 format strings
pub fn deserialize_instant<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // Parse ISO 8601 format
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return Err(serde::de::Error::custom("Invalid ISO 8601 format"));
    }

    let secs = parts[0].parse::<u64>()
        .map_err(serde::de::Error::custom)?;

    let nanos_str = parts[1].trim_end_matches('Z');
    let nanos = nanos_str.parse::<u32>()
        .map_err(serde::de::Error::custom)?;

    Ok(UNIX_EPOCH + Duration::new(secs, nanos))
}

/// Serializer for UUIDs from/to strings
pub fn serialize_uuid<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&uuid.to_string())
}

/// Deserializer for UUIDs from strings
pub fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Uuid::parse_str(&s).map_err(serde::de::Error::custom)
}

/// Serializer for WebSocket messages that also differentiate by type
pub fn serialize_websocket_message<S>(message: &WebSocketMessage, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Create a JSON object with the message type and content
    let mut obj = serde_json::Map::new();
    obj.insert("type".to_string(), json!(message.message_type().to_string()));

    // Add the message content based on its type
    match message {
        WebSocketMessage::InvalidMessage(msg) => {
            obj.insert("content".to_string(), json!(msg));
        },
        WebSocketMessage::GameStarted(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::UpdateGameData(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::ClientDisconnected(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::ClientReconnected(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::IncomingChatMessage(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::IncomingInvite(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::IncomingFriendRequest(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::FriendshipChanged(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::LobbyJoin(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::LobbyClosed(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::LobbyLeave(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::LobbyKick(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
        WebSocketMessage::AccountUpdated(msg) => {
            obj.insert("content".to_string(), serde_json::to_value(msg)?);
        },
    }

    Value::Object(obj).serialize(serializer)
}

/// Deserializer for WebSocket messages
pub fn deserialize_websocket_message<'de, D>(deserializer: D) -> Result<WebSocketMessage, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let obj = value.as_object().ok_or_else(|| {
        serde::de::Error::custom("Expected a JSON object")
    })?;

    // Get the message type
    let type_str = obj.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            serde::de::Error::custom("Missing 'type' field")
        })?;

    // Get the content
    let content = obj.get("content")
        .ok_or_else(|| {
            serde::de::Error::custom("Missing 'content' field")
        })?;

    // Parse based on the message type
    match WebSocketMessageType::from_str(type_str) {
        WebSocketMessageType::InvalidMessage => {
            let msg = content.as_str()
                .ok_or_else(|| serde::de::Error::custom("Expected string content for InvalidMessage"))?;
            Ok(WebSocketMessage::InvalidMessage(msg.to_string()))
        },
        WebSocketMessageType::GameStarted => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::GameStarted(msg))
        },
        WebSocketMessageType::UpdateGameData => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::UpdateGameData(msg))
        },
        WebSocketMessageType::ClientDisconnected => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::ClientDisconnected(msg))
        },
        WebSocketMessageType::ClientReconnected => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::ClientReconnected(msg))
        },
        WebSocketMessageType::IncomingChatMessage => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::IncomingChatMessage(msg))
        },
        WebSocketMessageType::IncomingInvite => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::IncomingInvite(msg))
        },
        WebSocketMessageType::IncomingFriendRequest => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::IncomingFriendRequest(msg))
        },
        WebSocketMessageType::FriendshipChanged => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::FriendshipChanged(msg))
        },
        WebSocketMessageType::LobbyJoin => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::LobbyJoin(msg))
        },
        WebSocketMessageType::LobbyClosed => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::LobbyClosed(msg))
        },
        WebSocketMessageType::LobbyLeave => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::LobbyLeave(msg))
        },
        WebSocketMessageType::LobbyKick => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::LobbyKick(msg))
        },
        WebSocketMessageType::AccountUpdated => {
            let msg = serde_json::from_value(content.clone())?;
            Ok(WebSocketMessage::AccountUpdated(msg))
        },
    }
}

/// Serializer for the WebSocket message type enum to make encoding/decoding as string work
pub fn serialize_websocket_message_type<S>(message_type: &WebSocketMessageType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&message_type.to_string())
}

/// Deserializer for the WebSocket message type enum from string
pub fn deserialize_websocket_message_type<'de, D>(deserializer: D) -> Result<WebSocketMessageType, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    WebSocketMessageType::from_str(&s).map_err(serde::de::Error::custom)
}

/// Serializer for the FriendshipEvent WebSocket message enum to make encoding/decoding as string work
pub fn serialize_friendship_event<S>(event: &FriendshipEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&event.to_string())
}

/// Deserializer for the FriendshipEvent WebSocket message enum from string
pub fn deserialize_friendship_event<'de, D>(deserializer: D) -> Result<FriendshipEvent, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    FriendshipEvent::from_str(&s).map_err(serde::de::Error::custom)
}