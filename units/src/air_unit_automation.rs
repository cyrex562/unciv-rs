
use crate::battle::air_interception::AirInterception;
use crate::battle::battle::Battle;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::battle::nuke::Nuke;
use crate::battle::target_helper::target_helper;
use crate::automation::unit::battle_helper::BattleHelper;
use crate::automation::unit::head_towards_enemy_city_automation::HeadTowardsEnemyCityAutomation;
use crate::civilization::civilization::Civilization;
use crate::map::MapUnit;
use crate::tile::tile::Tile;
use crate::unique::UniqueType;
use std::collections::{HashMap, HashSet};
use std::cmp::min;

pub fn automate_fighter(unit: &mut MapUnit) {
    if unit.health < 75 {
        return; // Wait and heal
    }

    let tiles_with_enemy_units_in_range = unit.civ.threat_manager.get_tiles_with_enemy_units_in_distance(unit.get_tile(), unit.get_range());

    // TODO: Optimize [friendlyAirUnitsInRange] by creating an alternate [ThreatManager.getTilesWithEnemyUnitsInDistance] that handles only friendly units
    let friendly_air_units_in_range: Vec<_> = unit.get_tile().get_tiles_in_distance(unit.get_range())
        .iter()
        .flat_map(|t| t.air_units.iter())
        .filter(|u| u.civ == unit.civ)
        .cloned()
        .collect();

    // Find all visible enemy air units
    let enemy_air_units_in_range: Vec<_> = tiles_with_enemy_units_in_range.iter()
        .flat_map(|t| t.air_units.iter())
        .filter(|u| u.civ.is_at_war_with(&unit.civ))
        .cloned()
        .collect();

    let enemy_fighters = enemy_air_units_in_range.len() / 2; // Assume half the planes are fighters
    let friendly_unused_fighter_count = friendly_air_units_in_range.iter()
        .filter(|u| u.health >= 50 && u.can_attack())
        .count();
    let friendly_used_fighter_count = friendly_air_units_in_range.iter()
        .filter(|u| u.health >= 50 && !u.can_attack())
        .count();

    // We need to be on standby in case they attack
    if friendly_unused_fighter_count < enemy_fighters {
        return;
    }

    if friendly_used_fighter_count <= enemy_fighters {
        let air_sweep_damage_percent_bonus = |unit: &MapUnit| -> i32 {
            unit.get_matching_uniques(UniqueType::StrengthWhenAirsweep)
                .iter()
                .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
                .sum()
        };

        // If we are outnumbered, don't heal after attacking and don't have an Air Sweep bonus
        // Then we shouldn't speed the air combat by killing our fighters, instead, focus on defending
        if friendly_used_fighter_count + friendly_unused_fighter_count < enemy_fighters
            && !unit.has_unique(UniqueType::HealsEvenAfterAction)
            && air_sweep_damage_percent_bonus(unit) <= 0
        {
            return;
        } else {
            if Self::try_air_sweep(unit, &tiles_with_enemy_units_in_range) {
                return;
            }
        }
    }

    if unit.health < 80 {
        return; // Wait and heal up, no point in moving closer to combat if we aren't healed
    }

    if BattleHelper::try_attack_nearby_enemy(unit, &unit.civ) {
        return;
    }

    if unit.cache.cannot_move {
        return; // from here on it's all "try to move somewhere else"
    }

    if Self::try_relocate_to_cities_with_enemy_near_by(unit) {
        return;
    }

    let paths_to_cities = unit.movement.get_aerial_paths_to_cities();
    if paths_to_cities.is_empty() {
        return; // can't actually move anywhere else
    }

    let cities_by_nearby_air_units: HashMap<_, _> = paths_to_cities.keys()
        .iter()
        .map(|city| {
            let nearby_enemy_air_units = city.get_tiles_in_distance(unit.get_max_movement_for_air_units())
                .iter()
                .filter(|t| {
                    if let Some(first_air_unit) = t.air_units.first() {
                        first_air_unit.civ.is_at_war_with(&unit.civ)
                    } else {
                        false
                    }
                })
                .count();
            (city.clone(), nearby_enemy_air_units)
        })
        .collect();

    if cities_by_nearby_air_units.values().any(|&count| count != 0) {
        let cities_with_most_need_of_air_units = cities_by_nearby_air_units.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(city, _)| city.clone())
            .unwrap();

        // Find the city with the shortest path
        let chosen_city = cities_with_most_need_of_air_units.iter()
            .min_by_key(|city| paths_to_cities.get(city).map(|path| path.len()).unwrap_or(0))
            .unwrap();

        let first_step_in_path = paths_to_cities.get(chosen_city).unwrap().first().unwrap();
        unit.movement.move_to_tile(first_step_in_path);
        return;
    }

    // no city needs fighters to defend, so let's attack stuff from the closest possible location
    Self::try_move_to_cities_to_aerial_attack_from(&paths_to_cities, unit);
}

