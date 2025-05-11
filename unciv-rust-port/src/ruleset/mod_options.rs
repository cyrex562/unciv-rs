use std::collections::{HashSet, VecDeque};
use std::fmt;

use crate::models::ruleset::unique::{IHasUniques, Unique, UniqueMap, UniqueTarget};

/// Represents mod options for configuring game mods
pub struct ModOptions {
    // Modder choices
    pub is_base_ruleset: bool,
    pub techs_to_remove: HashSet<String>,
    pub buildings_to_remove: HashSet<String>,
    pub units_to_remove: HashSet<String>,
    pub nations_to_remove: HashSet<String>,
    pub constants: ModConstants,
    pub unitset: Option<String>,
    pub tileset: Option<String>,

    // Metadata, automatic
    pub mod_url: String,
    pub default_branch: String,
    pub author: String,
    pub last_updated: String,
    pub mod_size: i32,
    pub topics: Vec<String>,

    // IHasUniques implementation
    pub name: String,
    pub uniques: Vec<String>,
    pub unique_objects: VecDeque<Unique>,
    pub unique_map: UniqueMap,
}

impl ModOptions {
    /// Creates a new ModOptions instance
    pub fn new() -> Self {
        Self {
            is_base_ruleset: false,
            techs_to_remove: HashSet::new(),
            buildings_to_remove: HashSet::new(),
            units_to_remove: HashSet::new(),
            nations_to_remove: HashSet::new(),
            constants: ModConstants::new(),
            unitset: None,
            tileset: None,
            mod_url: String::new(),
            default_branch: "master".to_string(),
            author: String::new(),
            last_updated: String::new(),
            mod_size: 0,
            topics: Vec::new(),
            name: "ModOptions".to_string(),
            uniques: Vec::new(),
            unique_objects: VecDeque::new(),
            unique_map: UniqueMap::new(),
        }
    }
}

impl IHasUniques for ModOptions {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_uniques(&self) -> &[String] {
        &self.uniques
    }

    fn get_unique_objects(&self) -> &VecDeque<Unique> {
        &self.unique_objects
    }

    fn get_unique_map(&self) -> &UniqueMap {
        &self.unique_map
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::ModOptions
    }
}

/// Represents mod constants
pub struct ModConstants {
    // Add fields as needed based on the ModConstants class
}

impl ModConstants {
    /// Creates a new ModConstants instance
    pub fn new() -> Self {
        Self {
            // Initialize fields as needed
        }
    }
}

impl fmt::Display for ModOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModOptions")
    }
}