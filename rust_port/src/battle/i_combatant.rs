use crate::civilization::Civilization;
use crate::map::tile::Tile;
use crate::models::unciv_sound::UncivSound;
use crate::models::unit_type::UnitType;
use std::sync::Arc;

/// Trait for entities that can participate in combat
pub trait ICombatant: Clone + std::fmt::Debug {
    /// Get the name of the combatant
    fn get_name(&self) -> String;

    /// Get the current health of the combatant
    fn get_health(&self) -> i32;

    /// Get the maximum health of the combatant
    fn get_max_health(&self) -> i32;

    /// Get the unit type of the combatant
    fn get_unit_type(&self) -> UnitType;

    /// Get the attacking strength of the combatant
    fn get_attacking_strength(&self) -> i32;

    /// Get the defending strength of the combatant
    fn get_defending_strength(&self, attacked_by_ranged: bool) -> i32;

    /// Apply damage to the combatant
    fn take_damage(&mut self, damage: i32);

    /// Check if the combatant is defeated
    fn is_defeated(&self) -> bool;

    /// Get the civilization information of the combatant
    fn get_civ_info(&self) -> Arc<Civilization>;

    /// Get the tile the combatant is on
    fn get_tile(&self) -> Arc<Tile>;

    /// Check if the combatant is invisible to a specific civilization
    fn is_invisible(&self, to: &Civilization) -> bool;

    /// Check if the combatant can attack
    fn can_attack(&self) -> bool;

    /// Check if the combatant matches a filter
    ///
    /// Implements UniqueParameterType.CombatantFilter
    fn matches_filter(&self, filter: &str, multi_filter: bool) -> bool;

    /// Get the attack sound of the combatant
    fn get_attack_sound(&self) -> UncivSound;

    /// Check if the combatant is a melee unit
    fn is_melee(&self) -> bool {
        !self.is_ranged()
    }

    /// Check if the combatant is a ranged unit
    fn is_ranged(&self) -> bool;

    /// Check if the combatant is an air unit
    fn is_air_unit(&self) -> bool;

    /// Check if the combatant is a water unit
    fn is_water_unit(&self) -> bool;

    /// Check if the combatant is a land unit
    fn is_land_unit(&self) -> bool;

    /// Check if the combatant is a city
    fn is_city(&self) -> bool;

    /// Check if the combatant is a civilian unit
    fn is_civilian(&self) -> bool;
}