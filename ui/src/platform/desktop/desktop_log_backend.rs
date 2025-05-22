// Source: orig_src/desktop/src/com/unciv/app/desktop/DesktopLogBackend.kt
// Ported to Rust

use std::env;
use crate::utils::log::DefaultLogBackend;
use crate::utils::system::SystemUtils;

/// Log backend implementation for desktop platform
pub struct DesktopLogBackend {
    release: bool,
}

impl DesktopLogBackend {
    /// Creates a new desktop log backend
    pub fn new() -> Self {
        // Check if running in release mode
        // -ea (enable assertions) or kotlin debugging property as marker for a debug run
        let release = !env::args().any(|arg| arg == "-ea")
            && env::var("kotlinx.coroutines.debug").is_err();

        Self { release }
    }
}

impl DefaultLogBackend for DesktopLogBackend {
    /// Checks if running in release mode
    fn is_release(&self) -> bool {
        self.release
    }

    /// Gets system information
    fn get_system_info(&self) -> String {
        SystemUtils::get_system_info()
    }
}