from typing import Dict, List, Optional, Set, Tuple
from com.unciv.logic.automation import Automation
from com.unciv.logic.automation.unit.civilian_unit_automation import try_run_away_if_necessary
from com.unciv.logic.battle import GreatGeneralImplementation
from com.unciv.logic.city import City
from com.unciv.logic.civilization.diplomacy import DiplomaticModifiers
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UnitActionType
from com.unciv.models.ruleset import Building
from com.unciv.models.ruleset.tile import TerrainType
from com.unciv.models.ruleset.unique import LocalUniqueCache, UniqueType
from com.unciv.models.stats import Stat
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActions, UnitActionsFromUniques

class SpecificUnitAutomation:
    """Handles automation for specific unit types."""
    
    @staticmethod
    def automate_great_general(unit: MapUnit) -> bool:
        """Automate great general unit behavior.
        
        Args:
            unit: The great general unit to automate
            
        Returns:
            True if any action was taken
        """
        # Try to follow nearby units. Do not garrison in city if possible
        max_affected_troops_tile = GreatGeneralImplementation.get_best_affected_troops_tile(unit)
        if not max_affected_troops_tile:
            return False
            
        unit.movement.head_towards(max_affected_troops_tile)
        return True

    @staticmethod
    def automate_citadel_placer(unit: MapUnit) -> bool:
        """Automate citadel placer unit behavior.
        
        Args:
            unit: The citadel placer unit to automate
            
        Returns:
            True if any action was taken
        """
        # Keep at least 2 generals alive
        if (unit.has_unique(UniqueType.StrengthBonusInRadius) 
            and sum(1 for u in unit.civ.units.get_civ_units() 
                   if u.has_unique(UniqueType.StrengthBonusInRadius)) < 3):
            return False
            
        # Try to revenge and capture their tiles
        enemy_cities = [
            city for civ in unit.civ.get_known_civs()
            if unit.civ.get_diplomacy_manager(civ).has_modifier(DiplomaticModifiers.StealingTerritory)
            for city in civ.cities
        ]
        
        # Find the suitable tiles (or their neighbours)
        tile_to_steal = next(
            (tile for city in enemy_cities
             for city_tile in city.get_tiles()  # City tiles
             for neighbor in city_tile.neighbors  # Neighbors of edge city tiles
             if (neighbor in unit.civ.viewable_tiles  # We can see them
                 and any(t.get_owner() == unit.civ for t in neighbor.neighbors)  # Close to our borders
                 and unit.movement.can_reach(neighbor)),  # Can reach
            None
        )
        
        # If there is a good tile to steal - go there
        if tile_to_steal:
            unit.movement.head_towards(tile_to_steal)
            if unit.has_movement() and unit.current_tile == tile_to_steal:
                actions = UnitActionsFromUniques.get_improvement_construction_actions_from_general_unique(unit, unit.current_tile)
                if actions:
                    actions[0].action()
            return True
            
        # Try to build a citadel for defensive purposes
        if unit.civ.get_worker_automation().evaluate_fort_placement(unit.current_tile, True):
            actions = UnitActionsFromUniques.get_improvement_construction_actions_from_general_unique(unit, unit.current_tile)
            if actions:
                actions[0].action()
            return True
            
        return False

    @staticmethod
    def automate_great_general_fallback(unit: MapUnit) -> None:
        """Handle fallback behavior for great general when no units to follow.
        
        Args:
            unit: The great general unit to handle
        """
        def reachable_test(tile: Tile) -> bool:
            return (tile.civilian_unit is None
                   and unit.movement.can_move_to(tile)
                   and unit.movement.can_reach(tile))
                   
        # Find closest reachable city
        city_to_garrison = next(
            (city.get_center_tile() for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.current_tile)
            ) if reachable_test(city.get_center_tile())),
            None
        )
        
        if not city_to_garrison:
            return
            
        if not unit.cache.has_citadel_placement_unique:
            unit.movement.head_towards(city_to_garrison)
            return
            
        # Try to find a good place for citadel nearby
        tile_for_citadel = next(
            (tile for tile in city_to_garrison.get_tiles_in_distance_range(range(3, 5))
             if reachable_test(tile)
             and unit.civ.get_worker_automation().evaluate_fort_placement(tile, True)),
            None
        )
        
        if not tile_for_citadel:
            unit.movement.head_towards(city_to_garrison)
            return
            
        unit.movement.head_towards(tile_for_citadel)
        if unit.has_movement() and unit.current_tile == tile_for_citadel:
            actions = UnitActionsFromUniques.get_improvement_construction_actions_from_general_unique(unit, unit.current_tile)
            if actions:
                actions[0].action()

    @staticmethod
    def automate_settler_actions(unit: MapUnit, dangerous_tiles: Set[Tile]) -> None:
        """Automate settler unit actions.
        
        Args:
            unit: The settler unit to automate
            dangerous_tiles: Set of tiles to avoid
        """
        # If we don't have any cities, we are probably at the start of the game with only one settler
        # If we are at the start of the game lets spend a maximum of 3 turns to settle our first city
        # As our turns progress lets shrink the area that we look at to make sure that we stay on target
        # If we have gone more than 3 turns without founding a city lets search a wider area
        range_to_search = None
        if unit.civ.cities and unit.civ.game_info.turns < 4:
            range_to_search = max(1, 3 - unit.civ.game_info.turns)
            
        # Get best tiles for city founding
        best_tiles_info = CityLocationTileRanker.get_best_tiles_to_found_city(unit, range_to_search, 50.0)
        best_city_location: Optional[Tile] = None
        
        # Special case: settle in place on turn 1
        if (unit.civ.game_info.turns == 0 
            and not unit.civ.cities 
            and unit.get_tile() in best_tiles_info.tile_rank_map):
            found_city_action = UnitActionsFromUniques.get_found_city_action(unit, unit.get_tile())
            if not found_city_action or not found_city_action.action:
                return
                
            # Handle multiple settlers case
            all_unsettled_settlers = [
                u for u in unit.civ.units.get_civ_units()
                if u.has_movement() and u.base_unit == unit.base_unit
            ]
            
            # Find best settler location
            best_settler = max(
                all_unsettled_settlers,
                key=lambda u: best_tiles_info.tile_rank_map.get(u.get_tile(), -1.0),
                default=None
            )
            
            if best_settler == unit and found_city_action.action:
                found_city_action.action()
                return
                
            # Update tile rankings if we have a better settler
            if best_settler:
                best_tiles_info.tile_rank_map = {
                    k: v for k, v in best_tiles_info.tile_rank_map.items()
                    if k.aerial_distance_to(best_settler.get_tile()) > 4
                }
                
        # Check if current tile is good enough
        if (unit.get_tile() in best_tiles_info.tile_rank_map
            and (not best_tiles_info.best_tile 
                 or best_tiles_info.tile_rank_map[unit.get_tile()] >= best_tiles_info.best_tile_rank - 2)):
            best_city_location = unit.get_tile()
            
        # Shortcut for nearby best tile
        if (not best_city_location 
            and best_tiles_info.best_tile 
            and 1 <= len(unit.movement.get_shortest_path(best_tiles_info.best_tile)) <= 3):
            best_city_location = best_tiles_info.best_tile
            
        # Find best tile within range
        if not best_city_location:
            def is_tile_rank_ok(entry: Tuple[Tile, float]) -> bool:
                tile, _ = entry
                if tile in dangerous_tiles and tile != unit.get_tile():
                    return False
                path_size = len(unit.movement.get_shortest_path(tile))
                return 1 <= path_size <= 3
                
            best_city_location = next(
                (tile for tile, rank in sorted(
                    best_tiles_info.tile_rank_map.items(),
                    key=lambda x: x[1],
                    reverse=True
                )
                if (not best_tiles_info.best_tile 
                    or rank >= best_tiles_info.best_tile_rank - 5)
                and is_tile_rank_ok((tile, rank))),
                None
            )
            
        # Try best tile if within 8 turns
        if (not best_city_location 
            and best_tiles_info.best_tile 
            and 1 <= len(unit.movement.get_shortest_path(best_tiles_info.best_tile)) <= 8):
            best_city_location = best_tiles_info.best_tile
            
        # Handle no good locations case
        if not best_city_location:
            def get_frontier_score(city: City) -> int:
                """Calculate how lonely a city is based on available tiles."""
                return sum(
                    1 for tile in city.get_center_tile().get_tiles_at_distance(
                        city.civ.game_info.ruleset.mod_options.constants.minimal_city_distance + 1
                    )
                    if tile.can_be_settled() 
                    and (tile.get_owner() is None or tile.get_owner() == city.civ)
                )
                
            frontier_city = max(
                unit.civ.cities,
                key=get_frontier_score,
                default=None
            )
            
            if (frontier_city 
                and get_frontier_score(frontier_city) > 0 
                and unit.movement.can_reach(frontier_city.get_center_tile())):
                unit.movement.head_towards(frontier_city.get_center_tile())
                
            if UnitAutomation.try_explore(unit):
                return
                
            UnitAutomation.wander(unit, tiles_to_avoid=dangerous_tiles)
            return
            
        # Handle city founding
        found_city_action = UnitActionsFromUniques.get_found_city_action(unit, best_city_location)
        if not found_city_action or not found_city_action.action:
            if unit.has_movement() and not unit.civ.is_one_city_challenger():
                raise Exception("City within distance")
            return
            
        should_settle = unit.get_tile() == best_city_location and unit.has_movement()
        if should_settle:
            found_city_action.action()
            return
            
        if try_run_away_if_necessary(unit):
            return
            
        unit.movement.head_towards(best_city_location)
        if should_settle:
            found_city_action.action()

    @staticmethod
    def automate_improvement_placer(unit: MapUnit) -> bool:
        """Automate improvement placer unit behavior.
        
        Args:
            unit: The improvement placer unit to automate
            
        Returns:
            True if any progress was made
        """
        improvement_building_unique = next(
            (u for u in unit.get_matching_uniques(UniqueType.ConstructImprovementInstantly)),
            None
        )
        if not improvement_building_unique:
            return False
            
        improvement_name = improvement_building_unique.params[0]
        improvement = unit.civ.game_info.ruleset.tile_improvements.get(improvement_name)
        if not improvement:
            return False
            
        related_stat = max(improvement.items(), key=lambda x: x[1])[0]
        
        # Sort cities by stat boost
        cities_by_stat_boost = sorted(
            unit.civ.cities,
            key=lambda c: c.city_stats.stat_percent_bonus_tree.total_stats[related_stat],
            reverse=True
        )
        
        # Calculate average terrain stats value
        average_terrain_stats_value = sum(
            Automation.rank_stats_value(terrain, unit.civ)
            for terrain in unit.civ.game_info.ruleset.terrains.values()
            if terrain.type == TerrainType.Land
        ) / len(unit.civ.game_info.ruleset.terrains)
        
        local_unique_cache = LocalUniqueCache()
        
        for city in cities_by_stat_boost:
            applicable_tiles = [
                tile for tile in city.get_workable_tiles()
                if (tile.is_land 
                    and tile.resource is None 
                    and not tile.is_city_center()
                    and (unit.current_tile == tile or unit.movement.can_move_to(tile))
                    and tile.improvement is None
                    and tile.improvement_functions.can_build_improvement(improvement, unit.civ)
                    and Automation.rank_tile(tile, unit.civ, local_unique_cache) > average_terrain_stats_value)
            ]
            
            if not applicable_tiles:
                continue
                
            path_to_city = unit.movement.get_shortest_path(city.get_center_tile())
            if not path_to_city:
                continue
                
            if len(path_to_city) > 2 and unit.get_tile().get_city() != city:
                # Check for enemy units nearby
                enemy_units_nearby = any(
                    any(u.is_military() and u.civ.is_at_war_with(unit.civ)
                         for u in tile.get_units())
                    for tile in unit.get_tile().get_tiles_in_distance(5)
                )
                
                # Don't move until accompanied by military unit if enemies nearby
                if unit.get_tile().military_unit is None and enemy_units_nearby:
                    return True
                    
                unit.movement.head_towards(city.get_center_tile())
                return True
                
            # Find best tile to improve
            chosen_tile = next(
                (tile for tile in sorted(
                    applicable_tiles,
                    key=lambda t: Automation.rank_tile(t, unit.civ, local_unique_cache),
                    reverse=True
                ) if unit.movement.can_reach(tile)),
                None
            )
            
            if not chosen_tile:
                continue
                
            unit_tile_before_movement = unit.current_tile
            unit.movement.head_towards(chosen_tile)
            
            if unit.current_tile == chosen_tile:
                if unit.current_tile.is_pillaged():
                    UnitActions.invoke_unit_action(unit, UnitActionType.Repair)
                else:
                    UnitActions.invoke_unit_action(unit, UnitActionType.CreateImprovement)
                return True
                
            return unit_tile_before_movement != unit.current_tile
            
        return False

    @staticmethod
    def conduct_trade_mission(unit: MapUnit) -> bool:
        """Conduct a trade mission with a city state.
        
        Args:
            unit: The unit to conduct the mission
            
        Returns:
            True if any progress was made
        """
        # Find closest city state
        closest_city_state_tile = next(
            (tile for civ in unit.civ.game_info.civilizations
             if (civ != unit.civ
                 and not unit.civ.is_at_war_with(civ)
                 and civ.is_city_state
                 and civ.cities)
             for city in [civ.cities[0]]
             for tile in city.get_tiles()
             if unit.civ.has_explored(tile)
             for path in [unit.movement.get_shortest_path(tile)]
             if 1 <= len(path) <= 10),  # 0 is unreachable, 10 is too far away
            None
        )
        
        if not closest_city_state_tile:
            return False
            
        # Try to conduct mission
        if UnitActions.invoke_unit_action(unit, UnitActionType.ConductTradeMission):
            return True
            
        # Move towards city state
        unit_tile_before_movement = unit.current_tile
        unit.movement.head_towards(closest_city_state_tile)
        
        return unit_tile_before_movement != unit.current_tile

    @staticmethod
    def speedup_wonder_construction(unit: MapUnit) -> bool:
        """Speed up wonder construction in nearby cities.
        
        Args:
            unit: The unit to use for speeding up construction
            
        Returns:
            True if any progress was made
        """
        def get_wonder_that_would_benefit_from_being_sped_up(city: City) -> Optional[Building]:
            """Find a wonder that would benefit from being sped up.
            
            Args:
                city: The city to check
                
            Returns:
                The best wonder to speed up, if any
            """
            return next(
                (building for building in city.city_constructions.get_buildable_buildings()
                 if (building.is_wonder
                     and not building.has_unique(UniqueType.CannotBeHurried)
                     and city.city_constructions.turns_to_construction(building.name) >= 5)),
                None
            )
            
        # Find nearby city with available wonders
        nearby_city_with_wonders = next(
            (city for city in unit.civ.cities
             if (city.population.population >= 3  # Don't speed up in small cities
                 and (unit.movement.can_move_to(city.get_center_tile())
                      or unit.current_tile == city.get_center_tile())
                 and get_wonder_that_would_benefit_from_being_sped_up(city)
                 for path in [unit.movement.get_shortest_path(city.get_center_tile())]
                 if path and len(path) <= 5),
            None
        )
        
        if not nearby_city_with_wonders:
            return False
            
        # Handle wonder construction
        if unit.current_tile == nearby_city_with_wonders.get_center_tile():
            wonder_to_hurry = get_wonder_that_would_benefit_from_being_sped_up(nearby_city_with_wonders)
            if wonder_to_hurry:
                nearby_city_with_wonders.city_constructions.construction_queue.insert(0, wonder_to_hurry.name)
                return (UnitActions.invoke_unit_action(unit, UnitActionType.HurryBuilding)
                        or UnitActions.invoke_unit_action(unit, UnitActionType.HurryWonder))
                
        # Move towards city
        tile_before_moving = unit.get_tile()
        unit.movement.head_towards(nearby_city_with_wonders.get_center_tile())
        return tile_before_moving != unit.current_tile

    @staticmethod
    def automate_add_in_capital(unit: MapUnit) -> None:
        """Automate adding unit to capital.
        
        Args:
            unit: The unit to add to capital
        """
        capital = unit.civ.get_capital()
        if not capital:  # safeguard
            return
            
        capital_tile = capital.get_center_tile()
        if unit.movement.can_reach(capital_tile):
            unit.movement.head_towards(capital_tile)
            
        if unit.get_tile() == capital_tile:
            UnitActions.invoke_unit_action(unit, UnitActionType.AddInCapital) 