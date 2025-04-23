use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};
use crate::models::{
    ruleset::{Ruleset, RulesetObject, UniqueTarget},
    ruleset::unique::{Unique, UniqueType},
    translations::tr,
    ui::{
        components::extensions::color_from_rgb,
        objectdescriptions::{uniques_to_civilopedia_text_lines, uniques_to_description},
        screens::{
            civilopediascreen::FormattedLine,
            pickerscreens::PromotionPickerScreen,
        },
    },
};

/// Represents a unit promotion in the game
#[derive(Clone, Serialize, Deserialize)]
pub struct Promotion {
    /// The name of the promotion
    pub name: String,
    /// The uniques associated with this promotion
    pub uniques: Vec<Unique>,
    /// The prerequisites for this promotion
    pub prerequisites: Vec<String>,
    /// The unit types this promotion can be applied to
    pub unit_types: Vec<String>,
    /// The inner color of the promotion icon
    pub inner_color: Option<Vec<i32>>,
    /// The outer color of the promotion icon
    pub outer_color: Option<Vec<i32>>,
    /// The row position in the promotion picker screen
    pub row: i32,
    /// The column position in the promotion picker screen
    pub column: i32,
}

impl Promotion {
    /// Creates a new empty promotion
    pub fn new() -> Self {
        Self {
            name: String::new(),
            uniques: Vec::new(),
            prerequisites: Vec::new(),
            unit_types: Vec::new(),
            inner_color: None,
            outer_color: None,
            row: -1,
            column: 0,
        }
    }

    /// Gets the inner color object
    pub fn inner_color_object(&self) -> Option<[u8; 3]> {
        self.inner_color.as_ref().map(|color| color_from_rgb(color))
    }

    /// Gets the outer color object
    pub fn outer_color_object(&self) -> Option<[u8; 3]> {
        self.outer_color.as_ref().map(|color| color_from_rgb(color))
    }

    /// Clones this promotion
    pub fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            uniques: self.uniques.clone(),
            prerequisites: self.prerequisites.clone(),
            unit_types: self.unit_types.clone(),
            inner_color: self.inner_color.clone(),
            outer_color: self.outer_color.clone(),
            row: self.row,
            column: self.column,
        }
    }

    /// Gets the unique target for this promotion
    pub fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Promotion
    }

    /// Gets a description of this promotion
    pub fn get_description(&self, promotions_for_unit_type: &[Promotion]) -> String {
        let mut text_list = Vec::new();

        uniques_to_description(&mut text_list, &self.uniques);

        if !self.prerequisites.is_empty() {
            let mut prerequisites_string = Vec::new();
            for prerequisite in self.prerequisites.iter().filter(|&p| {
                promotions_for_unit_type.iter().any(|promotion| promotion.name == *p)
            }) {
                prerequisites_string.push(tr(prerequisite));
            }

            if !prerequisites_string.is_empty() {
                text_list.push(format!("{}: {}", tr("Requires"), prerequisites_string.join(&format!(" {} ", tr("OR")))));
            }
        }

        text_list.join("\n")
    }

    /// Makes a link for this promotion
    pub fn make_link(&self) -> String {
        format!("Promotion/{}", self.name)
    }

    /// Gets the civilopedia text lines for this promotion
    pub fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        uniques_to_civilopedia_text_lines(&mut text_list, &self.uniques, None);

        let filtered_prerequisites: Vec<&Promotion> = self.prerequisites.iter()
            .filter_map(|p| ruleset.unit_promotions.get(p))
            .collect();

        if !filtered_prerequisites.is_empty() {
            text_list.push(FormattedLine::new());
            if filtered_prerequisites.len() == 1 {
                let prerequisite = filtered_prerequisites[0];
                text_list.push(FormattedLine::new_with_link(
                    &format!("Requires [{}]", prerequisite.name),
                    &prerequisite.make_link()
                ));
            } else {
                text_list.push(FormattedLine::new_with_text("Requires at least one of the following:"));
                for prerequisite in filtered_prerequisites {
                    text_list.push(FormattedLine::new_with_link(&prerequisite.name, &prerequisite.make_link()));
                }
            }
        }

        if !self.unit_types.is_empty() {
            text_list.push(FormattedLine::new());

            // Split unit types into those that exist in the ruleset and those that don't
            let (existing_types, non_existing_types): (Vec<_>, Vec<_>) = self.unit_types.iter()
                .partition(|&t| ruleset.units.contains_key(t));

            if self.unit_types.len() == 1 {
                if !existing_types.is_empty() {
                    let unit_type = existing_types[0];
                    text_list.push(FormattedLine::new_with_link(
                        &format!("Available for [{}]", unit_type),
                        &format!("Unit/{}", unit_type)
                    ));
                } else {
                    let unit_type = non_existing_types[0];
                    text_list.push(FormattedLine::new_with_link(
                        &format!("Available for [{}]", unit_type),
                        &format!("UnitType/{}", unit_type)
                    ));
                }
            } else {
                text_list.push(FormattedLine::new_with_text("Available for:"));
                for unit_type in existing_types {
                    text_list.push(FormattedLine::new_with_link_indent(unit_type, &format!("Unit/{}", unit_type), 1));
                }
                for unit_type in non_existing_types {
                    text_list.push(FormattedLine::new_with_link_indent(unit_type, &format!("UnitType/{}", unit_type), 1));
                }
            }
        }

        let free_for_units: Vec<_> = ruleset.units.iter()
            .filter(|(_, unit)| unit.promotions.contains(&self.name))
            .map(|(name, _)| name)
            .collect();

        if !free_for_units.is_empty() {
            text_list.push(FormattedLine::new());
            if free_for_units.len() == 1 {
                let unit_name = free_for_units[0];
                text_list.push(FormattedLine::new_with_link(
                    &format!("Free for [{}]", unit_name),
                    &format!("Unit/{}", unit_name)
                ));
            } else {
                text_list.push(FormattedLine::new_with_text("Free for:"));
                for unit_name in free_for_units {
                    text_list.push(FormattedLine::new_with_link(
                        unit_name,
                        &format!("Unit/{}", unit_name)
                    ));
                }
            }
        }

        let mut grantors = Vec::new();

        // Add buildings that grant this promotion
        for building in ruleset.buildings.values() {
            if building.get_matching_uniques(UniqueType::UnitStartingPromotions, &building.state)
                .iter()
                .any(|unique| unique.params[2] == self.name) {
                grantors.push(building);
            }
        }

        // Add terrains that grant this promotion
        for terrain in ruleset.terrains.values() {
            if terrain.get_matching_uniques(UniqueType::TerrainGrantsPromotion, &terrain.state)
                .iter()
                .any(|unique| self.name == unique.params[0]) {
                grantors.push(terrain);
            }
        }

        if !grantors.is_empty() {
            text_list.push(FormattedLine::new());
            if grantors.len() == 1 {
                let grantor = grantors[0];
                text_list.push(FormattedLine::new_with_link(
                    &format!("Granted by [{}]", grantor.name),
                    &grantor.make_link()
                ));
            } else {
                text_list.push(FormattedLine::new_with_text("Granted by:"));
                for grantor in grantors {
                    text_list.push(FormattedLine::new_with_link_indent(&grantor.name, &grantor.make_link(), 1));
                }
            }
        }

        text_list
    }
}

