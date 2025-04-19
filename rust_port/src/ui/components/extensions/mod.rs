// UI component extensions

// Scene2D extensions and utilities
mod scene2d_extensions;
pub use scene2d_extensions::*;

// Other extensions
mod add_separators;
mod center_extensions;
mod color_extensions;
mod focus_extensions;
mod padding_extensions;
mod size_extensions;
mod transform_extensions;

// Re-export common extension traits
pub use add_separators::*;
pub use center_extensions::*;
pub use color_extensions::*;
pub use focus_extensions::*;
pub use padding_extensions::*;
pub use size_extensions::*;
pub use transform_extensions::*;