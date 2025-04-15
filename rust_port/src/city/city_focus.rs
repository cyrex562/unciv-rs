use std::collections::HashSet;
use lazy_static::lazy_static;
use crate::models::stats::stat::Stat;
use crate::models::stats::stats::Stats;

/// Controls automatic worker-to-tile assignment
/// Order matters for building the CitizenManagementTable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CityFocus {
    /// Default focus with no specific stat emphasis
    NoFocus,
    /// Manual control of worker assignment
    Manual,
    /// Focus on Food production
    FoodFocus,
    /// Focus on Production
    ProductionFocus,
    /// Focus on Gold generation
    GoldFocus,
    /// Focus on Science generation
    ScienceFocus,
    /// Focus on Culture generation
    CultureFocus,
    /// Focus on Happiness
    HappinessFocus,
    /// Focus on Faith generation
    FaithFocus,
    /// Focus on both Gold and Food
    GoldGrowthFocus,
    /// Focus on both Production and Food
    ProductionGrowthFocus,
}

impl CityFocus {
    /// Gets the display label for the focus
    pub fn label(&self) -> &'static str {
        match self {
            CityFocus::NoFocus => "Default",
            CityFocus::Manual => "Manual",
            CityFocus::FoodFocus => "🍖",
            CityFocus::ProductionFocus => "⚒",
            CityFocus::GoldFocus => "⌛",
            CityFocus::ScienceFocus => "🔬",
            CityFocus::CultureFocus => "🎭",
            CityFocus::HappinessFocus => "😊",
            CityFocus::FaithFocus => "🕊",
            CityFocus::GoldGrowthFocus => "⌛🍖",
            CityFocus::ProductionGrowthFocus => "⚒🍖",
        }
    }

    /// Whether this focus should be shown in the citizen management table
    pub fn table_enabled(&self) -> bool {
        match self {
            CityFocus::HappinessFocus => false,
            _ => true,
        }
    }

    /// Gets the primary stat for this focus
    pub fn stat(&self) -> Option<Stat> {
        match self {
            CityFocus::NoFocus | CityFocus::Manual |
            CityFocus::GoldGrowthFocus | CityFocus::ProductionGrowthFocus => None,
            CityFocus::FoodFocus => Some(Stat::Food),
            CityFocus::ProductionFocus => Some(Stat::Production),
            CityFocus::GoldFocus => Some(Stat::Gold),
            CityFocus::ScienceFocus => Some(Stat::Science),
            CityFocus::CultureFocus => Some(Stat::Culture),
            CityFocus::HappinessFocus => Some(Stat::Happiness),
            CityFocus::FaithFocus => Some(Stat::Faith),
        }
    }

    /// Gets the multiplier for a given stat based on the focus
    pub fn get_stat_multiplier(&self, stat: Stat) -> f32 {
        match self {
            CityFocus::NoFocus | CityFocus::Manual => 1.0,
            CityFocus::GoldGrowthFocus => match stat {
                Stat::Gold => 2.0,
                Stat::Food => 1.5,
                _ => 1.0,
            },
            CityFocus::ProductionGrowthFocus => match stat {
                Stat::Production => 2.0,
                Stat::Food => 1.5,
                _ => 1.0,
            },
            _ => {
                if Some(stat) == self.stat() {
                    3.05 // on ties, prefer the Focus
                } else {
                    1.0
                }
            }
        }
    }

    /// Applies weight multipliers to stats based on the focus
    pub fn apply_weight_to(&self, stats: &mut Stats) {
        for stat in Stat::iter() {
            let multiplier = self.get_stat_multiplier(stat);
            if multiplier != 1.0 {
                let current_stat = stats[stat];
                if current_stat != 0.0 {
                    stats[stat] = current_stat * multiplier;
                }
            }
        }
    }

    /// Gets a CityFocus from a Stat, defaulting to NoFocus if not found
    pub fn from_stat(stat: Stat) -> Self {
        match stat {
            Stat::Food => CityFocus::FoodFocus,
            Stat::Production => CityFocus::ProductionFocus,
            Stat::Gold => CityFocus::GoldFocus,
            Stat::Science => CityFocus::ScienceFocus,
            Stat::Culture => CityFocus::CultureFocus,
            Stat::Happiness => CityFocus::HappinessFocus,
            Stat::Faith => CityFocus::FaithFocus,
            _ => CityFocus::NoFocus,
        }
    }
}

lazy_static! {
    /// Set of focuses that target 0 surplus food, used in Automation
    pub static ref ZERO_FOOD_FOCUSES: HashSet<CityFocus> = {
        let mut set = HashSet::new();
        set.insert(CityFocus::CultureFocus);
        set.insert(CityFocus::FaithFocus);
        set.insert(CityFocus::GoldFocus);
        set.insert(CityFocus::HappinessFocus);
        set.insert(CityFocus::ProductionFocus);
        set.insert(CityFocus::ScienceFocus);
        set
    };
}