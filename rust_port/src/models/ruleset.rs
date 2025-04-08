use std::collections::HashMap;

/// Represents a ruleset in the game.
pub struct Ruleset {
    pub technologies: HashMap<String, Technology>,
    pub units: UnitDefinitions,
    pub difficulties: HashMap<String, Difficulty>,
}

/// Represents a technology in the game.
pub struct Technology {
    pub name: String,
    pub cost: i32,
}

/// Contains unit definitions for the game.
pub struct UnitDefinitions {
    pub values: HashMap<String, BaseUnit>,
}

/// Represents a base unit in the game.
pub struct BaseUnit {
    pub name: String,
    pub is_military: bool,
    pub is_water_unit: bool,
    pub is_land_unit: bool,
    pub uniques: Vec<UniqueType>,
}

impl BaseUnit {
    /// Creates a new BaseUnit instance.
    pub fn new(name: String, is_ranged: bool, is_melee: bool) -> Self {
        BaseUnit {
            name,
            is_ranged,
            is_melee,
            is_military: false,
            is_water_unit: false,
            is_land_unit: false,
            uniques: Vec::new(),
        }
    }

    /// Checks if the unit is a ranged unit.
    pub fn is_ranged(&self) -> bool {
        self.is_ranged
    }

    /// Checks if the unit is a melee unit.
    pub fn is_melee(&self) -> bool {
        self.is_melee
    }

    /// Checks if this unit has the given unique type.
    pub fn has_unique(&self, unique_type: UniqueType) -> bool {
        self.uniques.contains(&unique_type)
    }

    /// Gets the force evaluation of this unit.
    pub fn get_force_evaluation(&self) -> i32 {
        // Placeholder implementation
        0
    }

    /// Checks if this unit is buildable by the given civilization.
    pub fn is_buildable(&self, civ: &crate::models::civilization::Civilization) -> bool {
        // Placeholder implementation
        true
    }
}

/// Represents a difficulty level in the game.
pub struct Difficulty {
    pub name: String,
    pub barbarian_spawn_delay: i32,
}

/// Represents a unique type in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniqueType {
    /// Cannot attack
    CannotAttack,
    /// Cannot be barbarian
    CannotBeBarbarian,
    /// Restricted buildable improvements
    RestrictedBuildableImprovements,
    /// Notified of barbarian encampments
    NotifiedOfBarbarianEncampments,
    // Add other unique types as needed
}