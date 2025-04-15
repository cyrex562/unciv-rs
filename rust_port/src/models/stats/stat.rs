use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::models::stats::GameResource;
use crate::models::skins::Color;
use crate::logic::civilization::NotificationIcon;
use crate::models::UncivSound;
use crate::ui::components::fonts::Fonts;

/// Represents a game statistic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stat {
    /// Production stat
    Production,
    /// Food stat
    Food,
    /// Gold stat
    Gold,
    /// Science stat
    Science,
    /// Culture stat
    Culture,
    /// Happiness stat
    Happiness,
    /// Faith stat
    Faith,
}

impl Stat {
    /// Get the notification icon for this stat
    pub fn notification_icon(&self) -> &str {
        match self {
            Stat::Production => NotificationIcon::Production,
            Stat::Food => NotificationIcon::Food,
            Stat::Gold => NotificationIcon::Gold,
            Stat::Science => NotificationIcon::Science,
            Stat::Culture => NotificationIcon::Culture,
            Stat::Happiness => NotificationIcon::Happiness,
            Stat::Faith => NotificationIcon::Faith,
        }
    }

    /// Get the purchase sound for this stat
    pub fn purchase_sound(&self) -> UncivSound {
        match self {
            Stat::Production => UncivSound::Click,
            Stat::Food => UncivSound::Click,
            Stat::Gold => UncivSound::Coin,
            Stat::Science => UncivSound::Chimes,
            Stat::Culture => UncivSound::Paper,
            Stat::Happiness => UncivSound::Click,
            Stat::Faith => UncivSound::Choir,
        }
    }

    /// Get the character representation for this stat
    pub fn character(&self) -> char {
        match self {
            Stat::Production => Fonts::production,
            Stat::Food => Fonts::food,
            Stat::Gold => Fonts::gold,
            Stat::Science => Fonts::science,
            Stat::Culture => Fonts::culture,
            Stat::Happiness => Fonts::happiness,
            Stat::Faith => Fonts::faith,
        }
    }

    /// Get the color for this stat
    pub fn color(&self) -> Color {
        match self {
            Stat::Production => Color::from_hex(0xc14d00),
            Stat::Food => Color::from_hex(0x24A348),
            Stat::Gold => Color::from_hex(0xffeb7f),
            Stat::Science => Color::from_hex(0x8c9dff),
            Stat::Culture => Color::from_hex(0x8b60ff),
            Stat::Happiness => Color::from_hex(0xffd800),
            Stat::Faith => Color::from_hex(0xcbdfff),
        }
    }

    /// Get a set of stats that can be used to buy things
    pub fn stats_usable_to_buy() -> HashSet<Stat> {
        let mut set = HashSet::new();
        set.insert(Stat::Gold);
        set.insert(Stat::Food);
        set.insert(Stat::Science);
        set.insert(Stat::Culture);
        set.insert(Stat::Faith);
        set
    }

    /// Get a set of stats that have civ-wide fields
    pub fn stats_with_civ_wide_field() -> HashSet<Stat> {
        let mut set = HashSet::new();
        set.insert(Stat::Gold);
        set.insert(Stat::Science);
        set.insert(Stat::Culture);
        set.insert(Stat::Faith);
        set
    }

    /// Get a stat from its name, or None if not found
    pub fn safe_value_of(name: &str) -> Option<Stat> {
        match name {
            "Production" => Some(Stat::Production),
            "Food" => Some(Stat::Food),
            "Gold" => Some(Stat::Gold),
            "Science" => Some(Stat::Science),
            "Culture" => Some(Stat::Culture),
            "Happiness" => Some(Stat::Happiness),
            "Faith" => Some(Stat::Faith),
            _ => None,
        }
    }

    /// Check if a string is a valid stat name
    pub fn is_stat(name: &str) -> bool {
        Stat::safe_value_of(name).is_some()
    }

    /// Get all stat names
    pub fn names() -> Vec<&'static str> {
        vec![
            "Production",
            "Food",
            "Gold",
            "Science",
            "Culture",
            "Happiness",
            "Faith",
        ]
    }
}

impl GameResource for Stat {
    fn name(&self) -> &str {
        match self {
            Stat::Production => "Production",
            Stat::Food => "Food",
            Stat::Gold => "Gold",
            Stat::Science => "Science",
            Stat::Culture => "Culture",
            Stat::Happiness => "Happiness",
            Stat::Faith => "Faith",
        }
    }
}