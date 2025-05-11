use serde::{Serialize, Deserialize};
use std::fmt;

/// Represents an Unciv Sound, either from a predefined set or custom with a specified filename.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UncivSound {
    /// The base filename without extension.
    pub file_name: String,
}

impl UncivSound {
    /// Creates a new UncivSound with the given filename
    pub fn new(file_name: String) -> Self {
        UncivSound { file_name }
    }

    /// Creates an empty UncivSound (for deserialization)
    pub fn empty() -> Self {
        UncivSound { file_name: String::new() }
    }
}

impl Default for UncivSound {
    fn default() -> Self {
        UncivSound::empty()
    }
}

impl fmt::Display for UncivSound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file_name)
    }
}

/// Predefined sound constants
pub struct UncivSounds;

impl UncivSounds {
    /// Bombard sound
    pub const BOMBARD: UncivSound = UncivSound { file_name: "bombard" };

    /// Chimes sound
    pub const CHIMES: UncivSound = UncivSound { file_name: "chimes" };

    /// Choir sound
    pub const CHOIR: UncivSound = UncivSound { file_name: "choir" };

    /// Click sound
    pub const CLICK: UncivSound = UncivSound { file_name: "click" };

    /// Coin sound
    pub const COIN: UncivSound = UncivSound { file_name: "coin" };

    /// Construction sound
    pub const CONSTRUCTION: UncivSound = UncivSound { file_name: "construction" };

    /// Fire sound
    pub const FIRE: UncivSound = UncivSound { file_name: "fire" };

    /// Fortify sound
    pub const FORTIFY: UncivSound = UncivSound { file_name: "fortify" };

    /// Notification1 sound
    pub const NOTIFICATION1: UncivSound = UncivSound { file_name: "notification1" };

    /// Notification2 sound
    pub const NOTIFICATION2: UncivSound = UncivSound { file_name: "notification2" };

    /// Paper sound
    pub const PAPER: UncivSound = UncivSound { file_name: "paper" };

    /// Policy sound
    pub const POLICY: UncivSound = UncivSound { file_name: "policy" };

    /// Promote sound
    pub const PROMOTE: UncivSound = UncivSound { file_name: "promote" };

    /// Setup sound
    pub const SETUP: UncivSound = UncivSound { file_name: "setup" };

    /// Silent sound (empty string)
    pub const SILENT: UncivSound = UncivSound { file_name: "" };

    /// Slider sound
    pub const SLIDER: UncivSound = UncivSound { file_name: "slider" };

    /// Swap sound
    pub const SWAP: UncivSound = UncivSound { file_name: "swap" };

    /// Upgrade sound
    pub const UPGRADE: UncivSound = UncivSound { file_name: "upgrade" };

    /// Whoosh sound
    pub const WHOOSH: UncivSound = UncivSound { file_name: "whoosh" };
}