impl RulesetObject for Promotion {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn uniques(&self) -> &[Unique] {
        &self.uniques
    }

    fn uniques_mut(&mut self) -> &mut Vec<Unique> {
        &mut self.uniques
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Promotion
    }
}

/// Represents the base name and level of a promotion
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionBaseNameAndLevel {
    /// The name without brackets
    pub name_without_brackets: String,
    /// The level of the promotion
    pub level: i32,
    /// The base promotion name
    pub base_promotion_name: String,
}

impl Promotion {
    /// Gets the base name and level of a promotion
    pub fn get_base_name_and_level(promotion_name: &str) -> PromotionBaseNameAndLevel {
        let name_without_brackets = promotion_name.replace("[", "").replace("]", "");
        let level = if name_without_brackets.ends_with(" I") {
            1
        } else if name_without_brackets.ends_with(" II") {
            2
        } else if name_without_brackets.ends_with(" III") {
            3
        } else {
            0
        };

        let base_promotion_name = if level == 0 {
            name_without_brackets.clone()
        } else {
            name_without_brackets[..name_without_brackets.len() - (level + 1)].to_string()
        };

        PromotionBaseNameAndLevel {
            name_without_brackets,
            level,
            base_promotion_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_base_name_and_level() {
        let result = Promotion::get_base_name_and_level("Drill II");
        assert_eq!(result.name_without_brackets, "Drill II");
        assert_eq!(result.level, 2);
        assert_eq!(result.base_promotion_name, "Drill");

        let result = Promotion::get_base_name_and_level("Combat");
        assert_eq!(result.name_without_brackets, "Combat");
        assert_eq!(result.level, 0);
        assert_eq!(result.base_promotion_name, "Combat");

        let result = Promotion::get_base_name_and_level("[Medic]");
        assert_eq!(result.name_without_brackets, "Medic");
        assert_eq!(result.level, 0);
        assert_eq!(result.base_promotion_name, "Medic");
    }
}