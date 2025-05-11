use crate::utils::color::Color;

/// Represents the level of relationship between civilizations in diplomatic negotiations.
///
/// This enum is ordered from worst to best relationship, which allows for comparisons
/// using the standard comparison operators (e.g., `>`, `<=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipLevel {
    /// Utterly hostile relationship, beyond repair
    Unforgivable,
    /// Active hostility and antagonism
    Enemy,
    /// Cautious relationship due to power imbalance
    Afraid,
    /// Slightly negative relationship
    Competitor,
    /// Neither positive nor negative
    Neutral,
    /// Slightly positive relationship
    Favorable,
    /// Strong positive relationship
    Friend,
    /// Strongest possible relationship
    Ally,
}

impl RelationshipLevel {
    /// Returns the color associated with this relationship level for UI display
    pub fn color(&self) -> Color {
        match self {
            Self::Unforgivable => Color::FIREBRICK,
            Self::Enemy => Color::YELLOW,
            Self::Afraid => Color::new(0x5300ffff),     // HSV(260,100,100)
            Self::Competitor => Color::new(0x1f998fff), // HSV(175,80,60)
            Self::Neutral => Color::new(0x1bb371ff),    // HSV(154,85,70)
            Self::Favorable => Color::new(0x14cc3cff),  // HSV(133,90,80)
            Self::Friend => Color::new(0x2ce60bff),     // HSV(111,95,90)
            Self::Ally => Color::CHARTREUSE,            // HSV(90,100,100)
        }
    }

    /// Shifts the relationship level up or down by the given delta
    ///
    /// # Examples
    /// ```
    /// let relationship = RelationshipLevel::Neutral;
    /// assert_eq!(relationship.add(1), RelationshipLevel::Favorable);
    /// assert_eq!(relationship.add(-1), RelationshipLevel::Competitor);
    ///
    /// // Clamped at boundaries
    /// assert_eq!(RelationshipLevel::Ally.add(1), RelationshipLevel::Ally);
    /// assert_eq!(RelationshipLevel::Unforgivable.add(-1), RelationshipLevel::Unforgivable);
    /// ```
    pub fn add(&self, delta: i32) -> Self {
        let values = [
            Self::Unforgivable,
            Self::Enemy,
            Self::Afraid,
            Self::Competitor,
            Self::Neutral,
            Self::Favorable,
            Self::Friend,
            Self::Ally,
        ];

        // Convert self to ordinal, add delta, clamp to valid range
        let current_ordinal = values.iter().position(|&r| r == *self).unwrap_or(0) as i32;
        let new_ordinal = (current_ordinal + delta).clamp(0, values.len() as i32 - 1) as usize;
        values[new_ordinal]
    }

    /// Returns true if this relationship level is greater than or equal to the specified level
    pub fn is_at_least(&self, other: Self) -> bool {
        *self >= other
    }

    /// Returns true if this relationship level is considered friendly
    /// (Favorable, Friend, or Ally)
    pub fn is_friendly(&self) -> bool {
        matches!(self, Self::Favorable | Self::Friend | Self::Ally)
    }

    /// Returns true if this relationship level is considered hostile
    /// (Competitor, Enemy, or Unforgivable)
    pub fn is_hostile(&self) -> bool {
        matches!(self, Self::Competitor | Self::Enemy | Self::Unforgivable)
    }

    /// Returns a descriptive string for this relationship level
    pub fn description(&self) -> &'static str {
        match self {
            Self::Unforgivable => "Unforgivable",
            Self::Enemy => "Enemy",
            Self::Afraid => "Afraid",
            Self::Competitor => "Competitor",
            Self::Neutral => "Neutral",
            Self::Favorable => "Favorable",
            Self::Friend => "Friend",
            Self::Ally => "Ally",
        }
    }
}