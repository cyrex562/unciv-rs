use crate::models::civilization::Civilization;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::ruleset::unique::UniqueType;
use crate::battle::attackable_tile::AttackableTile;
use crate::battle::battle::Battle;
use crate::battle::battle_damage::battle_damage;
use crate::battle::city_combatant::CityCombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::battle::target_helper::target_helper;
use crate::models::city::City;
use std::collections::HashMap;

/// Contains helper functions for battle-related automation
pub struct BattleHelper;

impl BattleHelper {
    /// Returns true if the unit cannot further move this turn - NOT if an attack was successful!
    pub fn try_attack_nearby_enemy(unit: &mut MapUnit, stay_on_tile: bool) -> bool {
        if unit.has_unique(UniqueType::CannotAttack) {
            return false;
        }

        let distance_to_tiles = unit.movement.get_distance_to_tiles();
        let attackable_enemies: Vec<_> = target_helper::get_attackable_enemies(unit, &distance_to_tiles, stay_on_tile, true)
            .iter()
            .filter(|enemy| {
                unit.has_unique(UniqueType::SelfDestructs) ||
                battle_damage::calculate_damage_to_attacker(
                    MapUnitCombatant::new(unit),
                    Battle::get_map_combatant_of_tile(&enemy.tile_to_attack).unwrap(),
                    &enemy.tile_to_attack_from,
                    false
                ) + unit.get_damage_from_terrain(&enemy.tile_to_attack_from) < unit.health
            })
            .cloned()
            .collect();

        let enemy_tile_to_attack = Self::choose_attack_target(unit, &attackable_enemies);

        if let Some(enemy_tile) = enemy_tile_to_attack {
            if enemy_tile.tile_to_attack.military_unit.is_none() && unit.base_unit.is_ranged()
                && unit.movement.can_move_to(&enemy_tile.tile_to_attack)
                && distance_to_tiles.contains_key(&enemy_tile.tile_to_attack)
            {
                // Ranged units should move to capture a civilian unit instead of attacking it
                unit.movement.move_to_tile(&enemy_tile.tile_to_attack);
            } else {
                Battle::move_and_attack(MapUnitCombatant::new(unit), &enemy_tile);
            }
        }

        !unit.has_movement()
    }

    /// Attempts to disembark a unit to an attack position
    pub fn try_disembark_unit_to_attack_position(unit: &mut MapUnit) -> bool {
        if !unit.base_unit.is_melee() || !unit.base_unit.is_land_unit || !unit.is_embarked() {
            return false;
        }

        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();

        let attackable_enemies_next_turn: Vec<_> = target_helper::get_attackable_enemies(unit, &unit_distance_to_tiles, false, false)
            .iter()
            .filter(|enemy| {
                battle_damage::calculate_damage_to_attacker(
                    MapUnitCombatant::new(unit),
                    Battle::get_map_combatant_of_tile(&enemy.tile_to_attack).unwrap(),
                    &enemy.tile_to_attack_from,
                    false
                ) < unit.health
            })
            .filter(|enemy| enemy.tile_to_attack_from.is_land)
            .cloned()
            .collect();

        let enemy_tile_to_attack_next_turn = Self::choose_attack_target(unit, &attackable_enemies_next_turn);

        if let Some(enemy_tile) = enemy_tile_to_attack_next_turn {
            unit.movement.move_to_tile(&enemy_tile.tile_to_attack_from);
            return true;
        }

        false
    }

    /// Chooses the best target in attackable_enemies, this could be a city or a unit.
    fn choose_attack_target(unit: &MapUnit, attackable_enemies: &[AttackableTile]) -> Option<AttackableTile> {
        // Get the highest valued attackableEnemy
        let mut highest_attack_value = 0;
        let mut attack_tile: Option<AttackableTile> = None;

        // We always have to calculate the attack value even if there is only one attackableEnemy
        for attackable_enemy in attackable_enemies {
            let temp_attack_value = if attackable_enemy.tile_to_attack.is_city_center() {
                if let Some(city) = attackable_enemy.tile_to_attack.get_city() {
                    Self::get_city_attack_value(unit, city)
                } else {
                    0
                }
            } else {
                Self::get_unit_attack_value(unit, attackable_enemy)
            };

            if temp_attack_value > highest_attack_value {
                highest_attack_value = temp_attack_value;
                attack_tile = Some(attackable_enemy.clone());
            }
        }

        // Only return that tile if it is actually a good tile to attack
        if highest_attack_value > 30 {
            attack_tile
        } else {
            None
        }
    }

