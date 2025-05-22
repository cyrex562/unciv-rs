/// Represents the diplomatic status between two civilizations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiplomaticStatus {
    /// Peaceful relations between civilizations
    Peace,

    /// City state's diplomacy for major civ can be marked as Protector, not vice versa
    Protector,

    /// At war with each other
    War,

    /// Have a defensive pact
    DefensivePact,
}