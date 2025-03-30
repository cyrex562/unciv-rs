from typing import List, Dict, Set, Optional, Tuple
from dataclasses import dataclass
import math

from com.unciv.logic.battle import AirInterception, Battle, MapUnitCombatant, Nuke, TargetHelper
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.logic.automation.unit import BattleHelper, HeadTowardsEnemyCityAutomation

class AirUnitAutomation:
    """Handles AI automation for air units."""

    @staticmethod
    def automate_fighter(unit: MapUnit) -> None:
        """Automate fighter unit behavior.
        
        Args:
            unit: The fighter unit to automate
        """
        if unit.health < 75:
            return  # Wait and heal

        tiles_with_enemy_units_in_range = unit.civ.threat_manager.get_tiles_with_enemy_units_in_distance(
            unit.get_tile(), unit.get_range()
        )
        # TODO: Optimize [friendlyAirUnitsInRange] by creating an alternate [ThreatManager.getTilesWithEnemyUnitsInDistance] that handles only friendly units
        friendly_air_units_in_range = [
            air_unit for tile in unit.get_tile().get_tiles_in_distance(unit.get_range())
            for air_unit in tile.air_units
            if air_unit.civ == unit.civ
        ]
        
        # Find all visible enemy air units
        enemy_air_units_in_range = [
            air_unit for tile in tiles_with_enemy_units_in_range
            for air_unit in tile.air_units
            if air_unit.civ.is_at_war_with(unit.civ)
        ]
        
        enemy_fighters = len(enemy_air_units_in_range) // 2  # Assume half the planes are fighters
        friendly_unused_fighter_count = sum(
            1 for u in friendly_air_units_in_range
            if u.health >= 50 and u.can_attack()
        )
        friendly_used_fighter_count = sum(
            1 for u in friendly_air_units_in_range
            if u.health >= 50 and not u.can_attack()
        )

        # We need to be on standby in case they attack
        if friendly_unused_fighter_count < enemy_fighters:
            return

        if friendly_used_fighter_count <= enemy_fighters:
            def air_sweep_damage_percent_bonus() -> int:
                return sum(
                    int(unique.params[0])
                    for unique in unit.get_matching_uniques(UniqueType.StrengthWhenAirsweep)
                )

            # If we are outnumbered, don't heal after attacking and don't have an Air Sweep bonus
            # Then we shouldn't speed the air battle by killing our fighters, instead, focus on defending
            if (friendly_used_fighter_count + friendly_unused_fighter_count < enemy_fighters
                and not unit.has_unique(UniqueType.HealsEvenAfterAction)
                and air_sweep_damage_percent_bonus() <= 0):
                return
            else:
                if AirUnitAutomation._try_air_sweep(unit, tiles_with_enemy_units_in_range):
                    return

        if unit.health < 80:
            return  # Wait and heal up, no point in moving closer to battle if we aren't healed

        if BattleHelper.try_attack_nearby_enemy(unit):
            return

        if unit.cache.cannot_move:
            return  # from here on it's all "try to move somewhere else"

        if AirUnitAutomation._try_relocate_to_cities_with_enemy_near_by(unit):
            return

        paths_to_cities = unit.movement.get_aerial_paths_to_cities()
        if not paths_to_cities:
            return  # can't actually move anywhere else

        cities_by_nearby_air_units = {}
        for city in paths_to_cities:
            nearby_enemy_air_units = sum(
                1 for tile in city.get_tiles_in_distance(unit.get_max_movement_for_air_units())
                if tile.air_units and tile.air_units[0].civ.is_at_war_with(unit.civ)
            )
            if nearby_enemy_air_units not in cities_by_nearby_air_units:
                cities_by_nearby_air_units[nearby_enemy_air_units] = []
            cities_by_nearby_air_units[nearby_enemy_air_units].append(city)

        if any(count != 0 for count in cities_by_nearby_air_units):
            cities_with_most_need = cities_by_nearby_air_units[max(cities_by_nearby_air_units.keys())]
            # todo: maybe group by size and choose highest priority within the same size turns
            chosen_city = min(
                cities_with_most_need,
                key=lambda c: len(paths_to_cities[c])
            )  # city with min path = least turns to get there
            first_step = paths_to_cities[chosen_city][0]
            unit.movement.move_to_tile(first_step)
            return

        # no city needs fighters to defend, so let's attack stuff from the closest possible location
        AirUnitAutomation._try_move_to_cities_to_aerial_attack_from(paths_to_cities, unit)

    @staticmethod
    def _try_air_sweep(unit: MapUnit, tiles_with_enemy_units_in_range: List[Tile]) -> bool:
        """Attempt to perform an air sweep.
        
        Args:
            unit: The unit performing the sweep
            tiles_with_enemy_units_in_range: List of tiles with enemy units
            
        Returns:
            bool: Whether the sweep was successful
        """
        target_tile = min(
            (tile for tile in tiles_with_enemy_units_in_range
             if any(u.civ.is_at_war_with(unit.civ) for u in tile.get_units())
             or (tile.is_city_center() and tile.get_city().civ.is_at_war_with(unit.civ))),
            key=lambda t: t.aerial_distance_to(unit.get_tile()),
            default=None
        )
        
        if target_tile is None:
            return False
            
        AirInterception.air_sweep(MapUnitCombatant(unit), target_tile)
        return not unit.has_movement()

    @staticmethod
    def automate_bomber(unit: MapUnit) -> None:
        """Automate bomber unit behavior.
        
        Args:
            unit: The bomber unit to automate
        """
        if unit.health < 75:
            return  # Wait and heal

        if BattleHelper.try_attack_nearby_enemy(unit):
            return

        if unit.health <= 90 or (unit.health < 100 and not unit.civ.is_at_war()):
            return  # Wait and heal

        if unit.cache.cannot_move:
            return  # from here on it's all "try to move somewhere else"

        if AirUnitAutomation._try_relocate_to_cities_with_enemy_near_by(unit):
            return

        paths_to_cities = unit.movement.get_aerial_paths_to_cities()
        if not paths_to_cities:
            return  # can't actually move anywhere else
            
        AirUnitAutomation._try_move_to_cities_to_aerial_attack_from(paths_to_cities, unit)

    @staticmethod
    def _try_move_to_cities_to_aerial_attack_from(
        paths_to_cities: Dict[Tile, List[Tile]], 
        air_unit: MapUnit
    ) -> None:
        """Try to move to cities that can attack from their position.
        
        Args:
            paths_to_cities: Dictionary mapping cities to paths to reach them
            air_unit: The air unit to move
        """
        cities_that_can_attack_from = [
            destination_city for destination_city in paths_to_cities
            if (destination_city != air_unit.current_tile
                and any(
                    it.is_city_center() and it.get_owner().is_at_war_with(air_unit.civ)
                    for it in destination_city.get_tiles_in_distance(air_unit.get_range())
                ))
        ]
        
        if not cities_that_can_attack_from:
            return

        # todo: this logic looks similar to some parts of automateFighter, maybe pull out common code
        # todo: maybe group by size and choose highest priority within the same size turns
        closest_city = min(
            cities_that_can_attack_from,
            key=lambda c: len(paths_to_cities[c])
        )
        first_step = paths_to_cities[closest_city][0]
        air_unit.movement.move_to_tile(first_step)

    @staticmethod
    def automate_nukes(unit: MapUnit) -> None:
        """Automate nuclear unit behavior.
        
        Args:
            unit: The nuclear unit to automate
        """
        if not unit.civ.is_at_war():
            return
            
        # We should *Almost* never want to nuke our own city, so don't consider it
        if unit.type.is_air_unit():
            tiles_in_range = unit.current_tile.get_tiles_in_distance_range(range(2, unit.get_range() + 1))
            highest_tile_nuke_value = max(
                ((tile, AirUnitAutomation._get_nuke_location_value(unit, tile))
                 for tile in tiles_in_range),
                key=lambda x: x[1],
                default=None
            )
            if highest_tile_nuke_value and highest_tile_nuke_value[1] > 0:
                Nuke.NUKE(MapUnitCombatant(unit), highest_tile_nuke_value[0])

            AirUnitAutomation._try_relocate_missile_to_nearby_attackable_cities(unit)
        else:
            attackable_tiles = TargetHelper.get_attackable_enemies(unit, unit.movement.get_distance_to_tiles())
            highest_tile_nuke_value = max(
                ((tile, AirUnitAutomation._get_nuke_location_value(unit, tile.tile_to_attack))
                 for tile in attackable_tiles),
                key=lambda x: x[1],
                default=None
            )
            if highest_tile_nuke_value and highest_tile_nuke_value[1] > 0:
                Battle.move_and_attack(MapUnitCombatant(unit), highest_tile_nuke_value[0])
            HeadTowardsEnemyCityAutomation.try_head_towards_enemy_city(unit)

    @staticmethod
    def _get_nuke_location_value(nuke: MapUnit, tile: Tile) -> int:
        """Calculate the value of nuking a specific location.
        
        Args:
            nuke: The nuclear unit
            tile: The target tile
            
        Returns:
            int: The calculated value of nuking this location
        """
        civ = nuke.civ
        if not Nuke.may_use_nuke(MapUnitCombatant(nuke), tile):
            return float('-inf')
            
        blast_radius = nuke.get_nuke_blast_radius()
        tiles_in_blast_radius = tile.get_tiles_in_distance(blast_radius)
        civs_in_blast_radius = (
            [t.get_owner() for t in tiles_in_blast_radius if t.get_owner()]
            + [u.civ for t in tiles_in_blast_radius for u in t.get_units() if u]
        )

        # Don't nuke if it means we will be declaring war on someone!
        if any(c != civ and not c.is_at_war_with(civ) for c in civs_in_blast_radius):
            return -100000
        # If there are no enemies to hit, don't nuke
        if not any(c.is_at_war_with(civ) for c in civs_in_blast_radius):
            return -100000

        # Launching a Nuke uses resources, therefore don't launch it by default
        explosion_value = -500

        def evaluate_civ_value(target_civ: Civilization, our_value: int, their_value: int) -> int:
            """Evaluate the value of nuking a target civilization.
            
            Args:
                target_civ: The target civilization
                our_value: Value if it's our civilization
                their_value: Value if it's an enemy civilization
                
            Returns:
                int: The evaluated value
            """
            if target_civ == civ:  # We are nuking something that we own!
                return our_value
            return their_value  # We are nuking an enemy!

        for target_tile in tiles_in_blast_radius:
            # We can only account for visible units
            if target_tile.is_visible(civ):
                for target_unit in target_tile.get_units():
                    if target_unit.is_invisible(civ):
                        continue
                    # If we are nuking a unit at ground zero, it is more likely to be destroyed
                    tile_explosion_value = 80 if target_tile == tile else 50

                    if target_unit.is_military():
                        explosion_value += (
                            evaluate_civ_value(target_unit.civ, -200, tile_explosion_value)
                            if target_tile == tile
                            else evaluate_civ_value(target_unit.civ, -150, 50)
                        )
                    elif target_unit.is_civilian():
                        explosion_value += evaluate_civ_value(
                            target_unit.civ, -100, tile_explosion_value // 2
                        )

            # Never nuke our own Civ, don't nuke single enemy civs as well
            if (target_tile.is_city_center()
                and not (target_tile.get_city().health <= 50
                        and any(it.military_unit.civ == civ for it in target_tile.neighbors))):  # Prefer not to nuke cities that we are about to take
                explosion_value += evaluate_civ_value(target_tile.get_city().civ, -100000, 250)
            elif target_tile.owning_city:
                owning_civ = target_tile.owning_city.civ
                # If there is a tile to add fallout to there is a 50% chance it will get fallout
                if not (tile.is_water or tile.is_impassible() or target_tile.has_fallout_equivalent()):
                    explosion_value += evaluate_civ_value(owning_civ, -40, 10)
                # If there is an improvement to pillage
                if target_tile.improvement and not target_tile.improvement_is_pillaged:
                    explosion_value += evaluate_civ_value(owning_civ, -40, 20)
                    
            # If the value is too low end the search early
            if explosion_value < -1000:
                return explosion_value
                
        return explosion_value

    @staticmethod
    def automate_missile(unit: MapUnit) -> None:
        """Automate missile unit behavior.
        
        Args:
            unit: The missile unit to automate
        """
        if BattleHelper.try_attack_nearby_enemy(unit):
            return
            
        AirUnitAutomation._try_relocate_missile_to_nearby_attackable_cities(unit)

    @staticmethod
    def _try_relocate_missile_to_nearby_attackable_cities(unit: MapUnit) -> None:
        """Try to relocate missile to cities that can attack from their position.
        
        Args:
            unit: The missile unit to relocate
        """
        tiles_in_range = unit.current_tile.get_tiles_in_distance(unit.get_range())
        immediately_reachable_cities = [
            city for city in tiles_in_range
            if unit.movement.can_move_to(city)
        ]

        for city in immediately_reachable_cities:
            if any(
                it.is_city_center() and it.get_owner().is_at_war_with(unit.civ)
                for it in city.get_tiles_in_distance(unit.get_range())
            ):
                unit.movement.move_to_tile(city)
                return

        paths_to_cities = unit.movement.get_aerial_paths_to_cities()
        if not paths_to_cities:
            return  # can't actually move anywhere else
            
        AirUnitAutomation._try_move_to_cities_to_aerial_attack_from(paths_to_cities, unit)

    @staticmethod
    def _try_relocate_to_cities_with_enemy_near_by(unit: MapUnit) -> bool:
        """Try to relocate unit to cities with enemies nearby.
        
        Args:
            unit: The unit to relocate
            
        Returns:
            bool: Whether relocation was successful
        """
        immediately_reachable_cities_and_carriers = [
            city for city in unit.current_tile.get_tiles_in_distance(unit.get_max_movement_for_air_units())
            if unit.movement.can_move_to(city)
        ]

        for city in immediately_reachable_cities_and_carriers:
            if any(
                it.is_visible(unit.civ)
                and TargetHelper.contains_attackable_enemy(it, MapUnitCombatant(unit))
                for it in city.get_tiles_in_distance(unit.get_range())
            ):
                unit.movement.move_to_tile(city)
                return True
                
        return False 