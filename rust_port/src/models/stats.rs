/// Represents different types of rankings in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RankingType {
    /// Military force ranking
    Force,
    /// Economic score ranking
    Score,
    /// Population ranking
    Population,
    /// Technology ranking
    Technology,
    /// Culture ranking
    Culture,
    /// Happiness ranking
    Happiness,
    /// Gold ranking
    Gold,
    /// Land ranking
    Land,
    /// Wonders ranking
    Wonders,
}