/// Attempts to perform an air sweep on enemy units
fn try_air_sweep(unit: &mut MapUnit, tiles_with_enemy_units_in_range: &[Tile]) -> bool {
    let target_tile = tiles_with_enemy_units_in_range.iter()
        .filter(|tile| {
            tile.get_units().iter().any(|u| u.civ.is_at_war_with(&unit.civ))
                || (tile.is_city_center() && tile.get_city().map(|c| c.civ.is_at_war_with(&unit.civ)).unwrap_or(false))
        })
        .min_by_key(|tile| tile.aerial_distance_to(unit.get_tile()))
        .cloned();

    if let Some(tile) = target_tile {
        AirInterception::air_sweep(&mut MapUnitCombatant::new(unit), &tile);
        !unit.has_movement()
    } else {
        false
    }
}

/// Automates bomber unit actions
pub fn automate_bomber(unit: &mut MapUnit) {
    if unit.health < 75 {
        return; // Wait and heal
    }

    if BattleHelper::try_attack_nearby_enemy(unit, &unit.civ) {
        return;
    }

    if unit.health <= 90 || (unit.health < 100 && !unit.civ.is_at_war()) {
        return; // Wait and heal
    }

    if unit.cache.cannot_move {
        return; // from here on it's all "try to move somewhere else"
    }

    if Self::try_relocate_to_cities_with_enemy_near_by(unit) {
        return;
    }

    let paths_to_cities = unit.movement.get_aerial_paths_to_cities();
    if paths_to_cities.is_empty() {
        return; // can't actually move anywhere else
    }

    Self::try_move_to_cities_to_aerial_attack_from(&paths_to_cities, unit);
}

/// Moves air units to cities from which they can attack
fn try_move_to_cities_to_aerial_attack_from(paths_to_cities: &HashMap<Tile, Vec<Tile>>, air_unit: &mut MapUnit) {
    let cities_that_can_attack_from: Vec<_> = paths_to_cities.keys()
        .iter()
        .filter(|destination_city| {
            **destination_city != air_unit.current_tile
                && destination_city.get_tiles_in_distance(air_unit.get_range())
                .iter()
                .any(|t| target_helper::contains_attackable_enemy(t, MapUnitCombatant::new(air_unit)))
        })
        .cloned()
        .collect();

    if cities_that_can_attack_from.is_empty() {
        return;
    }

    // Find the closest city that can attack from
    let closest_city_that_can_attack_from = cities_that_can_attack_from.iter()
        .min_by_key(|city| paths_to_cities.get(city).map(|path| path.len()).unwrap_or(0))
        .unwrap();

    let first_step_in_path = paths_to_cities.get(closest_city_that_can_attack_from).unwrap().first().unwrap();
    air_unit.movement.move_to_tile(first_step_in_path);
}

/// Automates nuclear weapons
pub fn automate_nukes(unit: &mut MapUnit) {
    if !unit.civ.is_at_war() {
        return;
    }

    // We should *Almost* never want to nuke our own city, so don't consider it
    if unit.type_.is_air_unit() {
        let tiles_in_range = unit.current_tile.get_tiles_in_distance_range(2..=unit.get_range());
        let highest_tile_nuke_value = tiles_in_range.iter()
            .map(|tile| (tile.clone(), Self::get_nuke_location_value(unit, tile)))
            .max_by_key(|(_, value)| *value);

        if let Some((tile, value)) = highest_tile_nuke_value {
            if value > 0 {
                Nuke::nuke(&mut MapUnitCombatant::new(unit), &tile);
            }
        }

        Self::try_relocate_missile_to_nearby_attackable_cities(unit);
    } else {
        let attackable_tiles = target_helper::get_attackable_enemies(
            unit,
            unit.movement.get_distance_to_tiles(),
            /* arg3 */ None,
            /* arg4 */ None
        );
        let highest_tile_nuke_value = attackable_tiles.iter()
            .map(|target| (target.tile_to_attack.clone(), Self::get_nuke_location_value(unit, &target.tile_to_attack)))
            .max_by_key(|(_, value)| *value);

        if let Some((tile, value)) = highest_tile_nuke_value {
            if value > 0 {
                Battle::move_and_attack(MapUnitCombatant::new(unit), &tile);
            }
        }

        HeadTowardsEnemyCityAutomation::try_head_towards_enemy_city(unit);
    }
}

