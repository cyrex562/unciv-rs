use crate::battle::battle::Battle;
use crate::battle::city_combatant::CityCombatant;
use crate::battle::i_combatant::ICombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::city::City;
use crate::constants::Constants;
use crate::map::map_unit::MapUnit;
use crate::map::map_unit::movement::paths_to_tiles_within_turn::PathsToTilesWithinTurn;
use crate::map::tile::Tile;
use crate::models::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::models::ruleset::unique::unique_type::UniqueType;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Module for determining which tiles a unit can attack and which enemies are attackable
pub mod TargetHelper {
    use super::*;

    /// Gets a list of tiles that can be attacked by a unit
    ///
    /// # Arguments
    ///
    /// * `unit` - The unit that will be attacking
    /// * `unit_distance_to_tiles` - The paths to tiles within the unit's movement range
    /// * `tiles_to_check` - Optional list of tiles to check for enemies
    /// * `stay_on_tile` - Whether the unit should stay on its current tile
    ///
    /// # Returns
    ///
    /// A list of attackable tiles
    pub fn get_attackable_enemies(
        unit: &MapUnit,
        unit_distance_to_tiles: &PathsToTilesWithinTurn,
        tiles_to_check: Option<&[Arc<Tile>]>,
        stay_on_tile: bool
    ) -> Vec<AttackableTile> {
        let range_of_attack = unit.get_range();
        let mut attackable_tiles = Vec::new();

        let unit_must_be_set_up = unit.has_unique(UniqueType::MustSetUp);
        let tiles_to_attack_from = if stay_on_tile || unit.base_unit.moves_like_air_units {
            vec![(unit.current_tile.clone(), unit.current_movement)]
        } else {
            get_tiles_to_attack_from_when_unit_moves(unit_distance_to_tiles, unit_must_be_set_up, unit)
        };

        let mut tiles_with_enemies: HashSet<Arc<Tile>> = HashSet::new();
        let mut tiles_without_enemies: HashSet<Arc<Tile>> = HashSet::new();

        for (reachable_tile, movement_left) in tiles_to_attack_from {
            // If we are a melee unit that is escorting, we only want to be able to attack from this
            // tile if the escorted unit can also move into the tile we are attacking if we kill the enemy unit.
            if unit.base_unit.is_melee() && unit.is_escorting() {
                if let Some(escorting_unit) = unit.get_other_escort_unit() {
                    if !escorting_unit.movement.can_reach_in_current_turn(&reachable_tile)
                        || escorting_unit.current_movement - escorting_unit.movement.get_distance_to_tiles().get(&reachable_tile)
                            .map_or(0.0, |distance| distance.total_movement) <= 0.0 {
                        continue;
                    }
                }
            }

            let tiles_in_attack_range = if unit.base_unit.is_melee() {
                reachable_tile.neighbors.iter().cloned().collect::<Vec<_>>()
            } else if unit.base_unit.moves_like_air_units || unit.has_unique(UniqueType::IndirectFire, check_civ_info_uniques: true) {
                reachable_tile.get_tiles_in_distance(range_of_attack)
            } else {
                reachable_tile.tile_map.get_viewable_tiles(reachable_tile.position, range_of_attack, true).iter().cloned().collect::<Vec<_>>()
            };

            for tile in tiles_in_attack_range {
                if tile == reachable_tile {
                    continue; // Since military units can technically enter tiles with enemy civilians,
                    // some try to move to to the tile and then attack the unit it contains, which is silly
                }

                if tiles_with_enemies.contains(&tile) {
                    attackable_tiles.push(AttackableTile::new(
                        reachable_tile.clone(),
                        tile.clone(),
                        movement_left,
                        Battle::get_map_combatant_of_tile(&tile)
                    ));
                } else if tiles_without_enemies.contains(&tile) {
                    continue; // avoid checking the same empty tile multiple times
                } else if tile_contains_attackable_enemy(unit, &tile, tiles_to_check) || unit.is_preparing_air_sweep() {
                    tiles_with_enemies.insert(tile.clone());
                    attackable_tiles.push(AttackableTile::new(
                        reachable_tile.clone(),
                        tile.clone(),
                        movement_left,
                        Battle::get_map_combatant_of_tile(&tile)
                    ));
                } else {
                    tiles_without_enemies.insert(tile.clone());
                }
            }
        }

        attackable_tiles
    }

    /// Gets the tiles a unit can attack from when it moves
    fn get_tiles_to_attack_from_when_unit_moves(
        unit_distance_to_tiles: &PathsToTilesWithinTurn,
        unit_must_be_set_up: bool,
        unit: &MapUnit
    ) -> Vec<(Arc<Tile>, f32)> {
        unit_distance_to_tiles.iter()
            .map(|(tile, distance)| {
                let movement_points_to_expend_after_movement = if unit_must_be_set_up { 1.0 } else { 0.0 };
                let movement_points_to_expend_here = if unit_must_be_set_up && !unit.is_set_up_for_siege() { 1.0 } else { 0.0 };
                let movement_points_to_expend_before_attack = if tile == unit.current_tile {
                    movement_points_to_expend_here
                } else {
                    movement_points_to_expend_after_movement
                };
                let movement_left = unit.current_movement - distance.total_movement - movement_points_to_expend_before_attack;
                (tile.clone(), movement_left)
            })
            // still got leftover movement points after all that, to attack
            .filter(|(_, movement_left)| *movement_left > Constants::minimum_movement_epsilon)
            .filter(|(tile, _)| *tile == unit.get_tile() || unit.movement.can_move_to(tile))
            .collect()
    }

