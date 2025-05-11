use std::collections::HashMap;

use crate::models::ruleset::{Ruleset, Building, Belief};
use crate::models::ruleset::unique::{Unique, UniqueType, StateForConditionals};
use crate::models::stats::Stat;
use crate::models::city::City;
use crate::ui::components::fonts::Fonts;
use crate::ui::components::extensions::ConsumesAmountExt;
use crate::ui::screens::civilopedia_screen::FormattedLine;

/// Module for generating descriptions of buildings in the game
pub struct BuildingDescriptions;

impl BuildingDescriptions {
    /// Used for AlertType.WonderBuilt, and as sub-text in Nation and Tech descriptions
    pub fn get_short_description(
        building: &Building,
        multiline: bool,
        unique_inclusion_filter: Option<impl Fn(&Unique) -> bool>,
    ) -> String {
        let mut info_list = Vec::new();

        // Add base stats
        if let Some(stats) = building.clone_stats() {
            if !stats.is_empty() {
                info_list.push(stats.to_string());
            }
        }

        // Add percentage bonuses
        for (stat, value) in building.get_stat_percentage_bonuses(None) {
            info_list.push(format!("+{}% {}", value.round() as i32, stat.name()));
        }

        // Add required nearby resources
        if let Some(resources) = &building.required_nearby_improved_resources {
            info_list.push(format!(
                "Requires improved [{}] near city",
                resources.join("/")
            ));
        }

        // Add uniques
        if !building.uniques.is_empty() {
            if !building.replacement_text_for_uniques.is_empty() {
                info_list.push(building.replacement_text_for_uniques.clone());
            } else {
                info_list.extend(building.get_uniques_strings_without_disablers(unique_inclusion_filter));
            }
        }

        // Add city stats
        if building.city_strength != 0 {
            info_list.push(format!("{{City strength}} +{}", building.city_strength));
        }
        if building.city_health != 0 {
            info_list.push(format!("{{City health}} +{}", building.city_health));
        }

        let separator = if multiline { "\n" } else { "; " };
        info_list.join(separator)
    }

    /// Used in CityScreen (ConstructionInfoTable)
    pub fn get_description(
        building: &Building,
        city: &City,
        show_additional_info: bool,
    ) -> String {
        let mut translated_lines = Vec::new();
        let stats = building.get_stats(city);
        let is_free = city.civ.civ_constructions.has_free_building(city, building);

        // Add unique/replaces info
        if let Some(unique_to) = &building.unique_to {
            translated_lines.push(
                if building.replaces.is_none() {
                    format!("Unique to [{}]", unique_to)
                } else {
                    format!("Unique to [{}], replaces [{}]", unique_to, building.replaces.as_ref().unwrap())
                }
            );
        }

        // Add wonder type
        if building.is_wonder {
            translated_lines.push("Wonder".to_string());
        }
        if building.is_national_wonder {
            translated_lines.push("National Wonder".to_string());
        }

        // Add resource requirements
        if !is_free {
            for (resource_name, amount) in building.get_resource_requirements_per_turn(&city.state) {
                let available = city.get_available_resource_amount(&resource_name);
                if let Some(resource) = city.get_ruleset().tile_resources.get(&resource_name) {
                    let consumes_string = resource_name.get_consumes_amount_string(amount, resource.is_stockpiled);
                    translated_lines.push(
                        if show_additional_info {
                            format!("{} ({} available)", consumes_string, available)
                        } else {
                            consumes_string
                        }
                    );
                }
            }
        }

        // Add uniques
        if !building.uniques.is_empty() {
            if !building.replacement_text_for_uniques.is_empty() {
                translated_lines.push(building.replacement_text_for_uniques.clone());
            } else {
                translated_lines.extend(
                    building.get_uniques_strings_without_disablers(|unique| {
                        unique.unique_type != UniqueType::ConsumesResources
                    })
                );
            }
        }

        // Add stats
        if !stats.is_empty() {
            translated_lines.push(stats.to_string());
        }

        // Add percentage bonuses
        for (stat, value) in building.get_stat_percentage_bonuses(Some(city)) {
            if value != 0.0 {
                translated_lines.push(format!("+{}% {{{}}}", value.round() as i32, stat.name()));
            }
        }

        // Add great person points
        for (great_person_name, value) in &building.great_person_points {
            translated_lines.push(format!("+{} [{}] points", value, great_person_name));
        }

        // Add specialist slots
        for (specialist_name, amount) in building.new_specialists() {
            translated_lines.push(format!("+{} [{}] slots", amount, specialist_name));
        }

        // Add required nearby resources
        if let Some(resources) = &building.required_nearby_improved_resources {
            translated_lines.push(format!(
                "Requires improved [{}] near city",
                resources.join("/")
            ));
        }

        // Add city stats
        if building.city_strength != 0 {
            translated_lines.push(format!("{{City strength}} +{}", building.city_strength));
        }
        if building.city_health != 0 {
            translated_lines.push(format!("{{City health}} +{}", building.city_health));
        }
        if building.maintenance != 0 && !is_free {
            translated_lines.push(format!("{{Maintenance cost}}: {} {{Gold}}", building.maintenance));
        }

        // Add additional description if needed
        if show_additional_info {
            Self::additional_description(building, city, &mut translated_lines);
        }

        translated_lines.join("\n").trim().to_string()
    }

