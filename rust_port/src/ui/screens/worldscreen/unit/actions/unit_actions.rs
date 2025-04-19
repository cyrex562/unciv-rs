// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/actions/UnitActions.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use crate::game::civilization::diplomacy::DiplomaticModifiers;
use crate::game::unit::MapUnit;
use crate::game::map::tile::Tile;
use crate::game::unit::{UnitAction, UnitActionType};
use crate::game::ruleset::unique::UniqueType;
use crate::ui::screens::worldscreen::WorldScreen;

/// Manages unit actions and their execution
pub struct UnitActions;

impl UnitActions {
    /// Invokes a unit action of the specified type
    /// Returns whether the action was invoked
    pub fn invoke_unit_action(unit: &mut MapUnit, unit_action_type: UnitActionType) -> bool {
        let actions = Self::get_unit_actions(unit, unit_action_type);
        if let Some(action) = actions.into_iter().find(|a| a.action.is_some()) {
            if let Some(action_fn) = action.action {
                action_fn();
                return true;
            }
        }
        false
    }

    /// Gets all currently possible unit actions
    pub fn get_unit_actions(unit: &MapUnit) -> Vec<UnitAction> {
        let mut actions = Vec::new();
        let tile = unit.get_tile();

        // Actions standardized with a directly callable invoke_unit_action
        for get_actions_fn in Self::action_type_to_functions().values() {
            actions.extend(get_actions_fn(unit, &tile));
        }

        // Actions not migrated to action_type_to_functions
        Self::add_unmapped_unit_actions(unit, &mut actions);

        actions
    }

    /// Gets unit actions for a specific action type
    pub fn get_unit_actions_by_type(unit: &MapUnit, unit_action_type: UnitActionType) -> Vec<UnitAction> {
        let tile = unit.get_tile();
        let mut actions = Vec::new();

        if let Some(get_actions_fn) = Self::action_type_to_functions().get(&unit_action_type) {
            actions.extend(get_actions_fn(unit, &tile));
        } else {
            Self::add_unmapped_unit_actions(unit, &mut actions);
            actions.retain(|action| action.action_type == unit_action_type);
        }

        actions
    }

    /// Gets the mapping of action types to their handler functions
    fn action_type_to_functions() -> HashMap<UnitActionType, fn(&MapUnit, &Tile) -> Vec<UnitAction>> {
        let mut map = HashMap::new();

        // Determined by unit uniques
        map.insert(UnitActionType::Transform, UnitActionsFromUniques::get_transform_actions);
        map.insert(UnitActionType::Paradrop, UnitActionsFromUniques::get_paradrop_actions);
        map.insert(UnitActionType::AirSweep, UnitActionsFromUniques::get_air_sweep_actions);
        map.insert(UnitActionType::SetUp, UnitActionsFromUniques::get_setup_actions);
        map.insert(UnitActionType::Guard, UnitActionsFromUniques::get_guard_actions);
        map.insert(UnitActionType::FoundCity, UnitActionsFromUniques::get_found_city_actions);
        map.insert(UnitActionType::ConstructImprovement, UnitActionsFromUniques::get_building_improvements_actions);
        map.insert(UnitActionType::ConnectRoad, UnitActionsFromUniques::get_connect_road_actions);
        map.insert(UnitActionType::Repair, UnitActionsFromUniques::get_repair_actions);
        map.insert(UnitActionType::HurryResearch, UnitActionsGreatPerson::get_hurry_research_actions);
        map.insert(UnitActionType::HurryPolicy, UnitActionsGreatPerson::get_hurry_policy_actions);
        map.insert(UnitActionType::HurryWonder, UnitActionsGreatPerson::get_hurry_wonder_actions);
        map.insert(UnitActionType::HurryBuilding, UnitActionsGreatPerson::get_hurry_building_actions);
        map.insert(UnitActionType::ConductTradeMission, UnitActionsGreatPerson::get_conduct_trade_mission_actions);
        map.insert(UnitActionType::FoundReligion, UnitActionsReligion::get_found_religion_actions);
        map.insert(UnitActionType::EnhanceReligion, UnitActionsReligion::get_enhance_religion_actions);
        map.insert(UnitActionType::CreateImprovement, UnitActionsFromUniques::get_improvement_creation_actions);
        map.insert(UnitActionType::SpreadReligion, UnitActionsReligion::get_spread_religion_actions);
        map.insert(UnitActionType::RemoveHeresy, UnitActionsReligion::get_remove_heresy_actions);
        map.insert(UnitActionType::TriggerUnique, UnitActionsFromUniques::get_trigger_unique_actions);
        map.insert(UnitActionType::AddInCapital, UnitActionsFromUniques::get_add_in_capital_actions);
        map.insert(UnitActionType::GiftUnit, UnitActions::get_gift_actions);

        map
    }

