use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::ruleset::tile::resource_type::ResourceType;


/// Represents a deposit amount for a resource
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DepositAmount {
    /// The amount for sparse deposits
    pub sparse: i32,
    /// The default amount
    pub default: i32,
    /// The amount for abundant deposits
    pub abundant: i32,
}

impl DepositAmount {
    /// Creates a new DepositAmount with default values
    pub fn new() -> Self {
        DepositAmount {
            sparse: 1,
            default: 2,
            abundant: 3,
        }
    }
}

/// Represents a tile resource in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TileResource {
    /// The type of resource
    pub resource_type: ResourceType,

    /// The terrains that this resource can be found on
    pub terrains_can_be_found_on: Vec<String>,

    /// The stats that this resource adds to a tile when improved
    pub improvement_stats: Option<Stats>,

    /// The technology that reveals this resource
    pub revealed_by: Option<String>,

    /// The legacy improvement that unlocks this resource
    pub improvement: Option<String>,

    /// The improvements that unlock this resource
    pub improved_by: Vec<String>,

    /// The amount for major deposits
    pub major_deposit_amount: DepositAmount,

    /// The amount for minor deposits
    pub minor_deposit_amount: DepositAmount,

    /// Cache for improvements
    #[serde(skip)]
    all_improvements: HashSet<String>,

    /// Whether improvements have been initialized
    #[serde(skip)]
    improvements_initialized: bool,

    /// The ruleset this resource belongs to
    #[serde(skip)]
    ruleset: Option<Ruleset>,
}

impl TileResource {
    /// Creates a new TileResource with default values
    pub fn new() -> Self {
        TileResource {
            resource_type: ResourceType::Bonus,
            terrains_can_be_found_on: Vec::new(),
            improvement_stats: None,
            revealed_by: None,
            improvement: None,
            improved_by: Vec::new(),
            major_deposit_amount: DepositAmount::new(),
            minor_deposit_amount: DepositAmount::new(),
            all_improvements: HashSet::new(),
            improvements_initialized: false,
            ruleset: None,
        }
    }

    /// Gets whether this resource is city-wide
    pub fn is_city_wide(&self) -> bool {
        self.has_unique(UniqueType::CityResource, &StateForConditionals::ignore_conditionals())
    }

    /// Gets whether this resource is stockpiled
    pub fn is_stockpiled(&self) -> bool {
        self.has_unique(UniqueType::Stockpiled, &StateForConditionals::ignore_conditionals())
    }

    /// Gets the improvements that can improve this resource
    pub fn get_improvements(&mut self) -> &HashSet<String> {
        if !self.improvements_initialized {
            let ruleset = self.ruleset.as_ref()
                .expect("No ruleset on TileResource when initializing improvements");

            if let Some(improvement) = &self.improvement {
                self.all_improvements.insert(improvement.clone());
            }

            self.all_improvements.extend(self.improved_by.iter().cloned());

            for improvement in ruleset.tile_improvements.values() {
                if improvement.get_matching_uniques(UniqueType::ImprovesResources)
                    .iter()
                    .any(|u| self.matches_filter(&u.params[0], None))
                {
                    self.all_improvements.insert(improvement.name.clone());
                }
            }

            self.improvements_initialized = true;
        }

        &self.all_improvements
    }

    /// Sets the transients for this resource
    pub fn set_transients(&mut self, ruleset: Ruleset) {
        self.all_improvements.clear();
        self.improvements_initialized = false;
        self.ruleset = Some(ruleset);
    }

    /// Checks if this resource is improved by a specific improvement
    pub fn is_improved_by(&mut self, improvement_name: &str) -> bool {
        self.get_improvements().contains(improvement_name)
    }

    /// Gets the improving improvement for a specific tile and civilization
    pub fn get_improving_improvement(&mut self, tile: &Tile, civ: &Civilization) -> Option<String> {
        self.get_improvements()
            .iter()
            .find(|improvement_name| {
                let improvement = civ.game_info.ruleset.tile_improvements.get(improvement_name)
                    .expect("Improvement not found in ruleset");
                tile.improvement_functions.can_build_improvement(improvement, civ)
            })
            .cloned()
    }

