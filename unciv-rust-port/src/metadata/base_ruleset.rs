/// Represents the base rulesets available in the game.
///
/// These enum variants have unusual names to match the original Kotlin implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseRuleset {
    /// Civilization V - Vanilla ruleset
    CivVVanilla,
    /// Civilization V - Gods & Kings ruleset
    CivVGnK,
}

impl BaseRuleset {
    /// Returns the full name of the ruleset.
    pub fn full_name(&self) -> &'static str {
        match self {
            BaseRuleset::CivVVanilla => "Civ V - Vanilla",
            BaseRuleset::CivVGnK => "Civ V - Gods & Kings",
        }
    }
}

impl std::fmt::Display for BaseRuleset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_name() {
        assert_eq!(BaseRuleset::CivVVanilla.full_name(), "Civ V - Vanilla");
        assert_eq!(BaseRuleset::CivVGnK.full_name(), "Civ V - Gods & Kings");
    }

    #[test]
    fn test_display() {
        assert_eq!(BaseRuleset::CivVVanilla.to_string(), "Civ V - Vanilla");
        assert_eq!(BaseRuleset::CivVGnK.to_string(), "Civ V - Gods & Kings");
    }
}