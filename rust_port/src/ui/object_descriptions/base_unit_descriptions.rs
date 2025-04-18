use std::collections::HashMap;
use ggez::graphics::{self, Color};

use crate::models::ruleset::{Ruleset, BaseUnit};
use crate::models::ruleset::unique::{Unique, UniqueType, StateForConditionals};
use crate::models::stats::Stat;
use crate::models::city::City;
use crate::ui::components::fonts::Fonts;
use crate::ui::components::extensions::ConsumesAmountExt;
use crate::ui::screens::civilopedia_screen::FormattedLine;

/// Module for generating descriptions of units in the game
pub struct BaseUnitDescriptions;

impl BaseUnitDescriptions {
    /// Generate short description as comma-separated string for Technology description "Units enabled" and GreatPersonPickerScreen
    pub fn get_short_description(
        base_unit: &BaseUnit,
        unique_exclusion_filter: impl Fn(&Unique) -> bool,
    ) -> String {
        let mut info_list = Vec::new();

        if base_unit.strength != 0 {
            info_list.push(format!("{}{}", base_unit.strength, Fonts::STRENGTH));
        }

        if base_unit.ranged_strength != 0 {
            info_list.push(format!("{}{}", base_unit.ranged_strength, Fonts::RANGED_STRENGTH));
        }

        if base_unit.movement != 2 {
            info_list.push(format!("{}{}", base_unit.movement, Fonts::MOVEMENT));
        }

        for promotion in &base_unit.promotions {
            info_list.push(promotion.to_string());
        }

        if !base_unit.replacement_text_for_uniques.is_empty() {
            info_list.push(base_unit.replacement_text_for_uniques.clone());
        } else {
            base_unit.uniques_to_description(&mut info_list, &unique_exclusion_filter);
        }

        info_list.join(", ")
    }

    /// Generate description as multi-line string for CityScreen addSelectedConstructionTable
    pub fn get_description(base_unit: &BaseUnit, city: &City) -> String {
        let mut lines = Vec::new();
        let available_resources = city.civ.get_civ_resources_by_name();

        // Add resource requirements
        for (resource_name, amount) in base_unit.get_resource_requirements_per_turn(&city.civ.state) {
            let available = available_resources.get(&resource_name).copied().unwrap_or(0);
            if let Some(resource) = base_unit.ruleset.tile_resources.get(&resource_name) {
                let consumes_string = resource_name.get_consumes_amount_string(amount, resource.is_stockpiled);
                lines.push(format!("{} ({} available)", consumes_string, available));
            }
        }

        // Add strength and movement info
        let mut strength_line = String::new();
        if base_unit.strength != 0 {
            strength_line.push_str(&format!("{}{}, ", base_unit.strength, Fonts::STRENGTH));
            if base_unit.ranged_strength != 0 {
                strength_line.push_str(&format!(
                    "{}{}, {}{}, ",
                    base_unit.ranged_strength,
                    Fonts::RANGED_STRENGTH,
                    base_unit.range,
                    Fonts::RANGE
                ));
            }
        }
        strength_line.push_str(&format!("{}{}", base_unit.movement, Fonts::MOVEMENT));
        lines.push(strength_line);

        // Add uniques
        if !base_unit.replacement_text_for_uniques.is_empty() {
            lines.push(base_unit.replacement_text_for_uniques.clone());
        } else {
            base_unit.uniques_to_description(&mut lines, |unique| {
                unique.unique_type == UniqueType::Unbuildable
                    || unique.unique_type == UniqueType::ConsumesResources
            });
        }

        // Add promotions
        if !base_unit.promotions.is_empty() {
            let prefix = if base_unit.promotions.len() == 1 {
                "Free promotion: "
            } else {
                "Free promotions: "
            };
            lines.push(format!(
                "{}{}",
                prefix,
                base_unit.promotions.join(", ")
            ));
        }

        lines.join("\n")
    }

