// Source: orig_src/desktop/src/com/unciv/app/desktop/DiscordUpdater.kt
// Ported to Rust

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use discord_rpc_client::{Client as DiscordClient, Event};
use crate::utils::log::Log;

/// Game information for Discord Rich Presence
#[derive(Clone, Default)]
pub struct DiscordGameInfo {
    pub game_nation: String,
    pub game_leader: String,
    pub game_turn: i32,
}

/// Updates Discord Rich Presence with game information
pub struct DiscordUpdater {
    on_update: Arc<Mutex<Option<Box<dyn Fn() -> Option<DiscordGameInfo> + Send + Sync>>>>,
    update_timer: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    client: Arc<Mutex<Option<DiscordClient>>>,
}

impl DiscordUpdater {
    /// Creates a new Discord updater
    pub fn new() -> Self {
        Self {
            on_update: Arc::new(Mutex::new(None)),
            update_timer: Arc::new(Mutex::new(None)),
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the callback for updating game information
    pub fn set_on_update<F>(&self, callback: F)
    where
        F: Fn() -> Option<DiscordGameInfo> + Send + Sync + 'static,
    {
        let mut on_update = self.on_update.lock().unwrap();
        *on_update = Some(Box::new(callback));
    }

    /// Starts updating Discord Rich Presence
    pub fn start_updates(&self) {
        // Clone Arc references for the thread
        let on_update = Arc::clone(&self.on_update);
        let update_timer = Arc::clone(&self.update_timer);
        let client = Arc::clone(&self.client);

        // Try to initialize Discord RPC
        let mut discord_client = match DiscordClient::new(647066573147996161) {
            Ok(client) => client,
            Err(e) => {
                Log::error("Could not initialize Discord", &e);
                return;
            }
        };

        // Set up event handler
        discord_client.on_event(|_client, event| {
            match event {
                Event::Ready => Log::info("Discord RPC ready"),
                Event::Error(e) => Log::error("Discord RPC error", &e),
                _ => {}
            }
        });

        // Start the client
        if let Err(e) = discord_client.start() {
            Log::error("Could not start Discord RPC", &e);
            return;
        }

        // Store the client
        *client.lock().unwrap() = Some(discord_client);

        // Create a thread for updating Discord Rich Presence
        let handle = thread::spawn(move || {
            let mut last_update = Instant::now();

            loop {
                // Update every second
                if last_update.elapsed() >= Duration::from_secs(1) {
                    Self::update_rpc(&on_update, &client);
                    last_update = Instant::now();
                }

                thread::sleep(Duration::from_millis(100));
            }
        });

        // Store the thread handle
        *update_timer.lock().unwrap() = Some(handle);
    }

    /// Stops updating Discord Rich Presence
    pub fn stop_updates(&self) {
        // Cancel the update timer
        let mut update_timer = self.update_timer.lock().unwrap();
        if let Some(handle) = update_timer.take() {
            // In Rust, we can't directly cancel a thread, but we can set the on_update to None
            // which will cause the update_rpc function to return early
            let mut on_update = self.on_update.lock().unwrap();
            *on_update = None;

            // Wait for the thread to finish
            let _ = handle.join();
        }

        // Shutdown the Discord client
        let mut client = self.client.lock().unwrap();
        if let Some(client) = client.take() {
            let _ = client.shutdown();
        }
    }

    /// Updates Discord Rich Presence with game information
    fn update_rpc(
        on_update: &Arc<Mutex<Option<Box<dyn Fn() -> Option<DiscordGameInfo> + Send + Sync>>>>>,
        client: &Arc<Mutex<Option<DiscordClient>>>,
    ) {
        // Get the callback
        let on_update = on_update.lock().unwrap();
        let callback = match &*on_update {
            Some(callback) => callback,
            None => return,
        };

        // Get the game information
        let info = match callback() {
            Some(info) => info,
            None => return,
        };

        // Get the Discord client
        let mut client = client.lock().unwrap();
        let client = match &mut *client {
            Some(client) => client,
            None => return,
        };

        // Update Discord Rich Presence
        let mut activity = discord_rpc_client::models::Activity::new();
        activity = activity.assets(
            discord_rpc_client::models::Assets::new()
                .large_image("logo")
        );

        if !info.game_leader.is_empty() && !info.game_nation.is_empty() {
            activity = activity.details(&format!("{} of {}", info.game_leader, info.game_nation));
            activity = activity.state(&format!("Turn {}", info.game_turn));
        }

        if let Err(e) = client.set_activity(activity) {
            Log::error("Exception while updating Discord Rich Presence", &e);
        }
    }
}