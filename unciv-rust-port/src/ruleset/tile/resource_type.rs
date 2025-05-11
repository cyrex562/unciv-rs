use serde::{Deserialize, Serialize};
use crate::models::ui::Color;

/// Represents different types of resources in the game
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    /// Luxury resources, represented by gold color
    Luxury,
    /// Strategic resources, represented by brown color
    Strategic,
    /// Bonus resources, represented by light blue color
    Bonus,
}

impl ResourceType {
    /// Gets the color associated with this resource type
    pub fn get_color(&self) -> Color {
        match self {
            ResourceType::Luxury => Color::from_hex("#ffd800"),
            ResourceType::Strategic => Color::from_hex("#c14d00"),
            ResourceType::Bonus => Color::from_hex("#a8c3c9"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_colors() {
        assert_eq!(ResourceType::Luxury.get_color(), Color::from_hex("#ffd800"));
        assert_eq!(ResourceType::Strategic.get_color(), Color::from_hex("#c14d00"));
        assert_eq!(ResourceType::Bonus.get_color(), Color::from_hex("#a8c3c9"));
    }
}