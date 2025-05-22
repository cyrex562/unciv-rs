use serde::{Deserialize, Serialize};
use std::collections::HashMap;


/// Type of Personality focus. Typically ranges from 0 (no focus) to 10 (double focus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersonalityValue {
    // Stat focused personalities
    Production,
    Food,
    Gold,
    Science,
    Culture,
    Happiness,
    Faith,
    // Behaviour focused personalities
    Military, // Building a military but not necessarily using it
    Aggressive, // How they use units aggressively or defensively in wars, or their priority on war related buildings
    DeclareWar, // Likelihood of declaring war and acceptance of war mongering, a zero means they won't declare war at all
    Commerce, // Trading frequency, open borders and liberating city-states, less negative diplomacy impact
    Diplomacy, // Likelihood of signing friendship, defensive pact, peace treaty and other diplomatic actions
    Loyal, // Likelihood to make a long lasting alliance with another civ and join wars with them
    Expansion, // Founding/capturing new cities, opposite of a cultural victory
    // Legacy values from original PersonalityValue enum
    BuildWonders,
    Research,
    UseNukes,
    UseEspionage,
    UseReligion,
    UseCulture,
}

impl PersonalityValue {
    /// Gets the PersonalityValue from a Stat
    pub fn from_stat(stat: Stat) -> Self {
        match stat {
            Stat::Production => PersonalityValue::Production,
            Stat::Food => PersonalityValue::Food,
            Stat::Gold => PersonalityValue::Gold,
            Stat::Science => PersonalityValue::Science,
            Stat::Culture => PersonalityValue::Culture,
            Stat::Happiness => PersonalityValue::Happiness,
            Stat::Faith => PersonalityValue::Faith,
        }
    }
}

/// Represents a personality in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Personality {
    pub ruleset_object: RulesetObject,
    /// Production focus value (0-10)
    pub production: f32,
    /// Food focus value (0-10)
    pub food: f32,
    /// Gold focus value (0-10)
    pub gold: f32,
    /// Science focus value (0-10)
    pub science: f32,
    /// Culture focus value (0-10)
    pub culture: f32,
    /// Happiness focus value (0-10)
    pub happiness: f32,
    /// Faith focus value (0-10)
    pub faith: f32,
    /// Military focus value (0-10)
    pub military: f32,
    /// Aggressive focus value (0-10)
    pub aggressive: f32,
    /// Declare war focus value (0-10)
    pub declare_war: f32,
    /// Commerce focus value (0-10)
    pub commerce: f32,
    /// Diplomacy focus value (0-10)
    pub diplomacy: f32,
    /// Loyal focus value (0-10)
    pub loyal: f32,
    /// Expansion focus value (0-10)
    pub expansion: f32,
    /// Priorities map
    pub priorities: HashMap<String, i32>,
    /// Preferred victory type
    pub preferred_victory_type: String,
    /// Whether this is a neutral personality
    pub is_neutral_personality: bool,
}

impl Personality {
    /// Creates a new Personality with default values
    pub fn new() -> Self {
        Personality {
            ruleset_object: RulesetObject::default(),
            production: 5.0,
            food: 5.0,
            gold: 5.0,
            science: 5.0,
            culture: 5.0,
            happiness: 5.0,
            faith: 5.0,
            military: 5.0,
            aggressive: 5.0,
            declare_war: 5.0,
            commerce: 5.0,
            diplomacy: 5.0,
            loyal: 5.0,
            expansion: 5.0,
            priorities: HashMap::new(),
            preferred_victory_type: "Neutral".to_string(),
            is_neutral_personality: false,
        }
    }

    /// Gets a neutral personality
    pub fn neutral() -> Self {
        let mut personality = Personality::new();
        personality.is_neutral_personality = true;
        personality
    }

