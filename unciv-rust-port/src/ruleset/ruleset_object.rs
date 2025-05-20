use std::fmt;
use std::collections::HashMap;

/// Base class for ruleset objects
pub struct RulesetObject {
    /// The name of this object
    pub name: String,

    /// The name of the ruleset this object belongs to
    pub origin_ruleset: String,

    /// The uniques of this object
    pub uniques: Vec<String>, // Can not be a hashset as that would remove doubles

    /// The civilopedia text of this object
    pub civilopedia_text: Vec<FormattedLine>,
}

impl RulesetObject {
    /// Create a new empty RulesetObject
    pub fn new() -> Self {
        Self {
            name: String::new(),
            origin_ruleset: String::new(),
            uniques: Vec::new(),
            civilopedia_text: Vec::new(),
        }
    }

    /// Get the unique objects of this object
    pub fn unique_objects(&self) -> Vec<Unique> {
        self.uniques.iter()
            .map(|unique_str| Unique::from_string(unique_str))
            .collect()
    }

    /// Get the unique map of this object
    pub fn unique_map(&self) -> UniqueMap {
        let mut map = HashMap::new();
        for unique in self.unique_objects() {
            map.insert(unique.unique_type, unique);
        }
        map
    }
}

impl fmt::Display for RulesetObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Base class for ruleset objects that also have stats
pub struct RulesetStatsObject {
    /// The base named stats
    pub stats: NamedStats,

    /// The name of the ruleset this object belongs to
    pub origin_ruleset: String,

    /// The uniques of this object
    pub uniques: Vec<String>, // Can not be a hashset as that would remove doubles

    /// The civilopedia text of this object
    pub civilopedia_text: Vec<FormattedLine>,
}

impl RulesetStatsObject {
    /// Create a new empty RulesetStatsObject
    pub fn new() -> Self {
        Self {
            stats: NamedStats::new(),
            origin_ruleset: String::new(),
            uniques: Vec::new(),
            civilopedia_text: Vec::new(),
        }
    }

    /// Get the unique objects of this object
    pub fn unique_objects(&self) -> Vec<Unique> {
        self.uniques.iter()
            .map(|unique_str| Unique::from_string(unique_str))
            .collect()
    }

    /// Get the unique map of this object
    pub fn unique_map(&self) -> UniqueMap {
        let mut map = HashMap::new();
        for unique in self.unique_objects() {
            map.insert(unique.unique_type, unique);
        }
        map
    }
}

impl fmt::Display for RulesetStatsObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.stats.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruleset_object() {
        let mut obj = RulesetObject::new();
        obj.set_name("Test".to_string());
        obj.set_origin_ruleset("Vanilla".to_string());
        obj.uniques_mut().push("Test unique".to_string());

        assert_eq!(obj.name(), "Test");
        assert_eq!(obj.origin_ruleset(), "Vanilla");
        assert_eq!(obj.uniques().len(), 1);
        assert_eq!(obj.to_string(), "Test");
    }

    #[test]
    fn test_ruleset_stats_object() {
        let mut obj = RulesetStatsObject::new();
        obj.set_name("Test".to_string());
        obj.set_origin_ruleset("Vanilla".to_string());
        obj.uniques_mut().push("Test unique".to_string());

        assert_eq!(obj.name(), "Test");
        assert_eq!(obj.origin_ruleset(), "Vanilla");
        assert_eq!(obj.uniques().len(), 1);
        assert_eq!(obj.to_string(), "Test");
    }
}