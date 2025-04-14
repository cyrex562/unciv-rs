use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::constants::REMOVE;
use crate::models::ruleset::{
    Belief, Ruleset, RulesetStatsObject, StateForConditionals, UniqueTarget, UniqueType,
};
use crate::models::ruleset::unit::BaseUnit;
use crate::models::ui::FormattedLine;
use crate::utils::{MultiFilter, ImprovementDescriptions};
use crate::models::ruleset::tile::{Terrain, TerrainType};
use crate::models::civilization::Civilization;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::RoadStatus;

/// Represents a tile improvement in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TileImprovement {
    /// The name of the improvement that this improvement replaces
    pub replaces: Option<String>,

    /// The terrains that this improvement can be built on
    pub terrains_can_be_built_on: Vec<String>,

    /// The technology required to build this improvement
    pub tech_required: Option<String>,

    /// The civilization that this improvement is unique to
    pub unique_to: Option<String>,

    /// The shortcut key for this improvement
    pub shortcut_key: Option<char>,

    /// The number of turns it takes to build this improvement
    /// A value of -1 means it is created instead of buildable
    pub turns_to_build: i32,

    /// Cache for filter matching results
    #[serde(skip)]
    cached_matches_filter_result: HashMap<String, bool>,
}

impl TileImprovement {
    /// Creates a new TileImprovement with default values
    pub fn new() -> Self {
        TileImprovement {
            replaces: None,
            terrains_can_be_built_on: Vec::new(),
            tech_required: None,
            unique_to: None,
            shortcut_key: None,
            turns_to_build: -1,
            cached_matches_filter_result: HashMap::new(),
        }
    }

    /// Gets the number of turns it takes to build this improvement for a specific civilization and unit
    pub fn get_turns_to_build(&self, civ_info: &Civilization, unit: &MapUnit) -> i32 {
        let state = StateForConditionals::new(civ_info, Some(unit), None);

        let build_speed_uniques = unit.get_matching_uniques(UniqueType::SpecificImprovementTime, &state, true)
            .iter()
            .filter(|u| self.matches_filter(&u.params[1], Some(&state)))
            .collect::<Vec<_>>();

        let build_speed_increases = unit.get_matching_uniques(UniqueType::ImprovementTimeIncrease, &state, true)
            .iter()
            .filter(|u| self.matches_filter(&u.params[0], Some(&state)))
            .collect::<Vec<_>>();

        let increase = build_speed_increases.iter()
            .map(|u| u.params[1].parse::<f64>().unwrap_or(0.0))
            .sum::<f64>() as f32;

        let build_time = if increase == 0.0 {
            0.0
        } else {
            (civ_info.game_info.speed.improvement_build_length_modifier * self.turns_to_build as f32) / increase
        };

        let calculated_turns_to_build = build_speed_uniques.iter()
            .fold(build_time, |acc, unique| {
                acc * unique.params[0].parse::<f32>().unwrap_or(1.0)
            });

        (calculated_turns_to_build.round() as i32).max(1)
    }

    /// Gets a description of this improvement
    pub fn get_description(&self, ruleset: &Ruleset) -> String {
        ImprovementDescriptions::get_description(self, ruleset)
    }

    /// Gets a short description of this improvement
    pub fn get_short_description(&self) -> String {
        ImprovementDescriptions::get_short_description(self)
    }

    /// Checks if this improvement is a great improvement
    pub fn is_great_improvement(&self) -> bool {
        self.has_unique(UniqueType::GreatImprovement)
    }

    /// Checks if this improvement is a road
    pub fn is_road(&self) -> bool {
        RoadStatus::iter().any(|status| status != RoadStatus::None && status.to_string() == self.name)
    }

    /// Checks if this improvement is equivalent to ancient ruins
    pub fn is_ancient_ruins_equivalent(&self) -> bool {
        self.has_unique(UniqueType::IsAncientRuinsEquivalent)
    }

    /// Checks if this improvement can be built on a specific terrain
    pub fn can_be_built_on(&self, terrain: &str) -> bool {
        self.terrains_can_be_built_on.contains(&terrain.to_string())
    }

    /// Checks if this improvement can be built on a specific terrain
    pub fn can_be_built_on_terrain(&self, terrain: &Terrain) -> bool {
        self.terrains_can_be_built_on.iter().any(|t| terrain.matches_filter(t, None))
    }

    /// Checks if this improvement is allowed on a specific terrain feature
    pub fn is_allowed_on_feature(&self, terrain: &Terrain) -> bool {
        self.can_be_built_on_terrain(terrain)
            || self.get_matching_uniques(UniqueType::NoFeatureRemovalNeeded)
                .iter()
                .any(|u| terrain.matches_filter(&u.params[0], None))
    }

    /// Implements UniqueParameterType.ImprovementFilter
    pub fn matches_single_filter(&self, filter: &str) -> bool {
        match filter {
            "all" | "All" => true,
            "Improvement" => true,
            "All Road" => self.is_road(),
            "Great Improvement" | "Great" => self.is_great_improvement(),
            _ => filter == self.name || self.replaces.as_ref().map_or(false, |r| r == filter),
        }
    }

