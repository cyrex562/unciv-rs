use std::fmt;

use crate::logic::GameInfo;
use crate::models::ruleset::RulesetObject;
use crate::models::ruleset::unique::UniqueTarget;

/// Represents a reward that can be obtained from ruins in the game
pub struct RuinReward {
    // Base RulesetObject fields
    pub name: String,
    pub uniques: Vec<String>,
    pub unique_objects: Vec<Unique>,
    pub unique_map: UniqueMap,

    // RuinReward specific fields
    pub notification: String,
    pub excluded_difficulties: Vec<String>,
    pub weight: i32,
    pub color: String,  // For Civilopedia
}

impl RuinReward {
    /// Creates a new RuinReward instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            uniques: Vec::new(),
            unique_objects: Vec::new(),
            unique_map: UniqueMap::new(),
            notification: String::new(),
            excluded_difficulties: Vec::new(),
            weight: 1,
            color: String::new(),
        }
    }
}

impl RulesetObject for RuinReward {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_uniques(&self) -> &[String] {
        &self.uniques
    }

    fn get_unique_objects(&self) -> &[Unique] {
        &self.unique_objects
    }

    fn get_unique_map(&self) -> &UniqueMap {
        &self.unique_map
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Ruins
    }

    fn make_link(&self) -> String {
        String::new() // No own category on Civilopedia screen
    }

    fn is_unavailable_by_settings(&self, game_info: &GameInfo) -> bool {
        self.excluded_difficulties.contains(&game_info.difficulty) ||
        super::is_unavailable_by_settings(self, game_info)
    }
}

impl fmt::Display for RuinReward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}