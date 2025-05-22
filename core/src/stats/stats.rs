use std::collections::{HashMap, HashSet};
use std::fmt;
use std::iter::Iterator;
use std::ops::{Add, Div, Mul, Sub};
use serde::{Serialize, Deserialize};
use regex::Regex;
use lazy_static::lazy_static;
use crate::models::stats::Stat;
use crate::models::translations::tr;

/// A container for the seven basic "currencies" in Unciv.
/// Mutable, allowing for easy merging of sources and applying bonuses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub production: f32,
    pub food: f32,
    pub gold: f32,
    pub science: f32,
    pub culture: f32,
    pub happiness: f32,
    pub faith: f32,
}

/// Represents one Stat/value pair returned by the iterator
#[derive(Debug, Clone)]
pub struct StatValuePair {
    pub key: Stat,
    pub value: f32,
}

impl Stats {
    /// Create a new Stats instance with default values (all zeros)
    pub fn new() -> Self {
        Stats {
            production: 0.0,
            food: 0.0,
            gold: 0.0,
            science: 0.0,
            culture: 0.0,
            happiness: 0.0,
            faith: 0.0,
        }
    }

    /// Create a new Stats instance with specified values
    pub fn with_values(
        production: f32,
        food: f32,
        gold: f32,
        science: f32,
        culture: f32,
        happiness: f32,
        faith: f32,
    ) -> Self {
        Stats {
            production,
            food,
            gold,
            science,
            culture,
            happiness,
            faith,
        }
    }

    /// Get a value for a given Stat
    pub fn get(&self, stat: Stat) -> f32 {
        match stat {
            Stat::Production => self.production,
            Stat::Food => self.food,
            Stat::Gold => self.gold,
            Stat::Science => self.science,
            Stat::Culture => self.culture,
            Stat::Happiness => self.happiness,
            Stat::Faith => self.faith,
        }
    }

    /// Set a value for a given Stat
    pub fn set(&mut self, stat: Stat, value: f32) {
        match stat {
            Stat::Production => self.production = value,
            Stat::Food => self.food = value,
            Stat::Gold => self.gold = value,
            Stat::Science => self.science = value,
            Stat::Culture => self.culture = value,
            Stat::Happiness => self.happiness = value,
            Stat::Faith => self.faith = value,
        }
    }

    /// Compare two Stats instances
    pub fn equals(&self, other: &Stats) -> bool {
        self.production == other.production
            && self.food == other.food
            && self.gold == other.gold
            && self.science == other.science
            && self.culture == other.culture
            && self.happiness == other.happiness
            && self.faith == other.faith
    }

    /// Create a new instance containing the same values as this one
    pub fn clone(&self) -> Stats {
        Stats {
            production: self.production,
            food: self.food,
            gold: self.gold,
            science: self.science,
            culture: self.culture,
            happiness: self.happiness,
            faith: self.faith,
        }
    }

    /// Check if all values are zero
    pub fn is_empty(&self) -> bool {
        self.production == 0.0
            && self.food == 0.0
            && self.gold == 0.0
            && self.science == 0.0
            && self.culture == 0.0
            && self.happiness == 0.0
            && self.faith == 0.0
    }

    /// Reset all values to zero
    pub fn clear(&mut self) {
        self.production = 0.0;
        self.food = 0.0;
        self.gold = 0.0;
        self.science = 0.0;
        self.culture = 0.0;
        self.happiness = 0.0;
        self.faith = 0.0;
    }

    /// Add each value of another Stats instance to this one in place
    pub fn add(&mut self, other: &Stats) -> &mut Stats {
        self.production += other.production;
        self.food += other.food;
        self.gold += other.gold;
        self.science += other.science;
        self.culture += other.culture;
        self.happiness += other.happiness;
        self.faith += other.faith;
        self
    }

    /// Add a value to a specific stat
    pub fn add_stat(&mut self, stat: Stat, value: f32) -> &mut Stats {
        self.set(stat, value + self.get(stat));
        self
    }

