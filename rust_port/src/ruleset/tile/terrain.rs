use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::constants::ALL;
use crate::models::ruleset::{
    Belief, Ruleset, RulesetStatsObject, StateForConditionals, UniqueTarget, UniqueType,
};
use crate::models::ui::{Color, FormattedLine};
use crate::utils::MultiFilter;

/// Represents a terrain type in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Terrain {
    /// The type of terrain
    #[serde(skip)]
    pub terrain_type: TerrainType,

    /// For terrain features - indicates the stats of this terrain override those of all previous layers
    pub override_stats: bool,

    /// If true, nothing can be built here - not even resource improvements
    pub unbuildable: bool,

    /// For terrain features
    pub occurs_on: Vec<String>,

    /// Used by Natural Wonders: it is the baseTerrain on top of which the Natural Wonder is placed
    /// Omitting it means the Natural Wonder is placed on whatever baseTerrain the Tile already had (limited by occursOn)
    pub turns_into: Option<String>,

    /// Natural Wonder weight: probability to be picked
    pub weight: i32,

    /// RGB color of base terrain
    pub rgb: Option<Vec<i32>>,

    /// Movement cost for units crossing this terrain
    pub movement_cost: i32,

    /// Defence bonus for units on this terrain
    pub defence_bonus: f32,

    /// Whether this terrain is impassable
    pub impassable: bool,

    /// Damage per turn for units on this terrain
    #[serde(skip)]
    pub damage_per_turn: i32,

    /// Cache for filter matching results
    #[serde(skip)]
    cached_matches_filter_result: HashMap<String, bool>,
}

impl Terrain {
    /// Creates a new Terrain with default values
    pub fn new() -> Self {
        Terrain {
            terrain_type: TerrainType::Land,
            override_stats: false,
            unbuildable: false,
            occurs_on: Vec::new(),
            turns_into: None,
            weight: 10,
            rgb: None,
            movement_cost: 1,
            defence_bonus: 0.0,
            impassable: false,
            damage_per_turn: 0,
            cached_matches_filter_result: HashMap::new(),
        }
    }

    /// Checks if this terrain is rough
    pub fn is_rough(&self) -> bool {
        self.has_unique(UniqueType::RoughTerrain)
    }

    /// Tests base terrains, features and natural wonders whether they should be treated as Land/Water
    pub fn display_as(&self, as_type: TerrainType, ruleset: &Ruleset) -> bool {
        self.terrain_type == as_type
            || self.occurs_on.iter().any(|occurs_name| {
                ruleset.terrains.values()
                    .filter(|t| t.terrain_type == as_type)
                    .any(|t| t.name == *occurs_name)
            })
            || self.turns_into.as_ref().and_then(|t| ruleset.terrains.get(t))
                .map_or(false, |t| t.terrain_type == as_type)
    }

    /// Gets a new Color instance from the RGB property
    pub fn get_color(&self) -> Color {
        match &self.rgb {
            Some(rgb) => Color::from_rgb(rgb),
            None => Color::GOLD,
        }
    }

    /// Sets transient values based on uniques
    pub fn set_transients(&mut self) {
        self.damage_per_turn = self.get_matching_uniques(UniqueType::DamagesContainingUnits)
            .iter()
            .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
            .sum();
    }

    /// Implements UniqueParameterType.TerrainFilter
    pub fn matches_single_filter(&self, filter: &str) -> bool {
        match filter {
            f if f == ALL => true,
            f if f == self.name => true,
            "Terrain" => true,
            "Open terrain" => !self.is_rough(),
            "Rough terrain" => self.is_rough(),
            f if f == self.terrain_type.to_string() => true,
            "Natural Wonder" => self.terrain_type == TerrainType::NaturalWonder,
            "Terrain Feature" => self.terrain_type == TerrainType::TerrainFeature,
            _ => false,
        }
    }

    /// Checks if this terrain matches a filter
    pub fn matches_filter(&mut self, filter: &str, state: Option<&StateForConditionals>, multi_filter: bool) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| {
                self.cached_matches_filter_result
                    .entry(f.to_string())
                    .or_insert_with(|| self.matches_single_filter(f))
                    || state.map_or_else(
                        || self.has_tag_unique(f),
                        |s| self.has_unique(f, s),
                    )
            })
        } else {
            self.cached_matches_filter_result
                .entry(filter.to_string())
                .or_insert_with(|| self.matches_single_filter(filter))
                || state.map_or_else(
                    || self.has_tag_unique(filter),
                    |s| self.has_unique(filter, s),
                )
        }
    }
}

