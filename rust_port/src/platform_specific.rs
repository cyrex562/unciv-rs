use std::path::PathBuf;
use std::sync::Arc;
use lazy_static::lazy_static;

/// Trait for platform-specific functionality
pub trait PlatformSpecific: Send + Sync {
    /// Notify the player with a message
    fn notify_player(&self, message: &str);

    /// Install audio hooks for the platform
    fn install_audio_hooks(&self);

    /// Get the custom data directory for the platform
    fn get_custom_data_directory(&self) -> Option<PathBuf>;
}

/// Default implementation of PlatformSpecific
pub struct DefaultPlatformSpecific;

impl PlatformSpecific for DefaultPlatformSpecific {
    fn notify_player(&self, _message: &str) {
        // Default implementation does nothing
    }

    fn install_audio_hooks(&self) {
        // Default implementation does nothing
    }

    fn get_custom_data_directory(&self) -> Option<PathBuf> {
        None
    }
}

lazy_static! {
    /// Global instance of the platform-specific implementation
    pub static ref PLATFORM_SPECIFIC: Arc<dyn PlatformSpecific> = Arc::new(DefaultPlatformSpecific);
}

/// Initialize the global PlatformSpecific instance with a custom implementation
pub fn init_platform_specific(platform: Arc<dyn PlatformSpecific>) {
    unsafe {
        let platform_ptr = &PLATFORM_SPECIFIC as *const Arc<dyn PlatformSpecific> as *mut Arc<dyn PlatformSpecific>;
        *platform_ptr = platform;
    }
}

/// Get the global PlatformSpecific instance
pub fn get_platform_specific() -> Arc<dyn PlatformSpecific> {
    PLATFORM_SPECIFIC.clone()
}