    fn additional_description(building: &Building, city: &City, lines: &mut Vec<String>) {
        // Check building uniques for conditional requirements
        for unique in &building.uniques {
            if unique.unique_type == UniqueType::OnlyAvailable
                || unique.unique_type == UniqueType::CanOnlyBeBuiltWhen
            {
                for conditional in unique.get_modifiers(UniqueType::ConditionalBuildingBuiltAll) {
                    Self::missing_city_text(
                        &conditional.params[0],
                        city,
                        &conditional.params[1],
                        lines
                    );
                }
            }
        }
    }

    fn missing_city_text(building: &str, city: &City, filter: &str, lines: &mut Vec<String>) {
        let missing_cities: Vec<_> = city.civ.cities
            .iter()
            .filter(|city| {
                city.matches_filter(filter)
                && !city.city_constructions.contains_building_or_equivalent(building)
            })
            .collect();

        if !missing_cities.is_empty() {
            lines.push(format!(
                "\n[{}] required: {}",
                city.civ.get_equivalent_building(building),
                missing_cities
                    .iter()
                    .map(|city| city.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    /// Lists differences: how a nation-unique Building compares to its replacement.
    /// Cost is included.
    pub fn get_differences(
        original_building: &Building,
        replacement_building: &Building,
    ) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Compare stats
        for stat in Stat::iter() {
            if replacement_building[stat] != original_building[stat] {
                lines.push(FormattedLine::new(
                    format!(
                        "{} [{}] vs [{}]",
                        stat.name(),
                        replacement_building[stat].round() as i32,
                        original_building[stat].round() as i32
                    ),
                    None,
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Compare stat percentage bonuses
        let original_stat_bonus = original_building.get_stat_percentage_bonuses(None);
        let replacement_stat_bonus = replacement_building.get_stat_percentage_bonuses(None);
        for stat in Stat::iter() {
            if replacement_stat_bonus[stat] != original_stat_bonus[stat] {
                lines.push(FormattedLine::new(
                    format!(
                        "[{}]% {} vs [{}]% {}",
                        replacement_stat_bonus[stat].round() as i32,
                        stat.name(),
                        original_stat_bonus[stat].round() as i32,
                        stat.name()
                    ),
                    None,
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Compare maintenance
        if replacement_building.maintenance != original_building.maintenance {
            lines.push(FormattedLine::new(
                format!(
                    "{{Maintenance}} [{}] vs [{}]",
                    replacement_building.maintenance,
                    original_building.maintenance
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        // Compare cost
        if replacement_building.cost != original_building.cost {
            lines.push(FormattedLine::new(
                format!(
                    "{{Cost}} [{}] vs [{}]",
                    replacement_building.cost,
                    original_building.cost
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        // Compare city stats
        if replacement_building.city_strength != original_building.city_strength {
            lines.push(FormattedLine::new(
                format!(
                    "{{City strength}} [{}] vs [{}]",
                    replacement_building.city_strength,
                    original_building.city_strength
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }
        if replacement_building.city_health != original_building.city_health {
            lines.push(FormattedLine::new(
                format!(
                    "{{City health}} [{}] vs [{}]",
                    replacement_building.city_health,
                    original_building.city_health
                ),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        }

        // Compare uniques
        if !replacement_building.replacement_text_for_uniques.is_empty() {
            lines.push(FormattedLine::new(
                replacement_building.replacement_text_for_uniques.clone(),
                None,
                None,
                Some(1),
                None,
                None,
            ));
        } else {
            // Check for new abilities
            let new_ability_predicate = |unique: &Unique| {
                original_building.uniques.contains(&unique.text) || unique.is_hidden_to_users()
            };
            for unique in replacement_building.uniques.iter().filter(|u| !new_ability_predicate(u)) {
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
                replacement_building.uniques.contains(&unique.text) || unique.is_hidden_to_users()
            };
            for unique in original_building.uniques.iter().filter(|u| !lost_ability_predicate(u)) {
                lines.push(FormattedLine::new(
                    format!(
                        "Lost ability (vs [{}]): [{}]",
                        original_building.name,
                        unique.text
                    ),
                    None,
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        lines
    }

    /// Generate civilopedia text lines for a building
    pub fn get_civilopedia_text_lines(building: &Building, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        // Add wonder type
        if building.is_any_wonder() {
            text_list.push(FormattedLine::new(
                if building.is_wonder { "Wonder" } else { "National Wonder" }.to_string(),
                None,
                Some("#CA4".to_string()),
                Some(3),
                None,
                None,
            ));
        }

        // Add unique to/replaces info
        if let Some(unique_to) = &building.unique_to {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                format!("Unique to [{}]", unique_to),
                Some(format!("Nation/{}", unique_to)),
                None,
                None,
                None,
                None,
            ));
            if let Some(replaces) = &building.replaces {
                if let Some(replaces_building) = ruleset.buildings.get(replaces) {
                    text_list.push(FormattedLine::new(
                        format!("Replaces [{}]", replaces),
                        Some(replaces_building.make_link()),
                        None,
                        Some(1),
                        None,
                        None,
                    ));
                }
            }
        }

        // Add cost information
        if building.cost > 0 {
            let mut stats = vec![format!("{}{}", building.cost, Fonts::PRODUCTION)];
            if building.can_be_purchased_with_stat(None, Stat::Gold) {
                stats.push(format!("{}{}", building.get_civilopedia_gold_cost(), Fonts::GOLD));
            }
            text_list.push(FormattedLine::new(
                format!("{{Cost}}: {}", stats.join("/")),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add tech requirements
        if let Some(required_tech) = &building.required_tech {
            text_list.push(FormattedLine::new(
                format!("Required tech: [{}]", required_tech),
                Some(format!("Technology/{}", required_tech)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add building requirements
        if let Some(required_building) = &building.required_building {
            text_list.push(FormattedLine::new(
                format!("Requires [{}] to be built in the city", required_building),
                Some(format!("Building/{}", required_building)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add resource requirements
        if let Some(required_resource) = &building.required_resource {
            text_list.push(FormattedLine::empty());
            if let Some(resource) = ruleset.tile_resources.get(required_resource) {
                text_list.push(FormattedLine::new(
                    required_resource.get_consumes_amount_string(1, resource.is_stockpiled),
                    Some(format!("Resources/{}", required_resource)),
                    Some("#F42".to_string()),
                    None,
                    None,
                    None,
                ));
            }
        }

        // Add stats and uniques section
        let stats = building.clone_stats();
        let percent_stats = building.get_stat_percentage_bonuses(None);
        let specialists = building.new_specialists();
        if !building.uniques.is_empty() || !stats.is_empty() || !percent_stats.is_empty()
            || !building.great_person_points.is_empty() || !specialists.is_empty()
        {
            text_list.push(FormattedLine::empty());
        }

        // Add uniques
        if !building.replacement_text_for_uniques.is_empty() {
            text_list.push(FormattedLine::new(
                building.replacement_text_for_uniques.clone(),
                None,
                None,
                None,
                None,
                None,
            ));
        } else {
            building.uniques_to_civilopedia_text_lines(&mut text_list, true);
        }

        // Add stats
        if !stats.is_empty() {
            text_list.push(FormattedLine::new(
                stats.to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add percentage bonuses
        for (stat, value) in percent_stats {
            if value != 0.0 {
                text_list.push(FormattedLine::new(
                    format!("{:+}% {{{}}}", value.round() as i32, stat.name()),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }

        // Add great person points
        for (great_person_name, value) in &building.great_person_points {
            text_list.push(FormattedLine::new(
                format!("+{} [{}] points", value, great_person_name),
                Some(format!("Unit/{}", great_person_name)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add specialist slots
        if !specialists.is_empty() {
            for (specialist_name, amount) in specialists {
                text_list.push(FormattedLine::new(
                    format!("+{} [{}] slots", amount, specialist_name),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }

        // Add required nearby resources
        if let Some(resources) = &building.required_nearby_improved_resources {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                "Requires at least one of the following resources improved near the city:".to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
            for resource in resources {
                text_list.push(FormattedLine::new(
                    resource.clone(),
                    Some(format!("Resource/{}", resource)),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Add city stats
        if building.city_strength != 0 || building.city_health != 0 || building.maintenance != 0 {
            text_list.push(FormattedLine::empty());
        }
        if building.city_strength != 0 {
            text_list.push(FormattedLine::new(
                format!("{{City strength}} +{}", building.city_strength),
                None,
                None,
                None,
                None,
                None,
            ));
        }
        if building.city_health != 0 {
            text_list.push(FormattedLine::new(
                format!("{{City health}} +{}", building.city_health),
                None,
                None,
                None,
                None,
                None,
            ));
        }
        if building.maintenance != 0 {
            text_list.push(FormattedLine::new(
                format!("{{Maintenance cost}}: {} {{Gold}}", building.maintenance),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add see also section
        let mut see_also = Vec::new();
        for (name, other_building) in &ruleset.buildings {
            if other_building.replaces.as_ref() == Some(&building.name)
                || other_building.uniques.iter().any(|unique| unique.params.contains(&building.name))
            {
                see_also.push(FormattedLine::new(
                    name.clone(),
                    Some(other_building.make_link()),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }
        see_also.extend(Belief::get_civilopedia_text_matching(&building.name, ruleset, false));

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
}

/// Extension trait for Building to add description-related functionality
pub trait BuildingDescriptionExt {
    /// Convert uniques to description lines
    fn uniques_to_description(&self, lines: &mut Vec<String>, filter: impl Fn(&Unique) -> bool);

    /// Get strings for uniques without disablers
    fn get_uniques_strings_without_disablers(&self, filter: Option<impl Fn(&Unique) -> bool>) -> Vec<String>;
}

impl BuildingDescriptionExt for Building {
    fn uniques_to_description(&self, lines: &mut Vec<String>, filter: impl Fn(&Unique) -> bool) {
        let mut tile_bonus_hashmap: HashMap<String, Vec<String>> = HashMap::new();

        for unique in &self.uniques {
            if !filter(unique) && !unique.is_hidden_to_users() {
                if unique.unique_type == UniqueType::StatsFromTiles && unique.params[2] == "in this city" {
                    let stats = unique.params[0].clone();
                    tile_bonus_hashmap.entry(stats).or_default().push(unique.params[1].clone());
                } else {
                    lines.push(unique.get_display_text());
                }
            }
        }

        for (stats, tile_filters) in tile_bonus_hashmap {
            lines.push(format!(
                "[{}] from [{}] tiles in this city",
                stats,
                if tile_filters.len() == 1 {
                    tile_filters[0].clone()
                } else {
                    tile_filters.join(", ")
                }
            ));
        }
    }

    fn get_uniques_strings_without_disablers(&self, filter: Option<impl Fn(&Unique) -> bool>) -> Vec<String> {
        let mut lines = Vec::new();
        self.uniques_to_description(&mut lines, |unique| {
            unique.is_hidden_to_users() || filter.as_ref().map_or(false, |f| f(unique))
        });
        lines
    }
}