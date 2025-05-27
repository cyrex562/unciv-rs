use crate::battle::battle::Battle;
use crate::battle::i_combatant::ICombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::civilization::civilization::Civilization;
use crate::civilization::player_type::PlayerType;
use crate::civilization::popup_alert::PopupAlert;

use std::cmp::min;
use std::collections::HashMap;
use std::f32;
use std::sync::Arc;

/// Module for combat unit capture mechanics
pub mod battle_unit_capture {
    use crate::map::unit;

    use super::*;

    /// Attempts to capture a military unit after combat
    ///
    /// Returns true if the unit was successfully captured
    pub fn try_capture_military_unit(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        attacked_tile: &Tile
    ) -> bool {
        // https://forums.civfanatics.com/threads/prize-ships-for-land-units.650196/
        // https://civilization.fandom.com/wiki/Module:Data/Civ5/GK/Defines
        // There are 3 ways of capturing a unit, we separate them for cleaner code but we also need to ensure a unit isn't captured twice

        let map_unit_defender = match defender.as_any().downcast_ref::<MapUnitCombatant>() {
            Some(combatant) => combatant,
            None => return false,
        };

        let map_unit_attacker = match attacker.as_any().downcast_ref::<MapUnitCombatant>() {
            Some(combatant) => combatant,
            None => return false,
        };

        let state = StateForConditionals::new(
            defender.get_civ_info(),
            Some(map_unit_defender.unit.clone()),
            Some(defender.clone()),
            Some(attacker.clone()),
            Some(attacked_tile.clone()),
            None,
        );

        if defender.has_unique(UniqueType::Uncapturable, &state) {
            return false;
        }

        if !defender.is_defeated() || defender.unit.is_civilian() {
            return false;
        }

        // Due to the way OR operators short-circuit, calling just A() || B() means B isn't called if A is true.
        // Therefore we run all functions before checking if one is true.
        let was_unit_captured = [
            unit_captured_prize_ships_unique(map_unit_attacker, map_unit_defender),
            unit_captured_from_encampment(map_unit_attacker, map_unit_defender, attacked_tile),
            unit_gain_from_defeating_unit(map_unit_attacker, map_unit_defender)
        ].iter().any(|&result| result);

        if !was_unit_captured {
            return false;
        }

        // This is called after takeDamage and so the defeated defender is already destroyed and
        // thus removed from the tile - but MapUnit.destroy() will not clear the unit's currentTile.
        // Therefore placeUnitNearTile _will_ place the new unit exactly where the defender was
        spawn_captured_unit(map_unit_defender, map_unit_attacker)
    }

    /// Checks if a unit can be captured as a prize ship
    fn unit_captured_prize_ships_unique(
        attacker: &MapUnitCombatant,
        defender: &MapUnitCombatant
    ) -> bool {
        let state = StateForConditionals::new(
            attacker.get_civ_info(),
            Some(attacker.unit.clone()),
            Some(defender.clone()),
            None,
            None,
            None,
        );
        
        if attacker.unit.get_matching_uniques(UniqueType::KillUnitCapture)
            .iter()
            .none(|unique| defender.matches_filter(&unique.params[0], &state)) {
            return false;
        }

        let capture_chance = min(
            0.8,
            0.1 + attacker.get_attacking_strength() / defender.get_defending_strength(false) * 0.4
        );

        // Between 0 and 1. Defaults to turn and location-based random to avoid save scumming
        let random = Random::new(
            (attacker.get_civ_info().game_info.turns * defender.get_tile().position.hash_code() as i64) as u64
        );

        random.next_float() <= capture_chance
    }

    /// Checks if a unit can be gained from defeating another unit
    fn unit_gain_from_defeating_unit(
        attacker: &MapUnitCombatant,
        defender: &MapUnitCombatant
    ) -> bool {
        if !attacker.is_melee() {
            return false;
        }

        let mut unit_captured = false;
        let state = StateForConditionals::new(
            attacker.get_civ_info(),
            Some(attacker.clone()),
            Some(defender.clone()),
            None,
            None,
            None,
        );

        for unique in attacker.get_matching_uniques(UniqueType::GainFromDefeatingUnit, &state, true) {
            if defender.unit.matches_filter(&unique.params[0]) {
                attacker.get_civ_info().add_gold(unique.params[1].parse::<i32>().unwrap_or(0));
                unit_captured = true;
            }
        }

        unit_captured
    }

