use std::sync::Arc;
use reqwest::{Client, Method, Request};
use serde::{Serialize, Deserialize};
use log::debug;
use uuid::Uuid;

use crate::models::multiplayer::chat_message::ChatMessage;
use crate::models::multiplayer::get_all_chats_response::GetAllChatsResponse;
use crate::models::multiplayer::get_chat_response::GetChatResponse;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;

use super::auth_helper::AuthHelper;
use super::endpoint_implementations::{request, get_default_retry, CACHE};

/// API wrapper for chat room handling (do not use directly; use the Api class instead)
pub struct ChatApi {
    client: Arc<Client>,
    auth_helper: Arc<AuthHelper>,
}

impl ChatApi {
    /// Creates a new ChatApi instance
    pub fn new(client: Client, auth_helper: AuthHelper) -> Self {
        Self {
            client: Arc::new(client),
            auth_helper: Arc::new(auth_helper),
        }
    }

    /// Retrieve all chats a user has access to
    ///
    /// In the response, you will find different room types / room categories.
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise GetAllChatsResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn list(&self, suppress: bool) -> Result<Option<GetAllChatsResponse>, UncivNetworkException> {
        let response = request(
            Method::GET,
            "api/v2/chats",
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<GetAllChatsResponse>().await?))
        } else {
            Ok(None)
        }
    }

    /// Retrieve the messages of a chatroom identified by room_uuid
    ///
    /// The ChatMessages should be sorted by their timestamps, ChatMessage.created_at.
    /// The ChatMessage.uuid should be used to uniquely identify chat messages. This is
    /// needed as new messages may be delivered via WebSocket as well. GetChatResponse.members
    /// holds information about all members that are currently in the chat room (including yourself).
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise GetChatResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn get(&self, room_uuid: Uuid, suppress: bool) -> Result<Option<GetChatResponse>, UncivNetworkException> {
        let response = request(
            Method::GET,
            &format!("api/v2/chats/{}", room_uuid),
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<GetChatResponse>().await?))
        } else {
            Ok(None)
        }
    }

    /// Send a message to a chat room
    ///
    /// The executing user must be a member of the chatroom and the message must not be empty.
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise ChatMessage or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn send(&self, message: &str, chat_room_uuid: Uuid, suppress: bool) -> Result<Option<ChatMessage>, UncivNetworkException> {
        #[derive(Serialize)]
        struct SendMessageRequest {
            message: String,
        }

        let response = request(
            Method::POST,
            &format!("api/v2/chats/{}", chat_room_uuid),
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&SendMessageRequest {
                    message: message.to_string(),
                });
            }),
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<ChatMessage>().await?))
        } else {
            Ok(None)
        }
    }
}

impl Clone for ChatApi {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            auth_helper: self.auth_helper.clone(),
        }
    }
}