use std::collections::HashSet;
use crate::models::civilization::Civilization;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::city::City;
use crate::models::ruleset::unique::UniqueType;
use crate::models::ruleset::unit::BaseUnit;
use crate::automation::Automation;
use crate::automation::unit::civilian_unit_automation::CivilianUnitAutomation;
use crate::automation::unit::battle_helper::BattleHelper;
use crate::automation::unit::head_towards_enemy_city_automation::HeadTowardsEnemyCityAutomation;
use crate::automation::unit::air_unit_automation::AirUnitAutomation;
use crate::battle::{Battle, BattleDamage, CityCombatant, ICombatant, MapUnitCombatant, TargetHelper};
use crate::ui::screens::worldscreen::unit::actions::{UnitActions, UnitActionsUpgrade, UnitActionsPillage};
use crate::unciv::Constants;
use crate::unciv::UncivGame;

/// Handles unit automation logic.
pub struct UnitAutomation;

const CLOSE_ENEMY_TILES_AWAY_LIMIT: i32 = 5;
const CLOSE_ENEMY_TURNS_AWAY_LIMIT: f32 = 3.0;

impl UnitAutomation {
    fn is_good_tile_to_explore(unit: &MapUnit, tile: &Tile) -> bool {
        (tile.get_owner().is_none() || !tile.get_owner().unwrap().is_city_state())
            && tile.neighbors.iter().any(|t| !unit.civ.has_explored(t))
            && (!unit.civ.is_city_state() || tile.neighbors.iter().any(|t| t.get_owner() == Some(&unit.civ)))
            && unit.get_damage_from_terrain(tile) <= 0
            && unit.civ.threat_manager.get_distance_to_closest_enemy_unit(tile, 3) > 3
            && unit.movement.can_move_to(tile)
            && unit.movement.can_reach(tile)
    }

    pub fn try_explore(unit: &mut MapUnit) -> bool {
        if Self::try_go_to_ruin_and_encampment(unit) && (!unit.has_movement() || unit.is_destroyed) {
            return true;
        }

        let explorable_tiles_this_turn: Vec<_> = unit.movement.get_distance_to_tiles()
            .keys()
            .filter(|tile| Self::is_good_tile_to_explore(unit, tile))
            .collect();

        if !explorable_tiles_this_turn.is_empty() {
            let best_tile = explorable_tiles_this_turn.iter()
                .sorted_by(|a, b| b.tile_height.cmp(&a.tile_height))
                .max_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
                .unwrap();
            unit.movement.head_towards(best_tile);
            return true;
        }

        // Nothing immediate, lets look further. Number increases exponentially with distance - at 10 this took a looong time
        for tile in unit.current_tile.get_tiles_in_distance(5) {
            if Self::is_good_tile_to_explore(unit, &tile) {
                unit.movement.head_towards(&tile);
                return true;
            }
        }
        false
    }

    fn try_go_to_ruin_and_encampment(unit: &mut MapUnit) -> bool {
        if !unit.civ.is_major_civ() {
            return false; // barbs don't have anything to do in ruins
        }

        let tile_with_ruin_or_encampment = unit.viewable_tiles
            .iter()
            .find(|tile| {
                (tile.get_tile_improvement().map_or(false, |imp| imp.is_ancient_ruins_equivalent())
                    || tile.improvement == Some(Constants::BARBARIAN_ENCAMPMENT.to_string()))
                    && unit.movement.can_move_to(tile)
                    && unit.movement.can_reach(tile)
            });

        match tile_with_ruin_or_encampment {
            Some(tile) => {
                unit.movement.head_towards(tile);
                true
            }
            None => false
        }
    }