    /// Gets the value for a personality value
    fn get_value(&self, value: PersonalityValue) -> f32 {
        match value {
            PersonalityValue::Production => self.production,
            PersonalityValue::Food => self.food,
            PersonalityValue::Gold => self.gold,
            PersonalityValue::Science => self.science,
            PersonalityValue::Culture => self.culture,
            PersonalityValue::Happiness => self.happiness,
            PersonalityValue::Faith => self.faith,
            PersonalityValue::Military => self.military,
            PersonalityValue::Aggressive => self.aggressive,
            PersonalityValue::DeclareWar => self.declare_war,
            PersonalityValue::Commerce => self.commerce,
            PersonalityValue::Diplomacy => self.diplomacy,
            PersonalityValue::Loyal => self.loyal,
            PersonalityValue::Expansion => self.expansion,
            // Legacy values - default to neutral value of 5.0
            PersonalityValue::BuildWonders => 5.0,
            PersonalityValue::Research => 5.0,
            PersonalityValue::UseNukes => 5.0,
            PersonalityValue::UseEspionage => 5.0,
            PersonalityValue::UseReligion => 5.0,
            PersonalityValue::UseCulture => 5.0,
        }
    }

    /// Sets the value for a personality value
    fn set_value(&mut self, value: PersonalityValue, new_value: f32) {
        match value {
            PersonalityValue::Production => self.production = new_value,
            PersonalityValue::Food => self.food = new_value,
            PersonalityValue::Gold => self.gold = new_value,
            PersonalityValue::Science => self.science = new_value,
            PersonalityValue::Culture => self.culture = new_value,
            PersonalityValue::Happiness => self.happiness = new_value,
            PersonalityValue::Faith => self.faith = new_value,
            PersonalityValue::Military => self.military = new_value,
            PersonalityValue::Aggressive => self.aggressive = new_value,
            PersonalityValue::DeclareWar => self.declare_war = new_value,
            PersonalityValue::Commerce => self.commerce = new_value,
            PersonalityValue::Diplomacy => self.diplomacy = new_value,
            PersonalityValue::Loyal => self.loyal = new_value,
            PersonalityValue::Expansion => self.expansion = new_value,
            // Legacy values - no-op for now
            PersonalityValue::BuildWonders => {},
            PersonalityValue::Research => {},
            PersonalityValue::UseNukes => {},
            PersonalityValue::UseEspionage => {},
            PersonalityValue::UseReligion => {},
            PersonalityValue::UseCulture => {},
        }
    }

    /// Scales the value to a more meaningful range, where 10 is 2, and 5 is 1, and 0 is 0
    pub fn scaled_focus(&self, value: PersonalityValue) -> f32 {
        self.get_value(value) / 5.0
    }

    /// Inverse scales the value to a more meaningful range, where 0 is 2, and 5 is 1 and 10 is 0
    pub fn inverse_scaled_focus(&self, value: PersonalityValue) -> f32 {
        (10.0 - self.get_value(value)) / 5.0
    }

    /// Returns a modifier between 0 and 2 centered around 1 based off of the personality value and the weight given
    ///
    /// # Arguments
    ///
    /// * `value` - The personality value to use
    /// * `weight` - A value between 0 and 1 that determines how much the modifier deviates from 1
    pub fn modifier_focus(&self, value: PersonalityValue, weight: f32) -> f32 {
        1.0 + (self.scaled_focus(value) - 1.0) * weight
    }

    /// An inverted version of modifier_focus, a personality value of 0 becomes a 10, 8 becomes a 2, etc.
    ///
    /// # Arguments
    ///
    /// * `value` - The personality value to use
    /// * `weight` - A value between 0 and 1 that determines how much the modifier deviates from 1
    pub fn inverse_modifier_focus(&self, value: PersonalityValue, weight: f32) -> f32 {
        1.0 - (self.inverse_scaled_focus(value) - 2.0) * weight
    }

    /// Scales the stats based on the personality and the weight given
    ///
    /// # Arguments
    ///
    /// * `stats` - The stats to scale
    /// * `weight` - A positive value that determines how much the personality should impact the stats given
    pub fn scale_stats(&self, mut stats: Stats, weight: f32) -> Stats {
        for stat in Stat::values() {
            stats[stat] *= self.modifier_focus(PersonalityValue::from_stat(stat), weight);
        }
        stats
    }

    pub fn origin_ruleset(&self) -> &str {
        &self.ruleset_object.origin_ruleset
    }

    pub fn set_origin_ruleset(&mut self, origin: String) {
        self.ruleset_object.origin_ruleset = origin;
    }
}
