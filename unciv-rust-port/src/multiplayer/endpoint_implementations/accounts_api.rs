use std::sync::Arc;
use reqwest::{Client, Method, Request};
use serde::{Serialize, Deserialize};
use log::debug;
use uuid::Uuid;

use crate::models::multiplayer::account_response::AccountResponse;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;

use super::auth_helper::AuthHelper;
use super::endpoint_implementations::{request, get_default_retry, CACHE};

/// API wrapper for account handling (do not use directly; use the Api class instead)
pub struct AccountsApi {
    client: Arc<Client>,
    auth_helper: Arc<AuthHelper>,
}

impl AccountsApi {
    /// Creates a new AccountsApi instance
    pub fn new(client: Client, auth_helper: AuthHelper) -> Self {
        Self {
            client: Arc::new(client),
            auth_helper: Arc::new(auth_helper),
        }
    }

    /// Retrieve information about the currently logged in user
    ///
    /// Unset cache to avoid using the cache and update the data from the server.
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise AccountResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn get(&self, cache: bool, suppress: bool) -> Result<Option<AccountResponse>, UncivNetworkException> {
        let mut cache_lock = CACHE.lock().unwrap();
        let response = cache_lock.get(
            "api/v2/accounts/me",
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            cache,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<AccountResponse>().await?))
        } else {
            Ok(None)
        }
    }

    /// Retrieve details for an account by its uuid (always preferred to using usernames)
    ///
    /// Unset cache to avoid using the cache and update the data from the server.
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise AccountResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn lookup_by_uuid(&self, uuid: Uuid, cache: bool, suppress: bool) -> Result<Option<AccountResponse>, UncivNetworkException> {
        let mut cache_lock = CACHE.lock().unwrap();
        let response = cache_lock.get(
            &format!("api/v2/accounts/{}", uuid),
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            cache,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<AccountResponse>().await?))
        } else {
            Ok(None)
        }
    }

    /// Retrieve details for an account by its username
    ///
    /// Important note: Usernames can be changed, so don't assume they can be
    /// cached to do lookups for their display names or UUIDs later. Always convert usernames
    /// to UUIDs when handling any user interactions (e.g., inviting, sending messages, ...).
    ///
    /// Use suppress to forbid throwing *any* errors (returns None, otherwise AccountResponse or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn lookup_by_username(&self, username: &str, suppress: bool) -> Result<Option<AccountResponse>, UncivNetworkException> {
        #[derive(Serialize)]
        struct LookupAccountUsernameRequest {
            username: String,
        }

        let response = request(
            Method::POST,
            "api/v2/accounts/lookup",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&LookupAccountUsernameRequest {
                    username: username.to_string(),
                });
            }),
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            Ok(Some(resp.json::<AccountResponse>().await?))
        } else {
            Ok(None)
        }
    }

    /// Set the username of the currently logged-in user
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn set_username(&self, username: &str, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct UpdateAccountRequest {
            username: Option<String>,
            display_name: Option<String>,
        }

        self.update(UpdateAccountRequest {
            username: Some(username.to_string()),
            display_name: None,
        }, suppress).await
    }

    /// Set the display_name of the currently logged-in user
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn set_display_name(&self, display_name: &str, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct UpdateAccountRequest {
            username: Option<String>,
            display_name: Option<String>,
        }

        self.update(UpdateAccountRequest {
            username: None,
            display_name: Some(display_name.to_string()),
        }, suppress).await
    }

    /// Update the currently logged in user information
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    async fn update<T: Serialize>(&self, r: T, suppress: bool) -> Result<bool, UncivNetworkException> {
        let response = request(
            Method::PUT,
            "api/v2/accounts/me",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&r);
            }),
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }

    /// Deletes the currently logged-in account (irreversible operation!)
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn delete(&self, suppress: bool) -> Result<bool, UncivNetworkException> {
        let response = request(
            Method::DELETE,
            "api/v2/accounts/me",
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }

    /// Set new_password for the currently logged-in account, provided the old_password was accepted as valid
    ///
    /// If not given, the old_password will be used from the login session cache, if available.
    /// However, if the old_password can't be determined, it will likely yield in a ApiStatusCode::InvalidPassword.
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn set_password(&self, new_password: &str, old_password: Option<&str>, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct SetPasswordRequest {
            old_password: String,
            new_password: String,
        }

        let mut old_local_password = old_password.unwrap_or("");
        let last_known_password = self.auth_helper.get_last_successful_credentials().map(|(_, p)| p);

        if old_local_password.is_empty() && last_known_password.is_some() {
            old_local_password = last_known_password.unwrap();
        }

        let response = request(
            Method::POST,
            "api/v2/accounts/setPassword",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&SetPasswordRequest {
                    old_password: old_local_password.to_string(),
                    new_password: new_password.to_string(),
                });
            }),
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await?;

        if let Some(resp) = response {
            if resp.status().is_success() {
                debug!("User's password has been changed successfully");
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Register a new user account
    ///
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn register(&self, username: &str, display_name: &str, password: &str, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct AccountRegistrationRequest {
            username: String,
            display_name: String,
            password: String,
        }

        let response = request(
            Method::POST,
            "api/v2/accounts/register",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&AccountRegistrationRequest {
                    username: username.to_string(),
                    display_name: display_name.to_string(),
                    password: password.to_string(),
                });
            }),
            suppress,
            None,
        ).await?;

        if let Some(resp) = response {
            if resp.status().is_success() {
                debug!("A new account for username '{}' has been created", username);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

impl Clone for AccountsApi {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            auth_helper: self.auth_helper.clone(),
        }
    }
}