use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::ruleset::tech::Technology;

/// Represents a column of technologies in the tech tree
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TechColumn {
    /// The column number in the tech tree
    pub column_number: i32,
    /// The era this column belongs to
    pub era: String,
    /// The technologies in this column
    pub techs: Vec<Technology>,
    /// The base cost for technologies in this column
    pub tech_cost: i32,
    /// The base cost for buildings in this column, -1 if not applicable
    pub building_cost: i32,
    /// The base cost for wonders in this column, -1 if not applicable
    pub wonder_cost: i32,
}

impl TechColumn {
    /// Creates a new TechColumn with default values
    pub fn new() -> Self {
        TechColumn {
            column_number: 0,
            era: String::new(),
            techs: Vec::new(),
            tech_cost: 0,
            building_cost: -1,
            wonder_cost: -1,
        }
    }

    /// Creates a new TechColumn with the specified values
    pub fn with_values(
        column_number: i32,
        era: String,
        tech_cost: i32,
        building_cost: i32,
        wonder_cost: i32,
    ) -> Self {
        TechColumn {
            column_number,
            era,
            techs: Vec::new(),
            tech_cost,
            building_cost,
            wonder_cost,
        }
    }

    /// Adds a technology to this column
    pub fn add_tech(&mut self, tech: Technology) {
        self.techs.push(tech);
    }

    /// Gets the number of technologies in this column
    pub fn tech_count(&self) -> usize {
        self.techs.len()
    }

    /// Gets a technology by index
    pub fn get_tech(&self, index: usize) -> Option<&Technology> {
        self.techs.get(index)
    }

    /// Gets a mutable reference to a technology by index
    pub fn get_tech_mut(&mut self, index: usize) -> Option<&mut Technology> {
        self.techs.get_mut(index)
    }

    /// Gets all technologies in this column
    pub fn get_techs(&self) -> &[Technology] {
        &self.techs
    }

    /// Gets a mutable reference to all technologies in this column
    pub fn get_techs_mut(&mut self) -> &mut Vec<Technology> {
        &mut self.techs
    }

    /// Clears all technologies from this column
    pub fn clear_techs(&mut self) {
        self.techs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_column_new() {
        let tech_column = TechColumn::new();
        assert_eq!(tech_column.column_number, 0);
        assert!(tech_column.era.is_empty());
        assert!(tech_column.techs.is_empty());
        assert_eq!(tech_column.tech_cost, 0);
        assert_eq!(tech_column.building_cost, -1);
        assert_eq!(tech_column.wonder_cost, -1);
    }

    #[test]
    fn test_tech_column_with_values() {
        let tech_column = TechColumn::with_values(1, "Ancient".to_string(), 50, 100, 200);
        assert_eq!(tech_column.column_number, 1);
        assert_eq!(tech_column.era, "Ancient");
        assert!(tech_column.techs.is_empty());
        assert_eq!(tech_column.tech_cost, 50);
        assert_eq!(tech_column.building_cost, 100);
        assert_eq!(tech_column.wonder_cost, 200);
    }

    #[test]
    fn test_add_tech() {
        let mut tech_column = TechColumn::new();
        let tech = Technology::new();
        tech_column.add_tech(tech);
        assert_eq!(tech_column.tech_count(), 1);
    }

    #[test]
    fn test_get_tech() {
        let mut tech_column = TechColumn::new();
        let mut tech = Technology::new();
        tech.name = "Test Tech".to_string();
        tech_column.add_tech(tech);

        let retrieved_tech = tech_column.get_tech(0);
        assert!(retrieved_tech.is_some());
        assert_eq!(retrieved_tech.unwrap().name, "Test Tech");
    }

    #[test]
    fn test_get_tech_mut() {
        let mut tech_column = TechColumn::new();
        let mut tech = Technology::new();
        tech.name = "Test Tech".to_string();
        tech_column.add_tech(tech);

        let retrieved_tech = tech_column.get_tech_mut(0);
        assert!(retrieved_tech.is_some());
        retrieved_tech.unwrap().name = "Updated Tech".to_string();

        assert_eq!(tech_column.get_tech(0).unwrap().name, "Updated Tech");
    }

    #[test]
    fn test_clear_techs() {
        let mut tech_column = TechColumn::new();
        let tech = Technology::new();
        tech_column.add_tech(tech);
        assert_eq!(tech_column.tech_count(), 1);

        tech_column.clear_techs();
        assert_eq!(tech_column.tech_count(), 0);
    }
}