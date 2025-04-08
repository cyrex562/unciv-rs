use std::collections::{HashMap, HashSet};
use crate::models::map_unit::MapUnit;
use crate::models::game_info::GameInfo;
use crate::models::diplomacy::{DiplomacyManager, RelationshipLevel};
use crate::models::personality::PersonalityValue;
use crate::models::stats::RankingType;
use crate::models::trade::TradeRequest;

/// Represents a civilization in the game.
pub struct Civilization {
    pub units: UnitManager,
    pub game_info: GameInfo,
    pub popup_alerts: Vec<PopupAlert>,
    pub name: String,
    pub is_barbarian: bool,
    pub is_spectator: bool,
    pub is_city_state: bool,
    pub personality: HashMap<PersonalityValue, f32>,
    pub diplomacy_managers: HashMap<String, DiplomacyManager>,
    pub threat_manager: ThreatManager,
    pub cities: Vec<City>,
    pub tech: TechManager,
    pub trade_requests: Vec<TradeRequest>,
}

impl Civilization {
    /// Gets all units belonging to this civilization.
    pub fn get_civ_units(&self) -> Vec<&MapUnit> {
        self.units.get_civ_units()
    }

    /// Gets the personality value for the given personality type.
    pub fn get_personality(&self) -> &HashMap<PersonalityValue, f32> {
        &self.personality
    }

    /// Gets the diplomacy manager for the given civilization.
    pub fn get_diplomacy_manager(&self, other_civ: &Civilization) -> Option<&DiplomacyManager> {
        self.diplomacy_managers.get(&other_civ.name)
    }

    /// Gets the diplomacy manager for the given civilization (mutable).
    pub fn get_diplomacy_manager_mut(&mut self, other_civ: &Civilization) -> Option<&mut DiplomacyManager> {
        self.diplomacy_managers.get_mut(&other_civ.name)
    }

    /// Gets the stat for the given ranking type.
    pub fn get_stat_for_ranking(&self, ranking_type: RankingType) -> f32 {
        match ranking_type {
            RankingType::Force => self.threat_manager.get_force(),
            RankingType::Score => self.threat_manager.get_score(),
            _ => 0.0, // Placeholder for other ranking types
        }
    }

    /// Gets the capital city of this civilization.
    pub fn get_capital(&self) -> Option<&City> {
        self.cities.iter().find(|city| city.is_capital)
    }

    /// Gets all civilizations this civilization is at war with.
    pub fn get_civs_at_war_with(&self) -> Vec<&Civilization> {
        let mut war_civs = Vec::new();
        for (civ_name, diplo_manager) in &self.diplomacy_managers {
            if diplo_manager.get_diplomatic_status() == crate::models::diplomacy::DiplomaticStatus::War {
                // This is a placeholder - in a real implementation, we would look up the civilization by name
                // For now, we'll just return an empty vector
            }
        }
        war_civs
    }

    /// Checks if this civilization is at war with the given civilization.
    pub fn is_at_war_with(&self, other_civ: &Civilization) -> bool {
        if let Some(diplo_manager) = self.get_diplomacy_manager(other_civ) {
            diplo_manager.get_diplomatic_status() == crate::models::diplomacy::DiplomaticStatus::War
        } else {
            false
        }
    }

    /// Checks if this civilization is a major civilization.
    pub fn is_major_civ(&self) -> bool {
        !self.is_barbarian && !self.is_city_state && !self.is_spectator
    }

    /// Checks if this civilization has the given unique type.
    pub fn has_unique(&self, unique_type: crate::models::ruleset::UniqueType) -> bool {
        // Placeholder implementation
        false
    }

    /// Checks if this civilization has explored the given tile.
    pub fn has_explored(&self, tile: &crate::models::tile::Tile) -> bool {
        // Placeholder implementation
        false
    }

    /// Adds a notification to this civilization.
    pub fn add_notification(
        &mut self,
        message: String,
        position: crate::models::game_info::Position,
        category: crate::models::civilization::NotificationCategory,
        icon: crate::models::civilization::NotificationIcon,
    ) {
        // Placeholder implementation
    }

    /// Sets the last seen improvement at the given position.
    pub fn set_last_seen_improvement(
        &mut self,
        position: crate::models::game_info::Position,
        improvement: String,
    ) {
        // Placeholder implementation
    }

    /// Checks if this civilization is defeated.
    pub fn is_defeated(&self) -> bool {
        // Placeholder implementation
        false
    }
}

/// Manages units for a civilization.
pub struct UnitManager {
    units: Vec<MapUnit>,
}

impl UnitManager {
    /// Gets all units belonging to this civilization.
    pub fn get_civ_units(&self) -> Vec<&MapUnit> {
        self.units.iter().collect()
    }
}

/// Represents a popup alert in the game.
pub struct PopupAlert {
    pub message: String,
}

impl PopupAlert {
    pub fn new(message: String) -> Self {
        PopupAlert { message }
    }
}

/// Manages threats for a civilization.
pub struct ThreatManager {
    force: f32,
    score: f32,
}

impl ThreatManager {
    /// Creates a new ThreatManager.
    pub fn new() -> Self {
        ThreatManager {
            force: 0.0,
            score: 0.0,
        }
    }

    /// Gets the force of this civilization.
    pub fn get_force(&self) -> f32 {
        self.force
    }

    /// Gets the score of this civilization.
    pub fn get_score(&self) -> f32 {
        self.score
    }

    /// Gets the combined force of all civilizations at war with this civilization.
    pub fn get_combined_force_of_warring_civs(&self) -> f32 {
        // Placeholder implementation
        0.0
    }

    /// Gets all neighboring civilizations.
    pub fn get_neighboring_civilizations(&self) -> HashSet<&Civilization> {
        // Placeholder implementation
        HashSet::new()
    }
}

/// Represents a city in the game.
pub struct City {
    pub name: String,
    pub is_capital: bool,
}

impl City {
    /// Gets the center tile of this city.
    pub fn get_center_tile(&self) -> crate::models::tile::Tile {
        // Placeholder implementation
        crate::models::tile::Tile::new()
    }
}

/// Manages technology for a civilization.
pub struct TechManager {
    pub techs_researched: HashSet<String>,
}

impl TechManager {
    /// Creates a new TechManager.
    pub fn new() -> Self {
        TechManager {
            techs_researched: HashSet::new(),
        }
    }
}

/// Represents a notification category in the game.
pub enum NotificationCategory {
    War,
    // Add other categories as needed
}

/// Represents a notification icon in the game.
pub enum NotificationIcon {
    War,
    // Add other icons as needed
}