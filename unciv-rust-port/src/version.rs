//! Version information for the game

use std::fmt;

/// Version information for the game
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Version {
    /// Version text (e.g., "4.0.1")
    pub text: String,
    
    /// Build number
    pub number: i32,
}

impl Version {
    /// Create a new version with the specified text and number
    pub fn new(text: String, number: i32) -> Self {
        Self { text, number }
    }

    /// Create a default version (-1, empty string)
    pub fn default() -> Self {
        Self { 
            text: String::new(), 
            number: -1 
        }
    }

    /// Convert to a nice string representation
    pub fn to_nice_string(&self) -> String {
        format!("{} (Build {})", self.text, self.number)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_nice_string())
    }
}

/// Marker trait for GameInfo serialization
pub trait IsPartOfGameInfoSerialization {}

impl IsPartOfGameInfoSerialization for Version {}