    // "Fog busting" is a strategy where you put your units slightly outside your borders to discourage barbarians from spawning
    fn try_fog_bust(unit: &mut MapUnit) -> bool {
        if !Automation::afraid_of_barbarians(&unit.civ) {
            return false; // Not if we're not afraid
        }

        // If everything around this unit is visible, we can stop.
        // Calculations below are quite expensive especially in the late game.
        if unit.current_tile.get_tiles_in_distance(5).iter().all(|t| t.is_visible(&unit.civ)) {
            return false;
        }

        let reachable_tiles_this_turn: Vec<_> = unit.movement.get_distance_to_tiles()
            .keys()
            .filter(|tile| Self::is_good_tile_for_fog_busting(unit, tile))
            .collect();

        if !reachable_tiles_this_turn.is_empty() {
            unit.movement.head_towards(reachable_tiles_this_turn.choose(&mut rand::thread_rng()).unwrap());
            return true;
        }

        // Nothing immediate, lets look further. Number increases exponentially with distance - at 10 this took a looong time
        for tile in unit.current_tile.get_tiles_in_distance(5) {
            if Self::is_good_tile_for_fog_busting(unit, &tile) {
                unit.movement.head_towards(&tile);
                return true;
            }
        }
        false
    }

    fn is_good_tile_for_fog_busting(unit: &MapUnit, tile: &Tile) -> bool {
        unit.movement.can_move_to(tile)
            && tile.get_owner().is_none()
            && tile.neighbors.iter().all(|t| t.get_owner().is_none())
            && unit.civ.has_explored(tile)
            && tile.get_tiles_in_distance(2).iter().any(|t| t.get_owner() == Some(&unit.civ))
            && unit.get_damage_from_terrain(tile) <= 0
            && unit.movement.can_reach(tile)
    }

    pub fn wander(unit: &mut MapUnit, stay_in_territory: bool, tiles_to_avoid: Option<&HashSet<Tile>>) {
        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();
        let reachable_tiles: Vec<_> = unit_distance_to_tiles.iter()
            .filter(|(tile, _)| {
                tiles_to_avoid.map_or(true, |avoid| !avoid.contains(tile))
                    && unit.movement.can_move_to(tile)
                    && unit.movement.can_reach(tile)
            })
            .collect();

        let reachable_tiles_max_walking_distance: Vec<_> = reachable_tiles.iter()
            .filter(|(tile, distance)| {
                distance.total_movement == unit.current_movement
                    && unit.get_damage_from_terrain(tile) <= 0
                    && (!stay_in_territory || tile.get_owner() == Some(&unit.civ))
            })
            .collect();

        if !reachable_tiles_max_walking_distance.is_empty() {
            let chosen_tile = reachable_tiles_max_walking_distance.choose(&mut rand::thread_rng()).unwrap().0;
            unit.movement.move_to_tile(chosen_tile);
        } else if !reachable_tiles.is_empty() {
            let chosen_tile = reachable_tiles.choose(&mut rand::thread_rng()).unwrap().0;
            unit.movement.move_to_tile(chosen_tile);
        }
    }

    /// Attempts to upgrade a unit.
    pub fn try_upgrade_unit(unit: &mut MapUnit) -> bool {
        if unit.civ.is_human() && !UncivGame::current().settings.automated_units_can_upgrade
            && UncivGame::current().world_screen.map_or(false, |screen| !screen.auto_play.is_auto_playing_and_full_auto_play_ai()) {
            return false;
        }

        let upgrade_units = Self::get_units_to_upgrade_to(unit);
        if upgrade_units.is_empty() {
            return false;
        }

        let upgraded_unit = upgrade_units.iter()
            .min_by_key(|unit| unit.cost)
            .unwrap();

        if upgraded_unit.get_resource_requirements_per_turn(&unit.cache.state)
            .keys()
            .any(|resource| !unit.requires_resource(resource)) {
            // The upgrade requires new resource types, so check if we are willing to invest them
            if !Automation::allow_spending_resource(&unit.civ, upgraded_unit) {
                return false;
            }
        }

        let upgrade_actions = UnitActionsUpgrade::get_upgrade_actions(unit);
        match upgrade_actions.iter()
            .find(|action| action.unit_to_upgrade_to == upgraded_unit) {
            Some(action) => {
                action.action();
                unit.is_destroyed
            }
            None => false
        }
    }

