use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::models::ruleset::{RulesetObject, UniqueTarget};
use crate::models::stats::Stat;
use crate::models::game_info::IsPartOfGameInfoSerialization;
use crate::ui::screens::civilopediascreen::FormattedLine;
use crate::ui::fonts::Fonts;

/// Represents a game speed setting that affects various game mechanics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Speed {
    /// Base modifier for all speed-related values
    pub modifier: f32,
    /// Modifier for gold costs
    pub gold_cost_modifier: f32,
    /// Modifier for production costs
    pub production_cost_modifier: f32,
    /// Modifier for science costs
    pub science_cost_modifier: f32,
    /// Modifier for culture costs
    pub culture_cost_modifier: f32,
    /// Modifier for faith costs
    pub faith_cost_modifier: f32,
    /// Modifier for gold gifts
    pub gold_gift_modifier: f32,
    /// Scaling interval for city-state tribute
    pub city_state_tribute_scaling_interval: f32,
    /// Modifier for barbarian spawns
    pub barbarian_modifier: f32,
    /// Modifier for improvement build length
    pub improvement_build_length_modifier: f32,
    /// Modifier for golden age length
    pub golden_age_length_modifier: f32,
    /// Religious pressure for adjacent cities
    pub religious_pressure_adjacent_city: i32,
    /// Duration of peace deals
    pub peace_deal_duration: i32,
    /// Duration of deals
    pub deal_duration: i32,
    /// Starting year
    pub start_year: f32,
    /// List of turns with their corresponding years
    pub turns: Vec<HashMap<String, f32>>,
}

impl Speed {
    /// Create a new Speed instance with default values
    pub fn new() -> Self {
        Self {
            modifier: 1.0,
            gold_cost_modifier: 1.0,
            production_cost_modifier: 1.0,
            science_cost_modifier: 1.0,
            culture_cost_modifier: 1.0,
            faith_cost_modifier: 1.0,
            gold_gift_modifier: 1.0,
            city_state_tribute_scaling_interval: 6.5,
            barbarian_modifier: 1.0,
            improvement_build_length_modifier: 1.0,
            golden_age_length_modifier: 1.0,
            religious_pressure_adjacent_city: 6,
            peace_deal_duration: 10,
            deal_duration: 30,
            start_year: -4000.0,
            turns: Vec::new(),
        }
    }

    /// Get the years per turn for each turn
    pub fn years_per_turn(&self) -> Vec<YearsPerTurn> {
        self.turns.iter()
            .map(|turn| YearsPerTurn {
                year_interval: turn["yearsPerTurn"].unwrap_or(1.0),
                until_turn: turn["untilTurn"].unwrap_or(1.0) as i32,
            })
            .collect()
    }

    /// Get the stat cost modifiers
    pub fn stat_cost_modifiers(&self) -> HashMap<Stat, f32> {
        let mut map = HashMap::new();
        for stat in Stat::iter() {
            let modifier = match stat {
                Stat::Production => self.production_cost_modifier,
                Stat::Gold => self.gold_cost_modifier,
                Stat::Science => self.science_cost_modifier,
                Stat::Faith => self.faith_cost_modifier,
                Stat::Culture => self.culture_cost_modifier,
                _ => 1.0,
            };
            map.insert(stat, modifier);
        }
        map
    }

    /// Get the total number of turns
    pub fn num_total_turns(&self) -> i32 {
        self.years_per_turn().last().map(|ypt| ypt.until_turn).unwrap_or(0)
    }
}

impl RulesetObject for Speed {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Speed
    }

    fn make_link(&self) -> String {
        format!("Speed/{}", self.name)
    }

    fn get_civilopedia_text_header(&self) -> FormattedLine {
        FormattedLine::new(self.name.clone(), 2)
    }

    fn get_civilopedia_text_lines(&self, _ruleset: &Ruleset) -> Vec<FormattedLine> {
        vec![
            FormattedLine::new(format!("General speed modifier: [{}]%{}", self.modifier * 100.0, Fonts::turn())),
            FormattedLine::new(format!("Production cost modifier: [{}]%{}", self.production_cost_modifier * 100.0, Fonts::production())),
            FormattedLine::new(format!("Gold cost modifier: [{}]%{}", self.gold_cost_modifier * 100.0, Fonts::gold())),
            FormattedLine::new(format!("Science cost modifier: [{}]%{}", self.science_cost_modifier * 100.0, Fonts::science())),
            FormattedLine::new(format!("Culture cost modifier: [{}]%{}", self.culture_cost_modifier * 100.0, Fonts::culture())),
            FormattedLine::new(format!("Faith cost modifier: [{}]%{}", self.faith_cost_modifier * 100.0, Fonts::faith())),
            FormattedLine::new(format!("Improvement build length modifier: [{}]%{}", self.improvement_build_length_modifier * 100.0, Fonts::turn())),
            FormattedLine::new(format!("Diplomatic deal duration: [{}] turns{}", self.deal_duration, Fonts::turn())),
            FormattedLine::new(format!("Gold gift influence gain modifier: [{}]%{}", self.gold_gift_modifier * 100.0, Fonts::gold())),
            FormattedLine::new(format!("City-state tribute scaling interval: [{}] turns{}", self.city_state_tribute_scaling_interval, Fonts::turn())),
            FormattedLine::new(format!("Barbarian spawn modifier: [{}]%{}", self.barbarian_modifier * 100.0, Fonts::strength())),
            FormattedLine::new(format!("Golden age length modifier: [{}]%{}", self.golden_age_length_modifier * 100.0, Fonts::happiness())),
            FormattedLine::new(format!("Adjacent city religious pressure: [{}]{}", self.religious_pressure_adjacent_city, Fonts::faith())),
            FormattedLine::new(format!("Peace deal duration: [{}] turns{}", self.peace_deal_duration, Fonts::turn())),
            FormattedLine::new(format!("Start year: [{} {}]",
                self.start_year.abs() as i32,
                if self.start_year < 0.0 { "BC" } else { "AD" }
            )),
        ]
    }

    fn get_sort_group(&self, _ruleset: &Ruleset) -> i32 {
        (self.modifier * 1000.0) as i32
    }
}

impl IsPartOfGameInfoSerialization for Speed {}

/// Represents the years per turn for a specific turn range
#[derive(Clone, Debug)]
pub struct YearsPerTurn {
    /// The number of years per turn
    pub year_interval: f32,
    /// The turn number this applies until
    pub until_turn: i32,
}

impl Speed {
    /// Default speed setting
    pub const DEFAULT: &'static str = "Quick";
    /// Default speed setting for simulation
    pub const DEFAULT_FOR_SIMULATION: &'static str = "Standard";
}