/// Represents the personality type of a city-state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CityStatePersonality {
    /// Friendly city-states are more likely to cooperate and provide benefits
    Friendly,
    /// Neutral city-states maintain balanced relationships
    Neutral,
    /// Hostile city-states are more likely to resist demands and provide fewer benefits
    Hostile,
    /// Irrational city-states have unpredictable behavior
    Irrational,
}

impl CityStatePersonality {
    /// Returns all possible city-state personalities
    pub fn entries() -> &'static [CityStatePersonality] {
        &[
            CityStatePersonality::Friendly,
            CityStatePersonality::Neutral,
            CityStatePersonality::Hostile,
            CityStatePersonality::Irrational,
        ]
    }
}