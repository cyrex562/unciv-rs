use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::constants::ALL;
use crate::logic::civilization::Civilization;
use crate::logic::multi_filter::MultiFilter;
use crate::models::ruleset::unique::{StateForConditionals, Unique, UniqueTarget, UniqueType};
use crate::models::ruleset::{Ruleset, RulesetObject};
use crate::ui::objectdescriptions::TechnologyDescriptions;

/// Replace a changed tech name
pub fn replace_updated_tech_name(
    tech_manager: &mut TechManager,
    old_tech_name: &str,
    new_tech_name: &str,
) {
    if tech_manager.techs_researched.contains(old_tech_name) {
        tech_manager.techs_researched.remove(old_tech_name);
        tech_manager
            .techs_researched
            .insert(new_tech_name.to_string());
    }

    if let Some(index) = tech_manager
        .techs_to_research
        .iter()
        .position(|t| t == old_tech_name)
    {
        tech_manager.techs_to_research[index] = new_tech_name.to_string();
    }

    if tech_manager.techs_in_progress.contains_key(old_tech_name) {
        let research = tech_manager.research_of_tech(old_tech_name);
        tech_manager
            .techs_in_progress
            .insert(new_tech_name.to_string(), research);
        tech_manager.techs_in_progress.remove(old_tech_name);
    }
}

/// Represents a technology in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Technology {
    /// The name of the technology
    pub name: String,
    /// The cost of researching this technology
    pub cost: i32,
    /// The prerequisites for this technology
    pub prerequisites: HashSet<String>,
    /// The column that this tech is in the tech tree
    pub column: Option<Box<crate::models::ruleset::tech::TechColumn>>,
    /// The row in the tech tree
    pub row: i32,
    /// A quote associated with this technology
    pub quote: String,
    /// The uniques associated with this technology
    pub uniques: Vec<Unique>,
}

impl Technology {
    /// Creates a new Technology with default values
    pub fn new() -> Self {
        Technology {
            name: String::new(),
            cost: 0,
            prerequisites: HashSet::new(),
            column: None,
            row: 0,
            quote: String::new(),
            uniques: Vec::new(),
        }
    }

    /// Gets the era this technology belongs to
    pub fn era(&self) -> String {
        self.column
            .as_ref()
            .map(|col| col.era.clone())
            .unwrap_or_default()
    }

    /// Checks if this technology is continuously researchable
    pub fn is_continually_researchable(&self) -> bool {
        self.has_unique(UniqueType::ResearchableMultipleTimes)
    }

    /// Gets a civilization-specific description for this technology
    pub fn get_description(&self, viewing_civ: &Civilization) -> String {
        TechnologyDescriptions::get_description(self, viewing_civ)
    }

    /// Gets the era object for this technology
    pub fn era_object(&self, ruleset: &Ruleset) -> Option<&crate::models::ruleset::tech::Era> {
        ruleset.eras.get(&self.era())
    }

