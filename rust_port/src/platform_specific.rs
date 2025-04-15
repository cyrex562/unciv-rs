/// Trait for platform-specific functionality
pub trait PlatformSpecific: Send + Sync {
    /// Notifies player that their multiplayer turn started
    fn notify_turn_started(&self) {}

    /// Install system audio hooks
    fn install_audio_hooks(&self) {}

    /// If not None, this is the path to the directory in which to store the local files - mods, saves, maps, etc
    fn custom_data_directory(&self) -> Option<String> {
        None
    }
}

/// Default implementation of PlatformSpecific
pub struct DefaultPlatformSpecific;

impl PlatformSpecific for DefaultPlatformSpecific {}

// Global instance
lazy_static::lazy_static! {
    static ref PLATFORM_SPECIFIC: std::sync::Arc<dyn PlatformSpecific> = std::sync::Arc::new(DefaultPlatformSpecific);
}

/// Initialize the global PlatformSpecific instance with a custom implementation
pub fn init_platform_specific(platform: std::sync::Arc<dyn PlatformSpecific>) {
    unsafe {
        let platform_ptr = &PLATFORM_SPECIFIC as *const std::sync::Arc<dyn PlatformSpecific> as *mut std::sync::Arc<dyn PlatformSpecific>;
        *platform_ptr = platform;
    }
}

/// Get the global PlatformSpecific instance
pub fn get_platform_specific() -> std::sync::Arc<dyn PlatformSpecific> {
    PLATFORM_SPECIFIC.clone()
}