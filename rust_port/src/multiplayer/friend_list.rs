use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::game::unciv_game::UncivGame;

/// Error types that can occur when managing friends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// No error occurred
    NoError,
    /// Error with the friend's name
    Name,
    /// Error with the friend's ID
    Id,
    /// Friend name is empty
    NoName,
    /// Friend ID is empty
    NoId,
    /// Cannot add yourself as a friend
    Yourself,
    /// Friend is already in the list
    AlreadyInList,
}

/// A friend in the friend list
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Friend {
    /// The friend's display name
    pub name: String,
    /// The friend's player ID
    pub player_id: String,
}

impl Friend {
    /// Create a new friend with the given name and player ID
    pub fn new(name: String, player_id: String) -> Self {
        Self { name, player_id }
    }

    /// Create an empty friend
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            player_id: String::new(),
        }
    }
}

/// A list of friends for multiplayer functionality
pub struct FriendList {
    /// The list of friends
    list_of_friends: Vec<Friend>,
}

impl FriendList {
    /// Create a new friend list
    pub fn new() -> Self {
        let settings = UncivGame::current().settings.clone();
        Self {
            list_of_friends: settings.multiplayer.friend_list.clone(),
        }
    }

    /// Add a friend to the list
    ///
    /// # Parameters
    ///
    /// * `friend_name` - The name of the friend to add
    /// * `player_id` - The player ID of the friend to add
    ///
    /// # Returns
    ///
    /// An error type indicating the result of the operation
    pub fn add(&mut self, friend_name: &str, player_id: &str) -> ErrorType {
        // Check if the friend name or ID already exists
        for friend in &self.list_of_friends {
            if friend.name == friend_name {
                return ErrorType::Name;
            } else if friend.player_id == player_id {
                return ErrorType::Id;
            }
        }

        // Validate input
        if friend_name.is_empty() {
            return ErrorType::NoName;
        } else if player_id.is_empty() {
            return ErrorType::NoId;
        } else if player_id == UncivGame::current().settings.multiplayer.user_id {
            return ErrorType::Yourself;
        }

        // Add the friend
        self.list_of_friends.push(Friend::new(friend_name.to_string(), player_id.to_string()));

        // Save the settings
        let mut settings = UncivGame::current().settings.clone();
        settings.multiplayer.friend_list = self.list_of_friends.clone();
        settings.save();

        ErrorType::NoError
    }

    /// Edit a friend in the list
    ///
    /// # Parameters
    ///
    /// * `friend` - The friend to edit
    /// * `name` - The new name for the friend
    /// * `player_id` - The new player ID for the friend
    pub fn edit(&mut self, friend: &Friend, name: &str, player_id: &str) {
        // Remove the old friend
        self.list_of_friends.retain(|f| f != friend);

        // Add the edited friend
        let edited_friend = Friend::new(name.to_string(), player_id.to_string());
        self.list_of_friends.push(edited_friend);

        // Save the settings
        let mut settings = UncivGame::current().settings.clone();
        settings.multiplayer.friend_list = self.list_of_friends.clone();
        settings.save();
    }

    /// Delete a friend from the list
    ///
    /// # Parameters
    ///
    /// * `friend` - The friend to delete
    pub fn delete(&mut self, friend: &Friend) {
        // Remove the friend
        self.list_of_friends.retain(|f| f != friend);

        // Save the settings
        let mut settings = UncivGame::current().settings.clone();
        settings.multiplayer.friend_list = self.list_of_friends.clone();
        settings.save();
    }

    /// Get the list of friends
    ///
    /// # Returns
    ///
    /// A reference to the list of friends
    pub fn get_friends_list(&self) -> &Vec<Friend> {
        &self.list_of_friends
    }

    /// Check if a friend name is in the list
    ///
    /// # Parameters
    ///
    /// * `name` - The name to check
    ///
    /// # Returns
    ///
    /// An error type indicating the result of the check
    pub fn is_friend_name_in_friend_list(&self, name: &str) -> ErrorType {
        if self.list_of_friends.iter().any(|f| f.name == name) {
            ErrorType::AlreadyInList
        } else {
            ErrorType::NoError
        }
    }

    /// Check if a friend ID is in the list
    ///
    /// # Parameters
    ///
    /// * `id` - The ID to check
    ///
    /// # Returns
    ///
    /// An error type indicating the result of the check
    pub fn is_friend_id_in_friend_list(&self, id: &str) -> ErrorType {
        if self.list_of_friends.iter().any(|f| f.player_id == id) {
            ErrorType::AlreadyInList
        } else {
            ErrorType::NoError
        }
    }

    /// Get a friend by ID
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the friend to get
    ///
    /// # Returns
    ///
    /// A reference to the friend, or None if not found
    pub fn get_friend_by_id(&self, id: &str) -> Option<&Friend> {
        self.list_of_friends.iter().find(|f| f.player_id == id)
    }

    /// Get a friend by name
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the friend to get
    ///
    /// # Returns
    ///
    /// A reference to the friend, or None if not found
    pub fn get_friend_by_name(&self, name: &str) -> Option<&Friend> {
        self.list_of_friends.iter().find(|f| f.name == name)
    }
}