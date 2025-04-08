from typing import List, Optional, Set
from com.badlogic.gdx.math import Vector2
from com.unciv.logic.civilization import Civilization, MapUnitAction, NotificationCategory, NotificationIcon
from com.unciv.logic.map import MapPathing
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import RoadStatus, Tile
from com.unciv.utils import Log

class RoadToAutomation:
    """Responsible for automation the "build road to" action.
    
    This is *pretty bad code* overall and needs to be cleaned up.
    """
    
    def __init__(self, civ_info: Civilization):
        """Initialize the automation.
        
        Args:
            civ_info: The civilization to automate for
        """
        self.civ_info = civ_info
        self.actual_best_road_available: RoadStatus = civ_info.tech.get_best_road_available()

    def automate_connect_road(self, unit: MapUnit, tiles_where_we_will_be_captured: Set[Tile]) -> None:
        """Automate the process of connecting a road between two points.
        
        Current thoughts:
        Will be a special case of MapUnit.automated property
        Unit has new attributes start_tile end_tile
        - We will progress towards the end path sequentially, taking absolute least distance w/o regard for movement cost
        - Cancel upon risk of capture
        - Cancel upon blocked
        - End automation upon finish
        
        Args:
            unit: The unit to automate
            tiles_where_we_will_be_captured: Set of tiles where the unit might be captured
        """
        if self.actual_best_road_available == RoadStatus.None:
            return
            
        current_tile = unit.get_tile()
        
        if not unit.automated_road_connection_destination:
            self.stop_and_clean_automation(unit)
            return
            
        destination_tile = unit.civ.game_info.tile_map[unit.automated_road_connection_destination]
        path_to_dest: Optional[List[Vector2]] = unit.automated_road_connection_path
        
        # The path does not exist, create it
        if path_to_dest is None:
            found_path: Optional[List[Tile]] = MapPathing.get_road_path(unit, current_tile, destination_tile)
            if found_path is None:
                Log.debug(f"WorkerAutomation: {unit} -> connect road failed")
                self.stop_and_clean_automation(unit)
                unit.civ.add_notification(
                    "Connect road failed!",
                    MapUnitAction(unit),
                    NotificationCategory.Units,
                    NotificationIcon.Construction
                )
                return
                
            path_to_dest = [tile.position for tile in found_path]  # Convert to a list of positions for serialization
            unit.automated_road_connection_path = path_to_dest
            Log.debug(
                "WorkerAutomation: %s -> found connect road path to destination tile: %s, %s",
                unit,
                destination_tile,
                path_to_dest
            )
            
        curr_tile_index = path_to_dest.index(current_tile.position)
        
        # The worker was somehow moved off its path, cancel the action
        if curr_tile_index == -1:
            Log.debug(f"{unit} -> was moved off its connect road path. Operation cancelled.")
            self.stop_and_clean_automation(unit)
            unit.civ.add_notification(
                "Connect road cancelled!",
                MapUnitAction(unit),
                NotificationCategory.Units,
                unit.name
            )
            return
            
        # Can not build a road on this tile, try to move on.
        # The worker should search for the next furthest tile in the path that:
        # - It can move to
        # - Can be improved/upgraded
        if unit.has_movement() and not self.should_build_road_on_tile(current_tile):
            if curr_tile_index == len(path_to_dest) - 1:  # The last tile in the path is unbuildable or has a road
                self.stop_and_clean_automation(unit)
                unit.civ.add_notification(
                    "Connect road completed",
                    MapUnitAction(unit),
                    NotificationCategory.Units,
                    unit.name
                )
                return
                
            if curr_tile_index < len(path_to_dest) - 1:  # Try to move to the next tile in the path
                tile_map = unit.civ.game_info.tile_map
                next_tile: Tile = current_tile
                
                # Create a new list with tiles where the index is greater than curr_tile_index
                future_tiles = [
                    tile_map[pos]
                    for pos in path_to_dest[curr_tile_index + 1:]
                ]
                
                for future_tile in future_tiles:  # Find the furthest tile we can reach in this turn, move to, and does not have a road
                    if (unit.movement.can_reach_in_current_turn(future_tile) 
                        and unit.movement.can_move_to(future_tile)):  # We can at least move to this tile
                        next_tile = future_tile
                        if self.should_build_road_on_tile(future_tile):
                            break  # Stop on this tile
                            
                unit.movement.move_to_tile(next_tile)
                current_tile = unit.get_tile()
                
        # We need to check current movement again after we've (potentially) moved
        if unit.has_movement():
            # Repair pillaged roads first
            if current_tile.road_status != RoadStatus.None and current_tile.road_is_pillaged:
                current_tile.set_repaired()
                return
                
            if (self.should_build_road_on_tile(current_tile) 
                and current_tile.improvement_in_progress != self.actual_best_road_available.name):
                improvement = self.actual_best_road_available.improvement(self.civ_info.game_info.ruleset)
                if improvement:
                    current_tile.start_working_on_improvement(improvement, self.civ_info, unit)
                    return

    def stop_and_clean_automation(self, unit: MapUnit) -> None:
        """Reset side effects from automation, return worker to non-automated state.
        
        Args:
            unit: The unit to clean up
        """
        unit.automated = False
        unit.action = None
        unit.automated_road_connection_destination = None
        unit.automated_road_connection_path = None
        unit.current_tile.stop_working_on_improvement()

    def should_build_road_on_tile(self, tile: Tile) -> bool:
        """Check conditions for whether it is acceptable to build a road on this tile.
        
        Args:
            tile: The tile to check
            
        Returns:
            True if a road should be built on this tile
        """
        if tile.road_is_pillaged:
            return True
            
        # Can't build road on city tiles
        if tile.is_city_center():
            return False
            
        # Special case for civs that treat forest/jungles as roads (inside their territory)
        # We shouldn't build if railroads aren't unlocked
        if tile.has_connection(self.civ_info) and self.actual_best_road_available == RoadStatus.Road:
            return False
            
        # Build (upgrade) if possible
        return tile.road_status != self.actual_best_road_available 