    fn get_units_to_upgrade_to(unit: &MapUnit) -> Vec<BaseUnit> {
        fn is_invalid_upgrade_destination(base_unit: &BaseUnit, unit: &MapUnit) -> bool {
            if !unit.civ.tech.is_researched(base_unit) {
                return true;
            }
            if unit.civ.is_barbarian() && base_unit.has_unique(UniqueType::CannotBeBarbarian) {
                return true;
            }
            base_unit.get_matching_uniques(UniqueType::OnlyAvailable, StateForConditionals::IgnoreConditionals)
                .iter()
                .any(|unique| !unique.conditionals_apply(&unit.cache.state))
        }

        unit.base_unit.get_ruleset_upgrade_units(&unit.cache.state)
            .iter()
            .map(|unit_name| unit.civ.get_equivalent_unit(unit_name))
            .filter(|base_unit| !is_invalid_upgrade_destination(base_unit, unit) && unit.upgrade.can_upgrade(base_unit))
            .collect()
    }

    /// Attempts to pillage an improvement.
    pub fn try_pillage_improvement(unit: &mut MapUnit, only_pillage_to_heal: bool) -> bool {
        if unit.is_civilian() {
            return false;
        }

        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();
        let tiles_that_can_walk_to_and_then_pillage = unit_distance_to_tiles.iter()
            .filter(|(tile, distance)| {
                distance.total_movement < unit.current_movement
                    && unit.movement.can_move_to(tile)
                    && UnitActionsPillage::can_pillage(unit, tile)
                    && (tile.can_pillage_tile_improvement()
                        || (!only_pillage_to_heal && tile.can_pillage_road()
                            && tile.get_road_owner().map_or(false, |owner| unit.civ.is_at_war_with(owner))))
            })
            .map(|(tile, _)| tile)
            .collect::<Vec<_>>();

        if tiles_that_can_walk_to_and_then_pillage.is_empty() {
            return false;
        }

        let tile_to_pillage = tiles_that_can_walk_to_and_then_pillage.iter()
            .max_by_key(|tile| tile.get_defensive_bonus(false, Some(unit)))
            .unwrap();

        if unit.get_tile() != *tile_to_pillage {
            unit.movement.move_to_tile(tile_to_pillage);
        }

        if unit.current_tile != *tile_to_pillage {
            return false;
        }

        // We CANNOT use invoke_unit_action, since the default unit action contains a popup, which - when automated -
        // runs a UI action on a side thread leading to crash!
        if let Some(action) = UnitActionsPillage::get_pillage_action(unit, unit.current_tile) {
            action.action();
        }
        true
    }

