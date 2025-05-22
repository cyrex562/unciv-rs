use crate::game::game_info::GameInfoPreview;
use crate::game::event_bus::Event;

/// Trait for events that have a multiplayer game name
pub trait HasMultiplayerGameName {
    /// The name of the game
    fn name(&self) -> &str;
}

/// Trait for events that indicate a multiplayer game update has ended
pub trait MultiplayerGameUpdateEnded: Event + HasMultiplayerGameName {}

/// Trait for events that indicate a multiplayer game update has succeeded
pub trait MultiplayerGameUpdateSucceeded: Event + HasMultiplayerGameName {
    /// The preview of the game
    fn preview(&self) -> &GameInfoPreview;
}

/// Event fired when a game successfully updated
pub struct MultiplayerGameUpdated {
    /// The name of the game
    name: String,
    /// The preview of the game
    preview: GameInfoPreview,
}

impl MultiplayerGameUpdated {
    /// Create a new MultiplayerGameUpdated event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `preview` - The preview of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdated event
    pub fn new(name: String, preview: GameInfoPreview) -> Self {
        Self { name, preview }
    }
}

impl HasMultiplayerGameName for MultiplayerGameUpdated {
    fn name(&self) -> &str {
        &self.name
    }
}

impl MultiplayerGameUpdateSucceeded for MultiplayerGameUpdated {
    fn preview(&self) -> &GameInfoPreview {
        &self.preview
    }
}

impl MultiplayerGameUpdateEnded for MultiplayerGameUpdated {}

/// Event fired when a game errored while updating
pub struct MultiplayerGameUpdateFailed {
    /// The name of the game
    name: String,
    /// The error that occurred
    error: Box<dyn std::error::Error>,
}

impl MultiplayerGameUpdateFailed {
    /// Create a new MultiplayerGameUpdateFailed event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `error` - The error that occurred
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateFailed event
    pub fn new(name: String, error: Box<dyn std::error::Error>) -> Self {
        Self { name, error }
    }
}

impl HasMultiplayerGameName for MultiplayerGameUpdateFailed {
    fn name(&self) -> &str {
        &self.name
    }
}

impl MultiplayerGameUpdateEnded for MultiplayerGameUpdateFailed {}

/// Event fired when a game updated successfully, but nothing changed
pub struct MultiplayerGameUpdateUnchanged {
    /// The name of the game
    name: String,
    /// The preview of the game
    preview: GameInfoPreview,
}

impl MultiplayerGameUpdateUnchanged {
    /// Create a new MultiplayerGameUpdateUnchanged event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `preview` - The preview of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateUnchanged event
    pub fn new(name: String, preview: GameInfoPreview) -> Self {
        Self { name, preview }
    }
}

impl HasMultiplayerGameName for MultiplayerGameUpdateUnchanged {
    fn name(&self) -> &str {
        &self.name
    }
}

impl MultiplayerGameUpdateSucceeded for MultiplayerGameUpdateUnchanged {
    fn preview(&self) -> &GameInfoPreview {
        &self.preview
    }
}

impl MultiplayerGameUpdateEnded for MultiplayerGameUpdateUnchanged {}

/// Event fired when a game starts updating
pub struct MultiplayerGameUpdateStarted {
    /// The name of the game
    name: String,
}

impl MultiplayerGameUpdateStarted {
    /// Create a new MultiplayerGameUpdateStarted event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameUpdateStarted event
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl HasMultiplayerGameName for MultiplayerGameUpdateStarted {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Event fired when a game's name got changed
pub struct MultiplayerGameNameChanged {
    /// The name of the game
    name: String,
    /// The new name of the game
    new_name: String,
}

impl MultiplayerGameNameChanged {
    /// Create a new MultiplayerGameNameChanged event
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the game
    /// * `new_name` - The new name of the game
    ///
    /// # Returns
    ///
    /// A new MultiplayerGameNameChanged event
    pub fn new(name: String, new_name: String) -> Self {
        Self { name, new_name }
    }
}

impl HasMultiplayerGameName for MultiplayerGameNameChanged {
    fn name(&self) -> &str {
        &self.name
    }
}