    /// Checks if this technology matches a filter
    pub fn matches_filter(
        &self,
        filter: &str,
        state: Option<&StateForConditionals>,
        multi_filter: bool,
    ) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, || {
                self.matches_single_filter(filter)
                    || state.map_or_else(
                        || self.has_tag_unique(filter),
                        |s| self.has_unique_with_state(filter, s),
                    )
            })
        } else {
            self.matches_single_filter(filter)
                || state.map_or_else(
                    || self.has_tag_unique(filter),
                    |s| self.has_unique_with_state(filter, s),
                )
        }
    }

    /// Checks if this technology matches a single filter
    pub fn matches_single_filter(&self, filter: &str) -> bool {
        match filter {
            f if f == ALL => true,
            f if f == self.name => true,
            f if f == self.era() => true,
            _ => false,
        }
    }

    /// Checks if a unique is a requirement for this technology
    pub fn unique_is_requirement_for_this_tech(&self, unique: &Unique) -> bool {
        unique.unique_type == UniqueType::OnlyAvailable
            && unique.modifiers.len() == 1
            && unique.modifiers[0].unique_type == UniqueType::ConditionalTech
            && unique.modifiers[0]
                .params
                .get(0)
                .map_or(false, |param| param == &self.name)
    }

    /// Checks if a unique is not a requirement for this technology
    pub fn unique_is_not_requirement_for_this_tech(&self, unique: &Unique) -> bool {
        !self.unique_is_requirement_for_this_tech(unique)
    }

    /// Checks if this technology has a specific unique
    pub fn has_unique(&self, unique_type: UniqueType) -> bool {
        self.uniques
            .iter()
            .any(|unique| unique.unique_type == unique_type)
    }

    /// Checks if this technology has a specific unique with a given state
    pub fn has_unique_with_state(&self, filter: &str, state: &StateForConditionals) -> bool {
        self.uniques
            .iter()
            .any(|unique| unique.unique_type.to_string() == filter && unique.matches_state(state))
    }

    /// Checks if this technology has a tag unique
    pub fn has_tag_unique(&self, tag: &str) -> bool {
        self.uniques
            .iter()
            .any(|unique| unique.unique_type.to_string() == tag)
    }
}

impl RulesetObject for Technology {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Tech
    }

    fn make_link(&self) -> String {
        format!("Technology/{}", self.name)
    }

    fn get_civilopedia_text_lines(
        &self,
        ruleset: &Ruleset,
    ) -> Vec<crate::ui::screens::civilopediascreen::FormattedLine> {
        TechnologyDescriptions::get_civilopedia_text_lines(self, ruleset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_technology_new() {
        let tech = Technology::new();
        assert!(tech.name.is_empty());
        assert_eq!(tech.cost, 0);
        assert!(tech.prerequisites.is_empty());
        assert!(tech.column.is_none());
        assert_eq!(tech.row, 0);
        assert!(tech.quote.is_empty());
        assert!(tech.uniques.is_empty());
    }

    #[test]
    fn test_era() {
        let mut tech = Technology::new();
        let mut column = crate::models::ruleset::tech::TechColumn::new();
        column.era = "Ancient".to_string();
        tech.column = Some(Box::new(column));
        assert_eq!(tech.era(), "Ancient");
    }

    #[test]
    fn test_is_continually_researchable() {
        let mut tech = Technology::new();
        assert!(!tech.is_continually_researchable());

        let mut unique = Unique::new();
        unique.unique_type = UniqueType::ResearchableMultipleTimes;
        tech.uniques.push(unique);
        assert!(tech.is_continually_researchable());
    }

    #[test]
    fn test_matches_single_filter() {
        let mut tech = Technology::new();
        tech.name = "Test Tech".to_string();

        let mut column = crate::models::ruleset::tech::TechColumn::new();
        column.era = "Ancient".to_string();
        tech.column = Some(Box::new(column));

        assert!(tech.matches_single_filter(ALL));
        assert!(tech.matches_single_filter("Test Tech"));
        assert!(tech.matches_single_filter("Ancient"));
        assert!(!tech.matches_single_filter("Other Tech"));
    }

    #[test]
    fn test_unique_is_requirement_for_this_tech() {
        let mut tech = Technology::new();
        tech.name = "Test Tech".to_string();

        let mut unique = Unique::new();
        unique.unique_type = UniqueType::OnlyAvailable;
        let mut modifier = crate::models::ruleset::unique::UniqueModifier::new();
        modifier.unique_type = UniqueType::ConditionalTech;
        modifier.params.push("Test Tech".to_string());
        unique.modifiers.push(modifier);

        assert!(tech.unique_is_requirement_for_this_tech(&unique));

        let mut other_tech = Technology::new();
        other_tech.name = "Other Tech".to_string();
        assert!(!other_tech.unique_is_requirement_for_this_tech(&unique));
    }
}
