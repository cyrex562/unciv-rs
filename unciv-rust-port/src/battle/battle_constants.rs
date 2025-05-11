/// Battle-related constants used throughout the game
///
/// Based on https://www.carlsguides.com/strategy/civilization5/war/combatbonuses.php
pub mod BattleConstants {
    /// Penalty for landing units
    pub const LANDING_MALUS: i32 = -50;

    /// Penalty for boarding units
    pub const BOARDING_MALUS: i32 = -50;

    /// Penalty for attacking across a river
    pub const ATTACKING_ACROSS_RIVER_MALUS: i32 = -20;

    /// Base bonus for flanking attacks
    pub const BASE_FLANKING_BONUS: f32 = 10.0;

    /// Penalty for missing required resources
    pub const MISSING_RESOURCES_MALUS: i32 = -25;

    /// Defense bonus for embarked units
    pub const EMBARKED_DEFENCE_BONUS: i32 = 100;

    /// Bonus for fortified units
    pub const FORTIFICATION_BONUS: i32 = 20;

    /// Ratio for damage reduction when a unit is wounded
    pub const DAMAGE_REDUCTION_WOUNDED_UNIT_RATIO_PERCENTAGE: f32 = 300.0;

    /// Base damage dealt to civilian units
    pub const DAMAGE_TO_CIVILIAN_UNIT: i32 = 40;
}