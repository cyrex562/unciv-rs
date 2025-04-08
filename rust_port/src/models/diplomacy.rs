use std::collections::HashMap;
use crate::models::civilization::Civilization;

/// Represents the level of relationship between two civilizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipLevel {
    Unforgivable,
    Unfavorable,
    Neutral,
    Favorable,
    Friendly,
    Allied,
}

impl RelationshipLevel {
    /// Checks if this relationship level is less than the given level.
    pub fn is_lt(&self, other: RelationshipLevel) -> bool {
        (*self as i32) < (other as i32)
    }

    /// Checks if this relationship level is less than or equal to the given level.
    pub fn is_le(&self, other: RelationshipLevel) -> bool {
        (*self as i32) <= (other as i32)
    }

    /// Checks if this relationship level is equal to the given level.
    pub fn is_eq(&self, other: RelationshipLevel) -> bool {
        *self == other
    }

    /// Checks if this relationship level is greater than or equal to the given level.
    pub fn is_ge(&self, other: RelationshipLevel) -> bool {
        (*self as i32) >= (other as i32)
    }
}

/// Represents the diplomatic status between two civilizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomaticStatus {
    Peace,
    War,
    DefensivePact,
    // Add other statuses as needed
}

/// Flags that can be set in diplomacy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiplomacyFlags {
    WaryOf,
    DeclinedJoinWarOffer,
    // Add other flags as needed
}

/// Manages diplomacy between civilizations.
pub struct DiplomacyManager {
    flags: HashMap<DiplomacyFlags, i32>,
    diplomatic_status: DiplomaticStatus,
    relationship_level: RelationshipLevel,
    opinion: f32,
    other_civ_name: String,
}

impl DiplomacyManager {
    /// Creates a new DiplomacyManager.
    pub fn new(other_civ_name: String) -> Self {
        DiplomacyManager {
            flags: HashMap::new(),
            diplomatic_status: DiplomaticStatus::Peace,
            relationship_level: RelationshipLevel::Neutral,
            opinion: 0.0,
            other_civ_name,
        }
    }

    /// Checks if the given flag is set.
    pub fn has_flag(&self, flag: DiplomacyFlags) -> bool {
        self.flags.contains_key(&flag)
    }

    /// Gets the value of the given flag.
    pub fn get_flag(&self, flag: DiplomacyFlags) -> i32 {
        *self.flags.get(&flag).unwrap_or(&0)
    }

    /// Sets the value of the given flag.
    pub fn set_flag(&mut self, flag: DiplomacyFlags, value: i32) {
        self.flags.insert(flag, value);
    }

    /// Gets the diplomatic status.
    pub fn get_diplomatic_status(&self) -> DiplomaticStatus {
        self.diplomatic_status
    }

    /// Sets the diplomatic status.
    pub fn set_diplomatic_status(&mut self, status: DiplomaticStatus) {
        self.diplomatic_status = status;
    }

    /// Gets the relationship level.
    pub fn get_relationship_level(&self) -> RelationshipLevel {
        self.relationship_level
    }

    /// Sets the relationship level.
    pub fn set_relationship_level(&mut self, level: RelationshipLevel) {
        self.relationship_level = level;
    }

    /// Gets the opinion of the other civilization.
    pub fn opinion_of_other_civ(&self) -> f32 {
        self.opinion
    }

    /// Sets the opinion of the other civilization.
    pub fn set_opinion(&mut self, opinion: f32) {
        self.opinion = opinion;
    }

    /// Checks if the relationship level is less than the given level.
    pub fn is_relationship_level_lt(&self, level: RelationshipLevel) -> bool {
        self.relationship_level.is_lt(level)
    }

    /// Checks if the relationship level is less than or equal to the given level.
    pub fn is_relationship_level_le(&self, level: RelationshipLevel) -> bool {
        self.relationship_level.is_le(level)
    }

    /// Checks if the relationship level is equal to the given level.
    pub fn is_relationship_level_eq(&self, level: RelationshipLevel) -> bool {
        self.relationship_level.is_eq(level)
    }

    /// Checks if the relationship level is greater than or equal to the given level.
    pub fn is_relationship_level_ge(&self, level: RelationshipLevel) -> bool {
        self.relationship_level.is_ge(level)
    }

    /// Declares war on the other civilization.
    pub fn declare_war(&mut self) {
        self.diplomatic_status = DiplomaticStatus::War;
    }

    /// Gets all civilizations that both this civilization and the other civilization know.
    pub fn get_common_known_civs(&self) -> Vec<&Civilization> {
        // Placeholder implementation
        Vec::new()
    }
}