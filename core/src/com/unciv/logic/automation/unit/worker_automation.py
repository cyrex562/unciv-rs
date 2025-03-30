from dataclasses import dataclass
from typing import Dict, List, Optional, Set, Tuple, Any
from com.unciv import Constants, UncivGame
from com.unciv.logic.automation import Automation
from com.unciv.logic.automation.civilization import NextTurnAutomation
from com.unciv.logic.automation.unit import UnitAutomation
from com.unciv.logic.civilization import Civilization, MapUnitAction, NotificationCategory
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.tile import ResourceType, Terrain, TileImprovement
from com.unciv.models.ruleset.unique import LocalUniqueCache, StateForConditionals, UniqueType
from com.unciv.models.stats import Stat, Stats
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActions, UnitActionsFromUniques
from com.unciv.utils import debug

@dataclass
class TileImprovementRank:
    """Represents the ranking of a tile improvement.
    
    Each object has two stages:
    1. First stage checks basic priority without improvements (tilePriority)
    2. Second stage sets improvementPriority and bestImprovement
    
    If tilePriority is -1 then it must be a dangerous tile.
    The improvementPriority and bestImprovement are by default not set.
    Once improvementPriority is set we have already checked for the best improvement.
    """
    tile_priority: float
    improvement_priority: Optional[float] = None
    best_improvement: Optional[TileImprovement] = None
    repair_improvement: Optional[bool] = None

