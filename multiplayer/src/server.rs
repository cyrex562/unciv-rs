use futures_util::stream::StreamExt;
use crate::server_state::ServerState;
// Source: orig_src/server/src/com/unciv/app/server/UncivServer.kt
// Ported to Rust
use axum::{
    extract::Extension,
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    http::{Request, Response},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::sink::SinkExt;
use clap::ArgAction;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::ws::Utf8Bytes;
use tokio::sync::broadcast;

const DEFAULT_REQUEST_TIMEOUT: u64 = 30000;
const DEFAULT_CONNECT_TIMEOUT: u64 = 10000;


#[derive(Debug, Serialize, Deserialize)]
struct IsAliveResponse {
    auth_version: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ServerConfig {
    /// Server port
    #[arg(short = 'p', long = "port", env = "UNCIV_SERVER_PORT", default_value = "8080")]
    port: u16,

    /// Multiplayer file's folder
    #[arg(short = 'f', long = "folder", env = "UNCIV_SERVER_FOLDER", default_value = "MultiplayerFiles")]
    folder: String,

    /// Enable Authentication
    #[arg(short = 'a', long = "auth", env = "UNCIV_SERVER_AUTH", action = ArgAction::SetTrue)]
    auth_v1_enabled: bool,

    /// Display each operation archive request IP to assist management personnel
    #[arg(short = 'i', long = "Identify", env = "UNCIV_SERVER_IDENTIFY", action = ArgAction::SetTrue)]
    identify_operators: bool,


    #[arg(short = 'a', long = "bind_address", env = "UNCIV_SERVER_BIND_ADDRESS", default_value = "127.0.0.1")]
    bind_address: String,
}
async fn health_check(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {

    Ok(Response::new(Body::from("ok")))
}
async fn version_get(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    Ok(Response::new(Body::from("1.0.0")))

}
async fn isalive(state: Extension<Arc<ServerState>>) -> Result<Response<Body>, hyper::Error> {
    let auth_version = if state.auth_enabled {
        "1".to_string()
    } else {
        "0".to_string()
    };
    Ok(Response::new(Body::from(serde_json::to_string(&IsAliveResponse { auth_version }).unwrap())))


}

static BROADCAST_CAPACITY: usize = 100;

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(tx): Extension<broadcast::Sender<String>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(mut socket: WebSocket, tx: broadcast::Sender<String>) {
    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();
    // Spawn a task to forward broadcast messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(Utf8Bytes::from(msg))).await.is_err() {
                break;
            }
        }
    });
    // Receive messages from the client and broadcast them
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let _ = tx.send(text.to_string());
        }
    }
    send_task.abort();
}

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    // Parse command line arguments
    let config = ServerConfig::parse();

    // Create server state
    let state = Arc::new(ServerState {
        auth_map: Arc::new(HashMap::new()),
        auth_enabled: config.auth_v1_enabled,
        identify_operators: config.identify_operators,
        folder: config.folder.clone(),
    });
    // Create a broadcast channel for WebSocket communication
    let (tx, _rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);

    // Create routes
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/version", get(version_get))
        .route("/isalive", get(isalive).layer(Extension(state.clone())))
        .route("/ws", get(ws_handler).layer(Extension(tx.clone())));

    // Start server
    let addr = (config.bind_address.parse().unwrap_or("127.0.0.1".parse().unwrap()), config.port).into();
    println!("Listening on http://{}", addr);
    tokio::spawn(async move {
        axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
            .await
            .unwrap();
    });

    Ok(())
}

pub fn is_compatible(version: &str) -> bool {
    // Check if the version is compatible
    // This is a placeholder implementation
    version == "1.0.0"
}

struct GameInfo {
    version: i32
}

pub fn get_game_info() -> GameInfo {
    // Return the game info
    // This is a placeholder implementation
    GameInfo {
        version: 1,
    }
}

pub fn send_text(text: &str) -> Result<(), String> {
    // Send a text message to the server
    // This is a placeholder implementation
    println!("Sending text: {}", text);
    Ok(())
}

pub fn send_ping() -> Result<(), String> {
    // Send a ping message to the server
    // wait for a response pong
    // This is a placeholder implementation
    println!("Sending ping");
    Ok(())
}

pub fn send_pong() -> Result<(), String> {
    // Send a pong message to the server
    // This is a placeholder implementation
    println!("Sending pong");
    Ok(())
}

pub fn after_login() -> Result<(), String> {
    // Perform actions after login
    // This is a placeholder implementation
    println!("Performing actions after login");
    Ok(())
}

pub fn after_logout() -> Result<(), String> {
    // Perform actions after logout
    // This is a placeholder implementation
    println!("Performing actions after logout");
    Ok(())
}

pub fn refresh_session() -> Result<(), String> {
    // Refresh the session
    // This is a placeholder implementation
    println!("Refreshing session");
    Ok(())
}

pub fn save_game_data(game_id: &str, data: &str) -> Result<(), String> {
    // Save game data
    // This is a placeholder implementation
    println!("Saving game data for game ID {}: {}", game_id, data);
    Ok(())
}

pub fn load_game_data(game_id: &str) -> Result<String, String> {
    // Load game data
    // This is a placeholder implementation
    println!("Loading game data for game ID {}", game_id);
    Ok("Game data".to_string())
}

pub fn delete_game_data(game_id: &str) -> Result<(), String> {
    // Delete game data
    // This is a placeholder implementation
    println!("Deleting game data for game ID {}", game_id);
    Ok(())
}

pub fn authenticate(user_id: &str, password: &str) -> Result<bool, String> {
    // Authenticate the user
    // This is a placeholder implementation
    println!("Authenticating user {} with password {}", user_id, password);
    Ok(true)
}

pub fn set_password(new_password: &str) -> Result<bool, String> {
    // Set a new password
    // This is a placeholder implementation
    println!("Setting new password: {}", new_password);
    Ok(true)
}