    /// Apply weighting for Production Ranking
    pub fn apply_ranking_weights(&mut self) {
        self.food *= 14.0;
        self.production *= 12.01; // tie break Production vs gold
        self.gold *= 6.0; // 2 gold worth about 1 production
        self.science *= 9.01; // 4 Science better than 3 Production
        self.culture *= 8.0;
        self.happiness *= 10.0; // base
        self.faith *= 7.0;
    }

    /// Get a sequence of non-zero Stat/value pairs
    pub fn as_sequence(&self) -> Vec<StatValuePair> {
        let mut result = Vec::new();
        if self.production != 0.0 {
            result.push(StatValuePair {
                key: Stat::Production,
                value: self.production,
            });
        }
        if self.food != 0.0 {
            result.push(StatValuePair {
                key: Stat::Food,
                value: self.food,
            });
        }
        if self.gold != 0.0 {
            result.push(StatValuePair {
                key: Stat::Gold,
                value: self.gold,
            });
        }
        if self.science != 0.0 {
            result.push(StatValuePair {
                key: Stat::Science,
                value: self.science,
            });
        }
        if self.culture != 0.0 {
            result.push(StatValuePair {
                key: Stat::Culture,
                value: self.culture,
            });
        }
        if self.happiness != 0.0 {
            result.push(StatValuePair {
                key: Stat::Happiness,
                value: self.happiness,
            });
        }
        if self.faith != 0.0 {
            result.push(StatValuePair {
                key: Stat::Faith,
                value: self.faith,
            });
        }
        result
    }

    /// Get all values as a sequence
    pub fn values(&self) -> Vec<f32> {
        vec![
            self.production,
            self.food,
            self.gold,
            self.science,
            self.culture,
            self.happiness,
            self.faith,
        ]
    }

