use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::Unique;
use crate::models::stats::INamed;
use crate::ui::components::fonts::Fonts;
use crate::ui::screens::civilopedia_screen::{FormattedLine, ICivilopediaText};

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
    /// Civilopedia text
    pub civilopedia_text: Vec<FormattedLine>,
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
            civilopedia_text: Vec::new(),
        }
    }

    /// Converts a float to a percentage
    fn to_percent(value: f32) -> i32 {
        (value * 100.0) as i32
    }
}

impl INamed for Difficulty {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl ICivilopediaText for Difficulty {
    fn make_link(&self) -> String {
        format!("Difficulty/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Player settings
        lines.push(FormattedLine::new("Player settings", 3, 0));
        lines.push(FormattedLine::new(
            format!("{{Base happiness}}: {} {}", self.base_happiness, Fonts::HAPPINESS),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Extra happiness per luxury}}: {} {}",
                self.extra_happiness_per_luxury as i32,
                Fonts::HAPPINESS
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Research cost modifier}}: {}% {}",
                Self::to_percent(self.research_cost_modifier),
                Fonts::SCIENCE
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Unit cost modifier}}: {}% {}",
                Self::to_percent(self.unit_cost_modifier),
                Fonts::PRODUCTION
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Building cost modifier}}: {}% {}",
                Self::to_percent(self.building_cost_modifier),
                Fonts::PRODUCTION
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Policy cost modifier}}: {}% {}",
                Self::to_percent(self.policy_cost_modifier),
                Fonts::CULTURE
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Unhappiness modifier}}: {}%",
                Self::to_percent(self.unhappiness_modifier)
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Bonus vs. Barbarians}}: {}% {}",
                Self::to_percent(self.barbarian_bonus),
                Fonts::STRENGTH
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!("{{Barbarian spawning delay}}: {}", self.barbarian_spawn_delay),
            0,
            1,
        ));

        // Player bonus starting units
        if !self.player_bonus_starting_units.is_empty() {
            lines.push(FormattedLine::new("", 0, 0));
            lines.push(FormattedLine::new("{Bonus starting units}:", 0, 1));
            let unit_counts: HashMap<&String, usize> = self
                .player_bonus_starting_units
                .iter()
                .fold(HashMap::new(), |mut map, unit| {
                    *map.entry(unit).or_insert(0) += 1;
                    map
                });
            for (unit, count) in unit_counts {
                let text = if count == 1 {
                    format!("[{}]", unit)
                } else {
                    format!("{} [{}]", count, unit)
                };
                lines.push(FormattedLine::new(
                    Unique::new(text, None, None),
                    0,
                    2,
                ));
            }
        }

        // AI settings
        lines.push(FormattedLine::new("", 0, 0));
        lines.push(FormattedLine::new("AI settings", 3, 0));
        if let Some(level) = &self.ai_difficulty_level {
            lines.push(FormattedLine::new(
                format!("{{AI difficulty level}}: {}", level),
                0,
                1,
            ));
        }
        lines.push(FormattedLine::new(
            format!(
                "{{AI city growth modifier}}: {}% {}",
                Self::to_percent(self.ai_city_growth_modifier),
                Fonts::FOOD
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI unit cost modifier}}: {}% {}",
                Self::to_percent(self.ai_unit_cost_modifier),
                Fonts::PRODUCTION
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI building cost modifier}}: {}% {}",
                Self::to_percent(self.ai_building_cost_modifier),
                Fonts::PRODUCTION
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI wonder cost modifier}}: {}% {}",
                Self::to_percent(self.ai_wonder_cost_modifier),
                Fonts::PRODUCTION
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI building maintenance modifier}}: {}% {}",
                Self::to_percent(self.ai_building_maintenance_modifier),
                Fonts::GOLD
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI unit maintenance modifier}}: {}% {}",
                Self::to_percent(self.ai_unit_maintenance_modifier),
                Fonts::GOLD
            ),
            0,
            1,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{AI unhappiness modifier}}: {}%",
                Self::to_percent(self.ai_unhappiness_modifier)
            ),
            0,
            1,
        ));

        // AI free techs
        if !self.ai_free_techs.is_empty() {
            lines.push(FormattedLine::new("", 0, 0));
            lines.push(FormattedLine::new("{AI free techs}:", 0, 1));
            for tech in &self.ai_free_techs {
                lines.push(FormattedLine::new_with_link(
                    tech.clone(),
                    format!("Technology/{}", tech),
                    0,
                    2,
                ));
            }
        }

        // AI major civ bonus starting units
        if !self.ai_major_civ_bonus_starting_units.is_empty() {
            lines.push(FormattedLine::new("", 0, 0));
            lines.push(FormattedLine::new(
                "{Major AI civilization bonus starting units}:",
                0,
                1,
            ));
            let unit_counts: HashMap<&String, usize> = self
                .ai_major_civ_bonus_starting_units
                .iter()
                .fold(HashMap::new(), |mut map, unit| {
                    *map.entry(unit).or_insert(0) += 1;
                    map
                });
            for (unit, count) in unit_counts {
                let text = if count == 1 {
                    format!("[{}]", unit)
                } else {
                    format!("{} [{}]", count, unit)
                };
                lines.push(FormattedLine::new(
                    Unique::new(text, None, None),
                    0,
                    2,
                ));
            }
        }

        // AI city state bonus starting units
        if !self.ai_city_state_bonus_starting_units.is_empty() {
            lines.push(FormattedLine::new("", 0, 0));
            lines.push(FormattedLine::new(
                "{City state bonus starting units}:",
                0,
                1,
            ));
            let unit_counts: HashMap<&String, usize> = self
                .ai_city_state_bonus_starting_units
                .iter()
                .fold(HashMap::new(), |mut map, unit| {
                    *map.entry(unit).or_insert(0) += 1;
                    map
                });
            for (unit, count) in unit_counts {
                let text = if count == 1 {
                    format!("[{}]", unit)
                } else {
                    format!("{} [{}]", count, unit)
                };
                lines.push(FormattedLine::new(
                    Unique::new(text, None, None),
                    0,
                    2,
                ));
            }
        }

        // Barbarian settings
        lines.push(FormattedLine::new("", 0, 0));
        lines.push(FormattedLine::new(
            format!(
                "{{Turns until barbarians enter player tiles}}: {} {}",
                self.turn_barbarians_can_enter_player_tiles,
                Fonts::TURN
            ),
            0,
            0,
        ));
        lines.push(FormattedLine::new(
            format!(
                "{{Gold reward for clearing barbarian camps}}: {} {}",
                self.clear_barbarian_camp_reward,
                Fonts::GOLD
            ),
            0,
            0,
        ));

        lines
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
        assert!(difficulty.civilopedia_text.is_empty());
    }

    #[test]
    fn test_to_percent() {
        assert_eq!(Difficulty::to_percent(1.0), 100);
        assert_eq!(Difficulty::to_percent(0.5), 50);
        assert_eq!(Difficulty::to_percent(0.0), 0);
        assert_eq!(Difficulty::to_percent(2.0), 200);
    }

    #[test]
    fn test_make_link() {
        let mut difficulty = Difficulty::new();
        difficulty.set_name("TestDifficulty".to_string());
        assert_eq!(difficulty.make_link(), "Difficulty/TestDifficulty");
    }
}