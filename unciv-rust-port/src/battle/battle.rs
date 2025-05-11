use crate::battle::air_interception::AirInterception;
use crate::battle::attackable_tile::AttackableTile;
use crate::battle::battle_damage::battle_damage;
use crate::battle::battle_unit_capture::battle_unit_capture;
use crate::battle::i_combatant::ICombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::battle::nuke::Nuke;
use crate::civilization::civilization::Civilization;
use crate::civilization::location_action::LocationAction;
use crate::civilization::map_unit_action::MapUnitAction;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::civilization::popup_alert::AlertType;
use crate::civilization::popup_alert::PopupAlert;
use crate::civilization::promote_unit_action::PromoteUnitAction;
use crate::constants::Constants;
use crate::map::tile::Tile;
use crate::models::stats::stat::Stat;
use crate::models::stats::stats::Stats;
use crate::models::stats::sub_stat::SubStat;
use crate::models::unique::state_for_conditionals::StateForConditionals;
use crate::models::unique::unique::Unique;
use crate::models::unique::unique_trigger_activation::UniqueTriggerActivation;
use crate::models::unique::unique_type::UniqueType;
use crate::models::unit_action_type::UnitActionType;
use crate::ui::unit_actions_pillage::UnitActionsPillage;
use crate::ui::unit_movement_memory_type::UnitMovementMemoryType;
use crate::utils::debug;
use crate::utils::random::Random;
use std::collections::HashMap;
use std::f32;
use std::sync::Arc;

/// Damage calculations according to civ v wiki and https://steamcommunity.com/sharedfiles/filedetails/?id=170194443
pub struct Battle;

/// Holder for battle result - actual damage.
///
/// # Fields
///
/// * `attacker_dealt` - Damage done by attacker to defender
/// * `defender_dealt` - Damage done by defender to attacker
#[derive(Debug, Clone, Copy)]
pub struct DamageDealt {
    pub attacker_dealt: i32,
    pub defender_dealt: i32,
}

impl DamageDealt {
    /// Creates a new DamageDealt with the specified values
    pub fn new(attacker_dealt: i32, defender_dealt: i32) -> Self {
        DamageDealt {
            attacker_dealt,
            defender_dealt,
        }
    }

    /// Returns a DamageDealt with no damage
    pub fn none() -> Self {
        DamageDealt {
            attacker_dealt: 0,
            defender_dealt: 0,
        }
    }

    /// Adds two DamageDealt values together
    pub fn add(&self, other: &DamageDealt) -> DamageDealt {
        DamageDealt {
            attacker_dealt: self.attacker_dealt + other.attacker_dealt,
            defender_dealt: self.defender_dealt + other.defender_dealt,
        }
    }
}

impl Battle {
    /// Moves [attacker] to [attackable_tile], handles siege setup then attacks if still possible
    /// (by calling [attack] or [Nuke.NUKE]). Does _not_ play the attack sound!
    ///
    /// Currently not used by UI, only by automation via [BattleHelper.tryAttackNearbyEnemy]
    pub fn move_and_attack(attacker: &dyn ICombatant, attackable_tile: &AttackableTile) {
        if !Self::move_preparing_attack(attacker, attackable_tile, true) {
            return;
        }
        Self::attack_or_nuke(attacker, attackable_tile);
    }

    /// Moves [attacker] to [attackable_tile], handles siege setup and returns `true` if an attack is still possible.
    ///
    /// This is a logic function, not UI, so e.g. sound needs to be handled after calling self.
    pub fn move_preparing_attack(
        attacker: &dyn ICombatant,
        attackable_tile: &AttackableTile,
        try_heal_pillage: bool,
    ) -> bool {
        let map_unit_combatant = match attacker.as_any().downcast_ref::<MapUnitCombatant>() {
            Some(combatant) => combatant,
            None => return true,
        };

        let tiles_moved_through = map_unit_combatant
            .unit
            .movement
            .get_distance_to_tiles()
            .get_path_to_tile(&attackable_tile.tile_to_attack_from);
        map_unit_combatant
            .unit
            .movement
            .move_to_tile(&attackable_tile.tile_to_attack_from);

        // When calculating movement distance, we assume that a hidden tile is 1 movement point,
        // which can lead to EXCEEDINGLY RARE edge cases where you think
        // that you can attack a tile by passing through a HIDDEN TILE,
        // but the hidden tile is actually IMPASSIBLE, so you stop halfway!
        if map_unit_combatant.get_tile() != &attackable_tile.tile_to_attack_from {
            return false;
        }

        // Rarely, a melee unit will target a civilian then move through the civilian to get
        // to attackable_tile.tile_to_attack_from, meaning that they take the civilian.
        // This can lead to:
        // A. the melee unit from trying to capture their own unit (see #7282)
        // B. The civilian unit disappearing entirely (e.g. Great Person) and trying to capture a non-existent unit (see #8563)
        let combatant = Self::get_map_combatant_of_tile(&attackable_tile.tile_to_attack);
        if combatant.is_none()
            || combatant.as_ref().unwrap().get_civ_info() == attacker.get_civ_info()
        {
            return false;
        }

        // Alternatively, maybe we DID reach that tile, but it turned out to be a hill or something,
        // so we expended all of our movement points!
        if map_unit_combatant.has_unique(UniqueType::MustSetUp)
            && !map_unit_combatant.unit.is_set_up_for_siege()
            && map_unit_combatant.unit.has_movement()
        {
            map_unit_combatant.unit.action = Some(UnitActionType::SetUp.value());
            map_unit_combatant.unit.use_movement_points(1.0);
        }

        if try_heal_pillage {
            // Now lets retroactively see if we can pillage any improvement on the path improvement to heal
            // while still being able to attack
            for tile_to_pillage in tiles_moved_through {
                if map_unit_combatant.unit.current_movement <= 1.0
                    || map_unit_combatant.unit.health > 90
                {
                    break; // We are done pillaging
                }

                if UnitActionsPillage::can_pillage(&map_unit_combatant.unit, &tile_to_pillage)
                    && tile_to_pillage.can_pillage_tile_improvement()
                {
                    if let Some(pillage_action) = UnitActionsPillage::get_pillage_action(
                        &map_unit_combatant.unit,
                        &tile_to_pillage,
                    ) {
                        if let Some(action) = pillage_action.action {
                            action();
                        }
                    }
                }
            }
        }

        map_unit_combatant.unit.has_movement()
    }

    /// This is meant to be called only after all prerequisite checks have been done.
    pub fn attack_or_nuke(
        attacker: &dyn ICombatant,
        attackable_tile: &AttackableTile,
    ) -> DamageDealt {
        if let Some(map_unit_combatant) = attacker.as_any().downcast_ref::<MapUnitCombatant>() {
            if map_unit_combatant.unit.is_nuclear_weapon() {
                Nuke::nuke(map_unit_combatant, &attackable_tile.tile_to_attack);
                return DamageDealt::none();
            }
        }

        let defender = Self::get_map_combatant_of_tile(&attackable_tile.tile_to_attack).unwrap();
        Self::attack(attacker, &*defender)
    }
}
