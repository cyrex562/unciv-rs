use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderValue};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use log::debug;

use crate::utils::simple_http::SimpleHttp;
use crate::multiplayer::storage::file_storage::{FileStorage, FileMetaData, FileStorageRateLimitReached, MultiplayerFileNotFoundException, MultiplayerAuthException};

/// A file storage implementation for the Unciv server
pub struct UncivServerFileStorage {
    pub auth_header: Option<HashMap<String, String>>,
    pub server_url: String,
    pub timeout: u64,
}

impl UncivServerFileStorage {
    /// Create a new UncivServerFileStorage
    pub fn new() -> Self {
        Self {
            auth_header: None,
            server_url: String::new(),
            timeout: 30000,
        }
    }

    /// Get the URL for a file
    fn file_url(&self, file_name: &str) -> String {
        format!("{}/files/{}", self.server_url, file_name)
    }
}

impl FileStorage for UncivServerFileStorage {
    /// Save file data to the server
    ///
    /// # Parameters
    ///
    /// * `file_name` - The name of the file to save
    /// * `data` - The data to save
    ///
    /// # Errors
    ///
    /// * [MultiplayerAuthException] - if authentication failed
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn save_file_data(&self, file_name: &str, data: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Ok(());
        let file_url = self.file_url(file_name);

        SimpleHttp::send_request(
            Method::PUT,
            &file_url,
            data,
            Some(std::time::Duration::from_millis(self.timeout)),
            self.auth_header.as_ref().map(|h| {
                let mut headers = HeaderMap::new();
                for (key, value) in h {
                    headers.insert(
                        key.parse().unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                headers
            }),
            |success, response_text, status_code| {
                if !success {
                    debug!("Error from UncivServer during save: {}", response_text);
                    result = match status_code {
                        Some(401) => Err(Box::new(MultiplayerAuthException::new(response_text))),
                        _ => Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            response_text,
                        ))),
                    };
                }
            },
        );

        result
    }

    /// Load file data from the server
    ///
    /// # Parameters
    ///
    /// * `file_name` - The name of the file to load
    ///
    /// # Returns
    ///
    /// The file data as a string
    ///
    /// # Errors
    ///
    /// * [MultiplayerFileNotFoundException] - if the file was not found
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn load_file_data(&self, file_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut file_data = String::new();
        let mut result = Ok(());
        let file_url = self.file_url(file_name);

        SimpleHttp::send_get_request(
            &file_url,
            Some(std::time::Duration::from_millis(self.timeout)),
            self.auth_header.as_ref().map(|h| {
                let mut headers = HeaderMap::new();
                for (key, value) in h {
                    headers.insert(
                        key.parse().unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                headers
            }),
            |success, response_text, status_code| {
                if !success {
                    debug!("Error from UncivServer during load: {}", response_text);
                    result = match status_code {
                        Some(404) => Err(Box::new(MultiplayerFileNotFoundException::new(response_text))),
                        _ => Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            response_text,
                        ))),
                    };
                } else {
                    file_data = response_text;
                }
            },
        );

        result?;
        Ok(file_data)
    }

    /// Get metadata for a file
    ///
    /// # Parameters
    ///
    /// * `file_name` - The name of the file to get metadata for
    ///
    /// # Returns
    ///
    /// The file metadata
    ///
    /// # Errors
    ///
    /// * [MultiplayerFileNotFoundException] - if the file was not found
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn get_file_meta_data(&self, file_name: &str) -> Result<Box<dyn FileMetaData>, Box<dyn std::error::Error + Send + Sync>> {
        // Not yet implemented
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "get_file_meta_data not yet implemented",
        )))
    }

    /// Delete a file from the server
    ///
    /// # Parameters
    ///
    /// * `file_name` - The name of the file to delete
    ///
    /// # Errors
    ///
    /// * [MultiplayerFileNotFoundException] - if the file was not found
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn delete_file(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Ok(());
        let file_url = self.file_url(file_name);

        SimpleHttp::send_request(
            Method::DELETE,
            &file_url,
            "",
            Some(std::time::Duration::from_millis(self.timeout)),
            self.auth_header.as_ref().map(|h| {
                let mut headers = HeaderMap::new();
                for (key, value) in h {
                    headers.insert(
                        key.parse().unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                headers
            }),
            |success, response_text, status_code| {
                if !success {
                    result = match status_code {
                        Some(404) => Err(Box::new(MultiplayerFileNotFoundException::new(response_text))),
                        _ => Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            response_text,
                        ))),
                    };
                }
            },
        );

        result
    }

    /// Authenticate with the server
    ///
    /// # Parameters
    ///
    /// * `user_id` - The user ID to authenticate with
    /// * `password` - The password to authenticate with
    ///
    /// # Returns
    ///
    /// true if authentication was successful, false otherwise
    ///
    /// # Errors
    ///
    /// * [MultiplayerAuthException] - if authentication failed
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn authenticate(&mut self, user_id: &str, password: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut authenticated = false;
        let mut result = Ok(());

        // Create Basic Auth header
        let pre_encoded_auth_value = format!("{}:{}", user_id, password);
        let encoded_auth = BASE64.encode(pre_encoded_auth_value);
        let mut auth_header = HashMap::new();
        auth_header.insert(
            "Authorization".to_string(),
            format!("Basic {}", encoded_auth),
        );
        self.auth_header = Some(auth_header);

        let auth_url = format!("{}/auth", self.server_url);

        SimpleHttp::send_get_request(
            &auth_url,
            Some(std::time::Duration::from_millis(self.timeout)),
            self.auth_header.as_ref().map(|h| {
                let mut headers = HeaderMap::new();
                for (key, value) in h {
                    headers.insert(
                        key.parse().unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                headers
            }),
            |success, response_text, status_code| {
                if !success {
                    debug!("Error from UncivServer during authentication: {}", response_text);
                    self.auth_header = None;
                    result = match status_code {
                        Some(401) => Err(Box::new(MultiplayerAuthException::new(response_text))),
                        _ => Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            response_text,
                        ))),
                    };
                } else {
                    authenticated = true;
                }
            },
        );

        result?;
        Ok(authenticated)
    }

    /// Set a new password for the server
    ///
    /// # Parameters
    ///
    /// * `new_password` - The new password to set
    ///
    /// # Returns
    ///
    /// true if the password was set successfully, false otherwise
    ///
    /// # Errors
    ///
    /// * [MultiplayerAuthException] - if authentication failed
    /// * [FileStorageRateLimitReached] - if the server is rate limiting requests
    fn set_password(&self, new_password: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if self.auth_header.is_none() {
            return Ok(false);
        }

        let mut set_successful = false;
        let mut result = Ok(());

        let auth_url = format!("{}/auth", self.server_url);

        SimpleHttp::send_request(
            Method::PUT,
            &auth_url,
            new_password,
            Some(std::time::Duration::from_millis(self.timeout)),
            self.auth_header.as_ref().map(|h| {
                let mut headers = HeaderMap::new();
                for (key, value) in h {
                    headers.insert(
                        key.parse().unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                headers
            }),
            |success, response_text, status_code| {
                if !success {
                    debug!("Error from UncivServer during password set: {}", response_text);
                    result = match status_code {
                        Some(401) => Err(Box::new(MultiplayerAuthException::new(response_text))),
                        _ => Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            response_text,
                        ))),
                    };
                } else {
                    set_successful = true;
                }
            },
        );

        result?;
        Ok(set_successful)
    }
}