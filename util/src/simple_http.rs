use std::io::{self, Read, Write};
use std::net::{DatagramSocket, InetAddr, SocketAddr};
use std::time::Duration;
use reqwest::{
    Client, ClientBuilder, Method, Request, Response,
    header::{HeaderMap, HeaderValue, USER_AGENT, CONTENT_TYPE},
};
use log::debug;
use serde_json;

use crate::game::unciv_game::UncivGame;
use crate::constants::Constants;

/// A simple HTTP client for making HTTP requests
pub struct SimpleHttp;

impl SimpleHttp {
    /// Send a GET request to the given URL
    ///
    /// # Parameters
    ///
    /// * `url` - The URL to send the request to
    /// * `timeout` - The timeout in milliseconds (default: 5000)
    /// * `header` - Optional headers to include in the request
    /// * `action` - Callback to handle the response
    pub fn send_get_request(
        url: &str,
        timeout: Option<Duration>,
        header: Option<&HeaderMap>,
        action: impl FnOnce(bool, String, Option<i32>) + Send + 'static,
    ) {
        Self::send_request(
            Method::GET,
            url,
            "",
            timeout,
            header,
            action,
        );
    }

    /// Send an HTTP request
    ///
    /// # Parameters
    ///
    /// * `method` - The HTTP method to use
    /// * `url` - The URL to send the request to
    /// * `content` - The content to send in the request body
    /// * `timeout` - The timeout in milliseconds (default: 5000)
    /// * `header` - Optional headers to include in the request
    /// * `action` - Callback to handle the response
    pub fn send_request(
        method: Method,
        url: &str,
        content: &str,
        timeout: Option<Duration>,
        header: Option<&HeaderMap>,
        action: impl FnOnce(bool, String, Option<i32>) + Send + 'static,
    ) {
        let timeout = timeout.unwrap_or(Duration::from_millis(5000));

        // Ensure URL has a scheme
        let url = if !url.contains("://") {
            format!("http://{}", url)
        } else {
            url.to_string()
        };

        // Create client with timeout
        let client = match ClientBuilder::new()
            .timeout(timeout)
            .build() {
                Ok(client) => client,
                Err(e) => {
                    debug!("Failed to create HTTP client: {}", e);
                    action(false, "Failed to create HTTP client".to_string(), None);
                    return;
                }
            };

        // Create request
        let mut request = match Request::new(method, url.parse().unwrap()) {
            Ok(request) => request,
            Err(e) => {
                debug!("Bad URL: {}", e);
                action(false, "Bad URL".to_string(), None);
                return;
            }
        };

        // Set headers
        let mut headers = HeaderMap::new();

        // Set User-Agent
        let user_agent = if UncivGame::is_current_initialized() {
            format!("Unciv/{}-GNU-Terry-Pratchett", UncivGame::VERSION.to_nice_string())
        } else {
            "Unciv/Turn-Checker-GNU-Terry-Pratchett".to_string()
        };
        headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent).unwrap());

        // Set Content-Type
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

        // Add custom headers
        if let Some(custom_headers) = header {
            for (name, value) in custom_headers.iter() {
                headers.insert(name, value.clone());
            }
        }

        request.headers_mut().extend(headers);

        // Add content if not empty
        if !content.is_empty() {
            request.body_mut().replace(content.as_bytes().to_vec().into());
        }

        // Send request
        match client.execute(request) {
            Ok(response) => {
                let status = response.status().as_u16() as i32;
                match response.text() {
                    Ok(text) => action(true, text, Some(status)),
                    Err(e) => {
                        debug!("Error reading response: {}", e);
                        action(false, e.to_string(), Some(status));
                    }
                }
            }
            Err(e) => {
                debug!("Error during HTTP request: {}", e);
                let error_message = if let Some(response) = e.response() {
                    match response.text() {
                        Ok(text) => text,
                        Err(_) => e.to_string(),
                    }
                } else {
                    e.to_string()
                };
                debug!("Returning error message [{}]", error_message);
                action(false, error_message, None);
            }
        }
    }

    /// Get the local IP address
    ///
    /// # Returns
    ///
    /// The local IP address as a string, or None if it could not be determined
    pub fn get_ip_address() -> Option<String> {
        let socket = match DatagramSocket::bind("0.0.0.0:0") {
            Ok(socket) => socket,
            Err(_) => return None,
        };

        match socket.connect("8.8.8.8:10002") {
            Ok(_) => {
                match socket.local_addr() {
                    Ok(addr) => Some(addr.ip().to_string()),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}