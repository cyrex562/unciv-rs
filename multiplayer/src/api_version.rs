use std::time::Duration;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::json;
use log::debug;

use crate::constants::Constants;
use crate::multiplayer::server_feature_set::ServerFeatureSet;
use crate::multiplayer::apiv2::{DEFAULT_CONNECT_TIMEOUT, UncivNetworkException, VersionResponse};
use crate::game::unciv_game::UncivGame;

/// Enum determining the version of a remote server API implementation
///
/// `APIv0` is used to reference DropBox. It doesn't support any further features.
/// `APIv1` is used for the UncivServer built-in server implementation as well as
/// for servers implementing this interface. Examples thereof include:
///  - https://github.com/Mape6/Unciv_server (Python)
///  - https://gitlab.com/azzurite/unciv-server (NodeJS)
///  - https://github.com/oynqr/rust_unciv_server (Rust)
///  - https://github.com/touhidurrr/UncivServer.xyz (NodeJS)
/// This servers may or may not support authentication. The `ServerFeatureSet` may
/// be used to inspect their functionality. `APIv2` is used to reference
/// the heavily extended REST-like HTTP API in combination with a WebSocket
/// functionality for communication. Examples thereof include:
///   - https://github.com/hopfenspace/runciv
///
/// A particular server may implement multiple interfaces simultaneously.
/// There's a server version check in the constructor of `Multiplayer`
/// which handles API auto-detection. The precedence of various APIs is
/// determined by that function:
/// @see `Multiplayer.determineServerAPI`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiVersion {
    /// APIv0 - Used for DropBox
    APIv0,
    /// APIv1 - Used for UncivServer and compatible servers
    APIv1,
    /// APIv2 - Used for REST-like HTTP API with WebSocket functionality
    APIv2,
}

impl ApiVersion {
    /// Check the server version by connecting to `base_url` without side-effects
    ///
    /// This function doesn't make use of any currently used workers or high-level
    /// connection pools, but instead opens and closes the transports inside it.
    ///
    /// It will first check if the `base_url` equals the `Constants.dropboxMultiplayerServer`
    /// to check for `ApiVersion::APIv0`. Dropbox may be unavailable, but this is **not**
    /// checked here. It will then try to connect to `/isalive` of `base_url`. If a
    /// HTTP 200 response is received, it will try to decode the response body as JSON.
    /// On success (regardless of the content of the JSON), `ApiVersion::APIv1` has been
    /// detected. Otherwise, it will try `/api/version` to detect `ApiVersion::APIv2`
    /// and try to decode its response as JSON. If any of the network calls result in
    /// timeout, connection refused or any other networking error, `suppress` is checked.
    /// If set, throwing *any* errors is forbidden, so it returns None, otherwise the
    /// detected `ApiVersion` is returned or the exception is thrown.
    ///
    /// # Parameters
    ///
    /// * `base_url` - The base URL of the server to check
    /// * `suppress` - Whether to suppress errors (default: true)
    /// * `timeout` - The timeout in milliseconds (default: DEFAULT_CONNECT_TIMEOUT)
    ///
    /// # Returns
    ///
    /// The detected API version, or None if no version could be detected
    ///
    /// # Errors
    ///
    /// * `UncivNetworkException` - thrown for any kind of network error
    ///   or de-serialization problems (only when `suppress` is false)
    pub async fn detect(
        base_url: &str,
        suppress: bool,
        timeout: Option<u64>,
    ) -> Result<Option<ApiVersion>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if the server is Dropbox
        if base_url == Constants::DROPBOX_MULTIPLAYER_SERVER {
            return Ok(Some(ApiVersion::APIv0));
        }

        // Ensure the base URL ends with a slash
        let fixed_base_url = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{}/", base_url)
        };

        // Create a client with the appropriate timeout
        let timeout_duration = Duration::from_millis(timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT));
        let client = Client::builder()
            .timeout(timeout_duration)
            .build()?;

        // Set up headers
        let mut headers = HeaderMap::new();
        let user_agent = if UncivGame::is_current_initialized() {
            format!("Unciv/{}-GNU-Terry-Pratchett", UncivGame::VERSION.to_nice_string())
        } else {
            "Unciv/Turn-Checker-GNU-Terry-Pratchett".to_string()
        };
        headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent)?);

        // Try to connect to an APIv1 server at first
        let isalive_url = format!("{}isalive", fixed_base_url);
        let response1 = match client.get(&isalive_url).headers(headers.clone()).send().await {
            Ok(response) => response,
            Err(e) => {
                debug!("Failed to fetch '/isalive' at {}: {}", fixed_base_url, e);
                if !suppress {
                    return Err(Box::new(UncivNetworkException::new(e)));
                }
                return Ok(None);
            }
        };

        if response1.status().is_success() {
            let body_text = response1.text().await?;

            // Some API implementations just return the text "true" on the `isalive` endpoint
            if body_text.starts_with("true") {
                debug!("Detected APIv1 at {} (no feature set)", fixed_base_url);
                return Ok(Some(ApiVersion::APIv1));
            }

            // Try to parse the response as a ServerFeatureSet
            match serde_json::from_str::<ServerFeatureSet>(&body_text) {
                Ok(server_feature_set) => {
                    debug!("Detected APIv1 at {}: {:?}", fixed_base_url, server_feature_set);
                    return Ok(Some(ApiVersion::APIv1));
                }
                Err(e) => {
                    debug!("Failed to de-serialize OK response body of '/isalive' at {}: {}", fixed_base_url, e);
                }
            }
        }

        // Then try to connect to an APIv2 server
        let version_url = format!("{}api/version", fixed_base_url);
        let response2 = match client.get(&version_url).headers(headers).send().await {
            Ok(response) => response,
            Err(e) => {
                debug!("Failed to fetch '/api/version' at {}: {}", fixed_base_url, e);
                if !suppress {
                    return Err(Box::new(UncivNetworkException::new(e)));
                }
                return Ok(None);
            }
        };

        if response2.status().is_success() {
            match response2.json::<VersionResponse>().await {
                Ok(server_version) => {
                    debug!("Detected APIv2 at {}: {:?}", fixed_base_url, server_version);
                    return Ok(Some(ApiVersion::APIv2));
                }
                Err(e) => {
                    debug!("Failed to de-serialize OK response body of '/api/version' at {}: {}", fixed_base_url, e);
                }
            }
        }

        Ok(None)
    }
}