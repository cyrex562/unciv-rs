pub mod duration;
pub mod last_seen_improvement;

// We'll re-export SerializableDuration when it's needed
pub use duration::SerializableDuration;

// We'll re-export LastSeenImprovement and SerializableVector2 when they're needed
// pub use last_seen_improvement::{LastSeenImprovement, SerializableVector2};