    /// Checks if this resource matches a filter
    pub fn matches_filter(&self, filter: &str, state: Option<&StateForConditionals>) -> bool {
        MultiFilter::multi_filter(filter, |f| {
            self.matches_single_filter(f)
                || state.map_or_else(
                    || self.has_tag_unique(f),
                    |s| self.has_unique(f, s),
                )
        })
    }

    /// Checks if this resource matches a single filter
    pub fn matches_single_filter(&self, filter: &str) -> bool {
        filter == self.name
            || filter == "any"
            || filter == "all"
            || filter == self.resource_type.to_string()
            || self.improvement_stats.as_ref()
                .map_or(false, |stats| stats.iter().any(|(key, _)| key == filter))
    }

    /// Checks if this resource generates naturally on a specific tile
    pub fn generates_naturally_on(&self, tile: &Tile) -> bool {
        if !self.terrains_can_be_found_on.contains(&tile.last_terrain.name) {
            return false;
        }

        let state_for_conditionals = StateForConditionals::new(None, None, Some(tile));

        if self.has_unique(UniqueType::NoNaturalGeneration, &state_for_conditionals) {
            return false;
        }

        if tile.all_terrains.iter().any(|t| t.has_unique(UniqueType::BlocksResources, &state_for_conditionals)) {
            return false;
        }

        if let (Some(temperature), Some(humidity)) = (tile.temperature, tile.humidity) {
            for unique in self.get_matching_uniques(UniqueType::TileGenerationConditions, &state_for_conditionals) {
                let min_temp = unique.params[0].parse::<f64>().unwrap_or(0.0);
                let max_temp = unique.params[1].parse::<f64>().unwrap_or(0.0);
                let min_humidity = unique.params[2].parse::<f64>().unwrap_or(0.0);
                let max_humidity = unique.params[3].parse::<f64>().unwrap_or(0.0);

                if !(min_temp..=max_temp).contains(&temperature)
                    || !(min_humidity..=max_humidity).contains(&humidity)
                {
                    return false;
                }
            }
        }

        true
    }
}

