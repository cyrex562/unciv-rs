use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use regex::Regex;

use crate::constants::{BARBARIANS, NEUTRAL_VICTORY_TYPE, RANDOM, SPECTATOR};
use crate::logic::MultiFilter;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::RulesetObject;
use crate::models::ruleset::unique::{StateForConditionals, UniqueMap, UniqueTarget, UniqueType};
use crate::models::translations::{square_brace_regex, tr};
use crate::ui::components::extensions::color_from_rgb;
use crate::ui::images::ImageGetter;
use crate::ui::objectdescriptions::{BaseUnitDescriptions, BuildingDescriptions, ImprovementDescriptions, uniques_to_civilopedia_text_lines};
use crate::ui::screens::civilopedia_screen::{FormattedLine, ICivilopediaText};

/// Represents a nation in the game
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Nation {
    /// The name of the nation
    pub name: String,
    /// The name of the nation's leader
    pub leader_name: String,
    /// The style of the nation
    pub style: String,
    /// The type of city-state (if this is a city-state)
    pub city_state_type: Option<String>,
    /// The preferred victory type for this nation
    pub preferred_victory_type: String,
    /// Audio clip for when other civilizations declare war
    pub declaring_war: String,
    /// Audio clip for when this nation is attacked
    pub attacked: String,
    /// Audio clip for when this nation is defeated
    pub defeated: String,
    /// Audio clip for first contact with another civilization
    pub introduction: String,
    /// Audio clip for trade requests
    pub trade_request: String,
    /// Audio clip for neutral greetings
    pub neutral_hello: String,
    /// Audio clip for hostile greetings
    pub hate_hello: String,
    /// The outer color of the nation (RGB values)
    pub outer_color: Vec<i32>,
    /// The unique name of the nation
    pub unique_name: String,
    /// The unique text description of the nation
    pub unique_text: String,
    /// The inner color of the nation (RGB values)
    pub inner_color: Option<Vec<i32>>,
    /// The start bias terrain preferences
    pub start_bias: Vec<String>,
    /// The personality of the nation
    pub personality: Option<String>,
    /// The first part of the start introduction
    pub start_intro_part1: String,
    /// The second part of the start introduction
    pub start_intro_part2: String,
    /// The names used for spies
    pub spy_names: Vec<String>,
    /// The favored religion of the nation
    pub favored_religion: Option<String>,
    /// The cities of the nation
    pub cities: Vec<String>,
    /// The unique map for this nation
    pub unique_map: UniqueMap,
    /// Cached outer color object
    #[serde(skip)]
    outer_color_object: OnceLock<u32>,
    /// Cached inner color object
    #[serde(skip)]
    inner_color_object: OnceLock<u32>,
    /// Cached flag for forests and jungles being roads
    #[serde(skip)]
    forests_and_jungles_are_roads: OnceLock<bool>,
    /// Cached flag for ignoring hill movement cost
    #[serde(skip)]
    ignore_hill_movement_cost: OnceLock<bool>,
}

impl Nation {
    /// Creates a new Nation with default values
    pub fn new() -> Self {
        Nation {
            name: String::new(),
            leader_name: String::new(),
            style: String::new(),
            city_state_type: None,
            preferred_victory_type: NEUTRAL_VICTORY_TYPE.to_string(),
            declaring_war: String::new(),
            attacked: String::new(),
            defeated: String::new(),
            introduction: String::new(),
            trade_request: String::new(),
            neutral_hello: String::new(),
            hate_hello: String::new(),
            outer_color: vec![255, 255, 255],
            unique_name: String::new(),
            unique_text: String::new(),
            inner_color: None,
            start_bias: Vec::new(),
            personality: None,
            start_intro_part1: String::new(),
            start_intro_part2: String::new(),
            spy_names: Vec::new(),
            favored_religion: None,
            cities: Vec::new(),
            unique_map: UniqueMap::new(),
            outer_color_object: OnceLock::new(),
            inner_color_object: OnceLock::new(),
            forests_and_jungles_are_roads: OnceLock::new(),
            ignore_hill_movement_cost: OnceLock::new(),
        }
    }

    /// Gets the leader display name
    pub fn get_leader_display_name(&self) -> String {
        if self.is_city_state() || self.is_spectator() {
            self.name.clone()
        } else {
            format!("[{}] of [{}]", self.leader_name, self.name)
        }
    }

    /// Gets the style or civilization name
    pub fn get_style_or_civ_name(&self) -> String {
        if self.style.is_empty() {
            self.name.clone()
        } else {
            self.style.clone()
        }
    }

