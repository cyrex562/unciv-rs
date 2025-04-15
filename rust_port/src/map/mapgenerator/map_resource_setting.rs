use std::collections::HashMap;

/// Settings for resource generation on the map
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapResourceSetting {
    /// Sparse resource distribution
    Sparse,
    /// Default resource distribution
    Default,
    /// Abundant resource distribution
    Abundant,
    /// Strategic balance resource distribution (deprecated)
    #[deprecated(since = "4.10.7", note = "Moved to mapParameters")]
    StrategicBalance,
    /// Legendary start resource distribution (deprecated)
    #[deprecated(since = "4.10.7", note = "Moved to mapParameters")]
    LegendaryStart,
}

impl MapResourceSetting {
    /// Gets the label for this resource setting
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sparse => "Sparse",
            Self::Default => "Default",
            Self::Abundant => "Abundant",
            Self::StrategicBalance => "Strategic Balance",
            Self::LegendaryStart => "Legendary Start",
        }
    }

    /// Gets the random luxuries percentage for this setting
    pub fn random_luxuries_percent(&self) -> i32 {
        match self {
            Self::Sparse => 80,
            Self::Default => 100,
            Self::Abundant => 133,
            Self::StrategicBalance => 100,
            Self::LegendaryStart => 100,
        }
    }

    /// Gets the regional luxuries delta for this setting
    pub fn regional_luxuries_delta(&self) -> i32 {
        match self {
            Self::Sparse => -1,
            Self::Default => 0,
            Self::Abundant => 1,
            Self::StrategicBalance => 0,
            Self::LegendaryStart => 0,
        }
    }

    /// Gets the special luxuries target factor for this setting
    pub fn special_luxuries_target_factor(&self) -> f32 {
        match self {
            Self::Sparse => 0.5,
            Self::Default => 0.75,
            Self::Abundant => 0.9,
            Self::StrategicBalance => 0.75,
            Self::LegendaryStart => 0.75,
        }
    }

    /// Gets the bonus frequency multiplier for this setting
    pub fn bonus_frequency_multiplier(&self) -> f32 {
        match self {
            Self::Sparse => 1.5,
            Self::Default => 1.0,
            Self::Abundant => 0.6667,
            Self::StrategicBalance => 1.0,
            Self::LegendaryStart => 1.0,
        }
    }

    /// Checks if this setting is currently active (not deprecated)
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::StrategicBalance | Self::LegendaryStart)
    }

    /// Gets a list of labels for all active settings
    pub fn active_labels() -> Vec<&'static str> {
        vec!["Sparse", "Default", "Abundant"]
    }

    /// Safely gets a MapResourceSetting from a label, defaulting to Default if not found
    pub fn from_label(label: &str) -> Self {
        match label {
            "Sparse" => Self::Sparse,
            "Default" => Self::Default,
            "Abundant" => Self::Abundant,
            "Strategic Balance" => Self::StrategicBalance,
            "Legendary Start" => Self::LegendaryStart,
            _ => Self::Default,
        }
    }
}

/// Extension trait for HashMap to provide safe value access
pub trait SafeValueAccess {
    /// Safely gets a value from a HashMap, returning None if not found
    fn safe_get<K, V>(&self, key: &K) -> Option<&V>
    where
        K: Eq + std::hash::Hash,
    {
        self.get(key)
    }
}

impl<K, V> SafeValueAccess for HashMap<K, V> {}