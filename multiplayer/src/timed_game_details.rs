use std::time::{Duration, SystemTime, UNIX_EPOCH};
use super::GameDetails;

/// A wrapper around GameDetails that includes timing information for caching
#[derive(Debug, Clone)]
pub struct TimedGameDetails {
    /// The game details
    pub details: GameDetails,

    /// When the details were last updated
    pub last_updated: SystemTime,
}

impl TimedGameDetails {
    /// Creates a new TimedGameDetails instance
    pub fn new(details: GameDetails) -> Self {
        Self {
            details,
            last_updated: SystemTime::now(),
        }
    }

    /// Checks if the cached details are stale based on the given maximum age
    pub fn is_stale(&self, max_age: Duration) -> bool {
        if let Ok(age) = SystemTime::now().duration_since(self.last_updated) {
            age > max_age
        } else {
            true // If we can't determine the age, consider it stale
        }
    }

    /// Refreshes the timestamp to now
    pub fn refresh(&mut self) {
        self.last_updated = SystemTime::now();
    }

    /// Updates the game details and refreshes the timestamp
    pub fn update(&mut self, details: GameDetails) {
        self.details = details;
        self.refresh();
    }
}