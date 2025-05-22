use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::constants::FONTS;
use crate::exceptions::UncivShowableException;
use crate::models::ruleset::{Ruleset, RulesetObject};
use crate::models::ruleset::unique::{StateForConditionals, UniqueTarget, UniqueType};
use crate::ui::components::color_from_rgb;
use crate::ui::objectdescriptions::uniques_to_civilopedia_text_lines;
use crate::ui::screens::civilopediascreen::FormattedLine;

/// Represents an era in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Era {
    pub ruleset_object: RulesetObject,
    /// The era number, used for sorting
    pub era_number: i32,
    /// Cost of research agreements in this era
    pub research_agreement_cost: i32,
    /// Number of starting settlers
    pub starting_settler_count: i32,
    /// Name of the starting settler unit
    pub starting_settler_unit: String,
    /// Number of starting workers
    pub starting_worker_count: i32,
    /// Name of the starting worker unit
    pub starting_worker_unit: String,
    /// Number of starting military units
    pub starting_military_unit_count: i32,
    /// Name of the starting military unit
    pub starting_military_unit: String,
    /// Starting gold amount
    pub starting_gold: i32,
    /// Starting culture amount
    pub starting_culture: i32,
    /// Population of starting settlers
    pub settler_population: i32,
    /// Buildings that come with settlers
    pub settler_buildings: Vec<String>,
    /// Wonders that are obsolete at the start of this era
    pub starting_obsolete_wonders: Vec<String>,
    /// Base cost for buying units
    pub base_unit_buy_cost: i32,
    /// Defense value when embarked
    pub embark_defense: i32,
    /// Start percentage
    pub start_percent: i32,
    /// City sound to play
    pub city_sound: String,
    /// Friend bonus map
    pub friend_bonus: HashMap<String, Vec<String>>,
    /// Ally bonus map
    pub ally_bonus: HashMap<String, Vec<String>>,
    /// Icon RGB color values
    icon_rgb: Option<Vec<i32>>,
    /// Name of the era
    pub name: String,
}

impl Era {
    /// Creates a new Era with default values
    pub fn new() -> Self {
        Era {
            ruleset_object: RulesetObject::default(),
            era_number: -1,
            research_agreement_cost: 300,
            starting_settler_count: 1,
            starting_settler_unit: "Settler".to_string(),
            starting_worker_count: 0,
            starting_worker_unit: "Worker".to_string(),
            starting_military_unit_count: 1,
            starting_military_unit: "Warrior".to_string(),
            starting_gold: 0,
            starting_culture: 0,
            settler_population: 1,
            settler_buildings: Vec::new(),
            starting_obsolete_wonders: Vec::new(),
            base_unit_buy_cost: 200,
            embark_defense: 3,
            start_percent: 0,
            city_sound: "cityClassical".to_string(),
            friend_bonus: HashMap::new(),
            ally_bonus: HashMap::new(),
            icon_rgb: None,
            name: String::new(),
        }
    }

    /// Gets the starting units for this era
    pub fn get_starting_units(&self, ruleset: &Ruleset) -> Vec<String> {
        let mut starting_units = Vec::new();

        // Get the starting settler unit name
        let starting_settler_name = if ruleset.units.contains_key(&self.starting_settler_unit) {
            self.starting_settler_unit.clone()
        } else {
            ruleset.units.values()
                .find(|unit| unit.is_city_founder())
                .map(|unit| unit.name.clone())
                .ok_or_else(|| UncivShowableException::new(format!("No Settler unit found for era {}", self.name)))
                .unwrap_or_else(|e| panic!("{}", e))
        };

        // Get the starting worker unit name
        let starting_worker_name = if self.starting_worker_count == 0 || ruleset.units.contains_key(&self.starting_worker_unit) {
            self.starting_worker_unit.clone()
        } else {
            ruleset.units.values()
                .find(|unit| unit.has_unique(UniqueType::BuildImprovements))
                .map(|unit| unit.name.clone())
                .ok_or_else(|| UncivShowableException::new(format!("No Worker unit found for era {}", self.name)))
                .unwrap_or_else(|e| panic!("{}", e))
        };

        // Add the units to the list
        for _ in 0..self.starting_settler_count {
            starting_units.push(starting_settler_name.clone());
        }

        for _ in 0..self.starting_worker_count {
            starting_units.push(starting_worker_name.clone());
        }

        for _ in 0..self.starting_military_unit_count {
            starting_units.push(self.starting_military_unit.clone());
        }

        starting_units
    }

    /// Gets the color for this era
    pub fn get_color(&self) -> [f32; 4] {
        if let Some(rgb) = &self.icon_rgb {
            color_from_rgb(rgb[0], rgb[1], rgb[2])
        } else {
            [1.0, 1.0, 1.0, 1.0] // White
        }
    }

    /// Gets the hex color string for this era
    pub fn get_hex_color(&self) -> String {
        let color = self.get_color();
        format!("#{:02x}{:02x}{:02x}",
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8)
    }

