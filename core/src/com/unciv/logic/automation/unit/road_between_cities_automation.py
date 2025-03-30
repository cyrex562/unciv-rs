from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from com.badlogic.gdx.math import Vector2
from com.unciv import Constants
from com.unciv.UncivGame import UncivGame
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map import BFS, HexMath, MapPathing
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import RoadStatus, Tile
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.utils import Log

# Constants for worker automation
class WorkerAutomationConst:
    """BFS max size is determined by the aerial distance of two cities to connect, padded with this"""
    # two tiles longer than the distance to the nearest connected city should be enough as the 'reach' of a BFS is increased by blocked tiles
    MAX_BFS_REACH_PADDING = 2

@dataclass
class RoadPlan:
    """Represents a plan for building roads between cities."""
    tiles: List[Tile]
    priority: float
    from_city: City
    to_city: City
    
    @property
    def number_of_roads_to_build(self) -> int:
        """Calculate the number of roads that need to be built."""
        return sum(1 for tile in self.tiles if tile.get_unpillaged_road() != self.best_road_available)

class RoadBetweenCitiesAutomation:
    """Responsible for the "connect cities" automation as part of worker automation."""
    
    def __init__(self, civ_info: Civilization, cached_for_turn: int, cloning_source: Optional['RoadBetweenCitiesAutomation'] = None):
        """Initialize the automation.
        
        Args:
            civ_info: The civilization to automate for
            cached_for_turn: The turn number for caching
            cloning_source: Optional source to clone from
        """
        self.civ_info = civ_info
        self.cached_for_turn = cached_for_turn
        
        # Cache BFS by city locations (cities needing connecting)
        # key: The city to connect from as hex position[Vector2]
        # value: The BFS searching from that city, whether successful or not
        self.bfs_cache: Dict[Vector2, BFS] = {}
        
        # Cache road to build for connecting cities unless option is off or ruleset removed all roads
        self.best_road_available: RoadStatus = (
            cloning_source.best_road_available if cloning_source
            else self._get_best_road_available()
        )
        
        # Cache of roads to connect cities each turn
        self.roads_to_build_by_cities_cache: Dict[City, List[RoadPlan]] = {}
        
        # Hashmap of all cached tiles in each list in roads_to_build_by_cities_cache
        self.tiles_of_roads_map: Dict[Tile, RoadPlan] = {}
        
        # Lazy initialization of connected cities
        self._tiles_of_connected_cities: Optional[List[Tile]] = None

    def _get_best_road_available(self) -> RoadStatus:
        """Get the best road type available for the civilization."""
        if (self.civ_info.is_human() 
            and not UncivGame.Current.settings.auto_building_roads
            and not UncivGame.Current.world_screen?.auto_play?.is_auto_playing_and_full_auto_play_ai()):
            return RoadStatus.None
        return self.civ_info.tech.get_best_road_available()

    @property
    def tiles_of_connected_cities(self) -> List[Tile]:
        """Get list of connected cities' center tiles."""
        if self._tiles_of_connected_cities is None:
            self._tiles_of_connected_cities = [
                city.get_center_tile()
                for city in self.civ_info.cities
                if city.is_capital() or city.city_stats.is_connected_to_capital(self.best_road_available)
            ]
            if Log.should_log():
                Log.debug(f"WorkerAutomation tilesOfConnectedCities for {self.civ_info.civ_name} turn {self.cached_for_turn}:")
                if not self._tiles_of_connected_cities:
                    Log.debug("\tempty")
                else:
                    for tile in self._tiles_of_connected_cities:
                        Log.debug(f"\t{tile}")
        return self._tiles_of_connected_cities

    def get_roads_to_build_from_city(self, city: City) -> List[RoadPlan]:
        """Get road plans for connecting a city to surrounding cities.
        
        Args:
            city: The city to get road plans for
            
        Returns:
            List of road plans to connect this city
        """
        if city in self.roads_to_build_by_cities_cache:
            return self.roads_to_build_by_cities_cache[city]
            
        # Get a worker unit for pathfinding
        worker_unit = next(
            (unit.get_map_unit(self.civ_info, Constants.NO_ID)
             for unit in self.civ_info.game_info.ruleset.units.values()
             if unit.has_unique(UniqueType.BuildImprovements)),
            None
        )
        if not worker_unit:
            return []
            
        road_to_capital_status = city.city_stats.get_road_type_of_connection_to_capital()
        
        def rank_road_capital_priority(road_status: RoadStatus) -> float:
            """Rank priority for connecting to capital."""
            if road_status == RoadStatus.None:
                return 2.0 if self.best_road_available != RoadStatus.None else 0.0
            if road_status == RoadStatus.Road:
                return 1.0 if self.best_road_available != RoadStatus.Road else 0.0
            return 0.0
            
        base_priority = rank_road_capital_priority(road_to_capital_status)
        roads_to_build: List[RoadPlan] = []
        
        # Check nearby cities
        for close_city in (city for city in city.neighboring_cities 
                         if (city.civ == self.civ_info 
                             and city.get_center_tile().aerial_distance_to(city.get_center_tile()) <= 8)):
            
            # Try to find if the other city has planned to build a road to this city
            if close_city in self.roads_to_build_by_cities_cache:
                road_to_build = next(
                    (plan for plan in self.roads_to_build_by_cities_cache[close_city]
                     if plan.from_city == city or plan.to_city == city),
                    None
                )
                if road_to_build:
                    roads_to_build.append(road_to_build)
                continue
                
            # Try to build a plan for the road to the city
            road_path = None
            if self.civ_info.cities.index(city) < self.civ_info.cities.index(close_city):
                road_path = MapPathing.get_road_path(worker_unit, city.get_center_tile(), close_city.get_center_tile())
            else:
                road_path = MapPathing.get_road_path(worker_unit, close_city.get_center_tile(), city.get_center_tile())
                
            if not road_path:
                continue
                
            worst_road_status = self._get_worst_road_type_in_path(road_path)
            if worst_road_status == self.best_road_available:
                continue
                
            # Calculate road priority
            road_priority = max(base_priority, rank_road_capital_priority(close_city.city_stats.get_road_type_of_connection_to_capital()))
            if worst_road_status == RoadStatus.None:
                road_priority += 2
            elif worst_road_status == RoadStatus.Road and self.best_road_available == RoadStatus.Railroad:
                road_priority += 1
            if close_city.city_stats.get_road_type_of_connection_to_capital() > road_to_capital_status:
                road_priority += 1
                
            new_road_plan = RoadPlan(
                road_path,
                road_priority + (city.population.population + close_city.population.population) / 4.0,
                city,
                close_city
            )
            roads_to_build.append(new_road_plan)
            
            # Update tiles map
            for tile in new_road_plan.tiles:
                if tile not in self.tiles_of_roads_map or self.tiles_of_roads_map[tile].priority < new_road_plan.priority:
                    self.tiles_of_roads_map[tile] = new_road_plan
                    
        # If no roads to close-by cities, check for road to capital
        if not roads_to_build and road_to_capital_status < self.best_road_available:
            road_to_capital = self._get_road_to_connect_city_to_capital(worker_unit, city)
            if road_to_capital:
                worst_road_status = self._get_worst_road_type_in_path(road_to_capital[1])
                road_priority = base_priority + (2.0 if worst_road_status == RoadStatus.None else 1.0)
                
                new_road_plan = RoadPlan(
                    road_to_capital[1],
                    road_priority + city.population.population / 2.0,
                    city,
                    road_to_capital[0]
                )
                roads_to_build.append(new_road_plan)
                
                # Update tiles map
                for tile in new_road_plan.tiles:
                    if tile not in self.tiles_of_roads_map or self.tiles_of_roads_map[tile].priority < new_road_plan.priority:
                        self.tiles_of_roads_map[tile] = new_road_plan
                        
        self.roads_to_build_by_cities_cache[city] = roads_to_build
        return roads_to_build

    def _get_worst_road_type_in_path(self, path: List[Tile]) -> RoadStatus:
        """Get the worst road type in a path.
        
        Args:
            path: List of tiles in the path
            
        Returns:
            The worst road status found
        """
        worst_road_tile = RoadStatus.Railroad
        for tile in path:
            if tile.get_unpillaged_road() < worst_road_tile:
                worst_road_tile = tile.get_unpillaged_road()
                if worst_road_tile == RoadStatus.None:
                    return RoadStatus.None
        return worst_road_tile

    def _get_road_to_connect_city_to_capital(self, unit: MapUnit, city: City) -> Optional[Tuple[City, List[Tile]]]:
        """Get a road path to connect a city to the capital.
        
        Args:
            unit: The unit to use for pathfinding
            city: The city to connect
            
        Returns:
            Tuple of (target city, path) if found, None otherwise
        """
        if not self.tiles_of_connected_cities:
            return None
            
        def is_candidate_tile(tile: Tile) -> bool:
            return tile.is_land and unit.movement.can_pass_through(tile)
            
        to_connect_tile = city.get_center_tile()
        bfs = self.bfs_cache.get(to_connect_tile.position)
        if not bfs:
            bfs = BFS(to_connect_tile, is_candidate_tile)
            bfs.max_size = HexMath.get_number_of_tiles_in_hexagon(
                WorkerAutomationConst.MAX_BFS_REACH_PADDING +
                min(tile.aerial_distance_to(to_connect_tile) for tile in self.tiles_of_connected_cities)
            )
            self.bfs_cache[to_connect_tile.position] = bfs
            
        city_tiles_to_seek = set(self.tiles_of_connected_cities)
        
        while True:
            next_tile = bfs.next_step()
            if not next_tile:
                break
                
            if next_tile in city_tiles_to_seek:
                city_tile = next_tile
                path_to_city = bfs.get_path_to(city_tile)
                return city_tile.get_city(), path_to_city
                
        return None

    def get_nearby_cities_to_connect(self, unit: MapUnit) -> List[City]:
        """Get list of nearby cities that need connecting.
        
        Args:
            unit: The unit to use for distance calculations
            
        Returns:
            List of cities that need connecting
        """
        if self.best_road_available == RoadStatus.None:
            return []
            
        candidate_cities = [
            city for city in self.civ_info.cities
            if (city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20 
                and self.get_roads_to_build_from_city(city))
        ]
        
        return candidate_cities

    def try_connecting_cities(self, unit: MapUnit, candidate_cities: List[City]) -> bool:
        """Try to connect cities with roads.
        
        Args:
            unit: The unit to use for building roads
            candidate_cities: List of candidate cities to connect
            
        Returns:
            Whether any action was taken
        """
        if self.best_road_available == RoadStatus.None or not candidate_cities:
            return False
            
        current_tile = unit.get_tile()
        
        # Search through ALL candidate cities for the closest tile to build a road on
        for to_connect_city in sorted(
            candidate_cities,
            key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile()),
            reverse=True
        ):
            tiles_by_priority = [
                (tile, road_plan.priority)
                for road_plan in self.get_roads_to_build_from_city(to_connect_city)
                for tile in road_plan.tiles
            ]
            
            tiles_sorted = sorted(
                [(tile, priority) for tile, priority in tiles_by_priority
                 if tile.get_unpillaged_road() < self.best_road_available],
                key=lambda x: x[0].aerial_distance_to(unit.get_tile()) + (x[1] / 10.0)
            )
            
            best_tile = next(
                (tile for tile, _ in tiles_sorted
                 if unit.movement.can_move_to(tile) and unit.movement.can_reach(tile)),
                None
            )
            
            if not best_tile:
                continue
                
            if best_tile != current_tile and unit.has_movement():
                unit.movement.head_towards(best_tile)
                
            if (unit.has_movement() and best_tile == current_tile
                    and current_tile.improvement_in_progress != self.best_road_available.name):
                improvement = self.best_road_available.improvement(self.civ_info.game_info.ruleset)
                if improvement:
                    best_tile.start_working_on_improvement(improvement, self.civ_info, unit)
                    
            return True
            
        return False 