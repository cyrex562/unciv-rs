use std::fmt;
use serde::{Serialize, Deserialize};

/// Represents different types of unique abilities in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniqueType {
    // Combat-related uniques
    Strength,
    RangedStrength,
    Combat,
    CombatModifier,
    DefensiveModifier,
    RangedModifier,

    // Movement-related uniques
    Movement,
    MovementCost,
    IgnoreTerrainCost,
    CannotMove,

    // Resource-related uniques
    ProvidesResources,
    ConsumesResources,
    ResourceAmountBonus,

    // Trigger-related uniques
    TriggerEvent,
    MarkTutorialComplete,
    OneTimeFreeUnit,
    ConditionalTimedUnique,

    // City-related uniques
    CityGrowth,
    CityStrength,
    CityHealth,
    CityProduction,

    // Unit-related uniques
    UnitStartingExperience,
    UnitUpgrade,
    UnitMaintenance,
    MaxNumberBuildable,

    // Terrain-related uniques
    TerrainFeature,
    TerrainDefense,
    TerrainCombatBonus,

    // Other common uniques
    Ability,
    Promotion,
    Policy,
    Wonder,
    Building,
    Technology,

    // Special uniques
    Hidden,
    Unbuildable,
    Uncapturable,
    Unsellable,

    /// Cannot attack
    CannotAttack,
    /// Cannot be barbarian
    CannotBeBarbarian,
    /// Restricted buildable improvements
    RestrictedBuildableImprovements,
    /// Notified of barbarian encampments
    NotifiedOfBarbarianEncampments,
    // Add other unique types as needed
    StrategicBalanceResource,
    RareFeature,
}

impl UniqueType {
    /// Returns true if this unique type represents a combat modifier
    pub fn is_combat_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::Strength |
            UniqueType::RangedStrength |
            UniqueType::Combat |
            UniqueType::CombatModifier |
            UniqueType::DefensiveModifier |
            UniqueType::RangedModifier
        )
    }

    /// Returns true if this unique type represents a movement modifier
    pub fn is_movement_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::Movement |
            UniqueType::MovementCost |
            UniqueType::IgnoreTerrainCost |
            UniqueType::CannotMove
        )
    }

    /// Returns true if this unique type represents a resource modifier
    pub fn is_resource_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::ProvidesResources |
            UniqueType::ConsumesResources |
            UniqueType::ResourceAmountBonus
        )
    }

    /// Returns true if this unique type represents a city modifier
    pub fn is_city_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::CityGrowth |
            UniqueType::CityStrength |
            UniqueType::CityHealth |
            UniqueType::CityProduction
        )
    }

    /// Returns true if this unique type represents a unit modifier
    pub fn is_unit_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::UnitStartingExperience |
            UniqueType::UnitUpgrade |
            UniqueType::UnitMaintenance |
            UniqueType::MaxNumberBuildable
        )
    }

    /// Returns true if this unique type represents a terrain modifier
    pub fn is_terrain_modifier(&self) -> bool {
        matches!(
            self,
            UniqueType::TerrainFeature |
            UniqueType::TerrainDefense |
            UniqueType::TerrainCombatBonus
        )
    }

    /// Returns true if this unique type represents a trigger
    pub fn is_trigger(&self) -> bool {
        matches!(
            self,
            UniqueType::TriggerEvent |
            UniqueType::MarkTutorialComplete |
            UniqueType::OneTimeFreeUnit |
            UniqueType::ConditionalTimedUnique
        )
    }

    /// Returns true if this unique type represents a special status
    pub fn is_special_status(&self) -> bool {
        matches!(
            self,
            UniqueType::Hidden |
            UniqueType::Unbuildable |
            UniqueType::Uncapturable |
            UniqueType::Unsellable
        )
    }

    /// Returns true if this unique type requires parameters
    pub fn requires_parameters(&self) -> bool {
        !matches!(
            self,
            UniqueType::Hidden |
            UniqueType::Unbuildable |
            UniqueType::Uncapturable |
            UniqueType::Unsellable
        )
    }

    /// Returns true if this unique type can be applied to buildings
    pub fn can_apply_to_building(&self) -> bool {
        matches!(
            self,
            UniqueType::Building |
            UniqueType::Wonder |
            UniqueType::CityProduction |
            UniqueType::ProvidesResources |
            UniqueType::ConsumesResources
        )
    }

    /// Returns true if this unique type can be applied to units
    pub fn can_apply_to_unit(&self) -> bool {
        matches!(
            self,
            UniqueType::Combat |
            UniqueType::Movement |
            UniqueType::Ability |
            UniqueType::Promotion |
            UniqueType::UnitStartingExperience |
            UniqueType::UnitUpgrade |
            UniqueType::UnitMaintenance
        )
    }
}

impl fmt::Display for UniqueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_combat_modifier() {
        assert!(UniqueType::Strength.is_combat_modifier());
        assert!(UniqueType::CombatModifier.is_combat_modifier());
        assert!(!UniqueType::Movement.is_combat_modifier());
    }

    #[test]
    fn test_is_movement_modifier() {
        assert!(UniqueType::Movement.is_movement_modifier());
        assert!(UniqueType::MovementCost.is_movement_modifier());
        assert!(!UniqueType::Strength.is_movement_modifier());
    }

    #[test]
    fn test_is_resource_modifier() {
        assert!(UniqueType::ProvidesResources.is_resource_modifier());
        assert!(UniqueType::ConsumesResources.is_resource_modifier());
        assert!(!UniqueType::Movement.is_resource_modifier());
    }

    #[test]
    fn test_is_city_modifier() {
        assert!(UniqueType::CityGrowth.is_city_modifier());
        assert!(UniqueType::CityStrength.is_city_modifier());
        assert!(!UniqueType::Combat.is_city_modifier());
    }

    #[test]
    fn test_is_unit_modifier() {
        assert!(UniqueType::UnitStartingExperience.is_unit_modifier());
        assert!(UniqueType::UnitMaintenance.is_unit_modifier());
        assert!(!UniqueType::CityGrowth.is_unit_modifier());
    }

    #[test]
    fn test_requires_parameters() {
        assert!(UniqueType::Combat.requires_parameters());
        assert!(!UniqueType::Hidden.requires_parameters());
    }

    #[test]
    fn test_can_apply_to_building() {
        assert!(UniqueType::Building.can_apply_to_building());
        assert!(UniqueType::Wonder.can_apply_to_building());
        assert!(!UniqueType::Combat.can_apply_to_building());
    }

    #[test]
    fn test_can_apply_to_unit() {
        assert!(UniqueType::Combat.can_apply_to_unit());
        assert!(UniqueType::Movement.can_apply_to_unit());
        assert!(!UniqueType::Building.can_apply_to_unit());
    }
}