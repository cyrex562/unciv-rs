use std::collections::HashSet;

/// Flags that can be applied to uniques
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UniqueFlag {
    /// Flag indicating that the unique is hidden from users
    HiddenToUsers,
    /// Flag indicating that the unique has no conditionals
    NoConditionals,
    /// Flag indicating that the unique accepts speed modifiers
    AcceptsSpeedModifier,
}

impl UniqueFlag {
    /// Get a set containing only the HiddenToUsers flag
    pub fn set_of_hidden_to_users() -> HashSet<Self> {
        let mut set = HashSet::new();
        set.insert(Self::HiddenToUsers);
        set
    }

    /// Get a set containing only the NoConditionals flag
    pub fn set_of_no_conditionals() -> HashSet<Self> {
        let mut set = HashSet::new();
        set.insert(Self::NoConditionals);
        set
    }

    /// Get a set containing both HiddenToUsers and NoConditionals flags
    pub fn set_of_hidden_no_conditionals() -> HashSet<Self> {
        let mut set = HashSet::new();
        set.insert(Self::HiddenToUsers);
        set.insert(Self::NoConditionals);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_of_hidden_to_users() {
        let set = UniqueFlag::set_of_hidden_to_users();
        assert_eq!(set.len(), 1);
        assert!(set.contains(&UniqueFlag::HiddenToUsers));
    }

    #[test]
    fn test_set_of_no_conditionals() {
        let set = UniqueFlag::set_of_no_conditionals();
        assert_eq!(set.len(), 1);
        assert!(set.contains(&UniqueFlag::NoConditionals));
    }

    #[test]
    fn test_set_of_hidden_no_conditionals() {
        let set = UniqueFlag::set_of_hidden_no_conditionals();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&UniqueFlag::HiddenToUsers));
        assert!(set.contains(&UniqueFlag::NoConditionals));
    }
}