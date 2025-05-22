use std::hash::{Hash, Hasher};

use crate::models::battle::{CityCombatant, CombatAction, ICombatant, MapUnitCombatant};
use crate::models::city::City;
use crate::models::civilization::Civilization;
use crate::models::game_info::GameInfo;
use crate::models::map::mapgenerator::mapregions::Region;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::stats::Stat;

/// State information for evaluating conditionals in the game.
#[derive(Clone, Debug)]
pub struct StateForConditionals {
    /// The civilization this state is for
    pub civ_info: Option<Civilization>,
    /// The city this state is for
    pub city: Option<City>,
    /// The unit this state is for
    pub unit: Option<MapUnit>,
    /// The tile this state is for
    pub tile: Option<Tile>,

    /// Our combatant in combat
    pub our_combatant: Option<Box<dyn ICombatant>>,
    /// Their combatant in combat
    pub their_combatant: Option<Box<dyn ICombatant>>,
    /// The tile being attacked
    pub attacked_tile: Option<Tile>,
    /// The combat action being performed
    pub combat_action: Option<CombatAction>,

    /// The region this state is for
    pub region: Option<Region>,
    /// The game info this state is for
    pub game_info: Option<GameInfo>,
    /// Whether to ignore conditionals
    pub ignore_conditionals: bool,
}

impl StateForConditionals {
    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            civ_info: None,
            city: None,
            unit: None,
            tile: None,
            our_combatant: None,
            their_combatant: None,
            attacked_tile: None,
            combat_action: None,
            region: None,
            game_info: None,
            ignore_conditionals: false,
        }
    }

    /// Create a new state with game info
    pub fn new_with_game_info(game_info: Option<GameInfo>, civ_info: Option<Civilization>, city: Option<City>) -> Self {
        Self {
            civ_info,
            city,
            unit: None,
            tile: city.as_ref().and_then(|c| c.get_center_tile()),
            our_combatant: None,
            their_combatant: None,
            attacked_tile: None,
            combat_action: None,
            region: None,
            game_info,
            ignore_conditionals: false,
        }
    }

    /// Create a new state for a city
    pub fn new_for_city(city: City) -> Self {
        Self::new_with_game_info(
            Some(city.game_info.clone()),
            Some(city.civ.clone()),
            Some(city),
        )
    }

    /// Create a new state for a unit
    pub fn new_for_unit(unit: MapUnit) -> Self {
        Self::new_with_game_info(
            Some(unit.game_info.clone()),
            Some(unit.civ.clone()),
            None,
        )
    }

    /// Create a new state for combat
    pub fn new_for_combat(
        our_combatant: Box<dyn ICombatant>,
        their_combatant: Option<Box<dyn ICombatant>>,
        attacked_tile: Option<Tile>,
        combat_action: Option<CombatAction>,
    ) -> Self {
        let civ_info = our_combatant.get_civ_info();
        let city = our_combatant
            .as_any()
            .downcast_ref::<CityCombatant>()
            .map(|c| c.city.clone());
        let unit = our_combatant
            .as_any()
            .downcast_ref::<MapUnitCombatant>()
            .map(|c| c.unit.clone());
        let tile = our_combatant.get_tile();

        Self {
            civ_info: Some(civ_info),
            city,
            unit,
            tile,
            our_combatant: Some(our_combatant),
            their_combatant,
            attacked_tile,
            combat_action,
            region: None,
            game_info: Some(civ_info.game_info.clone()),
            ignore_conditionals: false,
        }
    }

    /// Get the relevant unit for this state
    pub fn relevant_unit(&self) -> Option<&MapUnit> {
        if let Some(combatant) = &self.our_combatant {
            if let Some(unit_combatant) = combatant.as_any().downcast_ref::<MapUnitCombatant>() {
                return Some(&unit_combatant.unit);
            }
        }
        self.unit.as_ref()
    }

    /// Get the relevant tile for this state
    pub fn relevant_tile(&self) -> Option<&Tile> {
        self.attacked_tile
            .as_ref()
            .or(self.tile.as_ref())
            .or_else(|| {
                self.relevant_unit()
                    .and_then(|unit| unit.get_tile())
            })
            .or_else(|| {
                self.city
                    .as_ref()
                    .and_then(|city| city.get_center_tile())
            })
    }

    /// Get the relevant city for this state
    pub fn relevant_city(&self) -> Option<&City> {
        if let Some(city) = &self.city {
            return Some(city);
        }

        // Edge case: If we attack a city, the "relevant tile" becomes the attacked tile -
        // but we DO NOT want that city to become the relevant city because then *our* conditionals get checked against
        // the *other civ's* cities, leading to e.g. resource amounts being defined as the *other civ's* resource amounts
        let relevant_tile_for_city = self.tile
            .as_ref()
            .or_else(|| {
                self.relevant_unit()
                    .and_then(|unit| unit.get_tile())
            });

        if let Some(tile) = relevant_tile_for_city {
            if let Some(city) = tile.get_city() {
                if city.civ == self.civ_info || city.civ == self.relevant_unit().map(|u| &u.civ) {
                    return Some(city);
                }
            }
        }

        None
    }

    /// Get the relevant civilization for this state
    pub fn relevant_civ(&self) -> Option<&Civilization> {
        self.civ_info
            .as_ref()
            .or_else(|| self.relevant_city().map(|c| &c.civ))
            .or_else(|| self.relevant_unit().map(|u| &u.civ))
    }

    /// Get the amount of a resource
    pub fn get_resource_amount(&self, resource_name: &str) -> i32 {
        if let Some(city) = self.relevant_city() {
            return city.get_available_resource_amount(resource_name);
        }
        if let Some(civ) = self.relevant_civ() {
            return civ.get_resource_amount(resource_name);
        }
        0
    }

    /// Get the amount of a stat
    pub fn get_stat_amount(&self, stat: Stat) -> i32 {
        if let Some(city) = self.relevant_city() {
            return city.get_stat_reserve(stat);
        }
        if let Some(civ) = self.relevant_civ() {
            if Stat::stats_with_civ_wide_field().contains(&stat) {
                return civ.get_stat_reserve(stat);
            }
        }
        0
    }

    /// Get a state that ignores conditionals
    pub fn ignore_conditionals() -> Self {
        Self {
            ignore_conditionals: true,
            ..Self::new()
        }
    }

    /// Get an empty state
    pub fn empty_state() -> Self {
        Self::new()
    }
}

