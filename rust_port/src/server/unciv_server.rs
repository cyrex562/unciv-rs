// Source: orig_src/server/src/com/unciv/app/server/UncivServer.kt
// Ported to Rust

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header::Authorization;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::{Parser, ArgAction};
use env_logger::Env;
use log::{info, error};
use serde::{Deserialize, Serialize};
use tokio::fs as async_fs;
use tokio::io::AsyncReadExt;

/// Server configuration
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct ServerConfig {
    /// Server port
    #[arg(short = 'p', long = "port", env = "UncivServerPort", default_value = "8080")]
    pub port: u16,

    /// Multiplayer file's folder
    #[arg(short = 'f', long = "folder", env = "UncivServerFolder", default_value = "MultiplayerFiles")]
    pub folder: String,

    /// Enable Authentication
    #[arg(short = 'a', long = "auth", env = "UncivServerAuth", action = ArgAction::SetTrue)]
    pub auth_v1_enabled: bool,

    /// Display each operation archive request IP to assist management personnel
    #[arg(short = 'i', long = "Identify", env = "UncivServerIdentify", action = ArgAction::SetTrue)]
    pub identify_operators: bool,
}

/// Authentication credentials
#[derive(Debug, Clone)]
struct AuthCredentials {
    user_id: String,
    password: String,
}

/// Server state
struct ServerState {
    auth_map: Arc<HashMap<String, String>>,
    auth_enabled: bool,
    identify_operators: bool,
    folder: String,
}

/// Response for isalive endpoint
#[derive(Serialize)]
struct IsAliveResponse {
    auth_version: String,
}

/// Main server implementation
struct UncivServer;

impl UncivServer {
    /// Run the server with the given configuration
    async fn run(config: ServerConfig) -> io::Result<()> {
        // Initialize logger
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        // Create server state
        let auth_map = Arc::new(HashMap::new());
        let auth_enabled = config.auth_v1_enabled;
        let identify_operators = config.identify_operators;
        let folder = config.folder.clone();

        // Load auth file if enabled
        if auth_enabled {
            Self::load_auth_file(&auth_map).await?;
        }

        // Create server state
        let state = web::Data::new(ServerState {
            auth_map: auth_map.clone(),
            auth_enabled,
            identify_operators,
            folder: folder.clone(),
        });

        // Create folder if it doesn't exist
        fs::create_dir_all(&folder)?;

        // Start server
        let port = config.port;
        let port_str = if port == 80 { String::new() } else { format!(":{}", port) };
        info!("Starting UncivServer for {} on http://localhost{}",
            Path::new(&folder).canonicalize()?.display(), port_str);

        // Start HTTP server
        HttpServer::new(move || {
            App::new()
                .app_data(state.clone())
                .wrap(Logger::default())
                .service(isalive)
                .service(get_file)
                .service(put_file)
                .service(auth_status)
                .service(put_auth)
        })
        .bind(("127.0.0.1", port))?
        .run()
        .await?;

        Ok(())
    }

    /// Load authentication file
    async fn load_auth_file(auth_map: &Arc<HashMap<String, String>>) -> io::Result<()> {
        let auth_file = Path::new("server.auth");
        if !auth_file.exists() {
            info!("No server.auth file found, creating one");
            File::create(auth_file)?;
        } else {
            let content = fs::read_to_string(auth_file)?;
            let mut map = HashMap::new();
            for line in content.lines() {
                if let Some((user_id, password)) = line.split_once(':') {
                    map.insert(user_id.to_string(), password.to_string());
                }
            }
            *Arc::get_mut(auth_map).unwrap() = map;
        }
        Ok(())
    }

    /// Save authentication file
    async fn save_auth_file(auth_map: &Arc<HashMap<String, String>>) -> io::Result<()> {
        let auth_file = Path::new("server.auth");
        let mut content = String::new();
        for (user_id, password) in auth_map.as_ref() {
            content.push_str(&format!("{}:{}\n", user_id, password));
        }
        fs::write(auth_file, content)?;
        Ok(())
    }

    /// Extract authentication credentials from header
    fn extract_auth(auth_header: Option<&str>, auth_enabled: bool) -> Option<AuthCredentials> {
        if !auth_enabled {
            return None;
        }

        let auth_header = auth_header?;
        if !auth_header.starts_with("Basic ") {
            return None;
        }

        let encoded = &auth_header[6..];
        let decoded = match BASE64.decode(encoded) {
            Ok(bytes) => String::from_utf8(bytes).ok()?,
            Err(_) => return None,
        };

        let parts: Vec<&str> = decoded.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        Some(AuthCredentials {
            user_id: parts[0].to_string(),
            password: parts[1].to_string(),
        })
    }

    /// Validate authentication
    fn validate_auth(auth_creds: Option<&AuthCredentials>, auth_map: &HashMap<String, String>) -> bool {
        if auth_creds.is_none() {
            return true;
        }

        let auth_creds = auth_creds.unwrap();
        let stored_password = auth_map.get(&auth_creds.user_id);

        stored_password.is_none() || stored_password.unwrap() == &auth_creds.password
    }