    /// Returns a value which represents the attacker's motivation to attack a city.
    /// Siege units will almost always attack cities.
    /// Base value is 100(Melee) 110(Ranged) standard deviation is around 80 to 130
    fn get_city_attack_value(attacker: &MapUnit, city: &City) -> i32 {
        let attacker_unit = MapUnitCombatant::new(attacker);
        let city_unit = CityCombatant::new(city);
        let is_city_capturable = city.health == 1.0
            || (attacker.base_unit.is_melee() && city.health <= battle_damage::calculate_damage_to_defender(&attacker_unit, &city_unit, &city.get_center_tile(), false).max(1.0));

        if is_city_capturable {
            return if attacker.base_unit.is_melee() { 10000 } // Capture the city immediately!
            else { 0 }; // Don't attack the city anymore since we are a ranged unit
        }

        if attacker.base_unit.is_melee() {
            let battle_damage = battle_damage::calculate_damage_to_attacker(&attacker_unit, &city_unit, &city.get_center_tile(), false);
            if attacker.health - battle_damage * 2.0 <= 0.0 && !attacker.has_unique(UniqueType::SelfDestructs) {
                // The more friendly units around the city, the more willing we should be to just attack the city
                let friendly_units_around_city = city.get_center_tile().get_tiles_in_distance(3)
                    .iter()
                    .filter(|t| t.military_unit.as_ref().map_or(false, |u| u.civ == attacker.civ))
                    .count();

                // If we have more than 4 other units around the city, go for it
                if friendly_units_around_city < 5 {
                    let attacker_health_modifier = 1.0 + 1.0 / friendly_units_around_city as f64;
                    if attacker.health - battle_damage * attacker_health_modifier <= 0.0 {
                        return 0; // We'll probably die next turn if we attack the city
                    }
                }
            }
        }

        let mut attack_value = 100;
        // Siege units should really only attack the city
        if attacker.base_unit.is_probably_siege_unit() {
            attack_value += 100;
        }
        // Ranged units don't take damage from the city
        else if attacker.base_unit.is_ranged() {
            attack_value += 10;
        }
        // Lower health cities have a higher priority to attack ranges from -20 to 30
        attack_value -= (city.health - 60.0) as i32 / 2;

        // Add value based on number of units around the city
        let defending_city_civ = &city.civ;
        for tile in city.get_center_tile().get_tiles_in_distance(2).iter() {
            if let Some(military_unit) = &tile.military_unit {
                if military_unit.civ.is_at_war_with(&attacker.civ) {
                    attack_value -= 5;
                }
                if military_unit.civ.is_at_war_with(defending_city_civ) {
                    attack_value += 15;
                }
            }
        }

        attack_value
    }

    /// Returns a value which represents the attacker's motivation to attack a unit.
    /// Base value is 100 and standard deviation is around 80 to 130
    fn get_unit_attack_value(attacker: &MapUnit, attack_tile: &AttackableTile) -> i32 {
        // Base attack value, there is nothing there...
        let mut attack_value = i32::MIN_VALUE;

        // Prioritize attacking military
        if let Some(military_unit) = &attack_tile.tile_to_attack.military_unit {
            attack_value = 100;
            // Associate enemy units with number of hits from this unit to kill them
            let damage = battle_damage::calculate_damage_to_defender(
                MapUnitCombatant::new(attacker),
                MapUnitCombatant::new(military_unit),
                &attack_tile.tile_to_attack_from,
                false
            ).unwrap_or(1.0);
            
            let attacks_to_kill = (military_unit.health as f64 / damage)
                .max(1.0).min(Some(10.0));

            // We can kill them in this turn
            if attacks_to_kill <= Some(1.0) {
                attack_value += 30;
            }
            // On average, this should take around 3 turns, so -15
            else {
                attack_value -= (attacks_to_kill * 5.0).round() as i32;
            }
        } else if let Some(civilian_unit) = &attack_tile.tile_to_attack.civilian_unit {
            attack_value = 50;
            // Only melee units should really attack/capture civilian units, ranged units may be able to capture by moving
            if attacker.base_unit.is_melee() || attacker.movement.can_reach_in_current_turn(&attack_tile.tile_to_attack) {
                if civilian_unit.is_great_person() {
                    attack_value += 150;
                }
                if civilian_unit.has_unique(UniqueType::FoundCity) {
                    attack_value += 60;
                }
            } else if attacker.base_unit.is_ranged() && !civilian_unit.has_unique(UniqueType::Uncapturable) {
                return 10; // Don't shoot civilians that we can capture!
            }
        }

        // Prioritise closer units as they are generally more threatening to this unit
        // Moving around less means we are straying less into enemy territory
        // Average should be around 2.5-5 early game and up to 35 for tanks in late game
        attack_value += (attack_tile.movement_left_after_moving_to_attack_tile * 5.0).round() as i32;

        attack_value
    }
}