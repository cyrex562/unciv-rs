use std::time::Duration;

/// Name of the session cookie returned and expected by the server
pub const SESSION_COOKIE_NAME: &str = "id";

/// Default value for max number of players in a lobby if no other value is set
pub const DEFAULT_LOBBY_MAX_PLAYERS: u32 = 32;

/// Default ping frequency for outgoing WebSocket connection in milliseconds
pub const DEFAULT_WEBSOCKET_PING_FREQUENCY: u64 = 15_000;

/// Default session timeout expected from multiplayer servers (unreliable)
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Default cache expiry timeout to indicate that certain data needs to be re-fetched
pub const DEFAULT_CACHE_EXPIRY: Duration = Duration::from_secs(30 * 60);

/// Default timeout for a single request (milliseconds)
pub const DEFAULT_REQUEST_TIMEOUT: u64 = 10_000;

/// Default timeout for connecting to a remote server (milliseconds)
pub const DEFAULT_CONNECT_TIMEOUT: u64 = 5_000;

/// Default timeout for a single WebSocket PING-PONG roundtrip
pub const DEFAULT_WEBSOCKET_PING_TIMEOUT: u64 = 10_000;