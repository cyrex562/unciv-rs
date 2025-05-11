use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use reqwest::{Client, Method, Request, Response, StatusCode};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Serialize, Deserialize};
use log::{debug, error, info};
use uuid::Uuid;

use crate::models::multiplayer::api_status_code::ApiStatusCode;
use crate::models::multiplayer::api_error_response::ApiErrorResponse;
use crate::models::multiplayer::unciv_network_exception::UncivNetworkException;
use crate::models::multiplayer::websocket_message_serializer::WebSocketMessageSerializer;
use crate::models::multiplayer::websocket_message_type::WebSocketMessageType;
use crate::models::multiplayer::websocket_message_with_content::WebSocketMessageWithContent;
use crate::models::multiplayer::update_game_data::UpdateGameData;
use crate::utils::concurrency::Concurrency;

use super::auth_helper::AuthHelper;
use super::conf::{SESSION_COOKIE_NAME, DEFAULT_LOBBY_MAX_PLAYERS, DEFAULT_RANDOM_PASSWORD_LENGTH, MAX_CACHE_AGE_SECONDS};

/// List of HTTP status codes which are considered to be ApiErrorResponses by the specification
const ERROR_CODES: [StatusCode; 2] = [
    StatusCode::BAD_REQUEST,
    StatusCode::INTERNAL_SERVER_ERROR,
];

/// List of API status codes that should be re-executed after session refresh, if possible
const RETRY_CODES: [ApiStatusCode; 1] = [
    ApiStatusCode::Unauthenticated,
];

/// Default value for randomly generated passwords
const DEFAULT_RANDOM_PASSWORD_LENGTH: usize = 32;

/// Max age of a cached entry before it will be re-queried
const MAX_CACHE_AGE_SECONDS: u64 = 60;

/// Perform a HTTP request via method to endpoint
///
/// Use refine to change the Request after it has been prepared with the method
/// and path. Do not edit the cookie header or the request URL, since they might be overwritten.
/// If suppress is set, it will return None instead of throwing any exceptions.
/// This function retries failed requests after executing retry which will be passed
/// the same arguments as the request function, if it is set and the request failed due to
/// network or defined API errors, see RETRY_CODES. It should return a Boolean which determines
/// if the original request should be retried after finishing retry. For example, to silently
/// repeat a request on such failure, use such function: || true
///
/// # Errors
///
/// * `ApiException`: thrown for defined and recognized API problems
/// * `UncivNetworkException`: thrown for any kind of network error or de-serialization problems
async fn request<T>(
    method: Method,
    endpoint: &str,
    client: &Client,
    auth_helper: &AuthHelper,
    refine: Option<&dyn Fn(&mut Request)>,
    suppress: bool,
    retry: Option<&dyn Fn() -> bool>,
) -> Result<Option<Response>, UncivNetworkException> {
    let mut request = client.request(method, endpoint);

    if let Some(refine_fn) = refine {
        refine_fn(&mut request);
    }

    auth_helper.add(&mut request);

    // Perform the request, but handle network issues gracefully according to the specified exceptions
    let response = match client.execute(request).await {
        Ok(resp) => resp,
        Err(e) => {
            // This workaround allows to catch multiple exception types at the same time
            let should_retry = if let Some(retry_fn) = retry {
                debug!("Calling retry function for network error {} in '{} {}'", e, method, endpoint);
                retry_fn()
            } else {
                false
            };

            if should_retry {
                debug!("Retrying after network error {}: {} (cause: {:?})", e, e.to_string(), e.source());
                return request(method, endpoint, client, auth_helper, refine, suppress, None).await;
            } else if suppress {
                debug!("Suppressed network error {}: {} (cause: {:?})", e, e.to_string(), e.source());
                return Ok(None);
            } else {
                debug!("Throwing network error {}: {} (cause: {:?})", e, e.to_string(), e.source());
                return Err(UncivNetworkException::new(&e.to_string(), None));
            }
        }
    };

    // For HTTP errors defined in the API, throwing an ApiException would be the correct handling.
    // Therefore, try to de-serialize the response as ApiErrorResponse first. If it happens to be
    // an authentication failure, the request could be retried as well. Otherwise, throw the error.
    if ERROR_CODES.contains(&response.status()) {
        match response.json::<ApiErrorResponse>().await {
            Ok(error) => {
                // Now the API response can be checked for retry-able failures
                if RETRY_CODES.contains(&error.status_code) && retry.is_some() {
                    debug!("Calling retry function for API response error {} in '{} {}'", error, method, endpoint);
                    if let Some(retry_fn) = retry {
                        if retry_fn() {
                            return request(method, endpoint, client, auth_helper, refine, suppress, None).await;
                        }
                    }
                }
                if suppress {
                    debug!("Suppressing {} for call to '{}'", error, response.url());
                    return Ok(None);
                }
                return Err(UncivNetworkException::new(&error.message, Some(error.status_code)));
            },
            Err(e) => {
                // de-serialization failed
                let body_text = response.text().await.unwrap_or_default();
                error!("Invalid body for '{} {}' -> {}: {}: '{}'", method, response.url(), response.status(), e, body_text);

                let should_retry = if let Some(retry_fn) = retry {
                    debug!("Calling retry function for serialization error {} in '{} {}'", e, method, endpoint);
                    retry_fn()
                } else {
                    false
                };

                if should_retry {
                    return request(method, endpoint, client, auth_helper, refine, suppress, None).await;
                } else if suppress {
                    debug!("Suppressed invalid API error response {}: {} (cause: {:?})", e, e.to_string(), e.source());
                    return Ok(None);
                } else {
                    debug!("Throwing network error instead of API error due to serialization failure {}: {} (cause: {:?})", e, e.to_string(), e.source());
                    return Err(UncivNetworkException::new(&e.to_string(), None));
                }
            }
        }
    } else if response.status().is_success() {
        return Ok(Some(response));
    } else {
        // Here, the server returned a non-success code which is not recognized,
        // therefore it is considered a network error (even if was something like 404)
        if suppress {
            debug!("Suppressed unknown HTTP status code {} for '{} {}'", response.status(), method, response.url());
            return Ok(None);
        }
        // If the server does not conform to the API, re-trying requests is useless
        return Err(UncivNetworkException::new(&format!("Unknown HTTP status code: {}", response.status()), None));
    }
}