    /// Gets the outer color of the nation
    pub fn get_outer_color(&self) -> u32 {
        self.outer_color_object.get_or_init(|| {
            color_from_rgb(&self.outer_color)
        }).clone()
    }

    /// Gets the inner color of the nation
    pub fn get_inner_color(&self) -> u32 {
        self.inner_color_object.get_or_init(|| {
            if let Some(inner_color) = &self.inner_color {
                color_from_rgb(inner_color)
            } else {
                ImageGetter::CHARCOAL
            }
        }).clone()
    }

    /// Checks if this nation is a city-state
    pub fn is_city_state(&self) -> bool {
        self.city_state_type.is_some()
    }

    /// Checks if this nation is a major civilization
    pub fn is_major_civ(&self) -> bool {
        !self.is_barbarian() && !self.is_city_state() && !self.is_spectator()
    }

    /// Checks if this nation is barbarian
    pub fn is_barbarian(&self) -> bool {
        self.name == BARBARIANS
    }

    /// Checks if this nation is a spectator
    pub fn is_spectator(&self) -> bool {
        self.name == SPECTATOR
    }

    /// Sets the transient properties
    pub fn set_transients(&mut self) {
        self.outer_color_object.get_or_init(|| {
            color_from_rgb(&self.outer_color)
        });

        self.inner_color_object.get_or_init(|| {
            if let Some(inner_color) = &self.inner_color {
                color_from_rgb(inner_color)
            } else {
                ImageGetter::CHARCOAL
            }
        });

        self.forests_and_jungles_are_roads.get_or_init(|| {
            self.unique_map.has_unique(UniqueType::ForestsAndJunglesAreRoads)
        });

        self.ignore_hill_movement_cost.get_or_init(|| {
            self.unique_map.has_unique(UniqueType::IgnoreHillMovementCost)
        });
    }

