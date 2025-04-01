from typing import List, Optional, Sequence, Set, Tuple
from com.unciv import Constants
from com.unciv.logic.city import City
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.mapunit.movement import PathsToTilesWithinTurn
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant
from com.unciv.logic.battle.city_combatant import CityCombatant
from com.unciv.logic.battle.i_combatant import ICombatant
from com.unciv.logic.battle.battle import Battle
from com.unciv.logic.battle.combat_action import CombatAction
from com.unciv.logic.battle.attackable_tile import AttackableTile

class TargetHelper:
    @staticmethod
    def get_attackable_enemies(
        unit: MapUnit,
        unit_distance_to_tiles: PathsToTilesWithinTurn,
        tiles_to_check: Optional[List[Tile]] = None,
        stay_on_tile: bool = False
    ) -> List[AttackableTile]:
        range_of_attack = unit.get_range()
        attackable_tiles: List[AttackableTile] = []

        unit_must_be_set_up = unit.has_unique(UniqueType.MustSetUp)
        tiles_to_attack_from = (
            [(unit.current_tile, unit.current_movement)]
            if stay_on_tile or unit.base_unit.moves_like_air_units
            else TargetHelper._get_tiles_to_attack_from_when_unit_moves(unit_distance_to_tiles, unit_must_be_set_up, unit)
        )

        tiles_with_enemies: Set[Tile] = set()
        tiles_without_enemies: Set[Tile] = set()
        for reachable_tile, movement_left in tiles_to_attack_from:  # tiles we'll still have energy after we reach there
            # If we are a melee unit that is escorting, we only want to be able to attack from this
            # tile if the escorted unit can also move into the tile we are attacking if we kill the enemy unit.
            if unit.base_unit.is_melee() and unit.is_escorting():
                escorting_unit = unit.get_other_escort_unit()
                if escorting_unit is None:
                    continue
                if (not escorting_unit.movement.can_reach_in_current_turn(reachable_tile)
                    or escorting_unit.current_movement - escorting_unit.movement.get_distance_to_tiles()[reachable_tile].total_movement <= 0.0):
                    continue

            tiles_in_attack_range = (
                reachable_tile.neighbors
                if unit.base_unit.is_melee()
                else (
                    reachable_tile.get_tiles_in_distance(range_of_attack)
                    if unit.base_unit.moves_like_air_units or unit.has_unique(UniqueType.IndirectFire, check_civ_info_uniques=True)
                    else reachable_tile.tile_map.get_viewable_tiles(reachable_tile.position, range_of_attack, True)
                )
            )

            for tile in tiles_in_attack_range:
                if tile == reachable_tile:  # Since military units can technically enter tiles with enemy civilians,
                    continue  # some try to move to to the tile and then attack the unit it contains, which is silly

                if tile in tiles_with_enemies:
                    attackable_tiles.append(AttackableTile(
                        reachable_tile,
                        tile,
                        movement_left,
                        Battle.get_map_combatant_of_tile(tile)
                    ))
                    continue

                if tile in tiles_without_enemies:
                    continue  # avoid checking the same empty tile multiple times

                if TargetHelper._tile_contains_attackable_enemy(unit, tile, tiles_to_check) or unit.is_preparing_air_sweep():
                    tiles_with_enemies.add(tile)
                    attackable_tiles.append(AttackableTile(
                        reachable_tile,
                        tile,
                        movement_left,
                        Battle.get_map_combatant_of_tile(tile)
                    ))
                else:
                    tiles_without_enemies.add(tile)

        return attackable_tiles

    @staticmethod
    def _get_tiles_to_attack_from_when_unit_moves(
        unit_distance_to_tiles: PathsToTilesWithinTurn,
        unit_must_be_set_up: bool,
        unit: MapUnit
    ) -> List[Tuple[Tile, float]]:
        def process_tile(tile: Tile, distance: PathsToTilesWithinTurn.PathToTile) -> Optional[Tuple[Tile, float]]:
            movement_points_to_expend_after_movement = 1 if unit_must_be_set_up else 0
            movement_points_to_expend_here = 1 if unit_must_be_set_up and not unit.is_set_up_for_siege() else 0
            movement_points_to_expend_before_attack = (
                movement_points_to_expend_here
                if tile == unit.current_tile
                else movement_points_to_expend_after_movement
            )
            movement_left = unit.current_movement - distance.total_movement - movement_points_to_expend_before_attack
            return (tile, movement_left) if movement_left > Constants.minimum_movement_epsilon else None

        return [
            result for result in (
                process_tile(tile, distance)
                for tile, distance in unit_distance_to_tiles.items()
            )
            if result is not None
            and (result[0] == unit.get_tile() or unit.movement.can_move_to(result[0]))
        ]

    @staticmethod
    def _tile_contains_attackable_enemy(unit: MapUnit, tile: Tile, tiles_to_check: Optional[List[Tile]] = None) -> bool:
        if tile not in (tiles_to_check or unit.civ.viewable_tiles) or not TargetHelper.contains_attackable_enemy(tile, MapUnitCombatant(unit)):
            return False

        map_combatant = Battle.get_map_combatant_of_tile(tile)
        if map_combatant is None:
            return False

        return (
            not unit.base_unit.is_melee()
            or not isinstance(map_combatant, MapUnitCombatant)
            or not map_combatant.unit.is_civilian()
            or unit.movement.can_pass_through(tile)
        )

    @staticmethod
    def contains_attackable_enemy(tile: Tile, combatant: ICombatant) -> bool:
        if isinstance(combatant, MapUnitCombatant):
            if combatant.unit.is_embarked() and not combatant.has_unique(UniqueType.AttackOnSea):
                # Can't attack water units while embarked, only land
                if tile.is_water or combatant.is_ranged():
                    return False

        tile_combatant = Battle.get_map_combatant_of_tile(tile)
        if tile_combatant is None:
            return False

        if tile_combatant.get_civ_info() == combatant.get_civ_info():
            return False

        # If the user automates units, one may capture the city before the user had a chance to decide what to do with it,
        #  and then the next unit should not attack that city
        if isinstance(tile_combatant, CityCombatant) and tile_combatant.city.has_just_been_conquered:
            return False

        if not combatant.get_civ_info().is_at_war_with(tile_combatant.get_civ_info()):
            return False

        if (isinstance(combatant, MapUnitCombatant)
            and combatant.is_land_unit()
            and combatant.is_melee()
            and tile.is_water
            and not combatant.get_civ_info().tech.units_can_embark
            and not combatant.unit.cache.can_move_on_water):
            return False

        if isinstance(combatant, MapUnitCombatant):
            state_for_conditionals = StateForConditionals(
                unit=combatant.unit,
                tile=tile,
                our_combatant=combatant,
                their_combatant=tile_combatant,
                combat_action=CombatAction.Attack
            )

            if combatant.has_unique(UniqueType.CannotAttack, state_for_conditionals):
                return False

            if any(
                not tile_combatant.matches_filter(unique.params[0])
                for unique in combatant.unit.get_matching_uniques(UniqueType.CanOnlyAttackUnits, state_for_conditionals)
            ):
                return False

            if any(
                not tile.matches_filter(unique.params[0])
                for unique in combatant.unit.get_matching_uniques(UniqueType.CanOnlyAttackTiles, state_for_conditionals)
            ):
                return False

        # Only units with the right unique can view submarines (or other invisible units) from more then one tile away.
        # Garrisoned invisible units can be attacked by anyone, as else the city will be in invincible.
        if tile_combatant.is_invisible(combatant.get_civ_info()) and not tile.is_city_center():
            return (
                isinstance(combatant, MapUnitCombatant)
                and tile.position in {
                    tile.position for tile in combatant.get_civ_info().viewable_invisible_units_tiles
                }
            )

        return True

    @staticmethod
    def get_bombardable_tiles(city: City) -> Sequence[Tile]:
        """Get a list of visible tiles which have something attackable"""
        return [
            tile for tile in city.get_center_tile().get_tiles_in_distance(city.get_bombard_range())
            if tile.is_visible(city.civ) and TargetHelper.contains_attackable_enemy(tile, CityCombatant(city))
        ]