    /// Generate civilopedia text lines for a unit
    pub fn get_civilopedia_text_lines(base_unit: &BaseUnit, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut text_list = Vec::new();

        // Add pixel unit image if appropriate
        Self::add_pixel_unit_image(base_unit, &mut text_list);

        // Add unit type
        if let Some(unit_type) = ruleset.unit_types.get(&base_unit.unit_type) {
            text_list.push(FormattedLine::new(
                format!("Unit type: {}", base_unit.unit_type),
                Some(unit_type.make_link()),
                None,
                None,
                None,
                None,
            ));
        }

        // Add stats
        let mut stats = Vec::new();
        if base_unit.strength != 0 {
            stats.push(format!("{}{}", base_unit.strength, Fonts::STRENGTH));
        }
        if base_unit.ranged_strength != 0 {
            stats.push(format!("{}{}", base_unit.ranged_strength, Fonts::RANGED_STRENGTH));
            stats.push(format!("{}{}", base_unit.range, Fonts::RANGE));
        }
        if base_unit.movement != 0 && !ruleset.unit_types.get(&base_unit.unit_type).map_or(false, |ut| ut.is_air_unit()) {
            stats.push(format!("{}{}", base_unit.movement, Fonts::MOVEMENT));
        }
        if !stats.is_empty() {
            text_list.push(FormattedLine::new(
                stats.join(", "),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add cost information
        if base_unit.cost > 0 {
            let mut cost_stats = Vec::new();
            cost_stats.push(format!("{}{}", base_unit.cost, Fonts::PRODUCTION));
            if base_unit.can_be_purchased_with_stat(None, Stat::Gold) {
                cost_stats.push(format!("{}{}", base_unit.get_civilopedia_gold_cost(), Fonts::GOLD));
            }
            text_list.push(FormattedLine::new(
                format!("Cost: {}", cost_stats.join("/")),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add intercept range if applicable
        if base_unit.intercept_range > 0 {
            text_list.push(FormattedLine::new(
                format!("Air Intercept Range: [{}]", base_unit.intercept_range),
                None,
                None,
                None,
                None,
                None,
            ));
        }

        // Add uniques
        if !base_unit.replacement_text_for_uniques.is_empty() {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                base_unit.replacement_text_for_uniques.clone(),
                None,
                None,
                None,
                None,
                None,
            ));
        } else {
            base_unit.uniques_to_civilopedia_text_lines(&mut text_list, true, true);
        }

        // Add required resource
        if let Some(required_resource) = &base_unit.required_resource {
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

        // Add unique to/replaces information
        if let Some(unique_to) = &base_unit.unique_to {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                format!("Unique to [{}]", unique_to),
                Some(format!("Nation/{}", unique_to)),
                None,
                None,
                None,
                None,
            ));
            if let Some(replaces) = &base_unit.replaces {
                text_list.push(FormattedLine::new(
                    format!("Replaces [{}]", replaces),
                    Some(format!("Unit/{}", replaces)),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }

        // Add tech requirements and upgrade paths
        if base_unit.required_tech.is_some() || base_unit.upgrades_to.is_some() || base_unit.obsolete_tech.is_some() {
            text_list.push(FormattedLine::empty());
        }
        if let Some(required_tech) = &base_unit.required_tech {
            text_list.push(FormattedLine::new(
                format!("Required tech: [{}]", required_tech),
                Some(format!("Technology/{}", required_tech)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add upgrade from information
        let can_upgrade_from: Vec<_> = ruleset.units
            .iter()
            .filter(|(_, unit)| {
                (unit.upgrades_to.as_ref() == Some(&base_unit.name)
                    || unit.upgrades_to.as_ref() == base_unit.replaces.as_ref())
                    && (unit.unique_to.is_none() || unit.unique_to == base_unit.unique_to)
            })
            .map(|(name, _)| name.clone())
            .collect();

        if !can_upgrade_from.is_empty() {
            if can_upgrade_from.len() == 1 {
                text_list.push(FormattedLine::new(
                    format!("Can upgrade from [{}]", can_upgrade_from[0]),
                    Some(format!("Unit/{}", can_upgrade_from[0])),
                    None,
                    None,
                    None,
                    None,
                ));
            } else {
                text_list.push(FormattedLine::empty());
                text_list.push(FormattedLine::new(
                    "Can upgrade from:".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
                for unit_name in can_upgrade_from.iter().sorted() {
                    text_list.push(FormattedLine::new(
                        unit_name.clone(),
                        Some(format!("Unit/{}", unit_name)),
                        None,
                        Some(2),
                        None,
                        None,
                    ));
                }
                text_list.push(FormattedLine::empty());
            }
        }

        // Add upgrades to information
        if let Some(upgrades_to) = &base_unit.upgrades_to {
            text_list.push(FormattedLine::new(
                format!("Upgrades to [{}]", upgrades_to),
                Some(format!("Unit/{}", upgrades_to)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add obsolete tech information
        if let Some(obsolete_tech) = &base_unit.obsolete_tech {
            text_list.push(FormattedLine::new(
                format!("Obsolete with [{}]", obsolete_tech),
                Some(format!("Technology/{}", obsolete_tech)),
                None,
                None,
                None,
                None,
            ));
        }

        // Add promotions
        if !base_unit.promotions.is_empty() {
            text_list.push(FormattedLine::empty());
            for (index, promotion) in base_unit.promotions.iter().enumerate() {
                let prefix = match (base_unit.promotions.len(), index) {
                    (1, _) => "Free promotion: ",
                    (_, 0) => "Free promotions: ",
                    _ => "",
                };
                let suffix = if index == base_unit.promotions.len() - 1 || base_unit.promotions.len() == 1 {
                    ""
                } else {
                    ","
                };
                text_list.push(FormattedLine::new(
                    format!("{}{}{}", prefix, promotion, suffix),
                    Some(format!("Promotions/{}", promotion)),
                    None,
                    if index == 0 { None } else { Some(1) },
                    None,
                    None,
                ));
            }
        }

        // Add see also section
        let mut see_also = Vec::new();
        for (other, unit) in &ruleset.units {
            if unit.replaces.as_ref() == Some(&base_unit.name) || base_unit.uniques.contains(&format!("[{}]", base_unit.name)) {
                see_also.push(FormattedLine::new(
                    other.clone(),
                    Some(format!("Unit/{}", other)),
                    None,
                    Some(1),
                    None,
                    None,
                ));
            }
        }
        if !see_also.is_empty() {
            text_list.push(FormattedLine::empty());
            text_list.push(FormattedLine::new(
                "See also:".to_string(),
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

    /// Add pixel unit art to the civilopedia text lines if appropriate
    fn add_pixel_unit_image(base_unit: &BaseUnit, text_list: &mut Vec<FormattedLine>) {
        // Skip if unit already has extra images in civilopedia text
        if base_unit.civilopedia_text.iter().any(|line| !line.extra_image.is_empty()) {
            return;
        }

        // Get settings
        let settings = crate::GUI::get_settings();
        if settings.unit_set.is_empty() || settings.pedia_unit_art_size < 1.0 {
            return;
        }

        // Check if unit art exists
        let image_name = format!("TileSets/{}/Units/{}", settings.unit_set, base_unit.name);
        if !crate::ui::images::image_getter::IMAGE_GETTER.image_exists(&image_name) {
            return;
        }

        // Add the image
        text_list.push(FormattedLine::new(
            String::new(),
            None,
            None,
            None,
            Some(image_name),
            Some(settings.pedia_unit_art_size),
        ));
        text_list.push(FormattedLine::new(
            String::new(),
            None,
            Some("GRAY".to_string()),
            None,
            None,
            None,
        ));
    }

    /// Get differences between two units for upgrade information
    pub fn get_differences(base_unit: &BaseUnit, other_unit: &BaseUnit) -> Vec<String> {
        let mut differences = Vec::new();

        // Compare strength
        if base_unit.strength != other_unit.strength {
            differences.push(format!(
                "{}{} → {}{}",
                base_unit.strength,
                Fonts::STRENGTH,
                other_unit.strength,
                Fonts::STRENGTH
            ));
        }

        // Compare ranged strength
        if base_unit.ranged_strength != other_unit.ranged_strength {
            differences.push(format!(
                "{}{} → {}{}",
                base_unit.ranged_strength,
                Fonts::RANGED_STRENGTH,
                other_unit.ranged_strength,
                Fonts::RANGED_STRENGTH
            ));
        }

        // Compare range
        if base_unit.range != other_unit.range {
            differences.push(format!(
                "{}{} → {}{}",
                base_unit.range,
                Fonts::RANGE,
                other_unit.range,
                Fonts::RANGE
            ));
        }

        // Compare movement
        if base_unit.movement != other_unit.movement {
            differences.push(format!(
                "{}{} → {}{}",
                base_unit.movement,
                Fonts::MOVEMENT,
                other_unit.movement,
                Fonts::MOVEMENT
            ));
        }

        // Compare intercept range
        if base_unit.intercept_range != other_unit.intercept_range {
            differences.push(format!(
                "Intercept range {} → {}",
                base_unit.intercept_range,
                other_unit.intercept_range
            ));
        }

        // Compare resource requirements
        let base_resources = base_unit.get_resource_requirements_per_turn(&StateForConditionals::default());
        let other_resources = other_unit.get_resource_requirements_per_turn(&StateForConditionals::default());

        for (resource, amount) in &base_resources {
            if let Some(other_amount) = other_resources.get(resource) {
                if amount != other_amount {
                    differences.push(format!(
                        "{} {} → {} {}",
                        amount, resource, other_amount, resource
                    ));
                }
            } else {
                differences.push(format!("No longer requires {} {}", amount, resource));
            }
        }

        for (resource, amount) in &other_resources {
            if !base_resources.contains_key(resource) {
                differences.push(format!("Requires {} {}", amount, resource));
            }
        }

        // Compare uniques
        let base_uniques: Vec<_> = base_unit.uniques
            .iter()
            .filter(|unique| !unique.is_hidden_to_users())
            .collect();
        let other_uniques: Vec<_> = other_unit.uniques
            .iter()
            .filter(|unique| !unique.is_hidden_to_users())
            .collect();

        for unique in base_uniques {
            if !other_uniques.contains(&unique) {
                differences.push(format!("Lost unique: {}", unique));
            }
        }

        for unique in other_uniques {
            if !base_uniques.contains(&unique) {
                differences.push(format!("Gained unique: {}", unique));
            }
        }

        differences
    }

    /// Get upgrade information table for a unit
    pub fn get_upgrade_info_table(base_unit: &BaseUnit, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        if let Some(upgrades_to) = &base_unit.upgrades_to {
            if let Some(upgraded_unit) = ruleset.units.get(upgrades_to) {
                let differences = Self::get_differences(base_unit, upgraded_unit);
                if !differences.is_empty() {
                    lines.push(FormattedLine::new(
                        "When upgrading:".to_string(),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ));
                    for difference in differences {
                        lines.push(FormattedLine::new(
                            difference,
                            None,
                            None,
                            Some(1),
                            None,
                            None,
                        ));
                    }
                }

                let upgrade_cost = upgraded_unit.get_upgrade_cost(base_unit);
                lines.push(FormattedLine::new(
                    format!("Upgrade cost: {}{}", upgrade_cost, Fonts::GOLD),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }

        lines
    }
}

/// Extension trait for BaseUnit to add description-related functionality
pub trait BaseUnitDescriptionExt {
    /// Convert uniques to description lines
    fn uniques_to_description(&self, lines: &mut Vec<String>, filter: impl Fn(&Unique) -> bool);
}

impl BaseUnitDescriptionExt for BaseUnit {
    fn uniques_to_description(&self, lines: &mut Vec<String>, filter: impl Fn(&Unique) -> bool) {
        for unique in &self.uniques {
            if !filter(unique) && !unique.is_hidden_to_users() {
                lines.push(unique.get_display_text());
            }
        }
    }
}

/// Extension trait for UnitType to add civilopedia text generation
pub trait UnitTypeCivilopediaExt {
    /// Get civilopedia text lines for a unit type
    fn get_unit_type_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine>;
}

impl UnitTypeCivilopediaExt for UnitType {
    fn get_unit_type_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        if self.name.starts_with("Domain: ") {
            self.get_domain_lines(ruleset)
        } else {
            self.get_unit_type_lines(ruleset)
        }
    }
}

impl UnitType {
    fn get_domain_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut lines = Vec::new();
        lines.push(FormattedLine::new(
            "Unit types:".to_string(),
            None,
            None,
            Some(4),
            None,
            None,
        ));

        let my_movement_type = self.get_movement_type();
        for unit_type in ruleset.unit_types.values() {
            if unit_type.get_movement_type() != my_movement_type {
                continue;
            }
            if !unit_type.is_used(ruleset) {
                continue;
            }
            lines.push(FormattedLine::new(
                unit_type.name.clone(),
                Some(unit_type.make_link()),
                None,
                None,
                None,
                None,
            ));
        }

        lines
    }

    fn get_unit_type_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut lines = Vec::new();

        // Add domain information
        if let Some(movement_type) = self.get_movement_type() {
            let color = match movement_type {
                UnitMovementType::Land => "#ffc080",
                UnitMovementType::Water => "#80d0ff",
                UnitMovementType::Air => "#e0e0ff",
            };
            lines.push(FormattedLine::new(
                format!("Domain: [{}]", movement_type.name),
                Some(format!("UnitType/Domain: [{}]", movement_type.name)),
                Some(color.to_string()),
                None,
                None,
                None,
            ));
            lines.push(FormattedLine::separator());
        }

        // Add units section
        lines.push(FormattedLine::new(
            "Units:".to_string(),
            None,
            None,
            Some(4),
            None,
            None,
        ));
        for unit in ruleset.units.values() {
            if unit.unit_type != self.name {
                continue;
            }
            lines.push(FormattedLine::new(
                unit.name.clone(),
                Some(unit.make_link()),
                None,
                None,
                None,
                None,
            ));
        }

        // Add promotions section
        let relevant_promotions: Vec<_> = ruleset.unit_promotions
            .values()
            .filter(|promotion| promotion.unit_types.contains(&self.name))
            .collect();

        if !relevant_promotions.is_empty() {
            lines.push(FormattedLine::new(
                "Promotions".to_string(),
                None,
                None,
                Some(4),
                None,
                None,
            ));
            for promotion in relevant_promotions {
                lines.push(FormattedLine::new(
                    promotion.name.clone(),
                    Some(promotion.make_link()),
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }

        // Add uniques
        lines.extend(self.uniques_to_civilopedia_text_lines(true));

        lines
    }
}