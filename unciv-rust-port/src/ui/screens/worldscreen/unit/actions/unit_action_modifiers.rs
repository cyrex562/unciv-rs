// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/actions/UnitActionModifiers.kt

use std::rc::Rc;
use std::cell::RefCell;
use crate::game::unit::MapUnit;
use crate::game::ruleset::unique::{Unique, UniqueType};
use crate::game::stats::Stats;
use crate::utils::translations::{remove_conditionals, tr};
use crate::ui::components::fonts::{FontRulesetIcons, Fonts};

/// Modifiers and utilities for unit actions
pub struct UnitActionModifiers;

impl UnitActionModifiers {
    /// Checks if a unit can use a specific action unique
    pub fn can_use(unit: &MapUnit, action_unique: &Unique) -> bool {
        let usages_left = Self::usages_left(unit, action_unique);
        usages_left.is_none() || usages_left.unwrap() > 0
    }

    /// Gets usable unit action uniques for a given type
    pub fn get_usable_unit_action_uniques(unit: &MapUnit, action_unique_type: UniqueType) -> Vec<Unique> {
        unit.get_matching_uniques(action_unique_type)
            .into_iter()
            .filter(|unique| !unique.has_modifier(UniqueType::UnitActionExtraLimitedTimes))
            .filter(|unique| Self::can_use(unit, unique))
            .collect()
    }

    /// Gets the movement points required to use an action
    fn get_movement_points_to_use(unit: &MapUnit, action_unique: &Unique, default_all_movement: bool) -> i32 {
        if action_unique.has_modifier(UniqueType::UnitActionMovementCostAll) {
            return unit.get_movement_points();
        }

        for modifier in action_unique.get_modifiers(UniqueType::UnitActionMovementCost) {
            return modifier.params[0].parse().unwrap_or(1);
        }

        if default_all_movement {
            unit.get_movement_points()
        } else {
            1
        }
    }

    /// Gets the remaining usages of an action
    pub fn usages_left(unit: &MapUnit, action_unique: &Unique) -> Option<i32> {
        for modifier in action_unique.get_modifiers(UniqueType::UnitActionExtraLimitedTimes) {
            let max_uses = modifier.params[0].parse::<i32>().unwrap_or(0);
            let used = unit.action_uses.get(&action_unique.id).copied().unwrap_or(0);
            return Some(max_uses - used);
        }
        None
    }

    /// Checks if side effects can be activated for an action
    pub fn can_activate_side_effects(unit: &MapUnit, action_unique: &Unique) -> bool {
        let movement_cost = Self::get_movement_points_to_use(unit, action_unique, false);
        unit.get_movement_points() >= movement_cost
    }

    /// Activates side effects for an action
    pub fn activate_side_effects(unit: &mut MapUnit, action_unique: &Unique) {
        let movement_cost = Self::get_movement_points_to_use(unit, action_unique, false);
        unit.use_movement_points(movement_cost as f32);

        // Track usage if limited
        if action_unique.has_modifier(UniqueType::UnitActionExtraLimitedTimes) {
            let current = unit.action_uses.get(&action_unique.id).copied().unwrap_or(0);
            unit.action_uses.insert(action_unique.id.clone(), current + 1);
        }

        // Handle unit consumption
        if action_unique.has_modifier(UniqueType::UnitActionConsumeUnit) {
            unit.destroy();
        }
    }

    /// Gets the action text with side effects
    pub fn action_text_with_side_effects(base_text: &str, unique: &Unique, unit: &MapUnit) -> String {
        let mut text = base_text.to_string();

        // Add movement cost
        let movement_cost = Self::get_movement_points_to_use(unit, unique, false);
        if movement_cost > 0 {
            text.push_str(&format!(" ({}⚡)", movement_cost));
        }

        // Add remaining uses
        if let Some(uses_left) = Self::usages_left(unit, unique) {
            text.push_str(&format!(" ({} left)", uses_left));
        }

        text
    }
}