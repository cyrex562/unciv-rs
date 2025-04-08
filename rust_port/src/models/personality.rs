/// Represents different personality values that can be assigned to civilizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersonalityValue {
    /// Likelihood to declare war
    DeclareWar,
    /// Likelihood to build military units
    BuildMilitary,
    /// Likelihood to build wonders
    BuildWonders,
    /// Likelihood to expand
    Expand,
    /// Likelihood to trade
    Trade,
    /// Likelihood to research
    Research,
    /// Likelihood to use nukes
    UseNukes,
    /// Likelihood to use espionage
    UseEspionage,
    /// Likelihood to use religion
    UseReligion,
    /// Likelihood to use culture
    UseCulture,
}