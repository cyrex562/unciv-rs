use crate::battle::i_combatant::ICombatant;
use crate::civilization::Civilization;
use crate::map::map_unit::MapUnit;
use crate::map::tile::Tile;
use crate::models::unciv_sound::UncivSound;
use crate::models::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::models::ruleset::unique::unique::Unique;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::models::unit_type::UnitType;
use std::sync::Arc;
use std::fmt;

/// Represents a map unit as a combatant in battle
#[derive(Clone)]
pub struct MapUnitCombatant {
    /// The unit this combatant represents
    pub unit: Arc<MapUnit>,
}

impl std::fmt::Debug for MapUnitCombatant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapUnitCombatant")
            .field("unit", &format!("Arc<MapUnit>({:p})", Arc::as_ptr(&self.unit)))
            .finish()
    }
}

impl MapUnitCombatant {
    /// Creates a new MapUnitCombatant from a MapUnit
    pub fn new(unit: Arc<MapUnit>) -> Self {
        MapUnitCombatant { unit }
    }

    /// Gets matching uniques for the unit
    pub fn get_matching_uniques(
        &self,
        unique_type: UniqueType,
        conditional_state: &StateForConditionals,
        check_civ_uniques: bool
    ) -> Vec<Unique> {
        self.unit.get_matching_uniques(unique_type, conditional_state, check_civ_uniques)
    }

    /// Checks if the unit has a specific unique
    pub fn has_unique(
        &self,
        unique_type: UniqueType,
        conditional_state: Option<&StateForConditionals>
    ) -> bool {
        match conditional_state {
            Some(state) => self.unit.has_unique(unique_type, state),
            None => self.unit.has_unique(unique_type),
        }
    }
}

impl ICombatant for MapUnitCombatant {
    fn get_health(&self) -> i32 {
        self.unit.health
    }

    fn get_max_health(&self) -> i32 {
        100
    }

    fn get_civ_info(&self) -> Arc<Civilization> {
        self.unit.civ.clone()
    }

    fn get_tile(&self) -> Arc<Tile> {
        self.unit.get_tile()
    }

    fn get_name(&self) -> String {
        self.unit.name.clone()
    }

    fn is_defeated(&self) -> bool {
        self.unit.health <= 0
    }

    fn is_invisible(&self, to: &Civilization) -> bool {
        self.unit.is_invisible(to)
    }

    fn can_attack(&self) -> bool {
        self.unit.can_attack()
    }

    fn matches_filter(&self, filter: &str, multi_filter: bool) -> bool {
        self.unit.matches_filter(filter, multi_filter)
    }

    fn get_attack_sound(&self) -> UncivSound {
        match &self.unit.base_unit.attack_sound {
            Some(sound) => UncivSound::from(sound.clone()),
            None => UncivSound::Click,
        }
    }

    fn take_damage(&mut self, damage: i32) {
        self.unit.take_damage(damage);
    }

    fn get_attacking_strength(&self) -> i32 {
        if self.is_ranged() {
            self.unit.base_unit.ranged_strength
        } else {
            self.unit.base_unit.strength
        }
    }

    fn get_defending_strength(&self, attacked_by_ranged: bool) -> i32 {
        if self.unit.is_embarked() && !self.is_civilian() {
            self.unit.civ.get_era().embark_defense
        } else if self.is_ranged() && attacked_by_ranged {
            self.unit.base_unit.ranged_strength
        } else {
            self.unit.base_unit.strength
        }
    }

    fn get_unit_type(&self) -> UnitType {
        self.unit.type_.clone()
    }

    fn is_ranged(&self) -> bool {
        self.unit.base_unit.is_ranged()
    }

    fn is_air_unit(&self) -> bool {
        self.unit.base_unit.is_air_unit()
    }

    fn is_water_unit(&self) -> bool {
        self.unit.base_unit.is_water_unit
    }

    fn is_land_unit(&self) -> bool {
        self.unit.base_unit.is_land_unit
    }

    fn is_city(&self) -> bool {
        false
    }

    fn is_civilian(&self) -> bool {
        self.unit.is_civilian()
    }
}

impl fmt::Display for MapUnitCombatant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} of {}", self.unit.name, self.unit.civ.civ_name)
    }
}