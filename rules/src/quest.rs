use std::collections::HashMap;
use std::fmt;

use crate::models::stats::INamed;

/// Represents the different types of quests in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestName {
    Route,
    ClearBarbarianCamp,
    ConstructWonder,
    ConnectResource,
    GreatPerson,
    ConquerCityState,
    FindPlayer,
    FindNaturalWonder,
    GiveGold,
    PledgeToProtect,
    ContestCulture,
    ContestFaith,
    ContestTech,
    Invest,
    BullyCityState,
    DenounceCiv,
    SpreadReligion,
    None,
}

impl QuestName {
    /// Gets the string value of the quest name
    pub fn value(&self) -> &'static str {
        match self {
            QuestName::Route => "Route",
            QuestName::ClearBarbarianCamp => "Clear Barbarian Camp",
            QuestName::ConstructWonder => "Construct Wonder",
            QuestName::ConnectResource => "Connect Resource",
            QuestName::GreatPerson => "Acquire Great Person",
            QuestName::ConquerCityState => "Conquer City State",
            QuestName::FindPlayer => "Find Player",
            QuestName::FindNaturalWonder => "Find Natural Wonder",
            QuestName::GiveGold => "Give Gold",
            QuestName::PledgeToProtect => "Pledge to Protect",
            QuestName::ContestCulture => "Contest Culture",
            QuestName::ContestFaith => "Contest Faith",
            QuestName::ContestTech => "Contest Technologies",
            QuestName::Invest => "Invest",
            QuestName::BullyCityState => "Bully City State",
            QuestName::DenounceCiv => "Denounce Civilization",
            QuestName::SpreadReligion => "Spread Religion",
            QuestName::None => "",
        }
    }

    /// Finds a QuestName from a string value
    pub fn find(value: &str) -> QuestName {
        match value {
            "Route" => QuestName::Route,
            "Clear Barbarian Camp" => QuestName::ClearBarbarianCamp,
            "Construct Wonder" => QuestName::ConstructWonder,
            "Connect Resource" => QuestName::ConnectResource,
            "Acquire Great Person" => QuestName::GreatPerson,
            "Conquer City State" => QuestName::ConquerCityState,
            "Find Player" => QuestName::FindPlayer,
            "Find Natural Wonder" => QuestName::FindNaturalWonder,
            "Give Gold" => QuestName::GiveGold,
            "Pledge to Protect" => QuestName::PledgeToProtect,
            "Contest Culture" => QuestName::ContestCulture,
            "Contest Faith" => QuestName::ContestFaith,
            "Contest Technologies" => QuestName::ContestTech,
            "Invest" => QuestName::Invest,
            "Bully City State" => QuestName::BullyCityState,
            "Denounce Civilization" => QuestName::DenounceCiv,
            "Spread Religion" => QuestName::SpreadReligion,
            _ => QuestName::None,
        }
    }
}

/// Represents the type of quest (Individual or Global)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestType {
    Individual,
    Global,
}

/// Represents a quest in the game
///
/// Notes: This is **not** part of game info serialization, only Ruleset.
/// Saves contain QuestManagers instead, which contain lists of AssignedQuest instances.
/// These are matched to this Quest **by name**.
pub struct Quest {
    /// Unique identifier name of the quest, it is also shown.
    /// Must match a QuestName.value for the Quest to have any functionality.
    pub name: String,

    /// Description of the quest shown to players
    pub description: String,

    /// QuestType: it is either Individual or Global
    pub quest_type: QuestType,

    /// Influence reward gained on quest completion
    pub influence: f32,

    /// Maximum number of turns to complete the quest, 0 if there's no turn limit
    pub duration: i32,

    /// Minimum number of Civilizations needed to start the quest. It is meaningful only for Global
    /// quests.
    pub minimum_civs: i32,

    /// Certain city states are more likely to give certain quests
    /// This is based on both personality and city-state type
    /// Both are mapped here as 'how much to multiply the weight of this quest for this kind of city-state'
    pub weight_for_city_state_type: HashMap<String, f32>,
}

impl Quest {
    /// Creates a new Quest instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            quest_type: QuestType::Individual,
            influence: 40.0,
            duration: 0,
            minimum_civs: 1,
            weight_for_city_state_type: HashMap::new(),
        }
    }

    /// Gets the QuestName instance for this quest
    pub fn quest_name_instance(&self) -> QuestName {
        QuestName::find(&self.name)
    }

    /// Checks if this is a Global quest
    pub fn is_global(&self) -> bool {
        self.quest_type == QuestType::Global
    }

    /// Checks if this is an Individual quest
    pub fn is_individual(&self) -> bool {
        !self.is_global()
    }
}

impl INamed for Quest {
    fn get_name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for Quest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}