/// Ranks the tile to nuke based off of all tiles in it's blast radius
/// By default the value is -500 to prevent inefficient nuking.
fn get_nuke_location_value(nuke: &MapUnit, tile: &Tile) -> i32 {
    let civ = &nuke.civ;

    if !Nuke::may_use_nuke(&MapUnitCombatant::new(nuke), tile) {
        return i32::MIN_VALUE;
    }

    let blast_radius = nuke.get_nuke_blast_radius();
    let tiles_in_blast_radius = tile.get_tiles_in_distance(blast_radius);

    let mut civs_in_blast_radius: HashSet<_> = tiles_in_blast_radius.iter()
        .filter_map(|t| t.get_owner())
        .collect();

    civs_in_blast_radius.extend(
        tiles_in_blast_radius.iter()
            .filter_map(|t| Some(t.get_first_unit().map(|u| u.civ.clone())))
    );

    // Don't nuke if it means we will be declaring war on someone!
    if civs_in_blast_radius.iter().any(|c| c != civ && !c.is_at_war_with(civ)) {
        return -100000;
    }

    // If there are no enemies to hit, don't nuke
    if !civs_in_blast_radius.iter().any(|c| c.is_at_war_with(civ)) {
        return -100000;
    }

    // Launching a Nuke uses resources, therefore don't launch it by default
    let mut explosion_value = -500;

    // Returns either ourValue or theirValue depending on if the input Civ matches the Nuke's Civ
    let evaluate_civ_value = |target_civ: &Civilization, our_value: i32, their_value: i32| -> i32 {
        if target_civ == civ {
            // We are nuking something that we own!
            our_value
        } else {
            // We are nuking an enemy!
            their_value
        }
    };

    for target_tile in tiles_in_blast_radius.iter() {
        // We can only account for visible units
        if target_tile.is_visible(civ) {
            for target_unit in target_tile.get_units().iter() {
                if target_unit.is_invisible(civ) {
                    continue;
                }

                // If we are nuking a unit at ground zero, it is more likely to be destroyed
                let tile_explosion_value = if target_tile == tile { 80 } else { 50 };

                if target_unit.is_military() {
                    explosion_value += if target_tile == tile {
                        evaluate_civ_value(&target_unit.civ, -200, tile_explosion_value)
                    } else {
                        evaluate_civ_value(&target_unit.civ, -150, 50)
                    };
                } else if target_unit.is_civilian() {
                    explosion_value += evaluate_civ_value(&target_unit.civ, -100, tile_explosion_value / 2);
                }
            }
        }

        // Never nuke our own Civ, don't nuke single enemy civs as well
        if target_tile.is_city_center() {
            if let Some(city) = target_tile.get_city() {
                if !(city.health <= 50.0 && target_tile.neighbors.iter().any(|n| n.military_unit.as_ref().map_or(false, |u| u.civ == *civ))) {
                    // Prefer not to nuke cities that we are about to take
                    explosion_value += evaluate_civ_value(&city.civ, -100000, 250);
                }
            }
        } else if target_tile.owning_city.is_some() {
            let owning_civ = target_tile.owning_city.as_ref().unwrap().civ.clone();

            // If there is a tile to add fallout to there is a 50% chance it will get fallout
            if !(tile.is_water() || tile.is_impassible() || target_tile.has_fallout_equivalent()) {
                explosion_value += evaluate_civ_value(&owning_civ, -40, 10);
            }

            // If there is an improvement to pillage
            if target_tile.improvement.is_some() && !target_tile.improvement_is_pillaged {
                explosion_value += evaluate_civ_value(&owning_civ, -40, 20);
            }
        }

        // If the value is too low end the search early
        if explosion_value < -1000 {
            return explosion_value;
        }
    }

    explosion_value
}