    /// Checks if this nation matches a filter
    pub fn matches_filter(&self, filter: &str, state: Option<&StateForConditionals>, multi_filter: bool) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| {
                self.matches_single_filter(f) ||
                state.map_or_else(
                    || self.unique_map.has_tag_unique(f),
                    |s| self.unique_map.has_unique(f, s)
                )
            })
        } else {
            self.matches_single_filter(filter) ||
            state.map_or_else(
                || self.unique_map.has_tag_unique(filter),
                |s| self.unique_map.has_unique(filter, s)
            )
        }
    }

    /// Checks if this nation matches a single filter
    fn matches_single_filter(&self, filter: &str) -> bool {
        match filter {
            "All" | "all" => true,
            "Major" => self.is_major_civ(),
            "City-State" => self.is_city_state(),
            _ => filter == self.name,
        }
    }

    /// Gets the city state information for the civilopedia
    fn get_city_state_info(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        if let Some(city_state_type_name) = &self.city_state_type {
            if let Some(city_state_type) = ruleset.city_state_types.get(city_state_type_name) {
                text_list.push(FormattedLine::new_with_color(
                    format!("{{Type}}: {{{}}}", city_state_type.name),
                    4,
                    0,
                    format!("#{:06X}", city_state_type.get_color()),
                ));

                let mut show_resources = false;

                // Helper function to add bonus lines
                let add_bonus_lines = |header: &str, unique_map: &UniqueMap| {
                    let bonuses: Vec<_> = unique_map.get_all_uniques()
                        .filter(|u| !u.is_hidden_to_users())
                        .collect();

                    if bonuses.is_empty() {
                        return;
                    }

                    text_list.push(FormattedLine::new("", 0, 0));
                    text_list.push(FormattedLine::new(format!("{{{}}} ", header), 0, 0));

                    for unique in bonuses {
                        text_list.push(FormattedLine::new(unique.clone(), 0, 1));
                        if unique.unique_type == UniqueType::CityStateUniqueLuxury {
                            show_resources = true;
                        }
                    }
                };

                add_bonus_lines("When Friends:", &city_state_type.get_friend_bonus_unique_map());
                add_bonus_lines("When Allies:", &city_state_type.get_ally_bonus_unique_map());

                if show_resources {
                    let all_mercantile_resources: Vec<_> = ruleset.tile_resources.values()
                        .filter(|r| r.has_unique(UniqueType::CityStateOnlyResource))
                        .collect();

                    if !all_mercantile_resources.is_empty() {
                        text_list.push(FormattedLine::new("", 0, 0));
                        text_list.push(FormattedLine::new("The unique luxury is one of:", 0, 0));
                        for resource in all_mercantile_resources {
                            text_list.push(FormattedLine::new_with_link(
                                resource.name.clone(),
                                resource.make_link(),
                                0,
                                1,
                            ));
                        }
                    }
                }
                text_list.push(FormattedLine::new_with_separator(true));
            }
        }

        text_list
    }

    /// Gets the unique buildings text for the civilopedia
    fn get_unique_buildings_text(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        for building in ruleset.buildings.values() {
            if building.unique_to.is_none() {
                continue;
            }

            if !self.matches_filter(building.unique_to.as_ref().unwrap(), None, true) {
                continue;
            }

            if building.is_hidden_from_civilopedia(ruleset) {
                continue;
            }

            text_list.push(FormattedLine::new_with_separator(true));
            text_list.push(FormattedLine::new_with_link(
                format!("{{{}}} -", building.name),
                building.make_link(),
                0,
                0,
            ));

            if let Some(replaces) = &building.replaces {
                if let Some(original_building) = ruleset.buildings.get(replaces) {
                    text_list.push(FormattedLine::new_with_link(
                        format!("Replaces [{}]", original_building.name),
                        original_building.make_link(),
                        0,
                        1,
                    ));

                    for diff in BuildingDescriptions::get_differences(original_building, building) {
                        text_list.push(diff);
                    }

                    text_list.push(FormattedLine::new("", 0, 0));
                } else {
                    text_list.push(FormattedLine::new(
                        format!("Replaces [{}], which is not found in the ruleset!", replaces),
                        0,
                        1,
                    ));
                }
            } else {
                text_list.push(FormattedLine::new(
                    building.get_short_description(true),
                    0,
                    1,
                ));
            }
        }

        text_list
    }

    /// Gets the unique units text for the civilopedia
    fn get_unique_units_text(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        for unit in ruleset.units.values() {
            if unit.is_hidden_from_civilopedia(ruleset) {
                continue;
            }

            if unit.unique_to.is_none() || !self.matches_filter(unit.unique_to.as_ref().unwrap(), None, true) {
                continue;
            }

            text_list.push(FormattedLine::new_with_separator(true));
            text_list.push(FormattedLine::new_with_link(
                format!("{{{}}} -", unit.name),
                format!("Unit/{}", unit.name),
                0,
                0,
            ));

            if let Some(replaces) = &unit.replaces {
                if let Some(original_unit) = ruleset.units.get(replaces) {
                    text_list.push(FormattedLine::new_with_link(
                        format!("Replaces [{}]", original_unit.name),
                        format!("Unit/{}", original_unit.name),
                        0,
                        1,
                    ));

                    if unit.cost != original_unit.cost {
                        text_list.push(FormattedLine::new(
                            format!("{{Cost}} {} [{}] vs [{}]",
                                tr("Cost"),
                                unit.cost,
                                original_unit.cost
                            ),
                            0,
                            1,
                        ));
                    }

                    for (text, link) in BaseUnitDescriptions::get_differences(ruleset, original_unit, unit) {
                        text_list.push(FormattedLine::new_with_link(
                            text,
                            link.unwrap_or_default(),
                            0,
                            1,
                        ));
                    }
                } else {
                    text_list.push(FormattedLine::new(
                        format!("Replaces [{}], which is not found in the ruleset!", replaces),
                        0,
                        1,
                    ));
                }
            } else {
                for line in unit.get_civilopedia_text_lines(ruleset) {
                    text_list.push(FormattedLine::new_with_link_and_color(
                        line.text,
                        line.link.unwrap_or_default(),
                        line.indent + 1,
                        line.color,
                    ));
                }
            }

            text_list.push(FormattedLine::new("", 0, 0));
        }

        text_list
    }

    /// Gets the unique improvements text for the civilopedia
    fn get_unique_improvements_text(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        for improvement in ruleset.tile_improvements.values() {
            if improvement.is_hidden_from_civilopedia(ruleset) {
                continue;
            }

            if improvement.unique_to.is_none() || !self.matches_filter(improvement.unique_to.as_ref().unwrap(), None, true) {
                continue;
            }

            text_list.push(FormattedLine::new_with_separator(true));
            text_list.push(FormattedLine::new_with_link(
                improvement.name.clone(),
                format!("Improvement/{}", improvement.name),
                0,
                0,
            ));

            text_list.push(FormattedLine::new(
                improvement.clone_stats().to_string(),
                0,
                1,
            ));

            if let Some(replaces) = &improvement.replaces {
                if let Some(original_improvement) = ruleset.tile_improvements.get(replaces) {
                    text_list.push(FormattedLine::new_with_link(
                        format!("Replaces [{}]", original_improvement.name),
                        original_improvement.make_link(),
                        0,
                        1,
                    ));

                    for diff in ImprovementDescriptions::get_differences(ruleset, original_improvement, improvement) {
                        text_list.push(diff);
                    }

                    text_list.push(FormattedLine::new("", 0, 0));
                } else {
                    text_list.push(FormattedLine::new(
                        format!("Replaces [{}], which is not found in the ruleset!", replaces),
                        0,
                        1,
                    ));
                }
            } else {
                for line in improvement.get_short_description() {
                    text_list.push(line);
                }
            }
        }

        text_list
    }
}