impl Hash for StateForConditionals {
    /// Used ONLY for stateBasedRandom in [Conditionals.conditional_applies] to prevent save scumming on [UniqueType.ConditionalChance]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut result = 0;

        // Helper function to hash optional values
        let hash_opt = |value: Option<&impl Hash>| {
            value.map_or(0, |v| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                v.hash(&mut hasher);
                hasher.finish() as i32
            })
        };

        // Hash each field
        result = 31 * result + hash_opt(self.relevant_civ());
        result = 31 * result + hash_opt(self.relevant_city().map(|c| c as &dyn Hash));
        result = 31 * result + hash_opt(self.relevant_unit());
        result = 31 * result + hash_opt(self.relevant_tile());
        result = 31 * result + hash_opt(self.our_combatant.as_ref().map(|c| c as &dyn Hash));
        result = 31 * result + hash_opt(self.their_combatant.as_ref().map(|c| c as &dyn Hash));
        result = 31 * result + hash_opt(self.attacked_tile.as_ref());
        result = 31 * result + hash_opt(self.combat_action.as_ref());
        result = 31 * result + hash_opt(self.region.as_ref());
        result = 31 * result + self.ignore_conditionals.hash(state);

        result.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty_state() {
        let state = StateForConditionals::new();
        assert!(state.civ_info.is_none());
        assert!(state.city.is_none());
        assert!(state.unit.is_none());
        assert!(state.tile.is_none());
        assert!(state.our_combatant.is_none());
        assert!(state.their_combatant.is_none());
        assert!(state.attacked_tile.is_none());
        assert!(state.combat_action.is_none());
        assert!(state.region.is_none());
        assert!(state.game_info.is_none());
        assert!(!state.ignore_conditionals);
    }

    #[test]
    fn test_ignore_conditionals() {
        let state = StateForConditionals::ignore_conditionals();
        assert!(state.ignore_conditionals);
    }

    #[test]
    fn test_empty_state() {
        let state = StateForConditionals::empty_state();
        assert!(state.civ_info.is_none());
        assert!(state.city.is_none());
        assert!(state.unit.is_none());
        assert!(state.tile.is_none());
        assert!(state.our_combatant.is_none());
        assert!(state.their_combatant.is_none());
        assert!(state.attacked_tile.is_none());
        assert!(state.combat_action.is_none());
        assert!(state.region.is_none());
        assert!(state.game_info.is_none());
        assert!(!state.ignore_conditionals);
    }
}