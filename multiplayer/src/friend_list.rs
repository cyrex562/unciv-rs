use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::game::UncivGame;
use crate::logic::id_checker::IdChecker;
use crate::utils::translations::tr;

/// Represents a friend in the multiplayer system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub name: String,
    pub player_id: String,
}

/// Error types for friend list operations
#[derive(Debug, PartialEq)]
pub enum ErrorType {
    /// Friend name is already in the list
    Name,
    /// Player ID is already in the list
    Id,
    /// No name provided
    NoName,
    /// No ID provided
    NoId,
    /// Trying to add your own ID
    Yourself,
    /// Friend is already in the list
    AlreadyInList,
    /// Friend is not in the list
    NotInList,
    /// Operation successful
    Success,
}

/// Manages the list of friends for multiplayer
pub struct FriendList {
    friends: HashMap<String, Friend>,
    file_path: String,
}

impl FriendList {
    /// Create a new FriendList
    pub fn new() -> Self {
        let mut friend_list = Self {
            friends: HashMap::new(),
            file_path: String::new(),
        };
        friend_list.load();
        friend_list
    }

    /// Load friends from file
    fn load(&mut self) {
        self.file_path = format!(
            "{}/friends.json",
            UncivGame::current().settings.multiplayer.friends_directory
        );

        if Path::new(&self.file_path).exists() {
            match fs::read_to_string(&self.file_path) {
                Ok(contents) => match serde_json::from_str::<Vec<Friend>>(&contents) {
                    Ok(friends) => {
                        for friend in friends {
                            self.friends.insert(friend.name.clone(), friend);
                        }
                    }
                    Err(e) => eprintln!("Error parsing friends file: {}", e),
                },
                Err(e) => eprintln!("Error reading friends file: {}", e),
            }
        }
    }

    /// Save friends to file
    fn save(&self) {
        let friends_vec: Vec<&Friend> = self.friends.values().collect();

        match serde_json::to_string_pretty(&friends_vec) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.file_path, json) {
                    eprintln!("Error writing friends file: {}", e);
                }
            }
            Err(e) => eprintln!("Error serializing friends: {}", e),
        }
    }

    /// Add a friend to the list
    pub fn add(&mut self, name: String, player_id: String) -> ErrorType {
        // Check if name is empty
        if name.is_empty() {
            return ErrorType::NoName;
        }

        // Check if player ID is empty
        if player_id.is_empty() {
            return ErrorType::NoId;
        }

        // Check if trying to add yourself
        if player_id == UncivGame::current().settings.multiplayer.user_id {
            return ErrorType::Yourself;
        }

        // Check if name is already in the list
        if self.friends.contains_key(&name) {
            return ErrorType::Name;
        }

        // Check if player ID is already in the list
        for friend in self.friends.values() {
            if friend.player_id == player_id {
                return ErrorType::Id;
            }
        }

        // Add the friend
        let friend = Friend {
            name: name.clone(),
            player_id,
        };

        self.friends.insert(name, friend);
        self.save();

        ErrorType::Success
    }

    /// Edit a friend in the list
    pub fn edit(&mut self, friend: &Friend, new_name: String, new_player_id: String) -> ErrorType {
        // Check if name is empty
        if new_name.is_empty() {
            return ErrorType::NoName;
        }

        // Check if player ID is empty
        if new_player_id.is_empty() {
            return ErrorType::NoId;
        }

        // Check if trying to add yourself
        if new_player_id == UncivGame::current().settings.multiplayer.user_id {
            return ErrorType::Yourself;
        }

        // Check if name is already in the list (and not the same friend)
        if new_name != friend.name && self.friends.contains_key(&new_name) {
            return ErrorType::Name;
        }

        // Check if player ID is already in the list (and not the same friend)
        for existing_friend in self.friends.values() {
            if existing_friend.player_id == new_player_id && existing_friend.name != friend.name {
                return ErrorType::Id;
            }
        }

        // Remove the old friend
        self.friends.remove(&friend.name);

        // Add the edited friend
        let edited_friend = Friend {
            name: new_name.clone(),
            player_id: new_player_id,
        };

        self.friends.insert(new_name, edited_friend);
        self.save();

        ErrorType::Success
    }

    /// Delete a friend from the list
    pub fn delete(&mut self, friend: &Friend) {
        self.friends.remove(&friend.name);
        self.save();
    }

    /// Check if a friend name is in the list
    pub fn is_friend_name_in_friend_list(&self, name: &str) -> ErrorType {
        if self.friends.contains_key(name) {
            ErrorType::AlreadyInList
        } else {
            ErrorType::NotInList
        }
    }

    /// Check if a player ID is in the list
    pub fn is_friend_id_in_friend_list(&self, player_id: &str) -> ErrorType {
        for friend in self.friends.values() {
            if friend.player_id == player_id {
                return ErrorType::AlreadyInList;
            }
        }
        ErrorType::NotInList
    }

    /// Get the list of friends
    pub fn get_friends_list(&self) -> Vec<Friend> {
        self.friends.values().cloned().collect()
    }
}
