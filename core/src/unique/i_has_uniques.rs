use std::collections::HashSet;
use std::sync::OnceLock;

use crate::models::game::UncivGame;
use crate::models::game_info::GameInfo;
use crate::models::ruleset::{Ruleset, UniqueTarget};
use crate::models::ruleset::tech::{Era, TechColumn, Technology};
use crate::models::stats::INamed;
use crate::models::ui::to_percent;
use crate::models::unique::{Unique, UniqueMap};
use crate::models::unique::conditionals::Conditionals;
use crate::models::ruleset::StateForConditionals;
use crate::ruleset::tech::era::Era;
use crate::ruleset::tech::tech_column::TechColumn;
use crate::ruleset::unique::conditionals::Conditionals;
use crate::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::ruleset::unique::unique::UniqueMap;
use crate::ruleset::unique::unique_target::UniqueTarget;
use crate::ui::screens::civilopediascreen::formatted_line::INamed;
use crate::unique_type::UniqueType;

/// Common trait for all 'ruleset objects' that have Uniques, like BaseUnit, Nation, etc.
pub trait IHasUniques: INamed {
    /// The list of unique abilities as strings
    fn uniques(&self) -> &Vec<String>; // Can not be a hashset as that would remove doubles
    fn uniques_mut(&mut self) -> &mut Vec<String>;

    /// The list of unique abilities as Unique objects
    fn unique_objects(&self) -> &Vec<Unique> {
        static UNIQUE_OBJECTS: OnceLock<Vec<Unique>> = OnceLock::new();
        UNIQUE_OBJECTS.get_or_init(|| self.unique_objects_provider())
    }

    /// The map of unique abilities
    fn unique_map(&self) -> &UniqueMap {
        static UNIQUE_MAP: OnceLock<UniqueMap> = OnceLock::new();
        UNIQUE_MAP.get_or_init(|| self.unique_map_provider())
    }

    /// Provider for unique objects
    fn unique_objects_provider(&self) -> Vec<Unique> {
        self.unique_objects_provider_with_uniques(self.uniques())
    }

    /// Provider for unique map
    fn unique_map_provider(&self) -> UniqueMap {
        self.unique_map_provider_with_objects(self.unique_objects())
    }

    /// Provider for unique objects with a specific list of uniques
    fn unique_objects_provider_with_uniques(&self, uniques: &[String]) -> Vec<Unique> {
        if uniques.is_empty() {
            return Vec::new();
        }
        uniques.iter()
            .map(|u| Unique::new(u, self.get_unique_target(), self.name()))
            .collect()
    }

    /// Provider for unique map with a specific list of unique objects
    fn unique_map_provider_with_objects(&self, unique_objects: &[Unique]) -> UniqueMap {
        let mut new_unique_map = UniqueMap::new();
        if !unique_objects.is_empty() {
            new_unique_map.add_uniques(unique_objects);
        }
        new_unique_map
    }

    /// Get the unique target for this object
    fn get_unique_target(&self) -> UniqueTarget;

    /// Get matching uniques by type
    fn get_matching_uniques_by_type(&self, unique_type: UniqueType, state: &StateForConditionals) -> Vec<&Unique> {
        self.unique_map().get_matching_uniques(unique_type, state)
    }

    /// Get matching uniques by tag
    fn get_matching_uniques_by_tag(&self, unique_tag: &str, state: &StateForConditionals) -> Vec<&Unique> {
        self.unique_map().get_matching_uniques(unique_tag, state)
    }

    /// Check if this object has a unique by type
    fn has_unique_by_type(&self, unique_type: UniqueType, state: Option<&StateForConditionals>) -> bool {
        let state = state.unwrap_or(&StateForConditionals::empty_state());
        self.unique_map().has_matching_unique(unique_type, state)
    }

    /// Check if this object has a unique by tag
    fn has_unique_by_tag(&self, unique_tag: &str, state: Option<&StateForConditionals>) -> bool {
        let state = state.unwrap_or(&StateForConditionals::empty_state());
        self.unique_map().has_matching_unique(unique_tag, state)
    }

    /// Check if this object has a tag unique
    fn has_tag_unique(&self, tag_unique: &str) -> bool {
        self.unique_map().has_tag_unique(tag_unique)
    }

    /// Get availability uniques
    fn availability_uniques(&self) -> Vec<&Unique> {
        let mut result = self.get_matching_uniques_by_type(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals());
        result.extend(self.get_matching_uniques_by_type(UniqueType::CanOnlyBeBuiltWhen, &StateForConditionals::ignore_conditionals()));
        result
    }

    /// Get techs required by uniques
    fn techs_required_by_uniques(&self) -> Vec<String> {
        self.availability_uniques()
            .iter()
            .flat_map(|unique| &unique.modifiers)
            .filter(|modifier| modifier.unique_type == UniqueType::ConditionalTech)
            .map(|modifier| modifier.params[0].clone())
            .collect()
    }

