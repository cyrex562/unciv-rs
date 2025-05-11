use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::models::Counter;
use crate::models::stats::NamedStats;
use crate::ui::components::extensions::color_from_rgb;

/// Represents a specialist in the game, such as a scientist, engineer, or artist.
/// Specialists provide various benefits to cities and can generate great person points.
pub struct Specialist {
    /// The base named stats for this specialist
    pub stats: NamedStats,

    /// The RGB color components for this specialist
    pub color: Vec<i32>,

    /// The great person points this specialist generates
    pub great_person_points: Counter<String>,
}

impl Specialist {
    /// Create a new empty Specialist
    pub fn new() -> Self {
        Self {
            stats: NamedStats::new(),
            color: Vec::new(),
            great_person_points: Counter::new(),
        }
    }

    /// Get the color object for this specialist
    pub fn color_object(&self) -> &'static [u8; 4] {
        static COLOR_CACHE: Lazy<HashMap<Vec<i32>, [u8; 4]>> = Lazy::new(|| HashMap::new());

        COLOR_CACHE.entry(self.color.clone()).or_insert_with(|| {
            color_from_rgb(&self.color)
        })
    }

    /// Get the name of this specialist
    pub fn name(&self) -> &str {
        self.stats.name()
    }

    /// Set the name of this specialist
    pub fn set_name(&mut self, name: String) {
        self.stats.set_name(name);
    }

    /// Get the great person points for a specific great person type
    pub fn get_great_person_points(&self, great_person_type: &str) -> i32 {
        self.great_person_points.get(great_person_type)
    }

    /// Set the great person points for a specific great person type
    pub fn set_great_person_points(&mut self, great_person_type: String, points: i32) {
        self.great_person_points.set(great_person_type, points);
    }

    /// Add great person points for a specific great person type
    pub fn add_great_person_points(&mut self, great_person_type: String, points: i32) {
        self.great_person_points.add(great_person_type, points);
    }
}

impl std::fmt::Display for Specialist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_creation() {
        let mut specialist = Specialist::new();
        specialist.set_name("Scientist".to_string());
        specialist.color = vec![0, 0, 255]; // Blue
        specialist.set_great_person_points("Great Scientist".to_string(), 3);

        assert_eq!(specialist.name(), "Scientist");
        assert_eq!(specialist.color, vec![0, 0, 255]);
        assert_eq!(specialist.get_great_person_points("Great Scientist"), 3);
    }

    #[test]
    fn test_great_person_points() {
        let mut specialist = Specialist::new();
        specialist.set_great_person_points("Great Scientist".to_string(), 2);
        specialist.add_great_person_points("Great Scientist".to_string(), 1);

        assert_eq!(specialist.get_great_person_points("Great Scientist"), 3);
    }
}