    /// Checks if this improvement matches a filter
    pub fn matches_filter(&mut self, filter: &str, tile_state: Option<&StateForConditionals>, multi_filter: bool) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| {
                self.cached_matches_filter_result
                    .entry(f.to_string())
                    .or_insert_with(|| self.matches_single_filter(f))
                    || tile_state.map_or_else(
                        || self.has_tag_unique(f),
                        |s| self.has_unique(f, s),
                    )
            })
        } else {
            self.cached_matches_filter_result
                .entry(filter.to_string())
                .or_insert_with(|| self.matches_single_filter(filter))
                || tile_state.map_or_else(
                    || self.has_tag_unique(filter),
                    |s| self.has_unique(filter, s),
                )
        }
    }

    /// Gets the units that can construct this improvement
    pub fn get_constructor_units(&self, ruleset: &Ruleset) -> Vec<&BaseUnit> {
        if self.has_unique(UniqueType::Unbuildable) {
            return Vec::new();
        }

        let can_only_filters: HashSet<_> = self.get_matching_uniques(UniqueType::CanOnlyBeBuiltOnTile)
            .iter()
            .map(|u| {
                let param = &u.params[0];
                if param == "Coastal" { "Land" } else { param }
            })
            .collect();

        let cannot_filters: HashSet<_> = self.get_matching_uniques(UniqueType::CannotBuildOnTile)
            .iter()
            .map(|u| &u.params[0])
            .collect();

        let resources_improved_by_this: Vec<_> = ruleset.tile_resources.values()
            .filter(|r| r.is_improved_by(&self.name))
            .collect();

        let mut expanded_terrains_can_be_built_on: HashSet<_> = self.terrains_can_be_built_on.iter()
            .cloned()
            .collect();

        for terrain_name in &self.terrains_can_be_built_on {
            if let Some(terrain) = ruleset.terrains.get(terrain_name) {
                expanded_terrains_can_be_built_on.extend(terrain.occurs_on.iter().cloned());
            }
        }

        if self.has_unique(UniqueType::CanOnlyImproveResource) {
            for resource in &resources_improved_by_this {
                expanded_terrains_can_be_built_on.extend(resource.terrains_can_be_found_on.iter().cloned());
            }
        }

        if self.name.starts_with(REMOVE) {
            let base_name = self.name.strip_prefix(REMOVE).unwrap();
            expanded_terrains_can_be_built_on.insert(base_name.to_string());

            if let Some(terrain) = ruleset.terrains.get(base_name) {
                expanded_terrains_can_be_built_on.extend(terrain.occurs_on.iter().cloned());
            }

            if let Some(improvement) = ruleset.tile_improvements.get(base_name) {
                expanded_terrains_can_be_built_on.extend(improvement.terrains_can_be_built_on.iter().cloned());
            }
        }

        expanded_terrains_can_be_built_on.retain(|t| !cannot_filters.contains(t));

        let mut terrains_can_be_built_on_types: HashSet<_> = expanded_terrains_can_be_built_on.iter()
            .filter_map(|t| ruleset.terrains.get(t).map(|tr| tr.terrain_type))
            .collect();

        for terrain_type in TerrainType::iter() {
            if expanded_terrains_can_be_built_on.contains(&terrain_type.to_string()) {
                terrains_can_be_built_on_types.insert(terrain_type);
            }
        }

        terrains_can_be_built_on_types.retain(|t| !cannot_filters.contains(&t.to_string()));

        if !can_only_filters.is_empty() && can_only_filters.intersection(&expanded_terrains_can_be_built_on).next().is_none() {
            expanded_terrains_can_be_built_on.clear();
            if !terrains_can_be_built_on_types.iter().any(|t| can_only_filters.contains(&t.to_string())) {
                terrains_can_be_built_on_types.clear();
            }
        }

        let matches_build_improvements_filter = |filter: &str| {
            self.matches_filter(filter, None, true)
                || expanded_terrains_can_be_built_on.contains(filter)
                || terrains_can_be_built_on_types.iter().any(|t| t.to_string() == filter)
        };

        ruleset.units.values()
            .filter(|unit| {
                (self.turns_to_build != -1
                    && unit.get_matching_uniques(UniqueType::BuildImprovements, &StateForConditionals::ignore_conditionals())
                        .iter()
                        .any(|u| matches_build_improvements_filter(&u.params[0])))
                    || (unit.has_unique(UniqueType::CreateWaterImprovements)
                        && terrains_can_be_built_on_types.contains(&TerrainType::Water))
            })
            .collect()
    }

    /// Gets the units that can create this improvement instantly
    pub fn get_creating_units(&self, ruleset: &Ruleset) -> Vec<&BaseUnit> {
        ruleset.units.values()
            .filter(|unit| {
                unit.get_matching_uniques(UniqueType::ConstructImprovementInstantly, &StateForConditionals::ignore_conditionals())
                    .iter()
                    .any(|u| u.params[0] == self.name)
            })
            .collect()
    }
}

impl RulesetStatsObject for TileImprovement {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Improvement
    }

    fn make_link(&self) -> String {
        format!("Improvement/{}", self.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        ImprovementDescriptions::get_civilopedia_text_lines(self, ruleset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_improvement_new() {
        let improvement = TileImprovement::new();
        assert!(improvement.replaces.is_none());
        assert!(improvement.terrains_can_be_built_on.is_empty());
        assert!(improvement.tech_required.is_none());
        assert!(improvement.unique_to.is_none());
        assert!(improvement.shortcut_key.is_none());
        assert_eq!(improvement.turns_to_build, -1);
    }

    #[test]
    fn test_tile_improvement_matches_single_filter() {
        let mut improvement = TileImprovement::new();
        improvement.name = "Test".to_string();
        improvement.replaces = Some("OldTest".to_string());

        assert!(improvement.matches_single_filter("all"));
        assert!(improvement.matches_single_filter("All"));
        assert!(improvement.matches_single_filter("Improvement"));
        assert!(!improvement.matches_single_filter("All Road"));
        assert!(!improvement.matches_single_filter("Great Improvement"));
        assert!(!improvement.matches_single_filter("Great"));
        assert!(improvement.matches_single_filter("Test"));
        assert!(improvement.matches_single_filter("OldTest"));
        assert!(!improvement.matches_single_filter("Other"));
    }
}