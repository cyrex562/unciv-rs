// Source: orig_src/core/src/com/unciv/logic/files/PlatformSaverLoader.kt
// Ported to Rust

use std::error::Error;
use std::fmt;

/// Contract for platform-specific helper classes to handle saving and loading games to and from
/// arbitrary external locations.
///
/// Implementation note: If a game is loaded with `load_game` and the same game is saved with `save_game`,
/// the suggested_location in `save_game` will be the location returned by `load_game`.
pub trait PlatformSaverLoader {
    /// Save game data to a location
    ///
    /// # Arguments
    ///
    /// * `data` - Data to save
    /// * `suggested_location` - Proposed location
    /// * `on_saved` - On-save-complete callback
    /// * `on_error` - On-save-error callback
    fn save_game(
        &self,
        data: &str,
        suggested_location: &str,
        on_saved: impl FnOnce(&str) + Send + 'static,
        on_error: impl FnOnce(&dyn Error) + Send + 'static,
    );

    /// Load game data from a location
    ///
    /// # Arguments
    ///
    /// * `on_loaded` - On-load-complete callback
    /// * `on_error` - On-load-error callback
    fn load_game(
        &self,
        on_loaded: impl FnOnce(&str, &str) + Send + 'static,
        on_error: impl FnOnce(&dyn Error) + Send + 'static,
    );
}

/// Error that can be used with on_error callbacks to indicate the User cancelled the operation and needs no message
#[derive(Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Operation cancelled by user")
    }
}

impl Error for Cancelled {}

/// A no-op implementation of PlatformSaverLoader
pub struct None;

impl PlatformSaverLoader for None {
    fn save_game(
        &self,
        _data: &str,
        _suggested_location: &str,
        _on_saved: impl FnOnce(&str) + Send + 'static,
        _on_error: impl FnOnce(&dyn Error) + Send + 'static,
    ) {
        // No-op implementation
    }

    fn load_game(
        &self,
        _on_loaded: impl FnOnce(&str, &str) + Send + 'static,
        _on_error: impl FnOnce(&dyn Error) + Send + 'static,
    ) {
        // No-op implementation
    }
}

impl None {
    /// Create a new None implementation
    pub fn new() -> Self {
        None
    }
}

impl Default for None {
    fn default() -> Self {
        Self::new()
    }
}