/// Automates missile units
pub fn automate_missile(unit: &mut MapUnit) {
    if BattleHelper::try_attack_nearby_enemy(unit, &unit.civ) {
        return;
    }

    Self::try_relocate_missile_to_nearby_attackable_cities(unit);
}

/// Attempts to relocate missiles to cities from which they can attack
fn try_relocate_missile_to_nearby_attackable_cities(unit: &mut MapUnit) {
    let tiles_in_range = unit.current_tile.get_tiles_in_distance(unit.get_range());
    let immediately_reachable_cities: Vec<_> = tiles_in_range.iter()
        .filter(|t| unit.movement.can_move_to(t))
        .cloned()
        .collect();

    for city in immediately_reachable_cities.iter() {
        if city.get_tiles_in_distance(unit.get_range()).iter()
            .any(|t| t.is_city_center() && t.get_owner().map_or(false, |o| o.is_at_war_with(&unit.civ)))
        {
            unit.movement.move_to_tile(city);
            return;
        }
    }

    let paths_to_cities = unit.movement.get_aerial_paths_to_cities();
    if paths_to_cities.is_empty() {
        return; // can't actually move anywhere else
    }

    Self::try_move_to_cities_to_aerial_attack_from(&paths_to_cities, unit);
}

/// Attempts to relocate units to cities with enemies nearby
fn try_relocate_to_cities_with_enemy_near_by(unit: &mut MapUnit) -> bool {
    let immediately_reachable_cities_and_carriers: Vec<_> = unit.current_tile
        .get_tiles_in_distance(unit.get_max_movement_for_air_units())
        .iter()
        .filter(|t| unit.movement.can_move_to(t))
        .cloned()
        .collect();

    for city in immediately_reachable_cities_and_carriers.iter() {
        if city.get_tiles_in_distance(unit.get_range()).iter()
            .any(|t| t.is_visible(&unit.civ) && target_helper::contains_attackable_enemy(t, MapUnitCombatant::new(unit)))
        {
            unit.movement.move_to_tile(city);
            return true;
        }
    }

    false
}


pub fn air_sweep(attacker: UnitId, attacked_tile: &str) {
    // Air Sweep counts as an attack, even if nothing else happens
    attacker.unit.attacks_this_turn += 1;

    // copied and modified from reduceAttackerMovementPointsAndAttacks()
    // use up movement
    if attacker.unit.has_unique(UniqueType::CanMoveAfterAttacking) ||
        attacker.unit.max_attacks_per_turn() > attacker.unit.attacks_this_turn {
        // if it was a melee attack and we won, then the unit ALREADY got movement points deducted,
        // for the movement to the enemy's tile!
        // and if it's an air unit, it only has 1 movement anyway, so...
        if !attacker.unit.base_unit.moves_like_air_units {
            attacker.unit.use_movement_points(1.0);
        }
    } else {
        attacker.unit.current_movement = 0.0;
    }

    let attacker_name = attacker.get_name();

    // Make giant sequence of all potential Interceptors from all Civs isAtWarWith()
    let mut potential_interceptors = Vec::new();
    for intercepting_civ in attacker.get_civ_info().game_info.civilizations.iter()
        .filter(|c| attacker.get_civ_info().is_at_war_with(c)) {
        potential_interceptors.extend(
            intercepting_civ.units.get_civ_units()
                .filter(|u| u.can_intercept(attacked_tile))
        );
    }

    // first priority, only Air Units
    if potential_interceptors.iter().any(|u| u.base_unit.is_air_unit()) {
        potential_interceptors.retain(|u| u.base_unit.is_air_unit());
    }

    // Pick highest chance interceptor
    for interceptor in potential_interceptors.iter()
        .collect::<Vec<_>>()
        .shuffle(&mut rand::thread_rng())
        .into_iter()
        .sorted_by(|a, b| b.intercept_chance().partial_cmp(&a.intercept_chance()).unwrap_or(std::cmp::Ordering::Equal)) {

        // No chance of Interceptor to miss (unlike regular Interception). Always want to deal damage
        // pairs of LocationAction for Notification
        let locations = LocationAction::new(
            interceptor.current_tile.position,
            attacker.unit.current_tile.position
        );

        interceptor.attacks_this_turn += 1;  // even if you miss, you took the shot

        if !interceptor.base_unit.is_air_unit() {
            let interceptor_name = interceptor.name.clone();
            // Deal no damage (moddable in future?) and no XP
            let attacker_text = format!(
                "Our [{}] ([-0] HP) was attacked by an intercepting [{}] ([-0] HP)",
                attacker_name, interceptor_name
            );
            let interceptor_text = format!(
                "Our [{}] ([-0] HP) intercepted and attacked an enemy [{}] ([-0] HP)",
                interceptor_name, attacker_name
            );

            attacker.get_civ_info().add_notification(
                &attacker_text,
                &locations,
                NotificationCategory::War,
                &attacker_name,
                NotificationIcon::War,
                &interceptor_name
            );

            interceptor.civ.add_notification(
                &interceptor_text,
                &locations,
                NotificationCategory::War,
                &interceptor_name,
                NotificationIcon::War,
                &attacker_name
            );

            attacker.unit.action = None;
            return;
        }

        // Damage if Air v Air should work similar to Melee
        let damage_dealt = Battle::take_damage(attacker, MapUnitCombatant::new(interceptor));

        // 5 XP to both
        Battle::add_xp(MapUnitCombatant::new(interceptor), 5, attacker);
        Battle::add_xp(attacker, 5, MapUnitCombatant::new(interceptor));

        let locations_interceptor_unknown = LocationAction::new(
            attacked_tile.position,
            attacker.unit.current_tile.position
        );

        Self::add_air_sweep_interception_notifications(
            attacker,
            &*interceptor,
            damage_dealt,
            &locations_interceptor_unknown,
            &locations
        );

        attacker.unit.action = None;
        return;
    }

    // No Interceptions available
    let attacker_text = format!("Nothing tried to intercept our [{}]", attacker_name);
    attacker.get_civ_info().add_notification(
        &attacker_text,
        NotificationCategory::War,
        &attacker_name
    );

    attacker.unit.action = None;
}