impl RulesetObject for Nation {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Nation
    }
}

impl ICivilopediaText for Nation {
    fn make_link(&self) -> String {
        format!("Nation/{}", self.name)
    }

    fn get_sort_group(&self, _ruleset: &Ruleset) -> i32 {
        if self.is_city_state() {
            1
        } else if self.is_barbarian() {
            9
        } else {
            0
        }
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        if self.is_city_state() {
            text_list.extend(self.get_city_state_info(ruleset));
        }

        if !self.leader_name.is_empty() {
            text_list.push(FormattedLine::new_with_image(
                format!("LeaderIcons/{}", self.leader_name),
                200.0,
                0,
                0,
            ));
            text_list.push(FormattedLine::new_centered(
                self.get_leader_display_name(),
                3,
                0,
            ));
            text_list.push(FormattedLine::new("", 0, 0));
        }

        if !self.unique_name.is_empty() {
            text_list.push(FormattedLine::new(
                format!("{{{}}}:", self.unique_name),
                4,
                0,
            ));
        }

        if !self.unique_text.is_empty() {
            text_list.push(FormattedLine::new(self.unique_text.clone(), 0, 1));
        } else {
            uniques_to_civilopedia_text_lines(&mut text_list, &self.unique_map, None);
        }

        text_list.push(FormattedLine::new("", 0, 0));

        if !self.start_bias.is_empty() {
            for (i, bias) in self.start_bias.iter().enumerate() {
                let link = if !bias.contains('[') {
                    bias.clone()
                } else {
                    let re = square_brace_regex();
                    if let Some(caps) = re.captures(bias) {
                        caps[1].to_string()
                    } else {
                        bias.clone()
                    }
                };

                let prefix = if i == 0 { "[Start bias:] " } else { "" };
                let icon_crossed = bias.starts_with("Avoid ");

                text_list.push(FormattedLine::new_with_link_and_icon(
                    format!("{}{}", prefix, tr(bias)),
                    format!("Terrain/{}", link),
                    if i == 0 { 0 } else { 1 },
                    icon_crossed,
                ));
            }
            text_list.push(FormattedLine::new("", 0, 0));
        }

        text_list.extend(self.get_unique_buildings_text(ruleset));
        text_list.extend(self.get_unique_units_text(ruleset));
        text_list.extend(self.get_unique_improvements_text(ruleset));

        text_list
    }
}

/// Gets the relative luminance of a color
pub fn get_relative_luminance(color: u32) -> f64 {
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
    let b = (color & 0xFF) as f32 / 255.0;

    let get_relative_channel_luminance = |channel: f32| -> f64 {
        if channel < 0.03928 {
            channel as f64 / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4) as f64
        }
    };

    let r_luminance = get_relative_channel_luminance(r);
    let g_luminance = get_relative_channel_luminance(g);
    let b_luminance = get_relative_channel_luminance(b);

    0.2126 * r_luminance + 0.7152 * g_luminance + 0.0722 * b_luminance
}