    /// Checks if a unit can be captured from an encampment
    fn unit_captured_from_encampment(
        attacker: &MapUnitCombatant,
        defender: &MapUnitCombatant,
        attacked_tile: &Tile
    ) -> bool {
        if !defender.get_civ_info().is_barbarian {
            return false;
        }

        if attacked_tile.improvement != Some(Constants::BARBARIAN_ENCAMPMENT.to_string()) {
            return false;
        }

        let mut unit_captured = false;
        // German unique - needs to be checked before we try to move to the enemy tile, since the encampment disappears after we move in

        for unique in attacker.get_civ_info().get_matching_uniques(UniqueType::GainFromEncampment) {
            attacker.get_civ_info().add_gold(unique.params[0].parse::<i32>().unwrap_or(0));
            unit_captured = true;
        }

        unit_captured
    }

    /// Places a unit near a tile after being attacked
    ///
    /// Adds a notification to the attacker's civ and returns whether the captured unit could be placed
    fn spawn_captured_unit(
        defender: &MapUnitCombatant,
        attacker: &MapUnitCombatant
    ) -> bool {
        let defender_tile = defender.get_tile();
        let added_unit = match attacker.get_civ_info().units.place_unit_near_tile(
            defender_tile.position,
            defender.get_name()
        ) {
            Some(unit) => unit,
            None => return false,
        };

        added_unit.current_movement = 0.0;
        added_unit.health = 50;

        attacker.get_civ_info().add_notification(
            format!("An enemy [{}] has joined us!", defender.get_name()),
            MapUnitAction::new(added_unit.clone()),
            NotificationCategory::War,
            defender.get_name()
        );

        defender.get_civ_info().add_notification(
            format!("An enemy [{}] has captured our [{}]", attacker.get_name(), defender.get_name()),
            defender.get_tile().position,
            NotificationCategory::War,
            attacker.get_name(),
            NotificationIcon::War,
            defender.get_name()
        );

        let civilian_unit = defender_tile.civilian_unit.clone();
        // placeUnitNearTile might not have spawned the unit in exactly this tile, in which case no capture would have happened on this tile. So we need to do that here.
        if added_unit.get_tile() != defender_tile && civilian_unit.is_some() {
            capture_civilian_unit(attacker, &MapUnitCombatant::new(civilian_unit.unwrap()), false);
        }

        true
    }

