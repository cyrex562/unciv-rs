use std::sync::Arc;
use reqwest::{Client, Method, Request};
use serde::{Serialize, Deserialize};
use log::{debug, error};
use uuid::Uuid;

use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;

use super::auth_helper::AuthHelper;
use super::conf::SESSION_COOKIE_NAME;
use super::endpoint_implementations::{request, get_default_retry, CACHE};

/// API wrapper for authentication handling (do not use directly; use the Api class instead)
pub struct AuthApi {
    client: Arc<Client>,
    auth_helper: Arc<AuthHelper>,
    after_login: Arc<dyn Fn() -> Result<(), UncivNetworkException> + Send + Sync>,
    after_logout: Arc<dyn Fn(bool) -> Result<(), UncivNetworkException> + Send + Sync>,
}

impl AuthApi {
    /// Creates a new AuthApi instance
    pub fn new(
        client: Client,
        auth_helper: AuthHelper,
        after_login: impl Fn() -> Result<(), UncivNetworkException> + Send + Sync + 'static,
        after_logout: impl Fn(bool) -> Result<(), UncivNetworkException> + Send + Sync + 'static,
    ) -> Self {
        Self {
            client: Arc::new(client),
            auth_helper: Arc::new(auth_helper),
            after_login: Arc::new(after_login),
            after_logout: Arc::new(after_logout),
        }
    }

    /// Try logging in with username and password for testing purposes, don't set the session cookie
    ///
    /// This method won't raise *any* exception, just return the boolean value if login worked.
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn login_only(&self, username: &str, password: &str) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let response = request(
            Method::POST,
            "api/v2/auth/login",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&LoginRequest {
                    username: username.to_string(),
                    password: password.to_string(),
                });
            }),
            true,
            None,
        ).await?;

        Ok(response.map_or(false, |r| r.status().is_success()))
    }

    /// Try logging in with username and password to get a new session
    ///
    /// This method will also implicitly set a cookie in the in-memory cookie storage to authenticate
    /// further API calls and cache the username and password to refresh expired sessions.
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn login(&self, username: &str, password: &str, suppress: bool) -> Result<bool, UncivNetworkException> {
        #[derive(Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let response = request(
            Method::POST,
            "api/v2/auth/login",
            &self.client,
            &self.auth_helper,
            Some(&|req: &mut Request| {
                req.header("Content-Type", "application/json");
                req.json(&LoginRequest {
                    username: username.to_string(),
                    password: password.to_string(),
                });
            }),
            suppress,
            Some(&|| {
                error!("Failed to login. See previous debug logs for details.");
                false
            }),
        ).await?;

        if let Some(resp) = response {
            if resp.status().is_success() {
                // In a real implementation, we would extract the cookie from the response
                // For this example, we'll just simulate it
                let cookie_value = "session_cookie_value".to_string();
                let max_age = Some(3600);

                debug!("Received new session cookie: {}", cookie_value);

                self.auth_helper.set_cookie(
                    cookie_value,
                    max_age,
                    Some((username.to_string(), password.to_string())),
                );

                (self.after_login)()?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Logs out the currently logged in user
    ///
    /// This method will also clear the cookie and credentials to avoid further authenticated API calls.
    /// Use suppress to forbid throwing *any* errors (returns false, otherwise true or an error).
    ///
    /// # Errors
    ///
    /// * `ApiException`: thrown for defined and recognized API problems
    /// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
    pub async fn logout(&self, suppress: bool) -> Result<bool, UncivNetworkException> {
        let response = match request(
            Method::GET,
            "api/v2/auth/logout",
            &self.client,
            &self.auth_helper,
            None,
            suppress,
            Some(&get_default_retry(&self.client, &self.auth_helper)),
        ).await {
            Ok(resp) => resp,
            Err(e) => {
                self.auth_helper.unset();
                CACHE.lock().unwrap().clear();
                debug!("Logout failed due to {} ({}), dropped session anyways", e, e.to_string());
                (self.after_logout)(false)?;
                return Ok(false);
            }
        };

        CACHE.lock().unwrap().clear();

        if let Some(resp) = response {
            if resp.status().is_success() {
                self.auth_helper.unset();
                debug!("Logged out successfully, dropped session");
                (self.after_logout)(true)?;
                Ok(true)
            } else {
                self.auth_helper.unset();
                debug!("Logout failed for some reason, dropped session anyways");
                (self.after_logout)(false)?;
                Ok(false)
            }
        } else {
            self.auth_helper.unset();
            debug!("Logout failed for some reason, dropped session anyways");
            (self.after_logout)(false)?;
            Ok(false)
        }
    }
}

impl Clone for AuthApi {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            auth_helper: self.auth_helper.clone(),
            after_login: self.after_login.clone(),
            after_logout: self.after_logout.clone(),
        }
    }
}