impl RulesetStatsObject for TileResource {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Resource
    }

    fn make_link(&self) -> String {
        format!("Resource/{}", self.name)
    }

    fn get_civilopedia_text_lines(&mut self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        text_list.push(FormattedLine::new(
            format!("{} resource", self.resource_type.to_string()),
            Some(4),
            Some(self.resource_type.get_color()),
            None,
            None,
        ));
        text_list.push(FormattedLine::new(String::new(), None, None, None, None));

        uniques_to_civilopedia_text_lines(&mut text_list, self, true);

        if let Some(stats) = &self.improvement_stats {
            text_list.push(FormattedLine::new(stats.to_string(), None, None, None, None));
        }

        if !self.terrains_can_be_found_on.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));

            if self.terrains_can_be_found_on.len() == 1 {
                let terrain_name = &self.terrains_can_be_found_on[0];
                text_list.push(FormattedLine::new(
                    format!("{{Can be found on}} {{{}}}", terrain_name),
                    None,
                    None,
                    Some(format!("Terrain/{}", terrain_name)),
                    None,
                ));
            } else {
                text_list.push(FormattedLine::new("{Can be found on}:".to_string(), None, None, None, None));
                for terrain_name in &self.terrains_can_be_found_on {
                    text_list.push(FormattedLine::new(
                        terrain_name.clone(),
                        None,
                        None,
                        Some(format!("Terrain/{}", terrain_name)),
                        Some(1),
                    ));
                }
            }
        }

        for improvement in self.get_improvements() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                format!("Improved by [{}]", improvement),
                None,
                None,
                Some(format!("Improvement/{}", improvement)),
                None,
            ));

            if let Some(stats) = &self.improvement_stats {
                if !stats.is_empty() {
                    text_list.push(FormattedLine::new(
                        format!("{{Bonus stats for improvement}}: {}", stats),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
            }
        }

        let improvements_that_provide_this: Vec<_> = ruleset.tile_improvements.values()
            .filter(|improvement| {
                improvement.unique_objects.iter().any(|unique| {
                    unique.unique_type == UniqueType::ProvidesResources && unique.params[1] == self.name
                })
            })
            .collect();

        if !improvements_that_provide_this.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                "{Improvements that provide this resource}:".to_string(),
                None,
                None,
                None,
                None,
            ));
            for improvement in improvements_that_provide_this {
                text_list.push(FormattedLine::new(
                    improvement.name.clone(),
                    None,
                    None,
                    Some(improvement.make_link()),
                    Some(1),
                ));
            }
        }

        let buildings_that_provide_this: Vec<_> = ruleset.buildings.values()
            .filter(|building| {
                building.unique_objects.iter().any(|unique| {
                    unique.unique_type == UniqueType::ProvidesResources && unique.params[1] == self.name
                })
            })
            .collect();

        if !buildings_that_provide_this.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                "{Buildings that provide this resource}:".to_string(),
                None,
                None,
                None,
                None,
            ));
            for building in buildings_that_provide_this {
                text_list.push(FormattedLine::new(
                    building.name.clone(),
                    None,
                    None,
                    Some(building.make_link()),
                    Some(1),
                ));
            }
        }

        let buildings_that_consume_this: Vec<_> = ruleset.buildings.values()
            .filter(|building| {
                building.get_resource_requirements_per_turn(&StateForConditionals::ignore_conditionals())
                    .contains_key(&self.name)
            })
            .collect();

        if !buildings_that_consume_this.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                "{Buildings that consume this resource}:".to_string(),
                None,
                None,
                None,
                None,
            ));
            for building in buildings_that_consume_this {
                text_list.push(FormattedLine::new(
                    building.name.clone(),
                    None,
                    None,
                    Some(building.make_link()),
                    Some(1),
                ));
            }
        }

        let units_that_consume_this: Vec<_> = ruleset.units.values()
            .filter(|unit| {
                unit.get_resource_requirements_per_turn(&StateForConditionals::ignore_conditionals())
                    .contains_key(&self.name)
            })
            .collect();

        if !units_that_consume_this.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                "{Units that consume this resource}: ".to_string(),
                None,
                None,
                None,
                None,
            ));
            for unit in units_that_consume_this {
                text_list.push(FormattedLine::new(
                    unit.name.clone(),
                    None,
                    None,
                    Some(unit.make_link()),
                    Some(1),
                ));
            }
        }

        let buildings_requiring_this: Vec<_> = ruleset.buildings.values()
            .filter(|building| {
                building.required_nearby_improved_resources
                    .as_ref()
                    .map_or(false, |resources| resources.contains(&self.name))
            })
            .collect();

        if !buildings_requiring_this.is_empty() {
            text_list.push(FormattedLine::new(String::new(), None, None, None, None));
            text_list.push(FormattedLine::new(
                "{Buildings that require this resource improved near the city}: ".to_string(),
                None,
                None,
                None,
                None,
            ));
            for building in buildings_requiring_this {
                text_list.push(FormattedLine::new(
                    building.name.clone(),
                    None,
                    None,
                    Some(building.make_link()),
                    Some(1),
                ));
            }
        }

        text_list.extend(Belief::get_civilopedia_text_matching(&self.name, ruleset));

        text_list
    }
}

impl GameResource for TileResource {
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_resource_new() {
        let resource = TileResource::new();
        assert_eq!(resource.resource_type, ResourceType::Bonus);
        assert!(resource.terrains_can_be_found_on.is_empty());
        assert!(resource.improvement_stats.is_none());
        assert!(resource.revealed_by.is_none());
        assert!(resource.improvement.is_none());
        assert!(resource.improved_by.is_empty());
        assert_eq!(resource.major_deposit_amount.sparse, 1);
        assert_eq!(resource.major_deposit_amount.default, 2);
        assert_eq!(resource.major_deposit_amount.abundant, 3);
        assert_eq!(resource.minor_deposit_amount.sparse, 1);
        assert_eq!(resource.minor_deposit_amount.default, 2);
        assert_eq!(resource.minor_deposit_amount.abundant, 3);
    }

    #[test]
    fn test_tile_resource_matches_single_filter() {
        let mut resource = TileResource::new();
        resource.name = "Test".to_string();
        resource.resource_type = ResourceType::Luxury;

        assert!(resource.matches_single_filter("Test"));
        assert!(resource.matches_single_filter("any"));
        assert!(resource.matches_single_filter("all"));
        assert!(resource.matches_single_filter("Luxury"));
        assert!(!resource.matches_single_filter("Other"));
    }
}