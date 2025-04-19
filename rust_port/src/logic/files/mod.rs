// Files module for file handling

mod map_saver;
mod platform_saver_loader;
pub mod unciv_files;

pub use map_saver::MapSaver;
pub use platform_saver_loader::{PlatformSaverLoader, None as PlatformSaverLoaderNone, Cancelled};