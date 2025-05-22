use std::fmt;
use serde::{Serialize, Deserialize};
use crate::models::stats::{Stats, INamed};

/// A struct that combines Stats with a name
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedStats {
    /// The base stats
    #[serde(flatten)]
    pub stats: Stats,

    /// The name of the stats
    pub name: String,
}

impl NamedStats {
    /// Create a new NamedStats instance
    pub fn new() -> Self {
        Self {
            stats: Stats::new(),
            name: String::new(),
        }
    }

    /// Clone the stats part of this NamedStats
    pub fn clone_stats(&self) -> Stats {
        self.stats.clone()
    }
}

impl INamed for NamedStats {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl fmt::Display for NamedStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}