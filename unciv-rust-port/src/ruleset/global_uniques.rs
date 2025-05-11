use std::fmt;

use crate::models::ruleset::{
    Ruleset, RulesetObject, unique::{Unique, UniqueType, UniqueTarget},
};

/// Represents global uniques in the game
pub struct GlobalUniques {
    /// Base ruleset object that this global uniques extends
    base: RulesetObject,

    /// The name of the global uniques
    name: String,

    /// The uniques associated with this global uniques
    uniques: Vec<String>,

    /// The unit uniques associated with this global uniques
    unit_uniques: Vec<String>,

    /// The ruleset this global uniques belongs to
    ruleset: Option<Ruleset>,
}

impl GlobalUniques {
    /// Creates a new empty global uniques
    pub fn new() -> Self {
        Self {
            base: RulesetObject::new(),
            name: "GlobalUniques".to_string(),
            uniques: Vec::new(),
            unit_uniques: Vec::new(),
            ruleset: None,
        }
    }

    /// Gets the description of the source of a unique
    pub fn get_unique_source_description(unique: &Unique) -> String {
        if unique.modifiers.is_empty() {
            return "Global Effect".to_string();
        }

        match unique.modifiers.first().unwrap().type_ {
            UniqueType::ConditionalGoldenAge => "Golden Age".to_string(),
            UniqueType::ConditionalHappy => "Happiness".to_string(),
            UniqueType::ConditionalBetweenHappiness | UniqueType::ConditionalBelowHappiness => "Unhappiness".to_string(),
            UniqueType::ConditionalWLTKD => "We Love The King Day".to_string(),
            _ => "Global Effect".to_string(),
        }
    }
}

impl RulesetObject for GlobalUniques {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn uniques(&self) -> &[String] {
        &self.uniques
    }

    fn uniques_mut(&mut self) -> &mut Vec<String> {
        &mut self.uniques
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Global
    }

    fn make_link(&self) -> String {
        String::new() // No own category on Civilopedia screen
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        vec![]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Display for GlobalUniques {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_uniques_creation() {
        let global_uniques = GlobalUniques::new();
        assert_eq!(global_uniques.name, "GlobalUniques");
        assert!(global_uniques.uniques.is_empty());
        assert!(global_uniques.unit_uniques.is_empty());
    }

    #[test]
    fn test_get_unique_source_description() {
        // Test with empty modifiers
        let empty_unique = Unique::new();
        assert_eq!(
            GlobalUniques::get_unique_source_description(&empty_unique),
            "Global Effect"
        );

        // Test with ConditionalGoldenAge
        let mut golden_age_unique = Unique::new();
        golden_age_unique.modifiers.push(UniqueModifier::new(UniqueType::ConditionalGoldenAge));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&golden_age_unique),
            "Golden Age"
        );

        // Test with ConditionalHappy
        let mut happy_unique = Unique::new();
        happy_unique.modifiers.push(UniqueModifier::new(UniqueType::ConditionalHappy));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&happy_unique),
            "Happiness"
        );

        // Test with ConditionalBetweenHappiness
        let mut between_happy_unique = Unique::new();
        between_happy_unique.modifiers.push(UniqueModifier::new(UniqueType::ConditionalBetweenHappiness));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&between_happy_unique),
            "Unhappiness"
        );

        // Test with ConditionalBelowHappiness
        let mut below_happy_unique = Unique::new();
        below_happy_unique.modifiers.push(UniqueModifier::new(UniqueType::ConditionalBelowHappiness));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&below_happy_unique),
            "Unhappiness"
        );

        // Test with ConditionalWLTKD
        let mut wltkd_unique = Unique::new();
        wltkd_unique.modifiers.push(UniqueModifier::new(UniqueType::ConditionalWLTKD));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&wltkd_unique),
            "We Love The King Day"
        );

        // Test with unknown unique type
        let mut unknown_unique = Unique::new();
        unknown_unique.modifiers.push(UniqueModifier::new(UniqueType::Unknown));
        assert_eq!(
            GlobalUniques::get_unique_source_description(&unknown_unique),
            "Global Effect"
        );
    }
}