    /// Convert to string with translated values
    pub fn to_string(&self) -> String {
        self.as_sequence()
            .iter()
            .map(|pair| {
                let sign = if pair.value > 0.0 { "+" } else { "" };
                format!("{}{} {}", sign, tr(&pair.value.to_string()), tr(&pair.key.name()))
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Convert to string for notifications (in English)
    pub fn to_string_for_notifications(&self) -> String {
        self.as_sequence()
            .iter()
            .map(|pair| {
                let sign = if pair.value > 0.0 { "+" } else { "" };
                format!("{}{} {}", sign, pair.value.to_string(), pair.key.name())
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Convert to string with decimals
    pub fn to_string_with_decimals(&self) -> String {
        self.as_sequence()
            .iter()
            .map(|pair| {
                let sign = if pair.value > 0.0 { "+" } else { "" };
                let value_str = tr(&pair.value.to_string());
                let value_str = value_str.trim_end_matches(".0");
                format!("{}{} {}", sign, value_str, tr(&pair.key.name()))
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Convert to string without icons
    pub fn to_string_without_icons(&self) -> String {
        self.as_sequence()
            .iter()
            .map(|pair| {
                format!("{} {}", tr(&pair.value.to_string()), tr(&pair.key.name()[1..]))
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Convert to string with only icons
    pub fn to_string_only_icons(&self, add_plus_sign: bool) -> String {
        self.as_sequence()
            .iter()
            .map(|pair| {
                let sign = if add_plus_sign && pair.value > 0.0 { "+" } else { "" };
                format!("{}{} {}", sign, pair.value.to_string(), pair.key.character())
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Check if a string is a valid representation of Stats
    pub fn is_stats(string: &str) -> bool {
        if string.is_empty() || !string.starts_with(|c| c == '+' || c == '-') {
            return false; // very quick negative check before the heavy Regex
        }
        STATS_REGEX.is_match(string)
    }

    /// Parse a string to a Stats instance
    pub fn parse(string: &str) -> Stats {
        let mut result = Stats::new();
        let stats_with_bonuses = string.split(", ");
        for stat_with_bonuses in stats_with_bonuses {
            if let Some(captures) = STAT_REGEX.captures(stat_with_bonuses) {
                let stat_name = &captures[3];
                let stat_amount = captures[2].parse::<f32>().unwrap_or(0.0)
                    * if captures[1] == "-" { -1.0 } else { 1.0 };
                if let Some(stat) = Stat::safe_value_of(stat_name) {
                    result.add_stat(stat, stat_amount);
                }
            }
        }
        result
    }
}

// Implement standard traits
impl Default for Stats {
    fn default() -> Self {
        Stats::new()
    }
}

impl PartialEq for Stats {
    fn eq(&self, other: &Stats) -> bool {
        self.equals(other)
    }
}

impl Eq for Stats {}

// Implement arithmetic operators
impl Add for Stats {
    type Output = Stats;

    fn add(self, other: Stats) -> Stats {
        let mut result = self.clone();
        result.add(&other);
        result
    }
}

impl Sub for Stats {
    type Output = Stats;

    fn sub(self, other: Stats) -> Stats {
        let mut result = self.clone();
        result.add(&(other * -1.0));
        result
    }
}

impl Mul<f32> for Stats {
    type Output = Stats;

    fn mul(self, number: f32) -> Stats {
        Stats {
            production: self.production * number,
            food: self.food * number,
            gold: self.gold * number,
            science: self.science * number,
            culture: self.culture * number,
            happiness: self.happiness * number,
            faith: self.faith * number,
        }
    }
}

impl Mul<i32> for Stats {
    type Output = Stats;

    fn mul(self, number: i32) -> Stats {
        self * (number as f32)
    }
}

impl Div<f32> for Stats {
    type Output = Stats;

    fn div(self, number: f32) -> Stats {
        self * (1.0 / number)
    }
}

// Implement Display trait for string representation
impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Implement Iterator for Stats
impl<'a> IntoIterator for &'a Stats {
    type Item = StatValuePair;
    type IntoIter = std::vec::IntoIter<StatValuePair>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_sequence().into_iter()
    }
}

// Constants
impl Stats {
    /// Zero stats
    pub const ZERO: Stats = Stats {
        production: 0.0,
        food: 0.0,
        gold: 0.0,
        science: 0.0,
        culture: 0.0,
        happiness: 0.0,
        faith: 0.0,
    };

    /// Default city center minimum stats
    pub const DEFAULT_CITY_CENTER_MINIMUM: Stats = Stats {
        production: 1.0,
        food: 2.0,
        gold: 0.0,
        science: 0.0,
        culture: 0.0,
        happiness: 0.0,
        faith: 0.0,
    };
}

// Regex patterns for parsing
lazy_static! {
    static ref STAT_REGEX: Regex = {
        let all_stat_names = Stat::names().join("|");
        let pattern = format!("([+-])(\\d+) ({})", all_stat_names);
        Regex::new(&pattern).unwrap()
    };

    static ref STATS_REGEX: Regex = {
        let all_stat_names = Stat::names().join("|");
        let pattern = format!("([+-])(\\d+) ({})", all_stat_names);
        let full_pattern = format!("{}(, {})*", pattern, pattern);
        Regex::new(&full_pattern).unwrap()
    };
}

/// A map of strings to Stats
#[derive(Debug, Clone, Default)]
pub struct StatMap {
    map: HashMap<String, Stats>,
}

impl StatMap {
    /// Create a new empty StatMap
    pub fn new() -> Self {
        StatMap {
            map: HashMap::new(),
        }
    }

    /// Add stats from a source
    pub fn add(&mut self, source: &str, stats: &Stats) {
        // We always clone to avoid touching the mutable stats of uniques
        if !self.map.contains_key(source) {
            self.map.insert(source.to_string(), stats.clone());
        } else {
            if let Some(existing_stats) = self.map.get_mut(source) {
                existing_stats.add(stats);
            }
        }
    }

    /// Get stats for a source
    pub fn get(&self, source: &str) -> Option<&Stats> {
        self.map.get(source)
    }

    /// Check if a source exists
    pub fn contains_key(&self, source: &str) -> bool {
        self.map.contains_key(source)
    }

    /// Insert stats for a source
    pub fn insert(&mut self, source: String, stats: Stats) -> Option<Stats> {
        self.map.insert(source, stats)
    }
}

impl Default for StatMap {
    fn default() -> Self {
        StatMap::new()
    }
}