// TODO: Check overlap with addInterceptionNotifications, and unify what we can
fn add_air_sweep_interception_notifications(
    attacker: &MapUnitCombatant,
    interceptor: &MapUnit,
    damage_dealt: DamageDealt,
    locations_interceptor_unknown: &LocationAction,
    locations: &LocationAction
) {
    let attacker_name = attacker.get_name();
    let interceptor_name = interceptor.name.clone();

    let attacker_text = if attacker.is_defeated() {
        if attacker.get_civ_info().viewable_tiles.contains(&interceptor.get_tile()) {
            format!(
                "Our [{}] ([-{}] HP) was destroyed by an intercepting [{}] ([-{}] HP)",
                attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
            )
        } else {
            format!(
                "Our [{}] ([-{}] HP) was destroyed by an unknown interceptor",
                attacker_name, damage_dealt.defender_dealt
            )
        }
    } else if MapUnitCombatant::new(interceptor).is_defeated() {
        format!(
            "Our [{}] ([-{}] HP) destroyed an intercepting [{}] ([-{}] HP)",
            attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
        )
    } else {
        format!(
            "Our [{}] ([-{}] HP) was attacked by an intercepting [{}] ([-{}] HP)",
            attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
        )
    };

    attacker.get_civ_info().add_notification(
        &attacker_text,
        locations_interceptor_unknown,
        NotificationCategory::War,
        &attacker_name,
        NotificationIcon::War,
        NotificationIcon::Question
    );

    let interceptor_text = if attacker.is_defeated() {
        format!(
            "Our [{}] ([-{}] HP) intercepted and destroyed an enemy [{}] ([-{}] HP)",
            interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
        )
    } else if MapUnitCombatant::new(interceptor).is_defeated() {
        if interceptor.civ.viewable_tiles.contains(&attacker.get_tile()) {
            format!(
                "Our [{}] ([-{}] HP) intercepted and was destroyed by an enemy [{}] ([-{}] HP)",
                interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
            )
        } else {
            format!(
                "Our [{}] ([-{}] HP) intercepted and was destroyed by an unknown enemy",
                interceptor_name, damage_dealt.attacker_dealt
            )
        }
    } else {
        format!(
            "Our [{}] ([-{}] HP) intercepted and attacked an enemy [{}] ([-{}] HP)",
            interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
        )
    };

    interceptor.civ.add_notification(
        &interceptor_text,
        locations,
        NotificationCategory::War,
        &interceptor_name,
        NotificationIcon::War,
        &attacker_name
    );
}