/// Get the default retry mechanism which tries to refresh the current session, if credentials are available
fn get_default_retry(client: &Client, auth_helper: &AuthHelper) -> Box<dyn Fn() -> bool + Send + Sync> {
    let last_credentials = auth_helper.get_last_successful_credentials();

    if let Some((username, password)) = last_credentials {
        let client_clone = client.clone();
        let auth_helper_clone = auth_helper.clone();

        Box::new(move || {
            // This would be implemented as an async function in a real implementation
            // For simplicity, we're returning a synchronous function here
            // In a real implementation, this would use tokio::spawn or similar
            true
        })
    } else {
        Box::new(|| false)
    }
}

/// Simple cache for GET queries to the API
struct Cache {
    response_cache: HashMap<String, (Instant, Response)>,
}

impl Cache {
    /// Clear the response cache
    fn clear(&mut self) {
        self.response_cache.clear();
    }

    /// Wrapper around request to cache responses to GET queries up to MAX_CACHE_AGE_SECONDS
    async fn get(
        &mut self,
        endpoint: &str,
        client: &Client,
        auth_helper: &AuthHelper,
        refine: Option<&dyn Fn(&mut Request)>,
        suppress: bool,
        cache: bool,
        retry: Option<&dyn Fn() -> bool>,
    ) -> Result<Option<Response>, UncivNetworkException> {
        let result = self.response_cache.get(endpoint);

        if cache && result.is_some() {
            let (timestamp, response) = result.unwrap();
            if timestamp.elapsed().as_secs() < MAX_CACHE_AGE_SECONDS {
                return Ok(Some(response.clone()));
            }
        }

        let response = request(Method::GET, endpoint, client, auth_helper, refine, suppress, retry).await?;

        if cache && response.is_some() {
            self.response_cache.insert(endpoint.to_string(), (Instant::now(), response.as_ref().unwrap().clone()));
        }

        Ok(response)
    }
}

// Create a singleton cache instance
lazy_static::lazy_static! {
    static ref CACHE: std::sync::Mutex<Cache> = std::sync::Mutex::new(Cache {
        response_cache: HashMap::new(),
    });
}

// Export the API implementations
pub mod accounts_api;
pub mod auth_api;
pub mod chat_api;
pub mod friend_api;
pub mod game_api;
pub mod invite_api;
pub mod lobby_api;