impl RulesetStatsObject for Terrain {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Terrain
    }

    fn make_link(&self) -> String {
        format!("Terrain/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        if self.terrain_type == TerrainType::NaturalWonder {
            text_list.push(FormattedLine::new("Natural Wonder", 3, Some("#3A0".to_string())));
        }

        let stats = self.clone_stats();
        if !stats.is_empty() || self.override_stats {
            text_list.push(FormattedLine::new("", 0, None));
            text_list.push(FormattedLine::new(
                if stats.is_empty() { "No yields" } else { &stats.to_string() },
                0,
                None,
            ));
            if self.override_stats {
                text_list.push(FormattedLine::new("Overrides yields from underlying terrain", 0, None));
            }
        }

        if !self.occurs_on.is_empty() && !self.has_unique(UniqueType::NoNaturalGeneration) {
            text_list.push(FormattedLine::new("", 0, None));
            if self.occurs_on.len() == 1 {
                let occurs_on = &self.occurs_on[0];
                text_list.push(FormattedLine::new_with_link(
                    &format!("Occurs on [{}]", occurs_on),
                    &format!("Terrain/{}", occurs_on),
                    0,
                    None,
                ));
            } else {
                text_list.push(FormattedLine::new("Occurs on:", 0, None));
                for occurs_on in &self.occurs_on {
                    text_list.push(FormattedLine::new_with_link(
                        occurs_on,
                        &format!("Terrain/{}", occurs_on),
                        1,
                        None,
                    ));
                }
            }
        }

        let improvements_that_can_be_placed_here: Vec<_> = ruleset.tile_improvements.values()
            .filter(|i| i.terrains_can_be_built_on.contains(&self.name))
            .collect();

        if !improvements_that_can_be_placed_here.is_empty() {
            text_list.push(FormattedLine::new("{Tile Improvements}:", 0, None));
            for improvement in improvements_that_can_be_placed_here {
                text_list.push(FormattedLine::new_with_link(
                    &improvement.name,
                    &improvement.make_link(),
                    1,
                    None,
                ));
            }
        }

        if let Some(turns_into) = &self.turns_into {
            text_list.push(FormattedLine::new_with_link(
                &format!("Placed on [{}]", turns_into),
                &format!("Terrain/{}", turns_into),
                0,
                None,
            ));
        }

        let resources_found: Vec<_> = ruleset.tile_resources.values()
            .filter(|r| r.terrains_can_be_found_on.contains(&self.name))
            .collect();

        if !resources_found.is_empty() {
            text_list.push(FormattedLine::new("", 0, None));
            if resources_found.len() == 1 {
                let resource = &resources_found[0];
                text_list.push(FormattedLine::new_with_link(
                    &format!("May contain [{}]", resource.name),
                    &format!("Resource/{}", resource.name),
                    0,
                    None,
                ));
            } else {
                text_list.push(FormattedLine::new("May contain:", 0, None));
                for resource in resources_found {
                    text_list.push(FormattedLine::new_with_link(
                        &resource.name,
                        &format!("Resource/{}", resource.name),
                        1,
                        None,
                    ));
                }
            }
        }

        text_list.push(FormattedLine::new("", 0, None));
        if self.turns_into.is_none() && self.display_as(TerrainType::Land, ruleset) && !self.is_rough() {
            text_list.push(FormattedLine::new("Open terrain", 0, None));
        }
        self.uniques_to_civilopedia_text_lines(&mut text_list, None);

        text_list.push(FormattedLine::new("", 0, None));
        if self.impassable {
            text_list.push(FormattedLine::new("Impassable", 0, Some("#A00".to_string())));
        } else if self.movement_cost > 0 {
            text_list.push(FormattedLine::new(
                &format!("{{Movement cost}}: {}", self.movement_cost),
                0,
                None,
            ));
        }

        if self.defence_bonus != 0.0 {
            text_list.push(FormattedLine::new(
                &format!("{{Defence bonus}}: {}%", (self.defence_bonus * 100.0) as i32),
                0,
                None,
            ));
        }

        let mut see_also = Vec::new();
        for building in ruleset.buildings.values() {
            if building.unique_objects.iter().any(|u| u.params.iter().any(|p| p == &self.name)) {
                see_also.push(FormattedLine::new_with_link(
                    &building.name,
                    &building.make_link(),
                    1,
                    None,
                ));
            }
        }
        for unit in ruleset.units.values() {
            if unit.unique_objects.iter().any(|u| u.params.iter().any(|p| p == &self.name)) {
                see_also.push(FormattedLine::new_with_link(
                    &unit.name,
                    &unit.make_link(),
                    1,
                    None,
                ));
            }
        }
        see_also.extend(Belief::get_civilopedia_text_matching(&self.name, ruleset, false));
        if !see_also.is_empty() {
            text_list.push(FormattedLine::new("", 0, None));
            text_list.push(FormattedLine::new("{See also}:", 0, None));
            text_list.extend(see_also);
        }

        text_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_new() {
        let terrain = Terrain::new();
        assert_eq!(terrain.terrain_type, TerrainType::Land);
        assert!(!terrain.override_stats);
        assert!(!terrain.unbuildable);
        assert!(terrain.occurs_on.is_empty());
        assert!(terrain.turns_into.is_none());
        assert_eq!(terrain.weight, 10);
        assert!(terrain.rgb.is_none());
        assert_eq!(terrain.movement_cost, 1);
        assert_eq!(terrain.defence_bonus, 0.0);
        assert!(!terrain.impassable);
        assert_eq!(terrain.damage_per_turn, 0);
    }

    #[test]
    fn test_terrain_matches_single_filter() {
        let mut terrain = Terrain::new();
        terrain.name = "Test".to_string();
        terrain.terrain_type = TerrainType::Land;

        assert!(terrain.matches_single_filter(ALL));
        assert!(terrain.matches_single_filter("Test"));
        assert!(terrain.matches_single_filter("Terrain"));
        assert!(terrain.matches_single_filter("Open terrain"));
        assert!(!terrain.matches_single_filter("Rough terrain"));
        assert!(terrain.matches_single_filter("Land"));
        assert!(!terrain.matches_single_filter("Natural Wonder"));
        assert!(!terrain.matches_single_filter("Terrain Feature"));
        assert!(!terrain.matches_single_filter("Other"));
    }
}