class WorkerAutomation:
    """Contains the logic for worker automation.
    
    This is instantiated from Civilization.get_worker_automation and cached there.
    
    Args:
        civ_info: The Civilization - data common to all automated workers is cached once per Civ
        cached_for_turn: The turn number this was created for - a recreation of the instance is forced on different turn numbers
        cloning_source: Optional source WorkerAutomation to clone from
    """
    
    def __init__(self, civ_info: Civilization, cached_for_turn: int, cloning_source: Optional['WorkerAutomation'] = None):
        self.civ_info = civ_info
        self.cached_for_turn = cached_for_turn
        self.road_to_automation = RoadToAutomation(civ_info)
        self.road_between_cities_automation = RoadBetweenCitiesAutomation(
            civ_info, 
            cached_for_turn,
            cloning_source.road_between_cities_automation if cloning_source else None
        )
        self.rule_set = civ_info.game_info.ruleset
        self.tile_rankings: Dict[Tile, TileImprovementRank] = {}

    def automate_worker_action(self, unit: MapUnit, dangerous_tiles: Set[Tile], local_unique_cache: LocalUniqueCache = LocalUniqueCache()) -> None:
        """Automate one Worker - decide what to do and where, move, start or continue work.
        
        Args:
            unit: The worker unit to automate
            dangerous_tiles: Set of tiles considered dangerous
            local_unique_cache: Cache for unique checks
        """
        current_tile = unit.get_tile()
        # Must be called before any getPriority checks to guarantee the local road cache is processed
        cities_to_connect = self.road_between_cities_automation.get_nearby_cities_to_connect(unit)
        
        # Shortcut, we're working a suitable tile, and we're better off minimizing worker-turns by finishing everything on this tile
        if (current_tile.improvement_in_progress is not None 
            and current_tile not in dangerous_tiles
            and self.get_full_priority(unit.get_tile(), unit, local_unique_cache) >= 2):
            return
            
        tile_to_work = self.find_tile_to_work(unit, dangerous_tiles, local_unique_cache)

        if tile_to_work != current_tile and tile_to_work is not None:
            self.head_towards_tile_to_work(unit, tile_to_work, local_unique_cache)
            return

        if current_tile.improvement_in_progress is not None:
            return  # we're working!

        if tile_to_work == current_tile and self.tile_has_work_to_do(current_tile, unit, local_unique_cache):
            self.start_work_on_current_tile(unit)

        # Support Alpha Frontier-Style Workers that _also_ have the "May create improvements on water resources" unique
        if unit.cache.has_unique_to_create_water_improvements and self.automate_work_boats(unit):
            return

        if self.try_head_towards_undeveloped_city(unit, local_unique_cache, current_tile):
            return

        # Nothing to do, try again to connect cities
        if self.road_between_cities_automation.try_connecting_cities(unit, cities_to_connect):
            return

        debug(f"WorkerAutomation: {unit} -> nothing to do")
        unit.civ.add_notification(
            f"{unit.short_display_name()} has no work to do.",
            MapUnitAction(unit),
            NotificationCategory.Units,
            unit.name,
            "OtherIcons/Sleep"
        )

        # Idle CS units should wander so they don't obstruct players so much
        if unit.civ.is_city_state:
            UnitAutomation.wander(unit, stay_in_territory=True, avoid_tiles=dangerous_tiles)

    def try_head_towards_undeveloped_city(self, unit: MapUnit, local_unique_cache: LocalUniqueCache, current_tile: Tile) -> bool:
        """Try to move the worker towards an undeveloped city.
        
        Args:
            unit: The worker unit
            local_unique_cache: Cache for unique checks
            current_tile: Current tile of the unit
            
        Returns:
            True if the unit was moved towards an undeveloped city
        """
        cities_to_number_of_unimproved_tiles = {}
        
        for city in unit.civ.cities:
            cities_to_number_of_unimproved_tiles[city.id] = sum(
                1 for tile in city.get_tiles()
                if (tile.is_land 
                    and not any(u.cache.has_unique_to_build_improvements for u in tile.get_units())
                    and (tile.is_pillaged() or self.tile_has_work_to_do(tile, unit, local_unique_cache)))
            )

        closest_undeveloped_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: -c.get_center_tile().aerial_distance_to(current_tile)
            ) if cities_to_number_of_unimproved_tiles[city.id] > 0
            and unit.movement.can_reach(city.get_center_tile())),
            None
        )

        if closest_undeveloped_city is not None and closest_undeveloped_city != current_tile.owning_city:
            debug(f"WorkerAutomation: {unit} -> head towards undeveloped city {closest_undeveloped_city.name}")
            reached_tile = unit.movement.head_towards(closest_undeveloped_city.get_center_tile())
            if reached_tile != current_tile:
                unit.do_action()  # since we've moved, maybe we can do something here - automate
            return True
            
        return False

    def start_work_on_current_tile(self, unit: MapUnit) -> None:
        """Start working on the current tile.
        
        Args:
            unit: The worker unit
        """
        current_tile = unit.current_tile
        tile_ranking = self.tile_rankings[current_tile]
        
        if tile_ranking.repair_improvement:
            debug(f"WorkerAutomation: {unit} -> repairs {current_tile}")
            repair_action = UnitActionsFromUniques.get_repair_action(unit)
            if repair_action and repair_action.action:
                repair_action.action()
            return
            
        if tile_ranking.best_improvement is not None:
            debug(f"WorkerAutomation: {unit} -> start improving {current_tile}")
            current_tile.start_working_on_improvement(tile_ranking.best_improvement, self.civ_info, unit)
        else:
            raise ValueError("We didn't find anything to improve on this tile even though there was supposed to be something to improve!")

    def head_towards_tile_to_work(self, unit: MapUnit, tile_to_work: Tile, local_unique_cache: LocalUniqueCache) -> None:
        """Move the worker towards a tile that needs work.
        
        Args:
            unit: The worker unit
            tile_to_work: The target tile to work on
            local_unique_cache: Cache for unique checks
        """
        debug(f"WorkerAutomation: {unit} -> head towards {tile_to_work}")
        current_tile = unit.get_tile()
        reached_tile = unit.movement.head_towards(tile_to_work)

        if (tile_to_work in reached_tile.neighbors
            and unit.movement.can_move_to(tile_to_work, allow_swap=True)
            and not unit.movement.can_move_to(tile_to_work, allow_swap=False)
            and unit.movement.can_unit_swap_to(tile_to_work)):
            # There must be a unit on the target tile! Let's swap with it.
            unit.movement.swap_move_to_tile(tile_to_work)

        if reached_tile != current_tile:  # otherwise, we get a situation where the worker is automated, so it tries to move but doesn't, then tries to automate, then move, etc, forever. Stack overflow exception!
            unit.do_action()

        # If we have reached a fort tile that is in progress and shouldn't be there, cancel it.
        # TODO: Replace this code entirely and change [choose_improvement] to not continue building the improvement by default
        if (reached_tile == tile_to_work
            and reached_tile.improvement_in_progress == Constants.fort
            and self.evaluate_fort_surroundings(current_tile, False) <= 0):
            debug("Replacing fort in progress with new improvement")
            reached_tile.stop_working_on_improvement()

        if not unit.has_movement() or reached_tile != tile_to_work:
            return
            
        # If there's move still left, and this is even a tile we want, perform action
        # Unit may stop due to Enemy Unit within walking range during do_action() call

        # tile_rankings is updated in get_base_priority, which is only called if is_automation_workable_tile is true
        # Meaning, there are tiles we can't/shouldn't work, and they won't even be in tile_rankings
        if self.tile_has_work_to_do(unit.current_tile, unit, local_unique_cache):
            self.start_work_on_current_tile(unit) 

    def find_tile_to_work(self, unit: MapUnit, tiles_to_avoid: Set[Tile], local_unique_cache: LocalUniqueCache) -> Optional[Tile]:
        """Look for a worthwhile tile to improve.
        
        Args:
            unit: The worker unit
            tiles_to_avoid: Set of tiles to avoid
            local_unique_cache: Cache for unique checks
            
        Returns:
            The tile to work on, or None if no suitable tile was found
        """
        current_tile = unit.get_tile()
        
        if (self.is_automation_workable_tile(current_tile, tiles_to_avoid, current_tile, unit)
            and self.get_base_priority(current_tile, unit) >= 5
            and (current_tile.is_pillaged() or current_tile.has_fallout_equivalent() 
                 or self.tile_has_work_to_do(current_tile, unit, local_unique_cache))):
            return current_tile
        
        workable_tiles_center_first = [
            tile for tile in current_tile.get_tiles_in_distance(4)
            if (self.is_automation_workable_tile(tile, tiles_to_avoid, current_tile, unit) 
                and self.get_base_priority(tile, unit) > 1)
        ]

        workable_tiles_prioritized = {}
        for tile in workable_tiles_center_first:
            priority = self.get_base_priority(tile, unit)
            if priority not in workable_tiles_prioritized:
                workable_tiles_prioritized[priority] = []
            workable_tiles_prioritized[priority].append(tile)

        # Search through each group by priority
        # If we can find an eligible best tile in the group lets return that
        # under the assumption that best tile is better than tiles in all lower groups
        for priority in sorted(workable_tiles_prioritized.keys(), reverse=True):
            best_tile = None
            for tile_in_group in sorted(workable_tiles_prioritized[priority], 
                                      key=lambda t: unit.get_tile().aerial_distance_to(t)):
                # These are the expensive calculations (tile_can_be_improved, can_reach), 
                # so we only apply these filters after everything else is done.
                if not self.tile_has_work_to_do(tile_in_group, unit, local_unique_cache):
                    continue
                if unit.get_tile() == tile_in_group:
                    return unit.get_tile()
                if not unit.movement.can_reach(tile_in_group):
                    continue
                if (best_tile is None 
                    or self.get_full_priority(tile_in_group, unit, local_unique_cache) 
                    > self.get_full_priority(best_tile, unit, local_unique_cache)):
                    best_tile = tile_in_group
            if best_tile is not None:
                return best_tile
        return None

    def is_automation_workable_tile(self, tile: Tile, tiles_to_avoid: Set[Tile], current_tile: Tile, unit: MapUnit) -> bool:
        """Check if a tile can be worked by automated workers.
        
        Args:
            tile: The tile to check
            tiles_to_avoid: Set of tiles to avoid
            current_tile: Current tile of the unit
            unit: The worker unit
            
        Returns:
            True if the tile can be worked by automated workers
        """
        if tile in tiles_to_avoid:
            return False
        if not (tile == current_tile
               or (unit.is_civilian() and (tile.civilian_unit is None 
                   or not tile.civilian_unit.cache.has_unique_to_build_improvements))
               or (unit.is_military() and (tile.military_unit is None 
                   or not tile.military_unit.cache.has_unique_to_build_improvements))):
            return False
        if tile.owning_city is not None and tile.get_owner() != self.civ_info:
            return False
        if tile.is_city_center():
            return False
        # Don't try to improve tiles we can't benefit from at all
        if (not tile.has_viewable_resource(self.civ_info) 
            and not any(t.is_city_center() and t.get_city().civ == self.civ_info
                       for t in tile.get_tiles_in_distance(
                           self.civ_info.game_info.ruleset.mod_options.constants.city_work_range))):
            return False
        if (tile.get_tile_improvement() is not None 
            and tile.get_tile_improvement().has_unique(UniqueType.AutomatedUnitsWillNotReplace) 
            and not tile.is_pillaged()):
            return False
        return True

    def get_base_priority(self, tile: Tile, unit: MapUnit) -> float:
        """Calculate a priority for the tile without accounting for the improvement itself.
        This is a cheap guess on how helpful it might be to do work on this tile.
        
        Args:
            tile: The tile to evaluate
            unit: The worker unit
            
        Returns:
            The base priority value for the tile
        """
        unit_specific_priority = 2 - (tile.aerial_distance_to(unit.get_tile()) / 2.0).clamp(0.0, 2.0)
        if tile in self.tile_rankings:
            return self.tile_rankings[tile].tile_priority + unit_specific_priority

        priority = 0.0
        if tile.get_owner() == self.civ_info:
            priority += Automation.rank_stats_value(tile.stats.get_terrain_stats_breakdown().to_stats(), self.civ_info)
            if tile.provides_yield():
                priority += 2
            if tile.is_pillaged():
                priority += 1
            if tile.has_fallout_equivalent():
                priority += 1
            if (tile.terrain_features 
                and tile.last_terrain.has_unique(UniqueType.ProductionBonusWhenRemoved)):
                priority += 1  # removing our forests is good for tempo
            if tile.terrain_has_unique(UniqueType.FreshWater):
                priority += 1  # we want our farms up when unlocking Civil Service
        # give a minor priority to tiles that we could expand onto
        elif tile.get_owner() is None and any(t.get_owner() == self.civ_info for t in tile.neighbors):
            priority += 1

        if tile.has_viewable_resource(self.civ_info):
            priority += 1
            if tile.tile_resource.resource_type == ResourceType.Luxury:
                priority += 3  # luxuries are more important than other types of resources
    
        if tile in self.road_between_cities_automation.tiles_of_roads_map:
            priority += 3
            
        self.tile_rankings[tile] = TileImprovementRank(priority)
        return priority + unit_specific_priority

    def get_improvement_priority(self, tile: Tile, unit: MapUnit, local_unique_cache: LocalUniqueCache) -> float:
        """Calculate the priority of building an improvement on the tile.
        
        Args:
            tile: The tile to evaluate
            unit: The worker unit
            local_unique_cache: Cache for unique checks
            
        Returns:
            The priority value for improving the tile
        """
        self.get_base_priority(tile, unit)  # Ensure tile is in rankings
        rank = self.tile_rankings[tile]
        if rank.improvement_priority is None:
            # All values of rank have to be initialized
            rank.improvement_priority = -100.0
            rank.best_improvement = None
            rank.repair_improvement = False

            best_improvement = self.choose_improvement(unit, tile, local_unique_cache)
            if best_improvement is not None:
                rank.best_improvement = best_improvement
                # Increased priority if the improvement has been worked on longer
                time_spent_priority = (best_improvement.get_turns_to_build(unit.civ, unit) - tile.turns_to_improvement
                                     if tile.improvement_in_progress == best_improvement.name else 0)

                rank.improvement_priority = (self.get_improvement_ranking(tile, unit, rank.best_improvement.name, local_unique_cache) 
                                          + time_spent_priority)

            if tile.improvement is not None and tile.is_pillaged() and tile.owning_city is not None:
                # Value repairing higher when it is quicker and is in progress
                repair_bonus_priority = (tile.get_improvement_to_repair().get_turns_to_build(unit.civ, unit) 
                                       - UnitActionsFromUniques.get_repair_turns(unit))
                if tile.improvement_in_progress == Constants.repair:
                    repair_bonus_priority += (UnitActionsFromUniques.get_repair_turns(unit) 
                                            - tile.turns_to_improvement)

                repair_priority = repair_bonus_priority + Automation.rank_stats_value(
                    tile.stats.get_stat_diff_for_improvement(tile.get_tile_improvement(), unit.civ, tile.owning_city),
                    unit.civ
                )
                if repair_priority > rank.improvement_priority:
                    rank.improvement_priority = repair_priority
                    rank.best_improvement = None
                    rank.repair_improvement = True

        # A better tile than this unit can build might have been stored in the cache
        if not rank.repair_improvement and (rank.best_improvement is None 
                                          or not unit.can_build_improvement(rank.best_improvement, tile)):
            return -100.0
        return rank.improvement_priority

    def get_full_priority(self, tile: Tile, unit: MapUnit, local_unique_cache: LocalUniqueCache) -> float:
        """Calculate the full priority of the tile.
        
        Args:
            tile: The tile to evaluate
            unit: The worker unit
            local_unique_cache: Cache for unique checks
            
        Returns:
            The full priority value for the tile
        """
        return self.get_base_priority(tile, unit) + self.get_improvement_priority(tile, unit, local_unique_cache)

    def tile_has_work_to_do(self, tile: Tile, unit: MapUnit, local_unique_cache: LocalUniqueCache) -> bool:
        """Check if a tile has work that needs to be done.
        
        Args:
            tile: The tile to check
            unit: The worker unit
            local_unique_cache: Cache for unique checks
            
        Returns:
            True if the tile has work that needs to be done
        """
        if self.get_improvement_priority(tile, unit, local_unique_cache) <= 0:
            return False
        if not (self.tile_rankings[tile].best_improvement is not None 
                or self.tile_rankings[tile].repair_improvement):
            raise ValueError("There was an improvement_priority > 0 and nothing to do")
        return True 

    def choose_improvement(self, unit: MapUnit, tile: Tile, local_unique_cache: LocalUniqueCache) -> Optional[TileImprovement]:
        """Determine the improvement appropriate to a given tile and worker.
        
        Args:
            unit: The worker unit
            tile: The tile to improve
            local_unique_cache: Cache for unique checks
            
        Returns:
            The best improvement to build, or None if none is worth it
        """
        # You can keep working on half-built improvements, even if they're unique to another civ
        if tile.improvement_in_progress is not None:
            return self.rule_set.tile_improvements[tile.improvement_in_progress]

        potential_tile_improvements = {
            name: improvement for name, improvement in self.rule_set.tile_improvements.items()
            if ((improvement.unique_to is None 
                 or unit.civ.matches_filter(improvement.unique_to, StateForConditionals(unit=unit, tile=tile)))
                and unit.can_build_improvement(improvement, tile)
                and tile.improvement_functions.can_build_improvement(improvement, self.civ_info))
        }
        
        if not potential_tile_improvements:
            return None

        current_tile_stats = tile.stats.get_tile_stats(tile.get_city(), self.civ_info, local_unique_cache)
        best_buildable_improvement = max(
            ((improvement, self.get_improvement_ranking(tile, unit, improvement.name, local_unique_cache, current_tile_stats))
             for improvement in potential_tile_improvements.values()),
            key=lambda x: x[1] if x[1] > 0 else float('-inf'),
            default=(None, float('-inf'))
        )[0]

        if (tile.improvement is not None 
            and self.civ_info.is_human() 
            and not UncivGame.Current.settings.automated_workers_replace_improvements
            and not UncivGame.Current.world_screen.auto_play.is_auto_playing_and_full_auto_play_ai()):
            # Note that we might still want to build roads or remove fallout, 
            # so we can't exit the function immediately
            best_buildable_improvement = None

        last_terrain = tile.last_terrain

        def is_removable(terrain: Terrain) -> bool:
            return Constants.remove + terrain.name in potential_tile_improvements

        # Determine improvement for resource
        improvement_string_for_resource = None
        if tile.resource is not None and tile.has_viewable_resource(self.civ_info):
            if (tile.terrain_features
                and last_terrain.unbuildable
                and is_removable(last_terrain)
                and not tile.provides_resources(self.civ_info)
                and not self.is_resource_improvement_allowed_on_feature(tile, potential_tile_improvements)):
                improvement_string_for_resource = Constants.remove + last_terrain.name
            else:
                improvement_string_for_resource = max(
                    (imp for imp in tile.tile_resource.get_improvements() 
                     if imp in potential_tile_improvements or imp == tile.improvement),
                    key=lambda imp: self.get_improvement_ranking(tile, unit, imp, local_unique_cache),
                    default=None
                )

        # After gathering all the data, we conduct the hierarchy in one place
        improvement_string = None
        if best_buildable_improvement is not None and best_buildable_improvement.is_road():
            improvement_string = best_buildable_improvement.name
        elif (improvement_string_for_resource is not None 
              and tile.tile_resource.resource_type != ResourceType.Bonus):
            improvement_string = (None if improvement_string_for_resource == tile.improvement 
                                else improvement_string_for_resource)
        elif (tile.resource is not None 
              and tile.has_viewable_resource(self.civ_info)
              and tile.tile_resource.resource_type != ResourceType.Bonus
              and tile.tile_resource.get_improvements()):
            # If this is a resource that HAS an improvement that we can see, 
            # but this unit can't build it, don't waste your time
            return None
        elif best_buildable_improvement is None:
            improvement_string = None
        elif (tile.improvement is not None
              and self.get_improvement_ranking(tile, unit, tile.improvement, local_unique_cache) 
              > self.get_improvement_ranking(tile, unit, best_buildable_improvement.name, local_unique_cache)):
            # What we have is better, even if it's pillaged we should repair it
            improvement_string = None
        elif (is_removable(last_terrain)
              and (Automation.rank_stats_value(last_terrain, self.civ_info) < 0 
                   or last_terrain.has_unique(UniqueType.NullifyYields))):
            improvement_string = Constants.remove + last_terrain.name
        else:
            improvement_string = best_buildable_improvement.name

        # For mods, the tile improvement may not exist, so don't assume
        return self.rule_set.tile_improvements.get(improvement_string)

    def get_improvement_ranking(self, tile: Tile, unit: MapUnit, improvement_name: str,
                              local_unique_cache: LocalUniqueCache,
                              current_tile_stats: Optional[Stats] = None) -> float:
        """Get the ranking value for an improvement on a tile.
        
        Args:
            tile: The tile to evaluate
            unit: The worker unit
            improvement_name: Name of the improvement to evaluate
            local_unique_cache: Cache for unique checks
            current_tile_stats: Optional pre-calculated current tile stats
            
        Returns:
            The ranking value for the improvement
        """
        improvement = self.rule_set.tile_improvements[improvement_name]

        # Add the value of roads if we want to build it here
        if (improvement.is_road() 
            and self.road_between_cities_automation.best_road_available.improvement(self.rule_set) == improvement
            and tile in self.road_between_cities_automation.tiles_of_roads_map):
            road_plan = self.road_between_cities_automation.tiles_of_roads_map[tile]
            # We want some forest chopping and farm building first if the road doesn't have high priority
            value = road_plan.priority - 5
            return value

        # If this tile is not in our territory or neighboring it, it has no value
        if (tile.get_owner() != unit.civ
            and not (self.rule_set.tile_improvements[improvement_name].has_unique(UniqueType.CanBuildOutsideBorders)
                    and any(neighbor.get_owner() == unit.civ 
                           and neighbor.owning_city is not None
                           and tile.aerial_distance_to(neighbor.owning_city.get_center_tile()) 
                           <= self.civ_info.mod_constants.city_work_range
                           for neighbor in tile.neighbors))):
            return 0.0

        stats = tile.stats.get_stat_diff_for_improvement(
            improvement, 
            self.civ_info, 
            tile.get_city(), 
            local_unique_cache,
            current_tile_stats
        )

        is_resource_improved_by_new_improvement = (tile.has_viewable_resource(self.civ_info) 
                                                 and tile.tile_resource.is_improved_by(improvement_name))

        if improvement_name.startswith(Constants.remove):
            # We need to look beyond what we are doing right now and at the final improvement that will be on this tile
            removed_object = improvement_name.replace(Constants.remove, "")
            removed_feature = next((f for f in tile.terrain_features if f == removed_object), None)
            removed_improvement = removed_object if removed_object == tile.improvement else None

            if removed_feature is not None or removed_improvement is not None:
                new_tile = tile.clone(add_units=False)
                new_tile.set_terrain_transients()
                if removed_feature is not None:
                    new_tile.remove_terrain_feature(removed_feature)
                if removed_improvement is not None:
                    new_tile.remove_improvement()
                wanted_final_improvement = self.choose_improvement(unit, new_tile, local_unique_cache)
                if wanted_final_improvement is not None:
                    stat_diff = new_tile.stats.get_stat_diff_for_improvement(
                        wanted_final_improvement,
                        self.civ_info,
                        new_tile.get_city(),
                        local_unique_cache
                    )
                    stats.add(stat_diff)
                    # Take into account that the resource might be improved by the *final* improvement
                    is_resource_improved_by_new_improvement = (new_tile.resource is not None 
                                                             and new_tile.tile_resource.is_improved_by(wanted_final_improvement.name))
                    if (tile.terrain_features 
                        and tile.last_terrain.has_unique(UniqueType.ProductionBonusWhenRemoved)):
                        # We're gaining tempo by chopping the forest
                        # Adding an imaginary yield per turn is a way to correct for this
                        stats.add(Stat.Production, 0.5)

        # If the tile is a neighboring tile it has a lower value
        if tile.get_owner() != unit.civ:
            stats.div(3.0)

        value = Automation.rank_stats_value(stats, unit.civ)
        # Calculate the bonus from gaining the resources, this isn't included in the stats above
        if tile.resource is not None and tile.tile_resource.resource_type != ResourceType.Bonus:
            # A better resource ranking system might be required
            # We don't want the improvement ranking for resources to be too high
            if (tile.improvement is not None 
                and tile.tile_resource.is_improved_by(tile.improvement)):
                value -= min(max(tile.resource_amount / 2, 1), 2)
            if is_resource_improved_by_new_improvement:
                value += min(max(tile.resource_amount / 2, 1), 2)

        if self.is_improvement_probably_a_fort(improvement):
            value += self.evaluate_fort_surroundings(tile, improvement.has_unique(UniqueType.OneTimeTakeOverTilesInRadius))
        elif (tile.get_tile_improvement() is not None 
              and self.is_improvement_probably_a_fort(tile.get_tile_improvement())):
            # Replace/build improvements on other tiles before this one
            value /= 2

        return value

    def is_resource_improvement_allowed_on_feature(self, tile: Tile, 
                                                 potential_tile_improvements: Dict[str, TileImprovement]) -> bool:
        """Check if the improvement matching the tile resource requires any terrain feature to be removed first.
        
        Args:
            tile: The tile to check
            potential_tile_improvements: Dictionary of available improvements
            
        Returns:
            True if a resource improvement is allowed on the terrain feature
        
        Assumes the caller ensured that terrain_feature and resource are both present!
        """
        return any(
            resourceImprovementName in potential_tile_improvements
            and any(potential_tile_improvements[resourceImprovementName].is_allowed_on_feature(feature)
                   for feature in tile.terrain_feature_objects)
            for resourceImprovementName in tile.tile_resource.get_improvements()
        )

    @staticmethod
    def is_improvement_probably_a_fort(improvement_name: str) -> bool:
        """Check if an improvement is likely a fort.
        
        Args:
            improvement_name: Name of the improvement to check
            
        Returns:
            True if the improvement is probably a fort
        """
        return improvement_name == Constants.fort

    def evaluate_fort_surroundings(self, tile: Tile, is_citadel: bool) -> float:
        """Evaluate whether we want a Fort here considering surroundings.
        Does not check if there is already a fort here.
        
        Args:
            tile: The tile to evaluate
            is_citadel: Controls within borders check - true also allows 1 tile outside borders
            
        Returns:
            A value indicating how good the location is for a Fort
        """
        # Build on our land only
        if (tile.owning_city is None or tile.owning_city.civ != self.civ_info) and (
            # Except citadel which can be built near-by
            not is_citadel or all(t.get_owner() != self.civ_info for t in tile.neighbors)
        ):
            return 0.0

        if not self.is_acceptable_tile_for_fort(tile):
            return 0.0

        enemy_civs = self.civ_info.get_civs_at_war_with()
        if not enemy_civs:  # No potential enemies
            return 0.0

        value_of_fort = 1.0

        if self.civ_info.is_city_state and self.civ_info.get_ally_civ() is not None:
            value_of_fort -= 1.0  # Allied city states probably don't need to build forts

        if tile.has_viewable_resource(self.civ_info):
            value_of_fort -= 1.0

        # If this place is not perfect, let's see if there is a better one
        nearest_tiles = [t for t in tile.get_tiles_in_distance(1) 
                        if t.owning_city and t.owning_city.civ == self.civ_info]
        
        for close_tile in nearest_tiles:
            # Don't build forts too close to the cities
            if close_tile.is_city_center():
                value_of_fort -= 0.5
                continue

            # Don't build forts too close to other forts
            if (close_tile.improvement is not None
                and self.is_improvement_probably_a_fort(close_tile.get_tile_improvement())
                or (close_tile.improvement_in_progress is not None 
                    and self.is_improvement_probably_a_fort(close_tile.improvement_in_progress))):
                value_of_fort -= 1.0

            # There is probably another better tile for the fort
            if (not tile.is_hill() and close_tile.is_hill()
                and self.is_acceptable_tile_for_fort(close_tile)):
                value_of_fort -= 0.2

            # We want to build forts more in choke points
            if tile.is_impassible():
                value_of_fort += 0.2

        def threat_mapping(civ: Civilization) -> int:
            # The war is already a good nudge to build forts
            return (5 if self.civ_info.is_at_war_with(civ) else 0) + {
                ThreatLevel.VeryLow: 1,  # Do not build forts
                ThreatLevel.Low: 6,  # Too close, let's build until it is late
                ThreatLevel.Medium: 10,
                ThreatLevel.High: 15,  # They are strong, let's build until they reach us
                ThreatLevel.VeryHigh: 20
            }[Automation.threat_assessment(self.civ_info, civ)]

        enemy_civs_is_close_enough = [
            civ for civ in enemy_civs
            if NextTurnAutomation.get_min_distance_between_cities(self.civ_info, civ) <= threat_mapping(civ)
        ]

        # No threat, let's not build fort
        if not enemy_civs_is_close_enough:
            return 0.0

        # Make a list of enemy cities as sources of threat
        enemy_cities = []
        for enemy_civ in enemy_civs_is_close_enough:
            enemy_cities.extend(city.get_center_tile() for city in enemy_civ.cities)

        # Find closest enemy city
        closest_enemy_city = min(enemy_cities, key=lambda c: c.aerial_distance_to(tile))
        distance_to_enemy_city = tile.aerial_distance_to(closest_enemy_city)

        # Find our closest city to defend from this enemy city
        closest_city = min(
            (city.get_center_tile() for city in self.civ_info.cities),
            key=lambda c: c.aerial_distance_to(tile)
        )
        distance_between_cities = closest_enemy_city.aerial_distance_to(closest_city)

        # Find the distance between the target enemy city to our closest city
        distance_of_enemy_city_to_closest_city_of_us = min(
            city.get_center_tile().aerial_distance_to(closest_enemy_city)
            for city in self.civ_info.cities
        )

        # We don't want to defend city closest to this tile if it is behind other cities
        if distance_between_cities >= distance_of_enemy_city_to_closest_city_of_us + 2:
            return 0.0

        # This location is not between the city and the enemy
        if distance_to_enemy_city >= distance_between_cities or distance_to_enemy_city <= 2:
            return 0.0

        value_of_fort += 2 - abs(distance_between_cities - 1 - distance_to_enemy_city)
        # +2 is an acceptable deviation from the straight line between cities
        return max(value_of_fort, 0.0)

    def is_acceptable_tile_for_fort(self, tile: Tile) -> bool:
        """Check whether a given tile allows a Fort and whether a Fort may be undesirable.
        Does not check surroundings or if there is a fort already on the tile.
        
        Args:
            tile: The tile to check
            
        Returns:
            True if the tile is acceptable for a fort
        """
        if (tile.is_city_center()  # Don't build fort in the city
            or not tile.is_land  # Don't build fort in the water
            or (tile.has_viewable_resource(self.civ_info)  # Don't build on resource tiles
                and tile.tile_resource.resource_type != ResourceType.Bonus)
            or tile.contains_great_improvement()):  # Don't build on great improvements (including citadel)
            return False
        return True

    @staticmethod
    def automate_work_boats(unit: MapUnit) -> bool:
        """Try improving a Water Resource.
        
        Args:
            unit: The work boat unit
            
        Returns:
            Whether any progress was made (improved a tile or at least moved towards an opportunity)
            
        Todo:
            No logic to avoid capture by enemies yet!
        """
        closest_reachable_resource = next(
            (tile for tile in sorted(
                (tile for city in unit.civ.cities for tile in city.get_tiles()),
                key=lambda t: t.aerial_distance_to(unit.current_tile)
            ) if WorkerAutomation.has_workable_sea_resource(tile, unit.civ)
            and (unit.current_tile == tile or unit.movement.can_move_to(tile))
            and unit.movement.can_reach(tile)
            and WorkerAutomation.is_not_bonus_resource_or_workable(tile, unit.civ)),
            None
        )

        if not closest_reachable_resource:
            return False

        unit.movement.head_towards(closest_reachable_resource)
        if unit.current_tile != closest_reachable_resource:
            return True  # Moving counts as progress

        return UnitActions.invoke_unit_action(unit, UnitActionType.CreateImprovement)

    @staticmethod
    def has_workable_sea_resource(tile: Tile, civ_info: Civilization) -> bool:
        """Check whether tile is water and has a resource civInfo can improve.
        
        Args:
            tile: The tile to check
            civ_info: The civilization to check for
            
        Returns:
            True if the tile has a workable sea resource
            
        Does check whether a matching improvement can currently be built (e.g. Oil before Refrigeration).
        Can return True if there is an improvement that does not match the resource (for future modding abilities).
        Does not check tile ownership - caller automate_work_boats already did, 
        other callers need to ensure this explicitly.
        """
        if not tile.is_water:
            return False
        if tile.resource is None:
            return False
        if (tile.improvement is not None 
            and tile.tile_resource.is_improved_by(tile.improvement)):
            return False
        if not tile.has_viewable_resource(civ_info):
            return False
        return any(
            tile.improvement_functions.can_build_improvement(
                civ_info.game_info.ruleset.tile_improvements[improvement],
                civ_info
            )
            for improvement in tile.tile_resource.get_improvements()
        )

    @staticmethod
    def is_not_bonus_resource_or_workable(tile: Tile, civ_info: Civilization) -> bool:
        """Test whether improving the resource on tile benefits civInfo (yields or strategic or luxury).
        
        Args:
            tile: The tile to check
            civ_info: The civilization to check for
            
        Returns:
            True if the resource is not a bonus resource or is workable
            
        Only tests resource type and city range, not any improvement requirements.
        Raises:
            ValueError: If tile has no resource
        """
        if tile.resource is None:
            raise ValueError("Tile has no resource")
            
        return (tile.tile_resource.resource_type != ResourceType.Bonus  # Improve Oil even if no City reaps the yields
                or any(city.tiles_in_range.contains(tile)  # Improve Fish only if any of our Cities reaps the yields
                      for city in civ_info.cities)) 