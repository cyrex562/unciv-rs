use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::models::stats::GameResource;
use crate::models::civilization::NotificationIcon;

/// Additional statistics in the game that are not part of the main Stats struct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubStat {
    /// Golden Age points
    GoldenAgePoints,
    /// Total Culture accumulated
    TotalCulture,
    /// Stored Food in cities
    StoredFood,
}

impl SubStat {
    /// Get the display text for this SubStat
    pub fn text(&self) -> &'static str {
        match self {
            SubStat::GoldenAgePoints => "Golden Age points",
            SubStat::TotalCulture => "Total Culture",
            SubStat::StoredFood => "Stored Food",
        }
    }

    /// Get the notification icon for this SubStat
    pub fn icon(&self) -> NotificationIcon {
        match self {
            SubStat::GoldenAgePoints => NotificationIcon::Happiness,
            SubStat::TotalCulture => NotificationIcon::Culture,
            SubStat::StoredFood => NotificationIcon::Food,
        }
    }

    /// Get all SubStats that can be used to buy things
    pub fn useable_to_buy() -> HashSet<SubStat> {
        let mut set = HashSet::new();
        set.insert(SubStat::GoldenAgePoints);
        set.insert(SubStat::StoredFood);
        set
    }

    /// Get all SubStats that apply civilization-wide
    pub fn civ_wide_sub_stats() -> HashSet<SubStat> {
        let mut set = HashSet::new();
        set.insert(SubStat::GoldenAgePoints);
        set.insert(SubStat::TotalCulture);
        set
    }

    /// Safely convert a string to a SubStat
    pub fn safe_value_of(name: &str) -> Option<SubStat> {
        match name {
            "Golden Age points" => Some(SubStat::GoldenAgePoints),
            "Total Culture" => Some(SubStat::TotalCulture),
            "Stored Food" => Some(SubStat::StoredFood),
            _ => None,
        }
    }
}

impl GameResource for SubStat {
    fn name(&self) -> String {
        self.text().to_string()
    }
}

// Constants for commonly used sets
lazy_static::lazy_static! {
    /// Set of SubStats that can be used to buy things
    pub static ref USEABLE_TO_BUY: HashSet<SubStat> = SubStat::useable_to_buy();

    /// Set of SubStats that apply civilization-wide
    pub static ref CIV_WIDE_SUB_STATS: HashSet<SubStat> = SubStat::civ_wide_sub_stats();
}