    /// Get legacy required techs
    fn legacy_required_techs(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get all required techs
    fn required_techs(&self) -> Vec<String> {
        let mut result = self.legacy_required_techs();
        result.extend(self.techs_required_by_uniques());
        result
    }

    /// Get required technologies
    fn required_technologies(&self, ruleset: &Ruleset) -> Vec<Option<&Technology>> {
        self.required_techs()
            .iter()
            .map(|tech_name| ruleset.technologies.get(tech_name))
            .collect()
    }

    /// Get the era of this object
    fn era(&self, ruleset: &Ruleset) -> Option<&Era> {
        self.required_technologies(ruleset)
            .iter()
            .filter_map(|tech| tech.map(|t| t.era()))
            .filter_map(|era_name| ruleset.eras.get(era_name))
            .max_by_key(|era| era.era_number)
    }

    /// Get the tech column of this object
    fn tech_column(&self, ruleset: &Ruleset) -> Option<&TechColumn> {
        self.required_technologies(ruleset)
            .iter()
            .filter_map(|tech| tech.and_then(|t| t.column.as_ref()))
            .max_by_key(|column| column.column_number)
    }

    /// Check if this object is available in a specific era
    fn available_in_era(&self, ruleset: &Ruleset, requested_era: &str) -> bool {
        let era_available = match self.era(ruleset) {
            Some(era) => era,
            None => return true, // No technologies are required, so available in the starting era.
        };

        // This is not very efficient, because era() inspects the eraNumbers and then returns the whole object.
        // We could take a max of the eraNumbers directly.
        // But it's unlikely to make any significant difference.
        // Currently this is only used in CityStateFunctions.kt.
        let requested_era_obj = ruleset.eras.get(requested_era).unwrap();
        era_available.era_number <= requested_era_obj.era_number
    }

    /// Get weight for AI decision
    fn get_weight_for_ai_decision(&self, state_for_conditionals: &StateForConditionals) -> f32 {
        let mut weight = 1.0;
        for unique in self.get_matching_uniques_by_type(UniqueType::AiChoiceWeight, state_for_conditionals) {
            weight *= to_percent(&unique.params[0]);
        }
        weight
    }

    /// Check if this object is unavailable by settings
    fn is_unavailable_by_settings(&self, game_info: &GameInfo) -> bool {
        let game_based_conditionals = HashSet::from([
            UniqueType::ConditionalVictoryDisabled,
            UniqueType::ConditionalVictoryEnabled,
            UniqueType::ConditionalSpeed,
            UniqueType::ConditionalDifficulty,
            UniqueType::ConditionalReligionEnabled,
            UniqueType::ConditionalReligionDisabled,
            UniqueType::ConditionalEspionageEnabled,
            UniqueType::ConditionalEspionageDisabled,
        ]);

        let state_for_conditionals = StateForConditionals::new(Some(game_info), None, None);

        // Check if any unavailable uniques match the game-based conditionals
        if self.get_matching_uniques_by_type(UniqueType::Unavailable, &StateForConditionals::ignore_conditionals())
            .iter()
            .any(|unique| {
                unique.modifiers.iter().any(|modifier| {
                    game_based_conditionals.contains(&modifier.unique_type) &&
                    Conditionals::conditional_applies(None, modifier, &state_for_conditionals)
                })
            }) {
            return true;
        }

        // Check if any only-available uniques don't match the game-based conditionals
        if self.get_matching_uniques_by_type(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals())
            .iter()
            .any(|unique| {
                unique.modifiers.iter().any(|modifier| {
                    game_based_conditionals.contains(&modifier.unique_type) &&
                    !Conditionals::conditional_applies(None, modifier, &state_for_conditionals)
                })
            }) {
            return true;
        }

        false
    }

    /// Check if this object is hidden from civilopedia
    fn is_hidden_from_civilopedia_with_params(&self, game_info: Option<&GameInfo>, ruleset: Option<&Ruleset>) -> bool {
        if self.has_unique_by_type(UniqueType::HiddenFromCivilopedia, None) {
            return true;
        }

        if let Some(game_info) = game_info {
            if self.is_unavailable_by_settings(game_info) {
                return true;
            }
        }

        if game_info.is_none() {
            if let Some(ruleset) = ruleset {
                if ruleset.beliefs.is_empty() {
                    return true;
                }
            } else {
                panic!("Both game_info and ruleset are None");
            }
        }

        false
    }

    /// Check if this object is hidden from civilopedia with game info
    fn is_hidden_from_civilopedia_with_game_info(&self, game_info: &GameInfo, ruleset: Option<&Ruleset>) -> bool {
        self.is_hidden_from_civilopedia_with_params(Some(game_info), ruleset)
    }

    /// Check if this object is hidden from civilopedia with ruleset
    fn is_hidden_from_civilopedia_with_ruleset(&self, ruleset: &Ruleset) -> bool {
        self.is_hidden_from_civilopedia_with_params(UncivGame::get_game_info_or_null(), Some(ruleset))
    }
}

#[cfg(test)]
mod tests {
    use crate::ruleset::unique::unique_target::UniqueTarget;
    use crate::ui::screens::civilopediascreen::formatted_line::INamed;
    use super::*;

    // Mock implementation for testing
    struct MockHasUniques {
        name: String,
        uniques: Vec<String>,
    }

    impl INamed for MockHasUniques {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl IHasUniques for MockHasUniques {
        fn uniques(&self) -> &Vec<String> {
            &self.uniques
        }

        fn uniques_mut(&mut self) -> &mut Vec<String> {
            &mut self.uniques
        }

        fn get_unique_target(&self) -> UniqueTarget {
            UniqueTarget::Unit
        }
    }

    #[test]
    fn test_has_unique_by_type() {
        let mut mock = MockHasUniques {
            name: "Test".to_string(),
            uniques: vec!["Test unique".to_string()],
        };

        assert!(mock.has_unique_by_type(UniqueType::HiddenFromCivilopedia, None));
    }
}