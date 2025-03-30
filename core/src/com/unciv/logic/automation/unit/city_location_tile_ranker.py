from typing import List, Dict, Set, Optional
from dataclasses import dataclass
import math

from com.unciv.logic.automation import Automation
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags
from com.unciv.logic.map import HexMath
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.tile import ResourceType
from com.unciv.models.ruleset.unique import LocalUniqueCache, StateForConditionals, UniqueType

@dataclass
class BestTilesToFoundCity:
    """Data class to hold information about the best tiles to found a city."""
    tile_rank_map: Dict[Tile, float] = None
    best_tile: Optional[Tile] = None
    best_tile_rank: float = 0.0

    def __post_init__(self):
        if self.tile_rank_map is None:
            self.tile_rank_map = {}

class CityLocationTileRanker:
    """Handles AI automation for evaluating city founding locations."""

    @staticmethod
    def get_best_tiles_to_found_city(
        unit: MapUnit, 
        distance_to_search: Optional[int] = None, 
        minimum_value: float = 0.0
    ) -> BestTilesToFoundCity:
        """Get the best tiles to found a city.
        
        Args:
            unit: The unit that will found the city
            distance_to_search: Optional maximum distance to search
            minimum_value: Minimum value required for a tile to be considered
            
        Returns:
            BestTilesToFoundCity: Object containing the best tiles and their rankings
        """
        distance_modifier = 3.0  # percentage penalty per aerial distance from unit (Settler)
        
        # Calculate search range
        if distance_to_search is not None:
            range_value = distance_to_search
        else:
            distance_from_home = (
                0 if not unit.civ.cities 
                else min(
                    city.get_center_tile().aerial_distance_to(unit.get_tile())
                    for city in unit.civ.cities
                )
            )
            range_value = max(1, min(5, 8 - distance_from_home))  # Restrict vision when far from home to avoid death marches

        # Get nearby cities
        nearby_cities = [
            city for city in unit.civ.game_info.get_cities()
            if city.get_center_tile().aerial_distance_to(unit.get_tile()) <= 7 + range_value
        ]

        # Get possible city locations
        uniques = unit.get_matching_uniques(UniqueType.FoundCity)
        possible_city_locations = [
            tile for tile in unit.get_tile().get_tiles_in_distance(range_value)
            if (any(unique.conditionals_apply(StateForConditionals(unit=unit, tile=tile)) 
                   for unique in uniques)
                and CityLocationTileRanker._can_settle_tile(tile, unit.civ, nearby_cities)
                and (unit.get_tile() == tile or unit.movement.can_move_to(tile)))
        ]

        unique_cache = LocalUniqueCache()
        best_tiles_to_found_city = BestTilesToFoundCity()
        base_tile_map: Dict[Tile, float] = {}

        # Calculate rankings for possible locations
        possible_tile_locations_with_rank = []
        for tile in possible_city_locations:
            tile_value = CityLocationTileRanker._rank_tile_to_settle(
                tile, unit.civ, nearby_cities, base_tile_map, unique_cache
            )
            distance_score = min(99.0, unit.current_tile.aerial_distance_to(tile) * distance_modifier)
            tile_value *= (100 - distance_score) / 100
            
            if tile_value >= minimum_value:
                best_tiles_to_found_city.tile_rank_map[tile] = tile_value
                possible_tile_locations_with_rank.append((tile, tile_value))

        # Sort by rank and find best reachable tile
        possible_tile_locations_with_rank.sort(key=lambda x: x[1], reverse=True)
        best_reachable_tile = next(
            (tile for tile, _ in possible_tile_locations_with_rank 
             if unit.movement.can_reach(tile)),
            None
        )
        
        if best_reachable_tile is not None:
            best_tiles_to_found_city.best_tile = best_reachable_tile
            best_tiles_to_found_city.best_tile_rank = best_tiles_to_found_city.tile_rank_map[best_reachable_tile]

        return best_tiles_to_found_city

    @staticmethod
    def _can_settle_tile(tile: Tile, civ: Civilization, nearby_cities: List[City]) -> bool:
        """Check if a tile can be settled.
        
        Args:
            tile: The tile to check
            civ: The civilization attempting to settle
            nearby_cities: List of nearby cities
            
        Returns:
            bool: Whether the tile can be settled
        """
        mod_constants = civ.game_info.ruleset.mod_options.constants
        
        if not tile.is_land or tile.is_impassible():
            return False
        if tile.get_owner() is not None and tile.get_owner() != civ:
            return False
            
        for city in nearby_cities:
            distance = city.get_center_tile().aerial_distance_to(tile)
            # todo: AgreedToNotSettleNearUs is hardcoded for now but it may be better to softcode it below in getDistanceToCityModifier
            if (distance <= 6 
                and civ.knows(city.civ)
                and not civ.is_at_war_with(city.civ)
                # If the CITY OWNER knows that the UNIT OWNER agreed not to settle near them
                and city.civ.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.AgreedToNotSettleNearUs)):
                return False
                
            if tile.get_continent() == city.get_center_tile().get_continent():
                if distance <= mod_constants.minimal_city_distance:
                    return False
            else:
                if distance <= mod_constants.minimal_city_distance_on_different_continents:
                    return False
                    
        return True

    @staticmethod
    def _rank_tile_to_settle(
        new_city_tile: Tile, 
        civ: Civilization, 
        nearby_cities: List[City],
        base_tile_map: Dict[Tile, float],
        unique_cache: LocalUniqueCache
    ) -> float:
        """Calculate the rank of a tile for settling.
        
        Args:
            new_city_tile: The tile to rank
            civ: The civilization settling
            nearby_cities: List of nearby cities
            base_tile_map: Cache of base tile values
            unique_cache: Cache of unique effects
            
        Returns:
            float: The calculated rank value
        """
        tile_value = 0.0
        tile_value += CityLocationTileRanker._get_distance_to_city_modifier(new_city_tile, nearby_cities, civ)

        on_coast = new_city_tile.is_coastal_tile()
        on_hill = new_city_tile.is_hill()
        is_next_to_mountain = new_city_tile.is_adjacent_to("Mountain")
        # Only count a luxury resource that we don't have yet as unique once
        new_unique_luxury_resources: Set[str] = set()

        if on_coast:
            tile_value += 3
        # Hills are free production and defence
        if on_hill:
            tile_value += 7
        # Observatories are good, but current implementation not mod-friendly
        if is_next_to_mountain:
            tile_value += 5
        # This bonus for settling on river is a bit outsized for the importance, but otherwise they have a habit of settling 1 tile away
        if new_city_tile.is_adjacent_to_river():
            tile_value += 20
        # We want to found the city on an oasis because it can't be improved otherwise
        if new_city_tile.terrain_has_unique(UniqueType.Unbuildable):
            tile_value += 3
        # If we build the city on a resource tile, then we can't build any special improvements on it
        if new_city_tile.has_viewable_resource(civ):
            tile_value -= 4
        if (new_city_tile.has_viewable_resource(civ) 
            and new_city_tile.tile_resource.resource_type == ResourceType.Bonus):
            tile_value -= 8
        # Settling on bonus resources tends to waste a food
        # Settling on luxuries generally speeds up our game, and settling on strategics as well, as the AI cheats and can see them.

        tiles = 0
        for i in range(4):
            # Ideally, we shouldn't really count the center tile, as it's converted into 1 production 2 food anyways with special cases treated above, but doing so can lead to AI moving settler back and forth until forever
            for nearby_tile in new_city_tile.get_tiles_at_distance(i):
                tiles += 1
                tile_value += (
                    CityLocationTileRanker._rank_tile(
                        nearby_tile, civ, on_coast, new_unique_luxury_resources,
                        base_tile_map, unique_cache
                    ) * (3 / (i + 1))
                )
                # Tiles close to the city can be worked more quickly, and thus should gain higher weight.

        # Placing cities on the edge of the map is bad, we can't even build improvements on them!
        tile_value -= (HexMath.get_number_of_tiles_in_hexagon(3) - tiles) * 2.4
        return tile_value

    @staticmethod
    def _get_distance_to_city_modifier(
        new_city_tile: Tile,
        nearby_cities: List[City],
        civ: Civilization
    ) -> float:
        """Calculate the distance modifier for city placement.
        
        Args:
            new_city_tile: The tile being evaluated
            nearby_cities: List of nearby cities
            civ: The civilization settling
            
        Returns:
            float: The calculated distance modifier
        """
        modifier = 0.0
        for city in nearby_cities:
            distance_to_city = new_city_tile.aerial_distance_to(city.get_center_tile())
            distance_to_city_modifier = {
                7: 2.0,
                6: 4.0,
                5: 8.0,  # Settling further away sacrifices tempo
                4: 6.0,
                3: -25.0,
            }.get(distance_to_city, -30.0 if distance_to_city < 3 else 0.0)
            
            # We want a defensive ring around our capital
            if city.civ == civ:
                distance_to_city_modifier *= 2 if city.is_capital() else 1
            modifier += distance_to_city_modifier
            
        return modifier

    @staticmethod
    def _rank_tile(
        rank_tile: Tile,
        civ: Civilization,
        on_coast: bool,
        new_unique_luxury_resources: Set[str],
        base_tile_map: Dict[Tile, float],
        unique_cache: LocalUniqueCache
    ) -> float:
        """Calculate the rank of a tile.
        
        Args:
            rank_tile: The tile to rank
            civ: The civilization evaluating the tile
            on_coast: Whether the city will be on the coast
            new_unique_luxury_resources: Set of unique luxury resources already counted
            base_tile_map: Cache of base tile values
            unique_cache: Cache of unique effects
            
        Returns:
            float: The calculated rank value
        """
        if rank_tile.get_city() is not None:
            return -1.0
            
        location_specific_tile_value = 0.0
        # Don't settle near but not on the coast
        if rank_tile.is_coastal_tile() and not on_coast:
            location_specific_tile_value -= 2
            
        # Check if there are any new unique luxury resources
        if (rank_tile.has_viewable_resource(civ)
            and rank_tile.tile_resource.resource_type == ResourceType.Luxury
            and not (civ.has_resource(rank_tile.resource)
                    or rank_tile.resource in new_unique_luxury_resources)):
            location_specific_tile_value += 10
            new_unique_luxury_resources.add(rank_tile.resource)

        # Check if everything else has been calculated, if so return it
        if rank_tile in base_tile_map:
            return location_specific_tile_value + base_tile_map[rank_tile]
        if rank_tile.get_owner() is not None and rank_tile.get_owner() != civ:
            return 0.0

        rank_tile_value = Automation.rank_stats_value(
            rank_tile.stats.get_tile_stats(None, civ, unique_cache), 
            civ
        )

        if rank_tile.has_viewable_resource(civ):
            if rank_tile.tile_resource.resource_type == ResourceType.Bonus:
                rank_tile_value += 2.0
            elif rank_tile.tile_resource.resource_type == ResourceType.Strategic:
                rank_tile_value += 1.2 * rank_tile.resource_amount
            elif rank_tile.tile_resource.resource_type == ResourceType.Luxury:
                rank_tile_value += 10.0 * rank_tile.resource_amount  # very important for humans who might want to conquer the AI

        if rank_tile.terrain_has_unique(UniqueType.FreshWater):
            rank_tile_value += 0.5  # Taking into account freshwater farm food, maybe less important in baseruleset mods
            
        if (rank_tile.terrain_features 
            and rank_tile.last_terrain.has_unique(UniqueType.ProductionBonusWhenRemoved)):
            rank_tile_value += 0.5  # Taking into account yields from forest chopping

        if rank_tile.is_natural_wonder():
            rank_tile_value += 10

        base_tile_map[rank_tile] = rank_tile_value

        return rank_tile_value + location_specific_tile_value 