    pub fn automate_unit_moves(unit: &mut MapUnit) {
        assert!(!unit.civ.is_barbarian(), "Barbarians is not allowed here.");

        // Might die next turn - move!
        if unit.health <= unit.get_damage_from_terrain() && Self::try_heal_unit(unit) {
            return;
        }

        if unit.is_civilian() {
            CivilianUnitAutomation::automate_civilian_unit(unit, &Self::get_dangerous_tiles(unit));
            return;
        }

        while unit.promotions.can_be_promoted() &&
            // Restrict Human automated units from promotions via setting
            (UncivGame::current().settings.automated_units_choose_promotions || unit.civ.is_ai()) {
            let available_promotions = unit.promotions.get_available_promotions();
            let promotions_to_choose = if unit.health < 60
                && !(unit.base_unit.is_air_unit() || unit.base_unit.has_unique(UniqueType::CanMoveAfterAttacking))
                && available_promotions.iter().any(|p| p.has_unique(UniqueType::OneTimeUnitHeal)) {
                available_promotions.into_iter()
                    .filter(|p| p.has_unique(UniqueType::OneTimeUnitHeal))
                    .collect::<Vec<_>>()
            } else {
                available_promotions.into_iter()
                    .filter(|p| !p.has_unique(UniqueType::SkipPromotion))
                    .collect::<Vec<_>>()
            };

            if promotions_to_choose.is_empty() {
                break;
            }

            let free_promotions: Vec<_> = promotions_to_choose.iter()
                .filter(|p| p.has_unique(UniqueType::FreePromotion))
                .collect();

            let state_for_conditionals = &unit.cache.state;
            let chosen_promotion = if !free_promotions.is_empty() {
                free_promotions.choose_weighted(&mut rand::thread_rng(), |p| p.get_weight_for_ai_decision(state_for_conditionals)).unwrap()
            } else {
                promotions_to_choose.choose_weighted(&mut rand::thread_rng(), |p| p.get_weight_for_ai_decision(state_for_conditionals)).unwrap()
            };

            unit.promotions.add_promotion(&chosen_promotion.name);
        }

        // This allows for military units with certain civilian abilities to behave as civilians in peace and soldiers in war
        if (unit.has_unique(UniqueType::BuildImprovements)
            || unit.has_unique(UniqueType::FoundCity)
            || unit.has_unique(UniqueType::ReligiousUnit)
            || unit.has_unique(UniqueType::CreateWaterImprovements))
            && !unit.civ.is_at_war() {
            CivilianUnitAutomation::automate_civilian_unit(unit, &Self::get_dangerous_tiles(unit));
            return;
        }

        // Note that not all nukes have to be air units
        if unit.is_nuclear_weapon() {
            AirUnitAutomation::automate_nukes(unit);
            return;
        }

        if unit.base_unit.is_air_unit() {
            if unit.can_intercept() {
                AirUnitAutomation::automate_fighter(unit);
                return;
            }

            if unit.has_unique(UniqueType::SelfDestructs) {
                AirUnitAutomation::automate_missile(unit);
                return;
            }

            AirUnitAutomation::automate_bomber(unit);
            return;
        }

        // Accompany settlers
        if Self::try_accompany_settler_or_great_person(unit) {
            return;
        }

        if Self::try_go_to_ruin_and_encampment(unit) && !unit.has_movement() {
            return;
        }

        if unit.health < 50 && (Self::try_retreat(unit) || Self::try_heal_unit(unit)) {
            return; // do nothing but heal
        }

        // If there are no enemies nearby and we can heal here, wait until we are at full health
        if unit.health < 100 && Self::can_unit_heal_in_turns_on_current_tile(unit, 2, Some(4)) {
            return;
        }

        if Self::try_head_towards_our_sieged_city(unit) {
            return;
        }

        // if a embarked melee unit can land and attack next turn, do not attack from water.
        if BattleHelper::try_disembark_unit_to_attack_position(unit) {
            return;
        }

        // if there is an attackable unit in the vicinity, attack!
        if Self::try_attacking(unit) {
            return;
        }

        if Self::try_take_back_captured_city(unit) {
            return;
        }

        // Focus all units without a specific target on the enemy city closest to one of our cities
        if HeadTowardsEnemyCityAutomation::try_head_towards_enemy_city(unit) {
            return;
        }

        if Self::try_garrisoning_ranged_land_unit(unit) {
            return;
        }

        if Self::try_stationing_melee_naval_unit(unit) {
            return;
        }

        if unit.health < 80 && Self::try_heal_unit(unit) {
            return;
        }

        // move towards the closest reasonably attackable enemy unit within 3 turns of movement (and 5 tiles range)
        if Self::try_advance_towards_close_enemy(unit) {
            return;
        }

        if Self::try_head_towards_encampment(unit) {
            return;
        }

        if unit.health < 100 && Self::try_heal_unit(unit) {
            return;
        }

        if Self::try_prepare(unit) {
            return;
        }

        // else, try to go to unreached tiles
        if Self::try_explore(unit) {
            return;
        }

        if Self::try_fog_bust(unit) {
            return;
        }

        // Idle CS units should wander so they don't obstruct players so much
        if unit.civ.is_city_state() {
            Self::wander(unit, true, None);
        }
    }