pub fn try_intercept_air_attack(
    attacker: &mut MapUnitCombatant,
    attacked_tile: &Tile,
    intercepting_civ: &Civilization,
    defender: Option<&dyn ICombatant>
) -> DamageDealt {
    if attacker.unit.has_unique(
        UniqueType::CannotBeIntercepted,
        StateForConditionals::new(
            attacker.get_civ_info(),
            attacker,
            defender,
            None,
            Some(attacked_tile)
        )
    ) {
        return DamageDealt::None;
    }

    // Pick highest chance interceptor
    let interceptor = intercepting_civ.units.get_civ_units()
        .filter(|u| u.can_intercept(attacked_tile))
        .sorted_by(|a, b| b.intercept_chance().partial_cmp(&a.intercept_chance()).unwrap_or(std::cmp::Ordering::Equal))
        .find(|unit| {
            // Can't intercept if we have a unique preventing it
            let conditional_state = StateForConditionals::new(
                intercepting_civ,
                MapUnitCombatant::new(unit),
                attacker,
                Some(CombatAction::Intercept),
                Some(attacked_tile)
            );

            unit.get_matching_uniques(UniqueType::CannotInterceptUnits, conditional_state)
                .iter()
                .none(|u| attacker.matches_filter(&u.params[0]))
                // Defender can't intercept either
                && unit != defender.and_then(|d| d.downcast_ref::<MapUnitCombatant>()).map(|d| &d.unit).unwrap_or(&None)
        });

    let interceptor = match interceptor {
        Some(i) => i,
        None => return DamageDealt::None,
    };

    interceptor.attacks_this_turn += 1;  // even if you miss, you took the shot

    // Does Intercept happen? If not, exit
    if rand::thread_rng().gen::<f32>() > interceptor.intercept_chance() / 100.0 {
        return DamageDealt::None;
    }

    let mut damage = BattleDamage::calculate_damage_to_defender(
        MapUnitCombatant::new(interceptor),
        attacker
    );

    let mut damage_factor = 1.0 + interceptor.intercept_damage_percent_bonus() as f32 / 100.0;
    damage_factor *= attacker.unit.received_intercept_damage_factor();

    damage = (damage as f32 * damage_factor).min(attacker.unit.health as f32) as i32;

    attacker.take_damage(damage);
    if damage > 0 {
        Battle::add_xp(MapUnitCombatant::new(interceptor), 2, attacker);
    }

    Self::add_interception_notifications(attacker, &interceptor, damage);

    DamageDealt::new(0, damage)
}

fn add_interception_notifications(
    attacker: &MapUnitCombatant,
    interceptor: &MapUnit,
    damage: i32
) {
    let attacker_name = attacker.get_name();
    let interceptor_name = interceptor.name.clone();

    let locations = LocationAction::new(
        interceptor.current_tile.position,
        attacker.unit.current_tile.position
    );

    let attacker_text = if !attacker.is_defeated() {
        format!(
            "Our [{}] ([-{}] HP) was attacked by an intercepting [{}] ([-0] HP)",
            attacker_name, damage, interceptor_name
        )
    } else if attacker.get_civ_info().viewable_tiles.contains(&interceptor.get_tile()) {
        format!(
            "Our [{}] ([-{}] HP) was destroyed by an intercepting [{}] ([-0] HP)",
            attacker_name, damage, interceptor_name
        )
    } else {
        format!(
            "Our [{}] ([-{}] HP) was destroyed by an unknown interceptor",
            attacker_name, damage
        )
    };

    attacker.get_civ_info().add_notification(
        &attacker_text,
        &interceptor.current_tile.position,
        NotificationCategory::War,
        &attacker_name,
        NotificationIcon::War,
        &interceptor_name
    );

    let interceptor_text = if attacker.is_defeated() {
        format!(
            "Our [{}] ([-0] HP) intercepted and destroyed an enemy [{}] ([-{}] HP)",
            interceptor_name, attacker_name, damage
        )
    } else {
        format!(
            "Our [{}] ([-0] HP) intercepted and attacked an enemy [{}] ([-{}] HP)",
            interceptor_name, attacker_name, damage
        )
    };

    interceptor.civ.add_notification(
        &interceptor_text,
        &locations,
        NotificationCategory::War,
        &interceptor_name,
        NotificationIcon::War,
        &attacker_name
    );
}


