use std::sync::Arc;
use reqwest::{Client, Method, Request};
use serde::{Serialize, Deserialize};
use log::debug;
use uuid::Uuid;

use crate::models::multiplayer::friend_request_response::FriendRequestResponse;
use crate::models::multiplayer::friend_response::FriendResponse;
use crate::models::multiplayer::get_friend_response::GetFriendResponse;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;

use super::auth_helper::AuthHelper;
use super::endpoint_implementations::{request, get_default_retry};

/// API wrapper for friend handling (do not use directly; use the Api class instead)
pub struct FriendApi {
    client: Arc<Client>,
    auth_helper: Arc<AuthHelper>,
}

impl FriendApi {
    /// Creates a new FriendApi instance
    pub fn new(client: Client, auth_helper: AuthHelper) -> Self {
        Self {
            client: Arc::new(client),
            auth_helper: Arc::new(auth_helper),
        }
    }

    /// Retrieve a pair of the list of your established friendships and the list of your open friendship requests (incoming and outgoing)
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise a pair of lists or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn list(&self, suppress: bool) -> Result<Option<(Vec<FriendResponse>, Vec<FriendRequestResponse>)>, UncivNetworkException> {
        let response = request(
            Method::GET,
            "api/v2/friends",
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            let body: GetFriendResponse = resp.json().await?;
            Ok(Some((body.friends, body.friend_requests)))
        } else {
            Ok(None)
        }
    }

    /// Retrieve a list of your established friendships
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise a list of FriendResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn list_friends(&self, suppress: bool) -> Result<Option<Vec<FriendResponse>>, UncivNetworkException> {
        let result = self.list(suppress).await?;
        Ok(result.map(|(friends, _)| friends))
    }

    /// Retrieve a list of your open friendship requests (incoming and outgoing)
    ///
    /// If you have a request with FriendRequestResponse.from equal to your username, it means
    /// you have requested a friendship, but the destination hasn't accepted yet. In the other
    /// case, if your username is in FriendRequestResponse.to, you have received a friend request.
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise a list of FriendRequestResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn list_requests(&self, suppress: bool) -> Result<Option<Vec<FriendRequestResponse>>, UncivNetworkException> {
        let result = self.list(suppress).await?;
        Ok(result.map(|(_, requests)| requests))
    }

    /// Request friendship with another user
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn request(&self, other: Uuid, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct CreateFriendRequest {
            other: Uuid,
        }

        let response = request(
            Method::POST,
            "api/v2/friends",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&CreateFriendRequest {
                    other,
                });
            }),
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }

    /// Accept a friend request identified by friend_request_uuid
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn accept(&self, friend_request_uuid: Uuid, suppress: bool) -> Result<bool, UncivNetworkException> {
        let response = request(
            Method::PUT,
            &format!("api/v2/friends/{}", friend_request_uuid),
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }

    /// Don't want your friends anymore? Just delete them!
    ///
    /// This function accepts both friend UUIDs and friendship request UUIDs.
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn delete(&self, friend_uuid: Uuid, suppress: bool) -> Result<bool, UncivNetworkException> {
        let response = request(
            Method::DELETE,
            &format!("api/v2/friends/{}", friend_uuid),
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }
}

impl Clone for FriendApi {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            auth_helper: self.auth_helper.clone(),
        }
    }
}