    fn try_attacking(unit: &mut MapUnit) -> bool {
        for _ in 0..(unit.max_attacks_per_turn() - unit.attacks_this_turn) {
            if BattleHelper::try_attack_nearby_enemy(unit) {
                return true;
            }
            // Cavalry style tactic, attack and then retreat
            if unit.health < 50 && Self::try_retreat(unit) {
                return true;
            }
        }
        false
    }

    fn try_head_towards_encampment(unit: &mut MapUnit) -> bool {
        if unit.has_unique(UniqueType::SelfDestructs) {
            return false; // don't use single-use units against barbarians...
        }

        let known_encampments = unit.civ.game_info.tile_map.values()
            .filter(|tile| {
                tile.improvement == Some(Constants::BARBARIAN_ENCAMPMENT.to_string())
                    && unit.civ.has_explored(tile)
            });

        let cities = &unit.civ.cities;
        let encampments_close_to_cities = known_encampments
            .filter(|tile| cities.iter().any(|city| city.get_center_tile().aerial_distance_to(tile) < 6))
            .sorted_by_key(|tile| tile.aerial_distance_to(unit.current_tile));

        let encampment_to_head_towards = encampments_close_to_cities
            .find(|tile| unit.movement.can_reach(tile));

        match encampment_to_head_towards {
            Some(tile) => {
                unit.movement.head_towards(tile);
                true
            }
            None => false
        }
    }