/// Gets the contrast ratio between two colors
pub fn get_contrast_ratio(color1: u32, color2: u32) -> f64 {
    let inner_color_luminance = get_relative_luminance(color1);
    let outer_color_luminance = get_relative_luminance(color2);

    if inner_color_luminance > outer_color_luminance {
        (inner_color_luminance + 0.05) / (outer_color_luminance + 0.05)
    } else {
        (outer_color_luminance + 0.05) / (inner_color_luminance + 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nation_new() {
        let nation = Nation::new();
        assert!(nation.name.is_empty());
        assert!(nation.leader_name.is_empty());
        assert!(nation.style.is_empty());
        assert!(nation.city_state_type.is_none());
        assert_eq!(nation.preferred_victory_type, NEUTRAL_VICTORY_TYPE);
        assert!(nation.declaring_war.is_empty());
        assert!(nation.attacked.is_empty());
        assert!(nation.defeated.is_empty());
        assert!(nation.introduction.is_empty());
        assert!(nation.trade_request.is_empty());
        assert!(nation.neutral_hello.is_empty());
        assert!(nation.hate_hello.is_empty());
        assert_eq!(nation.outer_color, vec![255, 255, 255]);
        assert!(nation.unique_name.is_empty());
        assert!(nation.unique_text.is_empty());
        assert!(nation.inner_color.is_none());
        assert!(nation.start_bias.is_empty());
        assert!(nation.personality.is_none());
        assert!(nation.start_intro_part1.is_empty());
        assert!(nation.start_intro_part2.is_empty());
        assert!(nation.spy_names.is_empty());
        assert!(nation.favored_religion.is_none());
        assert!(nation.cities.is_empty());
    }

    #[test]
    fn test_get_leader_display_name() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();
        nation.leader_name = "TestLeader".to_string();

        assert_eq!(nation.get_leader_display_name(), "[TestLeader] of [TestNation]");

        nation.city_state_type = Some("TestCityState".to_string());
        assert_eq!(nation.get_leader_display_name(), "TestNation");

        nation.city_state_type = None;
        nation.name = SPECTATOR.to_string();
        assert_eq!(nation.get_leader_display_name(), SPECTATOR);
    }

    #[test]
    fn test_get_style_or_civ_name() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();
        nation.style = "TestStyle".to_string();

        assert_eq!(nation.get_style_or_civ_name(), "TestStyle");

        nation.style = String::new();
        assert_eq!(nation.get_style_or_civ_name(), "TestNation");
    }

    #[test]
    fn test_is_city_state() {
        let mut nation = Nation::new();
        assert!(!nation.is_city_state());

        nation.city_state_type = Some("TestCityState".to_string());
        assert!(nation.is_city_state());
    }

    #[test]
    fn test_is_major_civ() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();
        assert!(nation.is_major_civ());

        nation.name = BARBARIANS.to_string();
        assert!(!nation.is_major_civ());

        nation.name = "TestNation".to_string();
        nation.city_state_type = Some("TestCityState".to_string());
        assert!(!nation.is_major_civ());

        nation.city_state_type = None;
        nation.name = SPECTATOR.to_string();
        assert!(!nation.is_major_civ());
    }

    #[test]
    fn test_is_barbarian() {
        let mut nation = Nation::new();
        assert!(!nation.is_barbarian());

        nation.name = BARBARIANS.to_string();
        assert!(nation.is_barbarian());
    }

    #[test]
    fn test_is_spectator() {
        let mut nation = Nation::new();
        assert!(!nation.is_spectator());

        nation.name = SPECTATOR.to_string();
        assert!(nation.is_spectator());
    }

    #[test]
    fn test_matches_single_filter() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();

        assert!(nation.matches_single_filter("All"));
        assert!(nation.matches_single_filter("all"));
        assert!(!nation.matches_single_filter("Major"));
        assert!(!nation.matches_single_filter("City-State"));
        assert!(nation.matches_single_filter("TestNation"));
        assert!(!nation.matches_single_filter("OtherNation"));

        nation.name = BARBARIANS.to_string();
        assert!(!nation.matches_single_filter("Major"));

        nation.name = "TestNation".to_string();
        nation.city_state_type = Some("TestCityState".to_string());
        assert!(nation.matches_single_filter("City-State"));
    }

    #[test]
    fn test_make_link() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();
        assert_eq!(nation.make_link(), "Nation/TestNation");
    }

    #[test]
    fn test_get_sort_group() {
        let mut nation = Nation::new();
        nation.name = "TestNation".to_string();
        assert_eq!(nation.get_sort_group(&Ruleset::new()), 0);

        nation.city_state_type = Some("TestCityState".to_string());
        assert_eq!(nation.get_sort_group(&Ruleset::new()), 1);

        nation.city_state_type = None;
        nation.name = BARBARIANS.to_string();
        assert_eq!(nation.get_sort_group(&Ruleset::new()), 9);
    }

    #[test]
    fn test_get_relative_luminance() {
        assert_eq!(get_relative_luminance(0x000000), 0.0);
        assert!(get_relative_luminance(0xFFFFFF) > 0.0);
        assert!(get_relative_luminance(0xFF0000) > 0.0);
        assert!(get_relative_luminance(0x00FF00) > 0.0);
        assert!(get_relative_luminance(0x0000FF) > 0.0);
    }

    #[test]
    fn test_get_contrast_ratio() {
        assert_eq!(get_contrast_ratio(0x000000, 0x000000), 1.0);
        assert!(get_contrast_ratio(0xFFFFFF, 0x000000) > 1.0);
        assert!(get_contrast_ratio(0xFF0000, 0x00FF00) > 1.0);
    }
}