    /// Gets era-gated objects from the ruleset
    fn get_era_gated_objects(&self, ruleset: &Ruleset) -> Vec<RulesetObject> {
        let era_conditionals = vec![
            UniqueType::ConditionalBeforeEra,
            UniqueType::ConditionalDuringEra,
            UniqueType::ConditionalStartingFromEra,
            UniqueType::ConditionalIfStartingInEra,
        ];

        let mut result = Vec::new();

        // Add policy branches from this era
        for branch in ruleset.policy_branches.values() {
            if branch.era == self.name {
                result.push(branch.clone());
            }
        }

        // Add other ruleset objects with era conditionals
        for obj in ruleset.all_ruleset_objects() {
            let uniques = obj.get_matching_uniques(
                UniqueType::OnlyAvailable,
                StateForConditionals::IgnoreConditionals,
            );

            for unique in uniques {
                if unique.modifiers.iter().any(|m| era_conditionals.contains(&m.unique_type)) {
                    result.push(obj.clone());
                    break;
                }
            }
        }

        result
    }

    pub fn origin_ruleset(&self) -> &str {
        &self.ruleset_object.origin_ruleset
    }

    pub fn set_origin_ruleset(&mut self, origin: String) {
        self.ruleset_object.origin_ruleset = origin;
    }
}

impl RulesetObject for Era {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Era
    }

    fn make_link(&self) -> String {
        format!("Era/{}", self.name)
    }

    fn get_civilopedia_text_header(&self) -> FormattedLine {
        FormattedLine::new(
            self.name.clone(),
            Some(2),
            Some(self.get_hex_color()),
            None,
            None,
        )
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Add basic era information
        lines.push(FormattedLine::new(
            format!("Embarked strength: [{}]{}", self.embark_defense, FONTS.strength),
            None,
            None,
            None,
            None,
        ));

        lines.push(FormattedLine::new(
            format!("Base unit buy cost: [{}]{}", self.base_unit_buy_cost, FONTS.gold),
            None,
            None,
            None,
            None,
        ));

        lines.push(FormattedLine::new(
            format!("Research agreement cost: [{}]{}", self.research_agreement_cost, FONTS.gold),
            None,
            None,
            None,
            None,
        ));

        lines.push(FormattedLine::new(String::new(), None, None, None, None));

        // Add technologies from this era
        for tech in ruleset.technologies.values() {
            if tech.era() == self.name {
                lines.push(FormattedLine::new(
                    tech.name.clone(),
                    None,
                    None,
                    Some(tech.make_link()),
                    None,
                ));
            }
        }

        // Add uniques
        lines.extend(uniques_to_civilopedia_text_lines(self));

        // Add era-gated objects
        let era_gated_objects = self.get_era_gated_objects(ruleset);
        if !era_gated_objects.is_empty() {
            lines.push(FormattedLine::new(String::new(), None, None, None, None));
            lines.push(FormattedLine::new("See also:".to_string(), None, None, None, None));

            for obj in era_gated_objects {
                lines.push(FormattedLine::new(
                    obj.name().to_string(),
                    None,
                    None,
                    Some(obj.make_link()),
                    None,
                ));
            }
        }

        lines
    }

    fn get_sort_group(&self, _ruleset: &Ruleset) -> i32 {
        self.era_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_era_new() {
        let era = Era::new();
        assert_eq!(era.era_number, -1);
        assert_eq!(era.research_agreement_cost, 300);
        assert_eq!(era.starting_settler_count, 1);
        assert_eq!(era.starting_settler_unit, "Settler");
        assert_eq!(era.starting_worker_count, 0);
        assert_eq!(era.starting_worker_unit, "Worker");
        assert_eq!(era.starting_military_unit_count, 1);
        assert_eq!(era.starting_military_unit, "Warrior");
        assert_eq!(era.starting_gold, 0);
        assert_eq!(era.starting_culture, 0);
        assert_eq!(era.settler_population, 1);
        assert!(era.settler_buildings.is_empty());
        assert!(era.starting_obsolete_wonders.is_empty());
        assert_eq!(era.base_unit_buy_cost, 200);
        assert_eq!(era.embark_defense, 3);
        assert_eq!(era.start_percent, 0);
        assert_eq!(era.city_sound, "cityClassical");
        assert!(era.friend_bonus.is_empty());
        assert!(era.ally_bonus.is_empty());
        assert!(era.icon_rgb.is_none());
        assert!(era.name.is_empty());
    }

    #[test]
    fn test_get_hex_color() {
        let mut era = Era::new();
        era.icon_rgb = Some(vec![255, 0, 0]); // Red
        assert_eq!(era.get_hex_color(), "#ff0000");

        era.icon_rgb = Some(vec![0, 255, 0]); // Green
        assert_eq!(era.get_hex_color(), "#00ff00");

        era.icon_rgb = Some(vec![0, 0, 255]); // Blue
        assert_eq!(era.get_hex_color(), "#0000ff");

        era.icon_rgb = None;
        assert_eq!(era.get_hex_color(), "#ffffff"); // White (default)
    }
}