use std::io::{self, Read};
use std::time::{Duration, SystemTime};
use std::thread;
use std::sync::{Arc, Mutex};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use serde::{Serialize, Deserialize};
use log::{debug, error};
use chrono::{DateTime, Utc};

use crate::multiplayer::storage::file_storage::{FileStorage, FileMetaData};
use crate::multiplayer::storage::exceptions::{FileStorageRateLimitReached, MultiplayerFileNotFoundException, FileStorageConflictException};
use crate::utils::date_format::UncivDateFormat;

const DROPBOX_API_BASE: &str = "https://api.dropboxapi.com/2";
const DROPBOX_CONTENT_BASE: &str = "https://content.dropboxapi.com/2";
const DROPBOX_BEARER_TOKEN: &str = "LTdBbopPUQ0AAAAAAAACxh4_Qd1eVMM7IBK3ULV3BgxzWZDMfhmgFbuUNF_rXQWb";

/// Dropbox implementation of FileStorage
pub struct DropBox {
    client: reqwest::Client,
    remaining_rate_limit_seconds: Arc<Mutex<i32>>,
    rate_limit_timer: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl DropBox {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            remaining_rate_limit_seconds: Arc::new(Mutex::new(0)),
            rate_limit_timer: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the location in Dropbox for a file
    fn get_local_game_location(&self, file_name: &str) -> String {
        format!("/MultiplayerGames/{}", file_name)
    }

    /// Make a request to the Dropbox API
    async fn dropbox_api(
        &self,
        url: &str,
        data: &str,
        content_type: &str,
        dropbox_api_arg: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        // Check rate limit
        {
            let remaining = *self.remaining_rate_limit_seconds.lock().unwrap();
            if remaining > 0 {
                return Err(Box::new(FileStorageRateLimitReached::new(remaining)));
            }
        }

        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", DROPBOX_BEARER_TOKEN))?,
        );

        if !dropbox_api_arg.is_empty() {
            headers.insert(
                "Dropbox-API-Arg",
                HeaderValue::from_str(dropbox_api_arg)?,
            );
        }

        if !content_type.is_empty() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(content_type)?,
            );
        }

        // Make request
        let response = self.client
            .post(url)
            .headers(headers)
            .body(data.to_string())
            .send()
            .await?;

        // Handle errors
        if !response.status().is_success() {
            let error_text = response.text().await?;
            debug!("Dropbox error response: {}", error_text);

            let error: ErrorResponse = serde_json::from_str(&error_text)?;

            if error.error_summary.starts_with("too_many_requests/") {
                self.trigger_rate_limit(&error)?;
                return Err(Box::new(FileStorageRateLimitReached::new(
                    *self.remaining_rate_limit_seconds.lock().unwrap()
                )));
            } else if error.error_summary.starts_with("path/not_found/") {
                return Err(Box::new(MultiplayerFileNotFoundException::new()));
            } else if error.error_summary.starts_with("path/conflict/file") {
                return Err(Box::new(FileStorageConflictException::new()));
            }

            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("Dropbox API error: {}", error.error_summary)
            )));
        }

        Ok(response)
    }

    /// Trigger rate limiting based on Dropbox response
    fn trigger_rate_limit(&self, response: &ErrorResponse) -> Result<(), Box<dyn std::error::Error>> {
        let retry_after = response.error
            .as_ref()
            .and_then(|e| e.retry_after.parse::<i32>().ok())
            .unwrap_or(300);

        {
            let mut remaining = self.remaining_rate_limit_seconds.lock().unwrap();
            *remaining = retry_after;
        }

        // Cancel existing timer if any
        {
            let mut timer = self.rate_limit_timer.lock().unwrap();
            if let Some(handle) = timer.take() {
                handle.abort();
            }

            // Start new timer
            let remaining_arc = self.remaining_rate_limit_seconds.clone();
            let timer_arc = self.rate_limit_timer.clone();

            *timer = Some(thread::spawn(move || {
                for _ in 0..retry_after {
                    thread::sleep(Duration::from_secs(1));
                    let mut remaining = remaining_arc.lock().unwrap();
                    *remaining -= 1;
                    if *remaining <= 0 {
                        let mut timer = timer_arc.lock().unwrap();
                        *timer = None;
                        break;
                    }
                }
            }));
        }

        Ok(())
    }

    /// Download a file from Dropbox
    async fn download_file(&self, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.dropbox_api(
            &format!("{}/files/download", DROPBOX_CONTENT_BASE),
            "",
            "text/plain",
            &format!("{{\"path\":\"{}\"}}", file_name),
        ).await?;

        Ok(response.text().await?)
    }
}

impl FileStorage for DropBox {
    fn delete_file(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.get_local_game_location(file_name);
        let data = format!("{{\"path\":\"{}\"}}", path);

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            self.dropbox_api(
                &format!("{}/files/delete_v2", DROPBOX_API_BASE),
                &data,
                "application/json",
                "",
            ).await?;
            Ok(())
        })
    }

    fn get_file_meta_data(&self, file_name: &str) -> Result<FileMetaData, Box<dyn std::error::Error>> {
        let path = self.get_local_game_location(file_name);
        let data = format!("{{\"path\":\"{}\"}}", path);

        let runtime = tokio::runtime::Runtime::new()?;
        let response = runtime.block_on(async {
            self.dropbox_api(
                &format!("{}/files/get_metadata", DROPBOX_API_BASE),
                &data,
                "application/json",
                "",
            ).await
        })?;

        let meta_data: MetaData = response.json().await?;
        Ok(Box::new(meta_data) as Box<dyn FileMetaData>)
    }

    fn save_file_data(&self, file_name: &str, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.get_local_game_location(file_name);
        let dropbox_api_arg = format!(
            "{{\"path\":\"{}\",\"mode\":{{\".tag\":\"overwrite\"}}}}",
            path
        );

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            self.dropbox_api(
                &format!("{}/files/upload", DROPBOX_CONTENT_BASE),
                data,
                "application/octet-stream",
                &dropbox_api_arg,
            ).await?;
            Ok(())
        })
    }

    fn load_file_data(&self, file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = self.get_local_game_location(file_name);

        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.download_file(&path))
    }

    fn authenticate(&self, _user_id: &str, _password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        Err(Box::new(io::Error::new(
            io::ErrorKind::Unsupported,
            "Authentication not implemented for Dropbox storage"
        )))
    }

    fn set_password(&self, _new_password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        Err(Box::new(io::Error::new(
            io::ErrorKind::Unsupported,
            "Password setting not implemented for Dropbox storage"
        )))
    }
}

/// Metadata response from Dropbox
#[derive(Debug, Serialize, Deserialize)]
pub struct MetaData {
    #[serde(rename = "server_modified")]
    pub server_modified: String,
}

impl FileMetaData for MetaData {
    fn get_last_modified(&self) -> SystemTime {
        // Parse the date string from Dropbox
        let date = UncivDateFormat::parse_date(&self.server_modified)
            .unwrap_or_else(|_| SystemTime::now());
        date
    }
}

/// Error response from Dropbox
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(rename = "error_summary")]
    pub error_summary: String,
    pub error: Option<ErrorDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetails {
    #[serde(rename = "retry_after")]
    pub retry_after: String,
}