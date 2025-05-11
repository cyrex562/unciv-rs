use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::models::ruleset::unique::{Unique, UniqueMap, UniqueTarget};
use crate::models::stats::INamed;
use crate::ui::components::extensions::color_from_rgb;

/// Represents a city-state type in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CityStateType {
    /// The name of the city-state type
    pub name: String,
    /// List of uniques granted when friendly with this city-state
    pub friend_bonus_uniques: Vec<String>,
    /// List of uniques granted when allied with this city-state
    pub ally_bonus_uniques: Vec<String>,
    /// The RGB color values for this city-state type
    pub color: Vec<i32>,
    /// Cached color object
    #[serde(skip)]
    color_object: OnceLock<u32>,
    /// Cached friend bonus unique map
    #[serde(skip)]
    friend_bonus_unique_map: OnceLock<UniqueMap>,
    /// Cached ally bonus unique map
    #[serde(skip)]
    ally_bonus_unique_map: OnceLock<UniqueMap>,
}

impl CityStateType {
    /// Creates a new CityStateType with the given parameters
    pub fn new(
        name: String,
        friend_bonus_uniques: Vec<String>,
        ally_bonus_uniques: Vec<String>,
        color: Vec<i32>,
    ) -> Self {
        CityStateType {
            name,
            friend_bonus_uniques,
            ally_bonus_uniques,
            color,
            color_object: OnceLock::new(),
            friend_bonus_unique_map: OnceLock::new(),
            ally_bonus_unique_map: OnceLock::new(),
        }
    }

    /// Gets the color object for this city-state type
    pub fn get_color(&self) -> u32 {
        self.color_object.get_or_init(|| {
            color_from_rgb(&self.color)
        }).clone()
    }

    /// Gets the friend bonus unique map for this city-state type
    pub fn get_friend_bonus_unique_map(&self) -> &UniqueMap {
        self.friend_bonus_unique_map.get_or_init(|| {
            self.friend_bonus_uniques.iter()
                .map(|unique| Unique::new(
                    unique.clone(),
                    UniqueTarget::CityState,
                    self.name.clone(),
                ))
                .collect::<UniqueMap>()
        })
    }

    /// Gets the ally bonus unique map for this city-state type
    pub fn get_ally_bonus_unique_map(&self) -> &UniqueMap {
        self.ally_bonus_unique_map.get_or_init(|| {
            self.ally_bonus_uniques.iter()
                .map(|unique| Unique::new(
                    unique.clone(),
                    UniqueTarget::CityState,
                    self.name.clone(),
                ))
                .collect::<UniqueMap>()
        })
    }
}

impl INamed for CityStateType {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_city_state_type_new() {
        let city_state = CityStateType::new(
            "TestCity".to_string(),
            vec!["FriendBonus".to_string()],
            vec!["AllyBonus".to_string()],
            vec![255, 255, 255],
        );
        assert_eq!(city_state.name, "TestCity");
        assert_eq!(city_state.friend_bonus_uniques, vec!["FriendBonus"]);
        assert_eq!(city_state.ally_bonus_uniques, vec!["AllyBonus"]);
        assert_eq!(city_state.color, vec![255, 255, 255]);
    }

    #[test]
    fn test_get_color() {
        let city_state = CityStateType::new(
            "TestCity".to_string(),
            vec![],
            vec![],
            vec![255, 0, 0],
        );
        let color = city_state.get_color();
        assert_eq!(color, 0xFF0000);
    }

    #[test]
    fn test_get_friend_bonus_unique_map() {
        let city_state = CityStateType::new(
            "TestCity".to_string(),
            vec!["FriendBonus".to_string()],
            vec![],
            vec![255, 255, 255],
        );
        let unique_map = city_state.get_friend_bonus_unique_map();
        assert_eq!(unique_map.len(), 1);
        assert_eq!(unique_map[0].text, "FriendBonus");
        assert_eq!(unique_map[0].source_object_type, UniqueTarget::CityState);
        assert_eq!(unique_map[0].source_object_name, "TestCity");
    }

    #[test]
    fn test_get_ally_bonus_unique_map() {
        let city_state = CityStateType::new(
            "TestCity".to_string(),
            vec![],
            vec!["AllyBonus".to_string()],
            vec![255, 255, 255],
        );
        let unique_map = city_state.get_ally_bonus_unique_map();
        assert_eq!(unique_map.len(), 1);
        assert_eq!(unique_map[0].text, "AllyBonus");
        assert_eq!(unique_map[0].source_object_type, UniqueTarget::CityState);
        assert_eq!(unique_map[0].source_object_name, "TestCity");
    }
}