    /// Gets the preferred page to display a unit action
    pub fn get_action_default_page(unit: &MapUnit, unit_action_type: UnitActionType) -> i32 {
        if let Some(page_getter) = Self::action_type_to_page_getter().get(&unit_action_type) {
            page_getter(unit)
        } else {
            unit_action_type.default_page()
        }
    }

    /// Gets the mapping of action types to their page getter functions
    fn action_type_to_page_getter() -> HashMap<UnitActionType, fn(&MapUnit) -> i32> {
        let mut map = HashMap::new();

        map.insert(UnitActionType::Automate, |unit: &MapUnit| {
            if unit.cache.has_unique_to_build_improvements || unit.has_unique(UniqueType::AutomationPrimaryAction) {
                0
            } else {
                1
            }
        });

        map.insert(UnitActionType::Fortify, |unit: &MapUnit| {
            if unit.is_fortifying_until_healed() || (unit.health < 100 && !(unit.is_fortified() && !unit.is_action_until_healed())) {
                1
            } else {
                0
            }
        });

        map.insert(UnitActionType::FortifyUntilHealed, |unit: &MapUnit| {
            if unit.is_fortified() && !unit.is_action_until_healed() {
                1
            } else {
                0
            }
        });

        map.insert(UnitActionType::Sleep, |unit: &MapUnit| {
            if unit.is_sleeping_until_healed() || (unit.health < 100 && !(unit.is_sleeping() && !unit.is_action_until_healed())) {
                1
            } else {
                0
            }
        });

        map.insert(UnitActionType::SleepUntilHealed, |unit: &MapUnit| {
            if unit.is_sleeping() && !unit.is_action_until_healed() {
                1
            } else {
                0
            }
        });

        map.insert(UnitActionType::Explore, |unit: &MapUnit| {
            if unit.is_civilian() {
                1
            } else {
                0
            }
        });

        map
    }

    /// Adds unmapped unit actions to the provided vector
    fn add_unmapped_unit_actions(unit: &MapUnit, actions: &mut Vec<UnitAction>) {
        let tile = unit.get_tile();

        // General actions
        Self::add_automate_actions(unit, actions);

        if unit.is_moving() {
            actions.push(UnitAction::new(
                UnitActionType::StopMovement,
                20.0,
                None,
                Some(Box::new(move || { unit.action = None; }))
            ));
        }

        if unit.is_exploring() {
            actions.push(UnitAction::new(
                UnitActionType::StopExploration,
                20.0,
                None,
                Some(Box::new(move || { unit.action = None; }))
            ));
        }

        if unit.is_automated() {
            actions.push(UnitAction::new(
                UnitActionType::StopAutomation,
                10.0,
                None,
                Some(Box::new(move || {
                    unit.action = None;
                    unit.automated = false;
                }))
            ));
        }

        Self::add_promote_actions(unit, actions);
        Self::add_exploration_actions(unit, actions);
        Self::add_fortify_actions(unit, actions);
        Self::add_sleep_actions(unit, &tile, actions);
        Self::add_escort_action(unit, actions);
        Self::add_swap_action(unit, actions);
        Self::add_disband_action(unit, actions);
    }

    // TODO: Implement remaining helper methods for adding specific action types
    // These would include methods like add_automate_actions, add_promote_actions, etc.
}