use std::collections::HashSet;
use std::str::FromStr;

/// Types of parameters that can be used in uniques
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UniqueParameterType {
    /// A text parameter
    Text,
    /// A number parameter
    Number,
    /// A percentage parameter
    Percent,
    /// A stat parameter
    Stat,
    /// A tile parameter
    Tile,
    /// A unit parameter
    Unit,
    /// A building parameter
    Building,
    /// A tech parameter
    Tech,
    /// A policy parameter
    Policy,
    /// A promotion parameter
    Promotion,
    /// A resource parameter
    Resource,
    /// A terrain parameter
    Terrain,
    /// A nation parameter
    Nation,
    /// A belief parameter
    Belief,
    /// A great person parameter
    GreatPerson,
    /// A victory type parameter
    VictoryType,
    /// A difficulty parameter
    Difficulty,
    /// A speed parameter
    Speed,
    /// An era parameter
    Era,
    /// A combat type parameter
    CombatType,
    /// A combat modifier parameter
    CombatModifier,
    /// A combat bonus parameter
    CombatBonus,
    /// A combat penalty parameter
    CombatPenalty,
    /// A combat unit parameter
    CombatUnit,
    /// A combat terrain parameter
    CombatTerrain,
    /// A combat feature parameter
    CombatFeature,
}

impl UniqueParameterType {
    /// Check if a parameter value is valid for this type
    pub fn is_valid_parameter(&self, value: &str, ruleset: &Ruleset) -> bool {
        match self {
            Self::Number => i32::from_str(value).is_ok(),
            Self::Percent => {
                if let Ok(num) = i32::from_str(value) {
                    num >= -100 && num <= 100
                } else {
                    false
                }
            },
            Self::Stat => Stats::is_stats(value),
            Self::Tile => ruleset.terrains.contains_key(value) || ruleset.tile_improvements.contains_key(value),
            Self::Unit => ruleset.units.contains_key(value),
            Self::Building => ruleset.buildings.contains_key(value),
            Self::Tech => ruleset.technologies.contains_key(value),
            Self::Policy => ruleset.policies.contains_key(value),
            Self::Promotion => ruleset.unit_promotions.contains_key(value),
            Self::Resource => ruleset.tile_resources.contains_key(value),
            Self::Terrain => ruleset.terrains.contains_key(value),
            Self::Nation => ruleset.nations.contains_key(value),
            Self::Belief => ruleset.beliefs.contains_key(value),
            Self::GreatPerson => ruleset.great_people.contains_key(value),
            Self::VictoryType => ruleset.victories.contains_key(value),
            Self::Difficulty => ruleset.difficulties.contains_key(value),
            Self::Speed => ruleset.speeds.contains_key(value),
            Self::Era => ruleset.eras.contains_key(value),
            Self::CombatType => ruleset.combat_types.contains_key(value),
            Self::CombatModifier => ruleset.combat_modifiers.contains_key(value),
            Self::CombatBonus => ruleset.combat_bonuses.contains_key(value),
            Self::CombatPenalty => ruleset.combat_penalties.contains_key(value),
            Self::CombatUnit => ruleset.combat_units.contains_key(value),
            Self::CombatTerrain => ruleset.combat_terrains.contains_key(value),
            Self::CombatFeature => ruleset.combat_features.contains_key(value),
            _ => true, // Text type accepts any value
        }
    }

    /// Get a set of valid parameter values for this type
    pub fn get_valid_values(&self, ruleset: &Ruleset) -> HashSet<String> {
        match self {
            Self::Tile => {
                let mut values = ruleset.terrains.keys().cloned().collect::<HashSet<_>>();
                values.extend(ruleset.tile_improvements.keys().cloned());
                values
            },
            Self::Unit => ruleset.units.keys().cloned().collect(),
            Self::Building => ruleset.buildings.keys().cloned().collect(),
            Self::Tech => ruleset.technologies.keys().cloned().collect(),
            Self::Policy => ruleset.policies.keys().cloned().collect(),
            Self::Promotion => ruleset.unit_promotions.keys().cloned().collect(),
            Self::Resource => ruleset.tile_resources.keys().cloned().collect(),
            Self::Terrain => ruleset.terrains.keys().cloned().collect(),
            Self::Nation => ruleset.nations.keys().cloned().collect(),
            Self::Belief => ruleset.beliefs.keys().cloned().collect(),
            Self::GreatPerson => ruleset.great_people.keys().cloned().collect(),
            Self::VictoryType => ruleset.victories.keys().cloned().collect(),
            Self::Difficulty => ruleset.difficulties.keys().cloned().collect(),
            Self::Speed => ruleset.speeds.keys().cloned().collect(),
            Self::Era => ruleset.eras.keys().cloned().collect(),
            Self::CombatType => ruleset.combat_types.keys().cloned().collect(),
            Self::CombatModifier => ruleset.combat_modifiers.keys().cloned().collect(),
            Self::CombatBonus => ruleset.combat_bonuses.keys().cloned().collect(),
            Self::CombatPenalty => ruleset.combat_penalties.keys().cloned().collect(),
            Self::CombatUnit => ruleset.combat_units.keys().cloned().collect(),
            Self::CombatTerrain => ruleset.combat_terrains.keys().cloned().collect(),
            Self::CombatFeature => ruleset.combat_features.keys().cloned().collect(),
            _ => HashSet::new(), // Text, Number, Percent, and Stat types don't have predefined values
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_validation() {
        assert!(UniqueParameterType::Number.is_valid_parameter("42", &Ruleset::default()));
        assert!(UniqueParameterType::Number.is_valid_parameter("-42", &Ruleset::default()));
        assert!(!UniqueParameterType::Number.is_valid_parameter("not a number", &Ruleset::default()));
    }

    #[test]
    fn test_percent_validation() {
        assert!(UniqueParameterType::Percent.is_valid_parameter("50", &Ruleset::default()));
        assert!(UniqueParameterType::Percent.is_valid_parameter("-50", &Ruleset::default()));
        assert!(!UniqueParameterType::Percent.is_valid_parameter("150", &Ruleset::default()));
        assert!(!UniqueParameterType::Percent.is_valid_parameter("-150", &Ruleset::default()));
    }

    #[test]
    fn test_text_validation() {
        assert!(UniqueParameterType::Text.is_valid_parameter("any text", &Ruleset::default()));
    }
}