    /// Checks if a tile contains an attackable enemy
    fn tile_contains_attackable_enemy(unit: &MapUnit, tile: &Tile, tiles_to_check: Option<&[Arc<Tile>]>) -> bool {
        let viewable_tiles = tiles_to_check.unwrap_or(&unit.civ.viewable_tiles);
        if !viewable_tiles.contains(tile) || !contains_attackable_enemy(tile, &MapUnitCombatant::new(unit.clone())) {
            return false;
        }

        let map_combatant = Battle::get_map_combatant_of_tile(tile);

        !unit.base_unit.is_melee() ||
            map_combatant.map_or(true, |combatant| {
                !(combatant.is::<MapUnitCombatant>() && combatant.downcast_ref::<MapUnitCombatant>().map_or(false, |c| c.unit.is_civilian()) && !unit.movement.can_pass_through(tile))
            })
    }

    /// Checks if a tile contains an attackable enemy for a combatant
    pub fn contains_attackable_enemy(tile: &Tile, combatant: &dyn ICombatant) -> bool {
        if let Some(map_unit_combatant) = combatant.downcast_ref::<MapUnitCombatant>() {
            if map_unit_combatant.unit.is_embarked() && !map_unit_combatant.has_unique(UniqueType::AttackOnSea) {
                // Can't attack water units while embarked, only land
                if tile.is_water || map_unit_combatant.is_ranged() {
                    return false;
                }
            }
        }

        let tile_combatant = match Battle::get_map_combatant_of_tile(tile) {
            Some(combatant) => combatant,
            None => return false,
        };

        if tile_combatant.get_civ_info() == combatant.get_civ_info() {
            return false;
        }

        // If the user automates units, one may capture the city before the user had a chance to decide what to do with it,
        // and then the next unit should not attack that city
        if let Some(city_combatant) = tile_combatant.downcast_ref::<CityCombatant>() {
            if city_combatant.city.has_just_been_conquered {
                return false;
            }
        }

        if !combatant.get_civ_info().is_at_war_with(tile_combatant.get_civ_info()) {
            return false;
        }

        if let Some(map_unit_combatant) = combatant.downcast_ref::<MapUnitCombatant>() {
            if map_unit_combatant.is_land_unit() && map_unit_combatant.is_melee() && tile.is_water &&
                !map_unit_combatant.get_civ_info().tech.units_can_embark && !map_unit_combatant.unit.cache.can_move_on_water {
                return false;
            }
        }

        if let Some(map_unit_combatant) = combatant.downcast_ref::<MapUnitCombatant>() {
            let state_for_conditionals = StateForConditionals::new(
                unit: Some(map_unit_combatant.unit.clone()),
                tile: Some(tile.clone()),
                our_combatant: Some(map_unit_combatant.clone()),
                their_combatant: Some(tile_combatant.clone()),
                combat_action: Some(CombatAction::Attack)
            );

            if map_unit_combatant.has_unique(UniqueType::CannotAttack, Some(&state_for_conditionals)) {
                return false;
            }

            let can_only_attack_units = map_unit_combatant.unit.get_matching_uniques(UniqueType::CanOnlyAttackUnits, &state_for_conditionals);
            if !can_only_attack_units.is_empty() && !can_only_attack_units.iter().any(|unique| tile_combatant.matches_filter(&unique.params[0])) {
                return false;
            }

            let can_only_attack_tiles = map_unit_combatant.unit.get_matching_uniques(UniqueType::CanOnlyAttackTiles, &state_for_conditionals);
            if !can_only_attack_tiles.is_empty() && !can_only_attack_tiles.iter().any(|unique| tile.matches_filter(&unique.params[0])) {
                return false;
            }
        }

        // Only units with the right unique can view submarines (or other invisible units) from more then one tile away.
        // Garrisoned invisible units can be attacked by anyone, as else the city will be in invincible.
        if tile_combatant.is_invisible(combatant.get_civ_info()) && !tile.is_city_center() {
            if let Some(map_unit_combatant) = combatant.downcast_ref::<MapUnitCombatant>() {
                return map_unit_combatant.get_civ_info().viewable_invisible_units_tiles.iter()
                    .any(|t| t.position == tile.position);
            }
            return false;
        }

        true
    }

    /// Get a list of visible tiles which have something attackable
    pub fn get_bombardable_tiles(city: &City) -> Vec<Arc<Tile>> {
        city.get_center_tile().get_tiles_in_distance(city.get_bombard_range())
            .into_iter()
            .filter(|tile| tile.is_visible(&city.civ) && contains_attackable_enemy(tile, &CityCombatant::new(city.clone())))
            .collect()
    }
}

/// Represents a tile that can be attacked by a unit
#[derive(Debug, Clone)]
pub struct AttackableTile {
    /// The tile the unit will attack from
    pub from_tile: Arc<Tile>,
    /// The tile being attacked
    pub to_tile: Arc<Tile>,
    /// The movement points left after moving to the from_tile
    pub movement_left: f32,
    /// The combatant on the to_tile, if any
    pub combatant: Option<Box<dyn ICombatant>>,
}

impl AttackableTile {
    /// Creates a new AttackableTile
    pub fn new(from_tile: Arc<Tile>, to_tile: Arc<Tile>, movement_left: f32, combatant: Option<Box<dyn ICombatant>>) -> Self {
        AttackableTile {
            from_tile,
            to_tile,
            movement_left,
            combatant,
        }
    }
}