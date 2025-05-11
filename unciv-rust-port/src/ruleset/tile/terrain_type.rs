use serde::{Deserialize, Serialize};

/// Represents different types of terrain in the game
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum TerrainType {
    /// Base terrain type representing land
    Land,
    /// Base terrain type representing water
    Water,
    /// Non-base terrain type representing terrain features
    TerrainFeature,
    /// Non-base terrain type representing natural wonders
    NaturalWonder,
}

impl TerrainType {
    /// Checks if this terrain type is a base terrain
    pub fn is_base_terrain(&self) -> bool {
        match self {
            TerrainType::Land | TerrainType::Water => true,
            TerrainType::TerrainFeature | TerrainType::NaturalWonder => false,
        }
    }
}

impl std::fmt::Display for TerrainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerrainType::Land => write!(f, "Land"),
            TerrainType::Water => write!(f, "Water"),
            TerrainType::TerrainFeature => write!(f, "TerrainFeature"),
            TerrainType::NaturalWonder => write!(f, "NaturalWonder"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_type_is_base_terrain() {
        assert!(TerrainType::Land.is_base_terrain());
        assert!(TerrainType::Water.is_base_terrain());
        assert!(!TerrainType::TerrainFeature.is_base_terrain());
        assert!(!TerrainType::NaturalWonder.is_base_terrain());
    }

    #[test]
    fn test_terrain_type_display() {
        assert_eq!(TerrainType::Land.to_string(), "Land");
        assert_eq!(TerrainType::Water.to_string(), "Water");
        assert_eq!(TerrainType::TerrainFeature.to_string(), "TerrainFeature");
        assert_eq!(TerrainType::NaturalWonder.to_string(), "NaturalWonder");
    }
}