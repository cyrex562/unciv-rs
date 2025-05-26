use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use encyclopedia::civilopedia::Civilopedia;

/// Represents game difficulty settings
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Difficulty {
    /// The name of the difficulty level
    pub name: String,
    /// Base happiness for the player
    pub base_happiness: i32,
    /// Extra happiness per luxury resource
    pub extra_happiness_per_luxury: f32,
    /// Research cost modifier
    pub research_cost_modifier: f32,
    /// Unit cost modifier
    pub unit_cost_modifier: f32,
    /// Base unit supply limit
    pub unit_supply_base: i32,
    /// Unit supply per city
    pub unit_supply_per_city: i32,
    /// Building cost modifier
    pub building_cost_modifier: f32,
    /// Policy cost modifier
    pub policy_cost_modifier: f32,
    /// Unhappiness modifier
    pub unhappiness_modifier: f32,
    /// Bonus against barbarians
    pub barbarian_bonus: f32,
    /// Barbarian spawn delay
    pub barbarian_spawn_delay: i32,
    /// Bonus starting units for the player
    pub player_bonus_starting_units: Vec<String>,
    /// AI difficulty level
    pub ai_difficulty_level: Option<String>,
    /// AI city growth modifier
    pub ai_city_growth_modifier: f32,
    /// AI unit cost modifier
    pub ai_unit_cost_modifier: f32,
    /// AI building cost modifier
    pub ai_building_cost_modifier: f32,
    /// AI wonder cost modifier
    pub ai_wonder_cost_modifier: f32,
    /// AI building maintenance modifier
    pub ai_building_maintenance_modifier: f32,
    /// AI unit maintenance modifier
    pub ai_unit_maintenance_modifier: f32,
    /// AI unit supply modifier
    pub ai_unit_supply_modifier: f32,
    /// Free technologies for AI
    pub ai_free_techs: Vec<String>,
    /// Bonus starting units for major AI civilizations
    pub ai_major_civ_bonus_starting_units: Vec<String>,
    /// Bonus starting units for AI city-states
    pub ai_city_state_bonus_starting_units: Vec<String>,
    /// AI unhappiness modifier
    pub ai_unhappiness_modifier: f32,
    /// Turns until barbarians can enter player tiles
    pub turn_barbarians_can_enter_player_tiles: i32,
    /// Gold reward for clearing barbarian camps
    pub clear_barbarian_camp_reward: i32,
    /// Civilopedia component
    pub civilopedia: Civilopedia,
}

impl Difficulty {
    /// Creates a new Difficulty with default values
    pub fn new() -> Self {
        Difficulty {
            name: String::new(),
            base_happiness: 0,
            extra_happiness_per_luxury: 0.0,
            research_cost_modifier: 1.0,
            unit_cost_modifier: 1.0,
            unit_supply_base: 5,
            unit_supply_per_city: 2,
            building_cost_modifier: 1.0,
            policy_cost_modifier: 1.0,
            unhappiness_modifier: 1.0,
            barbarian_bonus: 0.0,
            barbarian_spawn_delay: 0,
            player_bonus_starting_units: Vec::new(),
            ai_difficulty_level: None,
            ai_city_growth_modifier: 1.0,
            ai_unit_cost_modifier: 1.0,
            ai_building_cost_modifier: 1.0,
            ai_wonder_cost_modifier: 1.0,
            ai_building_maintenance_modifier: 1.0,
            ai_unit_maintenance_modifier: 1.0,
            ai_unit_supply_modifier: 0.0,
            ai_free_techs: Vec::new(),
            ai_major_civ_bonus_starting_units: Vec::new(),
            ai_city_state_bonus_starting_units: Vec::new(),
            ai_unhappiness_modifier: 1.0,
            turn_barbarians_can_enter_player_tiles: 0,
            clear_barbarian_camp_reward: 25,
            civilopedia: Civilopedia::default(),
        }
    }

    /// Converts a float to a percentage
    fn to_percent(value: f32) -> i32 {
        (value * 100.0) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_new() {
        let difficulty = Difficulty::new();
        assert!(difficulty.name.is_empty());
        assert_eq!(difficulty.base_happiness, 0);
        assert_eq!(difficulty.extra_happiness_per_luxury, 0.0);
        assert_eq!(difficulty.research_cost_modifier, 1.0);
        assert_eq!(difficulty.unit_cost_modifier, 1.0);
        assert_eq!(difficulty.unit_supply_base, 5);
        assert_eq!(difficulty.unit_supply_per_city, 2);
        assert_eq!(difficulty.building_cost_modifier, 1.0);
        assert_eq!(difficulty.policy_cost_modifier, 1.0);
        assert_eq!(difficulty.unhappiness_modifier, 1.0);
        assert_eq!(difficulty.barbarian_bonus, 0.0);
        assert_eq!(difficulty.barbarian_spawn_delay, 0);
        assert!(difficulty.player_bonus_starting_units.is_empty());
        assert!(difficulty.ai_difficulty_level.is_none());
        assert_eq!(difficulty.ai_city_growth_modifier, 1.0);
        assert_eq!(difficulty.ai_unit_cost_modifier, 1.0);
        assert_eq!(difficulty.ai_building_cost_modifier, 1.0);
        assert_eq!(difficulty.ai_wonder_cost_modifier, 1.0);
        assert_eq!(difficulty.ai_building_maintenance_modifier, 1.0);
        assert_eq!(difficulty.ai_unit_maintenance_modifier, 1.0);
        assert_eq!(difficulty.ai_unit_supply_modifier, 0.0);
        assert!(difficulty.ai_free_techs.is_empty());
        assert!(difficulty.ai_major_civ_bonus_starting_units.is_empty());
        assert!(difficulty.ai_city_state_bonus_starting_units.is_empty());
        assert_eq!(difficulty.ai_unhappiness_modifier, 1.0);
        assert_eq!(difficulty.turn_barbarians_can_enter_player_tiles, 0);
        assert_eq!(difficulty.clear_barbarian_camp_reward, 25);
        assert!(difficulty.civilopedia.entry.is_empty());
    }

    #[test]
    fn test_to_percent() {
        assert_eq!(Difficulty::to_percent(1.0), 100);
        assert_eq!(Difficulty::to_percent(0.5), 50);
        assert_eq!(Difficulty::to_percent(0.0), 0);
        assert_eq!(Difficulty::to_percent(2.0), 200);
    }
}
