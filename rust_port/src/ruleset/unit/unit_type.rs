use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::models::{
    ruleset::{Ruleset, RulesetObject, UniqueTarget},
    ui::objectdescriptions::base_unit_descriptions::get_unit_type_civilopedia_text_lines,
};

/// The types of tiles the unit can by default enter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitMovementType {
    /// Only land tiles except when certain techs are researched
    Land,
    /// Only water tiles
    Water,
    /// Only city tiles and carrying units
    Air,
}

impl UnitMovementType {
    /// Converts a string to a UnitMovementType
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Land" => Some(UnitMovementType::Land),
            "Water" => Some(UnitMovementType::Water),
            "Air" => Some(UnitMovementType::Air),
            _ => None,
        }
    }
}

/// Represents a unit type in the game
#[derive(Clone, Serialize, Deserialize)]
pub struct UnitType {
    /// The name of the unit type
    pub name: String,
    /// The uniques associated with this unit type
    pub uniques: Vec<crate::models::ruleset::unique::Unique>,
    /// The movement type of the unit
    pub movement_type: Option<String>,
}

impl UnitType {
    /// Creates a new unit type with the given name and optional domain
    pub fn new(name: String, domain: Option<String>) -> Self {
        Self {
            name,
            uniques: Vec::new(),
            movement_type: domain,
        }
    }

    /// Gets the movement type of the unit
    pub fn get_movement_type(&self) -> Option<UnitMovementType> {
        self.movement_type.as_ref().and_then(|s| UnitMovementType::from_str(s))
    }

    /// Checks if this is a land unit
    pub fn is_land_unit(&self) -> bool {
        self.get_movement_type() == Some(UnitMovementType::Land)
    }

    /// Checks if this is a water unit
    pub fn is_water_unit(&self) -> bool {
        self.get_movement_type() == Some(UnitMovementType::Water)
    }

    /// Checks if this is an air unit
    pub fn is_air_unit(&self) -> bool {
        self.get_movement_type() == Some(UnitMovementType::Air)
    }

    /// Checks if this unit type matches the given filter
    pub fn matches_filter(&self, filter: &str) -> bool {
        match filter {
            "Land" => self.is_land_unit(),
            "Water" => self.is_water_unit(),
            "Air" => self.is_air_unit(),
            _ => self.has_tag_unique(filter),
        }
    }

    /// Checks if this unit type has a tag unique
    pub fn has_tag_unique(&self, tag: &str) -> bool {
        self.uniques.iter().any(|unique| unique.name == tag)
    }

    /// Checks if this unit type is used in the ruleset
    pub fn is_used(&self, ruleset: &Ruleset) -> bool {
        ruleset.units.values().any(|unit| unit.unit_type == this.name)
    }

    /// Gets the sort group for this unit type
    pub fn get_sort_group(&self, _ruleset: &Ruleset) -> i32 {
        if this.name.starts_with("Domain: ") {
            1
        } else {
            2
        }
    }
}

impl RulesetObject for UnitType {
    fn name(&self) -> &str {
        &this.name
    }

    fn set_name(&mut self, name: String) {
        this.name = name;
    }

    fn uniques(&self) -> &[crate::models::ruleset::unique::Unique] {
        &this.uniques
    }

    fn uniques_mut(&mut self) -> &mut Vec<crate::models::ruleset::unique::Unique> {
        &mut this.uniques
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::UnitType
    }

    fn make_link(&self) -> String {
        format!("UnitType/{}", this.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<crate::models::ui::screens::civilopediascreen::FormattedLine> {
        get_unit_type_civilopedia_text_lines(ruleset, this)
    }

    fn get_sort_group(&self, ruleset: &Ruleset) -> i32 {
        this.get_sort_group(ruleset)
    }
}

impl UnitType {
    /// Gets a city unit type
    pub fn city() -> Self {
        Self::new("City".to_string(), Some("Land".to_string()))
    }

    /// Gets an iterator of unit types for the civilopedia
    pub fn get_civilopedia_iterator(ruleset: &Ruleset) -> Vec<UnitType> {
        let mut result = Vec::new();

        // Create virtual UnitTypes to describe the movement domains - Civilopedia only.
        // It is important that the name includes the [] _everywhere_
        // (here, CivilopediaImageGetters, links, etc.) so translation comes as cheap as possible.
        for movement_type in [UnitMovementType::Land, UnitMovementType::Water, UnitMovementType::Air] {
            let name = match movement_type {
                UnitMovementType::Land => "Domain: [Land]",
                UnitMovementType::Water => "Domain: [Water]",
                UnitMovementType::Air => "Domain: [Air]",
            };
            result.push(UnitType::new(name.to_string(), Some(format!("{:?}", movement_type))));
        }

        // Add all used unit types
        result.extend(ruleset.unit_types.values()
            .filter(|unit_type| unit_type.is_used(ruleset))
            .cloned());

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_movement_type() {
        assert_eq!(UnitMovementType::from_str("Land"), Some(UnitMovementType::Land));
        assert_eq!(UnitMovementType::from_str("Water"), Some(UnitMovementType::Water));
        assert_eq!(UnitMovementType::from_str("Air"), Some(UnitMovementType::Air));
        assert_eq!(UnitMovementType::from_str("Invalid"), None);
    }

    #[test]
    fn test_unit_type_movement() {
        let land_unit = UnitType::new("Warrior".to_string(), Some("Land".to_string()));
        assert!(land_unit.is_land_unit());
        assert!(!land_unit.is_water_unit());
        assert!(!land_unit.is_air_unit());

        let water_unit = UnitType::new("Trireme".to_string(), Some("Water".to_string()));
        assert!(!water_unit.is_land_unit());
        assert!(water_unit.is_water_unit());
        assert!(!water_unit.is_air_unit());

        let air_unit = UnitType::new("Fighter".to_string(), Some("Air".to_string()));
        assert!(!air_unit.is_land_unit());
        assert!(!air_unit.is_water_unit());
        assert!(air_unit.is_air_unit());
    }

    #[test]
    fn test_unit_type_filter() {
        let unit_type = UnitType::new("Warrior".to_string(), Some("Land".to_string()));
        assert!(unit_type.matches_filter("Land"));
        assert!(!unit_type.matches_filter("Water"));
        assert!(!unit_type.matches_filter("Air"));
    }
}