    fn try_retreat(unit: &mut MapUnit) -> bool {
        // Precondition: This must be a military unit
        if unit.is_civilian() || unit.base_unit.is_air_unit() {
            return false;
        }
        // Better to do a more healing oriented move then
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, true) > 4 {
            return false;
        }

        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();
        let closest_city = unit.civ.cities.iter()
            .min_by_key(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()))
            .filter(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20);

        // Finding the distance to the closest enemy is expensive, so lets sort the tiles using a cheaper function
        let sorted_tiles_to_retreat_to = if let Some(city) = closest_city {
            // If we have a city, lets favor the tiles closer to that city
            unit_distance_to_tiles.keys()
                .sorted_by_key(|tile| tile.aerial_distance_to(city.get_center_tile()))
                .collect::<Vec<_>>()
        } else {
            // Rare case, what if we don't have a city nearby?
            // Lets favor the tiles that don't have enemies close by
            // Ideally we should check in a greater radius but might get way too expensive
            unit_distance_to_tiles.keys()
                .sorted_by_key(|tile| -unit.civ.threat_manager.get_distance_to_closest_enemy_unit(tile, 3, false))
                .collect::<Vec<_>>()
        };

        let our_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, false);
        // Lets check all tiles and swap with the first one
        for retreat_tile in sorted_tiles_to_retreat_to {
            let tile_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(retreat_tile, 6, false);
            if our_distance_to_closest_enemy >= tile_distance_to_closest_enemy {
                continue;
            }

            match retreat_tile.military_unit {
                None => {
                    // See if we can retreat to the tile
                    if !unit.movement.can_move_to(retreat_tile) {
                        continue;
                    }
                    unit.movement.move_to_tile(retreat_tile);
                    return true;
                }
                Some(ref other_unit) if other_unit.civ == unit.civ => {
                    // The tile is taken, lets see if we want to swap retreat to it
                    if other_unit.health <= 80 {
                        continue;
                    }
                    if other_unit.base_unit.is_ranged() {
                        // Don't swap ranged units closer than they have to be
                        let range = other_unit.base_unit.range;
                        if our_distance_to_closest_enemy < range {
                            continue;
                        }
                    }
                    if unit.movement.can_unit_swap_to(retreat_tile) {
                        unit.movement.head_towards(retreat_tile); // we need to move through the intermediate tiles
                        // if nothing changed
                        if unit.current_tile.neighbors.contains(other_unit.current_tile)
                            && unit.movement.can_unit_swap_to(retreat_tile) {
                            unit.movement.swap_move_to_tile(retreat_tile);
                        }
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn try_heal_unit(unit: &mut MapUnit) -> bool {
        if unit.base_unit.is_ranged() && unit.has_unique(UniqueType::HealsEvenAfterAction) {
            return false; // will heal anyway, and attacks don't hurt
        }

        // Try pillage improvements until healed
        while Self::try_pillage_improvement(unit, false) {
            // If we are fully healed and can still do things, lets keep on going by returning false
            if !unit.has_movement() || unit.health == 100 {
                return !unit.has_movement();
            }
        }

        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();
        if unit_distance_to_tiles.is_empty() {
            return true; // can't move, so...
        }

        // If the unit can heal on this tile in two turns, just heal here
        if Self::can_unit_heal_in_turns_on_current_tile(unit, 3, None) {
            return true;
        }

        let current_unit_tile = unit.get_tile();
        let dangerous_tiles = unit.civ.threat_manager.get_dangerous_tiles(unit, 4);

        let viable_tiles_for_healing = unit_distance_to_tiles.keys()
            .filter(|tile| !dangerous_tiles.contains(tile) && unit.movement.can_move_to(tile));

        let tiles_by_healing_rate: HashMap<_, Vec<_>> = viable_tiles_for_healing
            .into_iter()
            .group_by(|tile| unit.rank_tile_for_healing(tile))
            .into_iter()
            .collect();

        if tiles_by_healing_rate.keys().all(|&rate| rate == 0) {
            // We can't heal here at all! We're probably embarked
            if !unit.base_unit.moves_like_air_units {
                if let Some(reachable_city_tile) = unit.civ.cities.iter()
                    .map(|city| city.get_center_tile())
                    .sorted_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
                    .find(|tile| unit.movement.can_reach(tile)) {
                    unit.movement.head_towards(reachable_city_tile);
                } else {
                    Self::wander(unit, false, None);
                }
                return true;
            }
            // Try to get closer to an empty city
            let empty_cities = unit.civ.cities.iter()
                .map(|city| city.get_center_tile())
                .filter(|tile| unit.movement.can_move_to(tile));

            if empty_cities.clone().next().is_none() {
                return false; // Nowhere to move to heal
            }

            let next_tile_to_move = unit_distance_to_tiles.keys()
                .filter(|tile| unit.movement.can_move_to(tile))
                .min_by_key(|tile| {
                    empty_cities.clone()
                        .map(|city| city.aerial_distance_to(tile))
                        .min()
                        .unwrap_or(i32::MAX)
                });

            if let Some(tile) = next_tile_to_move {
                unit.movement.move_to_tile(tile);
            }
            return true;
        }

        let best_tiles_for_healing = tiles_by_healing_rate.iter()
            .max_by_key(|&(rate, _)| rate)
            .map(|(_, tiles)| tiles)
            .unwrap();

        let best_tile_for_healing = best_tiles_for_healing.iter()
            .max_by_key(|tile| tile.get_defensive_bonus(Some(unit)))
            .unwrap();

        let best_tile_for_healing_rank = unit.rank_tile_for_healing(best_tile_for_healing);

        if current_unit_tile != *best_tile_for_healing
            && best_tile_for_healing_rank > unit.rank_tile_for_healing(&current_unit_tile) - unit.get_damage_from_terrain() {
            unit.movement.move_to_tile(best_tile_for_healing);
        }

        unit.fortify_if_can();
        true
    }

    fn can_unit_heal_in_turns_on_current_tile(unit: &MapUnit, turns: i32, no_enemy_distance: Option<i32>) -> bool {
        if unit.has_unique(UniqueType::HealsEvenAfterAction) {
            return false; // We can keep on moving
        }

        // Check if we are not in a safe city and there is an enemy nearby this isn't a good tile to heal on
        let no_enemy_distance = no_enemy_distance.unwrap_or(3);
        if !(unit.get_tile().is_city_center() && unit.get_tile().get_city().map_or(false, |city| city.health > 50))
            && unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), no_enemy_distance) <= no_enemy_distance {
            return false;
        }

        let health_required_per_turn = (100 - unit.health) / turns;
        health_required_per_turn <= unit.rank_tile_for_healing(&unit.get_tile())
    }

    fn get_dangerous_tiles(unit: &MapUnit) -> HashSet<Tile> {
        let nearby_enemy_units = unit.current_tile.get_tiles_in_distance(3)
            .iter()
            .flat_map(|tile| tile.get_units())
            .filter(|unit_on_tile| unit.civ.is_at_war_with(&unit_on_tile.civ))
            .collect::<Vec<_>>();

        let tiles_in_range_of_attack = nearby_enemy_units.iter()
            .flat_map(|enemy| {
                enemy.get_tile().get_tiles_in_distance((enemy.get_max_movement() - 1) + enemy.get_range())
            });

        let tiles_within_bombardment_range = unit.current_tile.get_tiles_in_distance(3)
            .iter()
            .filter(|tile| {
                tile.is_city_center()
                    && tile.get_city().map_or(false, |city| city.civ.is_at_war_with(&unit.civ))
            })
            .flat_map(|tile| {
                tile.get_tiles_in_distance(tile.get_city().unwrap().get_bombard_range())
            });

        let tiles_with_terrain_damage = unit.current_tile.get_tiles_in_distance(3)
            .iter()
            .filter(|tile| unit.get_damage_from_terrain(tile) > 0);

        tiles_in_range_of_attack
            .chain(tiles_within_bombardment_range)
            .chain(tiles_with_terrain_damage)
            .collect()
    }

    fn try_advance_towards_close_enemy(unit: &mut MapUnit) -> bool {
        // this can be sped up if we check each layer separately
        let unit_distance_to_tiles = unit.movement.get_movement_to_tiles_at_position(
            unit.get_tile().position,
            (unit.get_max_movement() as f32 * CLOSE_ENEMY_TURNS_AWAY_LIMIT) as i32
        );

        let mut close_enemies = TargetHelper::get_attackable_enemies(
            unit,
            &unit_distance_to_tiles,
            Some(&unit.get_tile().get_tiles_in_distance(CLOSE_ENEMY_TILES_AWAY_LIMIT))
        ).filter(|target| {
            // Ignore units that would 1-shot you if you attacked. Account for taking terrain damage after the fact.
            BattleDamage::calculate_damage_to_attacker(
                &MapUnitCombatant::new(unit),
                &Battle::get_map_combatant_of_tile(&target.tile_to_attack).unwrap()
            ) + unit.get_damage_from_terrain(&target.tile_to_attack_from) < unit.health
        }).collect::<Vec<_>>();

        if unit.base_unit.is_ranged() {
            close_enemies.retain(|target| {
                !(target.tile_to_attack.is_city_center()
                    && target.tile_to_attack.get_city().map_or(false, |city| city.health == 1))
            });
        }

        let closest_enemy = close_enemies.iter()
            .filter(|target| unit.get_damage_from_terrain(&target.tile_to_attack_from) <= 0)  // Don't attack from a mountain
            .min_by_key(|target| target.tile_to_attack.aerial_distance_to(unit.get_tile()));

        if let Some(target) = closest_enemy {
            unit.movement.head_towards(&target.tile_to_attack_from);
            true
        } else {
            false
        }
    }

    fn try_head_towards_our_sieged_city(unit: &mut MapUnit) -> bool {
        let sieged_cities = unit.civ.cities.iter()
            .filter(|city| {
                unit.civ == city.civ
                    && city.health < (city.get_max_health() as f32 * 0.75) as i32
            }); //Weird health issues and making sure that not all forces move to good defenses

        if sieged_cities.clone().any(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()) <= 2) {
            return false;
        }

        let reachable_tile_near_sieged_city = sieged_cities
            .flat_map(|city| city.get_center_tile().get_tiles_at_distance(2))
            .sorted_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
            .find(|tile| {
                unit.movement.can_move_to(tile)
                    && unit.movement.can_reach(tile)
                    && unit.get_damage_from_terrain(tile) <= 0 // Avoid ending up on damaging terrain
            });

        if let Some(tile) = reachable_tile_near_sieged_city {
            unit.movement.head_towards(&tile);
        }
        !unit.has_movement()
    }

    pub fn try_enter_own_closest_city(unit: &mut MapUnit) -> bool {
        let closest_city = unit.civ.cities.iter()
            .map(|city| city.get_center_tile())
            .sorted_by_key(|tile| tile.aerial_distance_to(unit.get_tile()))
            .find(|tile| unit.movement.can_reach(tile));

        match closest_city {
            Some(tile) => {
                unit.movement.head_towards(&tile);
                true
            }
            None => false // Panic!
        }
    }

    pub fn try_bombard_enemy(city: &mut City) -> bool {
        if !city.can_bombard() {
            return false;
        }

        if let Some(enemy) = Self::choose_bombard_target(city) {
            Battle::attack(&CityCombatant::new(city), &enemy);
            true
        } else {
            false
        }
    }

    fn choose_bombard_target(city: &City) -> Option<Box<dyn ICombatant>> {
        let mut targets = TargetHelper::get_bombardable_tiles(city)
            .iter()
            .map(|tile| Battle::get_map_combatant_of_tile(tile).unwrap())
            .filter(|target| {
                !matches!(target, MapUnitCombatant(unit) if unit.is_civilian() && !unit.has_unique(UniqueType::Uncapturable))
            })
            .collect::<Vec<_>>();

        if targets.is_empty() {
            return None;
        }

        let siege_units = targets.iter()
            .filter(|target| {
                matches!(target, MapUnitCombatant(unit) if unit.base_unit.is_probably_siege_unit())
            })
            .collect::<Vec<_>>();

        let non_embarked_siege = siege_units.iter()
            .filter(|target| {
                matches!(target, MapUnitCombatant(unit) if !unit.is_embarked())
            })
            .collect::<Vec<_>>();

        if !non_embarked_siege.is_empty() {
            targets = non_embarked_siege;
        } else if !siege_units.is_empty() {
            targets = siege_units;
        } else {
            let ranged_units = targets.iter()
                .filter(|target| target.is_ranged())
                .collect::<Vec<_>>();
            if !ranged_units.is_empty() {
                targets = ranged_units;
            }
        }

        let hits_to_kill = targets.iter()
            .map(|target| {
                let health = target.get_health() as f32;
                let damage = BattleDamage::calculate_damage_to_defender(
                    &CityCombatant::new(city),
                    target
                ) as f32;
                (target, health / damage.max(1.0))
            })
            .collect::<HashMap<_, _>>();

        hits_to_kill.iter()
            .filter(|(_, hits)| **hits <= 1.0)
            .max_by_key(|(target, _)| target.get_attacking_strength())
            .map(|(target, _)| *target)
            .or_else(|| {
                hits_to_kill.iter()
                    .min_by(|(_, hits1), (_, hits2)| hits1.partial_cmp(hits2).unwrap())
                    .map(|(target, _)| *target)
            })
            .map(|target| Box::new(target.clone()) as Box<dyn ICombatant>)
    }

    pub fn automated_explore(unit: &mut MapUnit) {
        if Self::try_go_to_ruin_and_encampment(unit) && (!unit.has_movement() || unit.is_destroyed) {
            return;
        }
        if unit.health < 80 && Self::try_heal_unit(unit) {
            return;
        }
        if Self::try_explore(unit) {
            return;
        }
        unit.civ.add_notification(
            &format!("{} finished exploring.", unit.short_display_name()),
            MapUnitAction::new(unit),
            NotificationCategory::Units,
            &unit.name,
            "OtherIcons/Sleep"
        );
        unit.action = None;
    }
}