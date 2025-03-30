from typing import List, Optional, Set
from dataclasses import dataclass

from com.unciv.logic.automation.civilization import NextTurnAutomation
from com.unciv.logic.battle import BattleDamage, CityCombatant, MapUnitCombatant
from com.unciv.logic.city import City
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.mapunit.movement import PathsToTilesWithinTurn
from com.unciv.logic.map.tile import Tile

# Constants
MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA = 5
MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA = 3

class HeadTowardsEnemyCityAutomation:
    """Handles AI automation for units moving towards enemy cities."""

    @staticmethod
    def try_head_towards_enemy_city(unit: MapUnit) -> bool:
        """Try to move the unit towards an enemy city.
        
        Args:
            unit: The unit to move
            
        Returns:
            bool: Whether the unit has taken this action
        """
        if not unit.civ.cities:
            return False

        # Only focus on *attacking* 1 enemy at a time otherwise you'll lose on both fronts
        closest_reachable_enemy_city = next(
            (city for city in HeadTowardsEnemyCityAutomation._get_enemy_cities_by_priority(unit)
             if unit.movement.can_reach(city.get_center_tile())),
            None
        )
        
        if not closest_reachable_enemy_city:
            return False  # No enemy city reachable

        return HeadTowardsEnemyCityAutomation.head_towards_enemy_city(
            unit,
            closest_reachable_enemy_city.get_center_tile(),
            # This should be cached after the `can_reach` call above.
            unit.movement.get_shortest_path(closest_reachable_enemy_city.get_center_tile())
        )

    @staticmethod
    def _get_enemy_cities_by_priority(unit: MapUnit) -> List[City]:
        """Get list of enemy cities sorted by priority.
        
        Args:
            unit: The unit to consider
            
        Returns:
            List[City]: List of enemy cities sorted by priority
        """
        enemies = [
            civ for civ in unit.civ.get_known_civs()
            if unit.civ.is_at_war_with(civ) and civ.cities
        ]

        closest_enemy_city = min(
            (NextTurnAutomation.get_closest_cities(unit.civ, enemy)
             for enemy in enemies),
            key=lambda x: x.aerial_distance if x else float('inf'),
            default=None
        )
        
        if not closest_enemy_city:
            return []  # No attackable cities found

        # Our main attack target is the closest city, but we're fine with deviating from that a bit
        enemy_cities_by_priority = [
            city for city in closest_enemy_city.city2.civ.cities
            if city.get_center_tile().aerial_distance_to(closest_enemy_city.city2.get_center_tile()) <= 10
        ]
        enemy_cities_by_priority.sort(
            key=lambda city: city.get_center_tile().aerial_distance_to(
                closest_enemy_city.city2.get_center_tile()
            )
        )

        if unit.base_unit.is_ranged():  # Ranged units don't harm capturable cities, waste of a turn
            enemy_cities_by_priority = [
                city for city in enemy_cities_by_priority
                if city.health > 1
            ]

        return enemy_cities_by_priority

    @staticmethod
    def head_towards_enemy_city(
        unit: MapUnit,
        closest_reachable_enemy_city: Tile,
        shortest_path: List[Tile]
    ) -> bool:
        """Move the unit towards an enemy city.
        
        Args:
            unit: The unit to move
            closest_reachable_enemy_city: The target city's center tile
            shortest_path: The shortest path to the city
            
        Returns:
            bool: Whether the unit has taken this action
        """
        unit_distance_to_tiles = unit.movement.get_distance_to_tiles()
        unit_range = unit.get_range()

        if unit_range > 2:  # Long-ranged unit, should never be in a bombardable position
            return HeadTowardsEnemyCityAutomation._head_towards_enemy_city_long_range(
                closest_reachable_enemy_city,
                unit_distance_to_tiles,
                unit_range,
                unit
            )

        next_tile_in_path = shortest_path[0]

        # None of the stuff below is relevant if we're still quite far away from the city
        if (unit.current_tile.aerial_distance_to(closest_reachable_enemy_city) 
            > MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA
            and shortest_path.size > MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA):
            unit.movement.move_to_tile(next_tile_in_path)
            return True

        our_units_around_enemy_city = [
            unit for tile in closest_reachable_enemy_city.get_tiles_in_distance(6)
            for unit in tile.get_units()
            if unit.is_military() and unit.civ == unit.civ
        ]

        city = closest_reachable_enemy_city.get_city()

        if HeadTowardsEnemyCityAutomation._cannot_take_city_soon(our_units_around_enemy_city, city):
            return HeadTowardsEnemyCityAutomation._head_to_landing_grounds(
                closest_reachable_enemy_city,
                unit
            )

        unit.movement.move_to_tile(next_tile_in_path)  # Go for it!
        return True

    @staticmethod
    def _cannot_take_city_soon(
        our_units_around_enemy_city: List[MapUnit],
        city: City
    ) -> bool:
        """Check if we cannot take the city soon.
        
        Args:
            our_units_around_enemy_city: List of our units near the city
            city: The target city
            
        Returns:
            bool: Whether we cannot take the city soon
        """
        city_combatant = CityCombatant(city)
        expected_damage_per_turn = sum(
            BattleDamage.calculate_damage_to_defender(
                MapUnitCombatant(unit),
                city_combatant
            )
            for unit in our_units_around_enemy_city
        )

        city_healing_per_turn = 20
        return (expected_damage_per_turn < city.health  # Cannot take immediately
                and (expected_damage_per_turn <= city_healing_per_turn  # No lasting damage
                     or city.health / (expected_damage_per_turn - city_healing_per_turn) > 5))  # Will take more than 5 turns

    @staticmethod
    def _head_to_landing_grounds(
        closest_reachable_enemy_city: Tile,
        unit: MapUnit
    ) -> bool:
        """Move the unit to a landing ground near the city.
        
        Args:
            closest_reachable_enemy_city: The target city's center tile
            unit: The unit to move
            
        Returns:
            bool: Whether the unit has taken this action
        """
        # Don't head straight to the city, try to head to landing grounds -
        # This is against the AI's brilliant plan of having everyone embarked and attacking via sea when unnecessary.
        landing_tiles = [
            tile for tile in closest_reachable_enemy_city.get_tiles_in_distance_range(
                MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA,
                MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA
            )
            if (tile.is_land
                and unit.get_damage_from_terrain(tile) <= 0  # Don't head for hurty terrain
                and (unit.movement.can_move_to(tile) or tile == unit.current_tile)
                and unit.movement.can_reach(tile))
        ]
        
        if landing_tiles:
            tile_to_head_to = min(
                landing_tiles,
                key=lambda tile: tile.aerial_distance_to(unit.current_tile)
            )
            unit.movement.head_towards(tile_to_head_to)
            
        return True

    @staticmethod
    def _head_towards_enemy_city_long_range(
        closest_reachable_enemy_city: Tile,
        unit_distance_to_tiles: PathsToTilesWithinTurn,
        unit_range: int,
        unit: MapUnit
    ) -> bool:
        """Move a long-range unit towards an enemy city.
        
        Args:
            closest_reachable_enemy_city: The target city's center tile
            unit_distance_to_tiles: Available movement paths
            unit_range: The unit's attack range
            unit: The unit to move
            
        Returns:
            bool: Whether the unit has taken this action
        """
        tiles_in_bombard_range = set(closest_reachable_enemy_city.get_tiles_in_distance(2))
        
        tile_to_move_to = min(
            ((tile, dist) for tile, dist in unit_distance_to_tiles.items()
             if (tile.aerial_distance_to(closest_reachable_enemy_city) <= unit_range
                 and tile not in tiles_in_bombard_range
                 and unit.get_damage_from_terrain(tile) <= 0)),  # Don't set up on a mountain
            key=lambda x: x[1].total_movement,
            default=None
        )
        
        if not tile_to_move_to:
            return False  # No suitable tile to move to
            
        # Move into position far away enough that the bombard doesn't hurt
        unit.movement.head_towards(tile_to_move_to[0])
        return True 