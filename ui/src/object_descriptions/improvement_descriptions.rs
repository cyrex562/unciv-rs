use std::collections::HashMap;
use crate::models::ruleset::{Ruleset, Belief};
use crate::models::ruleset::tile::{TileImprovement, TileResource};
use crate::models::ruleset::unique::{Unique, UniqueType};
use crate::models::stats::Stat;
use crate::ui::screens::civilopedia_screen::FormattedLine;

/// Module for generating descriptions of tile improvements in the game
pub struct ImprovementDescriptions;

impl ImprovementDescriptions {
    /// Lists differences: how a nation-unique Improvement compares to its replacement.
    /// Result as indented, non-linking FormattedLines
    pub fn get_differences(
        ruleset: &Ruleset,
        original_improvement: &TileImprovement,
        replacement_improvement: &TileImprovement,
    ) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Compare stats
        for stat in Stat::iter() {
            if replacement_improvement[stat] != original_improvement[stat] {
                lines.push(FormattedLine::new(
                    format!(
                        "{} [{}] vs [{}]",
                        stat.name().tr(),
                        replacement_improvement[stat].round() as i32,
                        original_improvement[stat].round() as i32
                    ),
                    None,
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Compare terrains that can be built on
        for terrain in &replacement_improvement.terrains_can_be_built_on {
            if !original_improvement.terrains_can_be_built_on.contains(terrain) {
                lines.push(FormattedLine::new(
                    format!("Can be built on [{}]", terrain),
                    ruleset.terrains.get(terrain).map(|t| t.make_link()),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        for terrain in &original_improvement.terrains_can_be_built_on {
            if !replacement_improvement.terrains_can_be_built_on.contains(terrain) {
                lines.push(FormattedLine::new(
                    format!("Cannot be built on [{}]", terrain),
                    ruleset.terrains.get(terrain).map(|t| t.make_link()),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Compare turns to build
        if replacement_improvement.turns_to_build != original_improvement.turns_to_build {
            lines.push(FormattedLine::new(
                format!(
                    "{{Turns to build}} [{}] vs [{}]",
                    replacement_improvement.turns_to_build,
                    original_improvement.turns_to_build
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        // Check for new abilities
        let new_ability_predicate = |unique: &Unique| {
            original_improvement.uniques.contains(&unique.text) || unique.is_hidden_to_users()
        };
        for unique in replacement_improvement.unique_objects().filter(|u| !new_ability_predicate(u)) {
            lines.push(FormattedLine::new(
                unique.get_display_text(),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        // Check for lost abilities
        let lost_ability_predicate = |unique: &Unique| {
            replacement_improvement.uniques.contains(&unique.text) || unique.is_hidden_to_users()
        };
        for unique in original_improvement.unique_objects().filter(|u| !lost_ability_predicate(u)) {
            lines.push(FormattedLine::new(
                format!(
                    "Lost ability (vs [{}]): [{}]",
                    original_improvement.name,
                    unique.get_display_text().tr()
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        lines
    }

    /// Generate civilopedia text lines for a tile improvement
    pub fn get_civilopedia_text_lines(improvement: &TileImprovement, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        // Add stats description
        let stats_desc = improvement.clone_stats().to_string();
        if !stats_desc.is_empty() {
            text_list.push(FormattedLine::new(
                stats_desc,
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add unique to/replaces info
        if let Some(unique_to) = &improvement.unique_to {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                format!("Unique to [{}]", unique_to),
                Some(format!("Nation/{}", unique_to)),
                None,
                None,
                None,
                None,
            ));
        }
        if let Some(replaces) = &improvement.replaces {
            if let Some(replace_improvement) = ruleset.tile_improvements.get(replaces) {
                text_list.push(FormattedLine::new(
                    format!("Replaces [{}]", replaces),
                    Some(replace_improvement.make_link()),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Get constructor and creating units
        let constructor_units = improvement.get_constructor_units(ruleset);
        let creating_units = improvement.get_creating_units(ruleset);
        let creator_exists = !constructor_units.is_empty() || !creating_units.is_empty();

        // Add terrain information
        if creator_exists && !improvement.terrains_can_be_built_on.is_empty() {
            text_list.push(FormattedLine::empty());
            if improvement.terrains_can_be_built_on.len() == 1 {
                let terrain = &improvement.terrains_can_be_built_on[0];
                text_list.push(FormattedLine::new(
                    format!("{{Can be built on}} {{{}}}", terrain),
                    Some(format!("Terrain/{}", terrain)),
                    None,
                    None,
                    None,
                    None,
                ));
            } else {
                text_list.push(FormattedLine::new(
                    "{Can be built on}:".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
                for terrain in &improvement.terrains_can_be_built_on {
                    text_list.push(FormattedLine::new(
                        terrain.clone(),
                        Some(format!("Terrain/{}", terrain)),
                        None,
                        Some(1),
                        None,
                        None,
                    ));
                }
            }
        }

        // Add resource bonus information
        let mut added_line_before_resource_bonus = false;
        for resource in ruleset.tile_resources.values() {
            if resource.improvement_stats.is_none() || !resource.is_improved_by(&improvement.name) {
                continue;
            }
            if !added_line_before_resource_bonus {
                added_line_before_resource_bonus = true;
                text_list.push(FormattedLine::empty());
            }
            let stats_string = resource.improvement_stats.as_ref().unwrap().to_string();
            // Line intentionally modeled as UniqueType.Stats + ConditionalInTiles
            text_list.push(FormattedLine::new(
                format!("[{}] <in [{}] tiles>", stats_string, resource.name),
                Some(resource.make_link()),
                None,
                None,
                None,
                None,
            ));
        }

        // Add tech requirement
        if let Some(tech_required) = &improvement.tech_required {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                format!("Required tech: [{}]", tech_required),
                Some(format!("Technology/{}", tech_required)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add uniques
        improvement.uniques_to_civilopedia_text_lines_mut(&mut text_list, None, false, false, |_| false);

        // Add feature removal note if needed
        if creator_exists
            && !improvement.is_empty() // Has any Stats
            && !improvement.has_unique(UniqueType::NoFeatureRemovalNeeded)
            && !improvement.has_unique(UniqueType::RemovesFeaturesIfBuilt)
            && improvement.terrains_can_be_built_on.iter().all(|t| !ruleset.terrains.contains_key(t))
        {
            text_list.push(FormattedLine::new(
                "Needs removal of terrain features to be built".to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add ancient ruins rewards if applicable
        if improvement.is_ancient_ruins_equivalent() && !ruleset.ruin_rewards.is_empty() {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                "The possible rewards are:".to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
            for reward in ruleset.ruin_rewards.values().filter(|r| !r.is_hidden_from_civilopedia(ruleset)) {
                text_list.push(FormattedLine::new(
                    reward.name.clone(),
                    None,
                    Some(reward.color.clone()),
                    None,
                    None,
                    Some(true),
                ));
                text_list.extend(reward.civilopedia_text.clone());
            }
        }

        // Add constructor and creating units
        if creator_exists {
            text_list.push(FormattedLine::empty());
        }
        for unit in constructor_units {
            text_list.push(FormattedLine::new(
                format!("{{Can be constructed by}} {{{}}}", unit),
                Some(unit.make_link()),
                None,
                None,
                None,
                None,
            ));
        }
        for unit in creating_units {
            text_list.push(FormattedLine::new(
                format!("{{Can be created instantly by}} {{{}}}", unit),
                Some(unit.make_link()),
                None,
                None,
                None,
                None,
            ));
        }

        // Add see also section
        let mut see_also = Vec::new();
        for (_, also_improvement) in &ruleset.tile_improvements {
            if also_improvement.replaces.as_ref() == Some(&improvement.name) {
                see_also.push(FormattedLine::new(
                    also_improvement.name.clone(),
                    Some(also_improvement.make_link()),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        see_also.extend(Belief::get_civilopedia_text_matching(&improvement.name, ruleset, false));

        if !see_also.is_empty() {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                "{See also}:".to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
            text_list.extend(see_also);
        }

        text_list
    }

    /// Generate a description of a tile improvement
    pub fn get_description(improvement: &TileImprovement, ruleset: &Ruleset) -> String {
        let mut lines = Vec::new();

        // Add stats description
        let stats_desc = improvement.clone_stats().to_string();
        if !stats_desc.is_empty() {
            lines.push(stats_desc);
        }

        // Add unique to/replaces info
        if let Some(unique_to) = &improvement.unique_to {
            lines.push(format!("Unique to [{}]", unique_to).tr());
        }
        if let Some(replaces) = &improvement.replaces {
            lines.push(format!("Replaces [{}]", replaces).tr());
        }

        // Add terrain information
        if !improvement.terrains_can_be_built_on.is_empty() {
            let terrains_can_be_built_on_string: Vec<String> = improvement.terrains_can_be_built_on
                .iter()
                .map(|t| t.tr())
                .collect();
            lines.push(format!(
                "{} {}",
                "Can be built on".tr(),
                terrains_can_be_built_on_string.join(", ")
            ));
        }

        // Add resource bonus information
        for resource in ruleset.tile_resources.values().filter(|r| r.is_improved_by(&improvement.name)) {
            if let Some(stats) = &resource.improvement_stats {
                let stats_string = stats.to_string();
                lines.push(format!("[{}] <in [{}] tiles>", stats_string, resource.name).tr());
            }
        }

        // Add tech requirement
        if let Some(tech_required) = &improvement.tech_required {
            lines.push(format!("Required tech: [{}]", tech_required).tr());
        }

        // Add uniques
        improvement.uniques_to_description(&mut lines, |_| false);

        lines.join("\n")
    }

    /// Generate a short description of a tile improvement
    pub fn get_short_description(improvement: &TileImprovement) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Add terrain information
        if !improvement.terrains_can_be_built_on.is_empty() {
            for (index, terrain) in improvement.terrains_can_be_built_on.iter().enumerate() {
                lines.push(FormattedLine::new(
                    if index == 0 {
                        format!("{{Can be built on}} {{{}}}", terrain)
                    } else {
                        format!("or [{}]", terrain)
                    },
                    Some(format!("Terrain/{}", terrain)),
                    None,
                    Some(if index == 0 { 1 } else { 2 }),
                    None,
                    None,
                ));
            }
        }

        // Add uniques
        for unique in improvement.unique_objects() {
            if unique.is_hidden_to_users() {
                continue;
            }
            lines.push(FormattedLine::from_unique_with_indent(unique, 1));
        }

        lines
    }
}