    /// Captures a civilian unit
    ///
    /// # Arguments
    ///
    /// * `attacker` - The attacking unit
    /// * `defender` - The defending civilian unit
    /// * `check_defeat` - Whether to check if the defending civilization is defeated
    ///
    /// # Panics
    ///
    /// Panics if the attacker and defender belong to the same civilization
    pub fn capture_civilian_unit(
        attacker: &dyn ICombatant,
        defender: &MapUnitCombatant,
        check_defeat: bool
    ) {
        assert_ne!(attacker.get_civ_info(), defender.get_civ_info(), "Can't capture our own unit!");

        // need to save this because if the unit is captured its owner will be overwritten
        let defender_civ = defender.get_civ_info();

        let captured_unit = defender.unit.clone();
        // Stop current action
        captured_unit.action = None;
        captured_unit.automated = false;

        let captured_unit_tile = captured_unit.get_tile();
        let original_owner = if captured_unit.original_owner.is_some() {
            Some(captured_unit.civ.game_info.get_civilization(captured_unit.original_owner.unwrap()))
        } else {
            None
        };

        let mut was_destroyed_instead = false;

        // Uncapturable units are destroyed
        if defender.unit.has_unique(UniqueType::Uncapturable) {
            captured_unit.destroy();
            was_destroyed_instead = true;
        }
        // City states can never capture settlers at all
        else if captured_unit.has_unique(UniqueType::FoundCity) && attacker.get_civ_info().is_city_state {
            captured_unit.destroy();
            was_destroyed_instead = true;
        }
        // Is it our old unit?
        else if attacker.get_civ_info() == original_owner.as_ref().map(|c| c.as_ref()) {
            // Then it is recaptured without converting settlers to workers
            captured_unit.captured_by(attacker.get_civ_info());
        }
        // Return captured civilian to its original owner?
        else if defender.get_civ_info().is_barbarian
            && original_owner.is_some()
            && !original_owner.as_ref().unwrap().is_barbarian
            && attacker.get_civ_info() != original_owner.as_ref().unwrap()
            && attacker.get_civ_info().knows(original_owner.as_ref().unwrap())
            && original_owner.as_ref().unwrap().is_alive()
            && !attacker.get_civ_info().is_at_war_with(original_owner.as_ref().unwrap())
            && attacker.get_civ_info().player_type == PlayerType::Human // Only humans get the choice
        {
            captured_unit.captured_by(attacker.get_civ_info());
            attacker.get_civ_info().popup_alerts.push(
                PopupAlert::new(
                    AlertType::RecapturedCivilian,
                    captured_unit.current_tile.position.to_string()
                )
            );
        }
        else {
            if capture_or_convert_to_worker(&mut captured_unit, attacker.get_civ_info().as_ref()).is_none() {
                was_destroyed_instead = true;
            }
        }

        if !was_destroyed_instead {
            defender_civ.add_notification(
                format!("An enemy [{}] has captured our [{}]", attacker.get_name(), defender.get_name()),
                defender.get_tile().position,
                NotificationCategory::War,
                attacker.get_name(),
                NotificationIcon::War,
                defender.get_name()
            );
        } else {
            defender_civ.add_notification(
                format!("An enemy [{}] has destroyed our [{}]", attacker.get_name(), defender.get_name()),
                defender.get_tile().position,
                NotificationCategory::War,
                attacker.get_name(),
                NotificationIcon::War,
                defender.get_name()
            );
            Battle::trigger_defeat_uniques(defender, attacker, &captured_unit_tile);
        }

        if check_defeat {
            Battle::destroy_if_defeated(defender_civ, attacker.get_civ_info());
        }

        captured_unit.update_visible_tiles();
    }

    /// Capture wrapper that also implements the rule that non-barbarians get a Worker as replacement for a captured Settler.
    ///
    /// # Returns
    ///
    /// Position the captured unit is in afterwards - can rarely be a different tile if the unit is no longer allowed where it originated.
    /// Returns `None` if there is no Worker replacement for a Settler in the ruleset or placeUnitNearTile couldn't place it.
    pub fn capture_or_convert_to_worker(
        captured_unit: &mut MapUnit,
        capturing_civ: &Civilization
    ) -> Option<(i32, i32)> {
        // Captured settlers are converted to workers unless captured by barbarians (so they can be returned later).
        if !captured_unit.has_unique(UniqueType::FoundCity) || capturing_civ.is_barbarian {
            captured_unit.captured_by(capturing_civ);
            return Some(captured_unit.current_tile.position); // if capturedBy has moved the unit, this is updated
        }

        captured_unit.destroy();
        // This is so that future checks which check if a unit has been captured are caught give the right answer
        //  For example, in postBattleMoveToAttackedTile
        captured_unit.civ = capturing_civ.clone();
        captured_unit.cache.state = Some(StateForConditionals::new(captured_unit));

        let worker_type_unit = capturing_civ.game_info.ruleset.units.values()
            .find(|unit| {
                unit.is_civilian()
                && unit.get_matching_uniques(UniqueType::BuildImprovements)
                    .iter()
                    .any(|unique| unique.params[0] == "Land")
            });

        match worker_type_unit {
            Some(unit) => {
                capturing_civ.units.place_unit_near_tile(
                    captured_unit.current_tile.position,
                    unit.name.clone(),
                    Some(captured_unit.id)
                ).map(|unit| unit.current_tile.position)
            },
            None => None
        }
    }
}