    /// Validate game access
    fn validate_game_access(file: &Path, auth_creds: Option<&AuthCredentials>, auth_map: &HashMap<String, String>) -> bool {
        if !file.exists() {
            return true;
        }

        Self::validate_auth(auth_creds, auth_map)
    }
}

/// Isalive endpoint
#[get("/isalive")]
async fn isalive(state: web::Data<ServerState>) -> impl Responder {
    let auth_version = if state.auth_enabled { "1" } else { "0" };
    HttpResponse::Ok().json(IsAliveResponse {
        auth_version: auth_version.to_string(),
    })
}

/// Get file endpoint
#[get("/files/{fileName}")]
async fn get_file(
    path: web::Path<String>,
    state: web::Data<ServerState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let file_name = path.into_inner();
    let file_path = Path::new(&state.folder).join(&file_name);

    // Log request with IP if enabled
    if state.identify_operators {
        let ip = req.connection_info().peer_addr().unwrap_or("unknown");
        info!("File requested: {} --Operation sourced from {}", file_name, ip);
    } else {
        info!("File requested: {}", file_name);
    }

    // Check if file exists
    if !file_path.exists() {
        if state.identify_operators {
            let ip = req.connection_info().peer_addr().unwrap_or("unknown");
            info!("File {} not found --Operation sourced from {}", file_name, ip);
        } else {
            info!("File {} not found", file_name);
        }
        return HttpResponse::NotFound().body("File does not exist");
    }

    // Read file content
    match async_fs::read_to_string(&file_path).await {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(e) => {
            error!("Error reading file {}: {}", file_name, e);
            HttpResponse::InternalServerError().body("Error reading file")
        }
    }
}

/// Put file endpoint
#[put("/files/{fileName}")]
async fn put_file(
    path: web::Path<String>,
    state: web::Data<ServerState>,
    req: actix_web::HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let file_name = path.into_inner();
    let file_path = Path::new(&state.folder).join(&file_name);

    // Log request with IP if enabled
    if state.identify_operators {
        let ip = req.connection_info().peer_addr().unwrap_or("unknown");
        info!("Receiving file: {} --Operation sourced from {}", file_name, ip);
    } else {
        info!("Receiving file: {}", file_name);
    }

    // Extract auth credentials
    let auth_header = req.headers().get(Authorization::<String>())
        .and_then(|h| h.to_str().ok());
    let auth_creds = UncivServer::extract_auth(auth_header, state.auth_enabled);

    // Validate access
    if !UncivServer::validate_game_access(&file_path, auth_creds.as_ref(), &state.auth_map) {
        return HttpResponse::Unauthorized().finish();
    }

    // Read payload
    let mut bytes = Vec::new();
    let mut payload = payload.into_inner();
    while let Some(chunk) = payload.next().await {
        match chunk {
            Ok(chunk) => bytes.extend_from_slice(&chunk),
            Err(e) => {
                error!("Error reading payload: {}", e);
                return HttpResponse::InternalServerError().body("Error reading payload");
            }
        }
    }

    // Write file
    match async_fs::write(&file_path, bytes).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Error writing file {}: {}", file_name, e);
            HttpResponse::InternalServerError().body("Error writing file")
        }
    }
}

/// Auth status endpoint
#[get("/auth")]
async fn auth_status(
    state: web::Data<ServerState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    info!("Received auth request from {}", req.connection_info().peer_addr().unwrap_or("unknown"));

    let auth_header = req.headers().get(Authorization::<String>())
        .and_then(|h| h.to_str().ok());
    let auth_creds = UncivServer::extract_auth(auth_header, state.auth_enabled);

    if UncivServer::validate_auth(auth_creds.as_ref(), &state.auth_map) {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

/// Put auth endpoint
#[put("/auth")]
async fn put_auth(
    state: web::Data<ServerState>,
    req: actix_web::HttpRequest,
    payload: String,
) -> impl Responder {
    info!("Received auth password set from {}", req.connection_info().peer_addr().unwrap_or("unknown"));

    let auth_header = req.headers().get(Authorization::<String>())
        .and_then(|h| h.to_str().ok());
    let auth_creds = UncivServer::extract_auth(auth_header, state.auth_enabled);

    if UncivServer::validate_auth(auth_creds.as_ref(), &state.auth_map) {
        if let Some(auth_creds) = auth_creds {
            let mut auth_map = HashMap::new();
            for (k, v) in state.auth_map.as_ref() {
                auth_map.insert(k.clone(), v.clone());
            }
            auth_map.insert(auth_creds.user_id, payload);
            *Arc::get_mut(&state.auth_map).unwrap() = auth_map;
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::BadRequest().finish()
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

/// Main entry point
#[tokio::main]
async fn main() -> io::Result<()> {
    // Parse command line arguments
    let config = ServerConfig::parse();

    // Run server
    UncivServer::run(config).await
}