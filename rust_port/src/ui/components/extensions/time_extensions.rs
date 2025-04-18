use std::time::{Duration, Instant};

/// Extension trait for Duration to add comparison functionality
pub trait DurationExt {
    /// Returns true if this duration is larger than the other duration
    fn is_larger_than(&self, other: &Duration) -> bool;
}

impl DurationExt for Duration {
    fn is_larger_than(&self, other: &Duration) -> bool {
        self > other
    }
}

/// Extension trait for Instant to add comparison functionality
pub trait InstantExt {
    /// Returns true if this instant is larger than (comes after) the other instant
    fn is_larger_than(&self, other: &Instant) -> bool;
}

impl InstantExt for Instant {
    fn is_larger_than(&self, other: &Instant) -> bool {
        self > other
    }
}