from typing import Set, Optional, List

from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.managers import ReligionState
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UnitActionType
from com.unciv.models.ruleset.unique import UniqueTriggerActivation, UniqueType
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActionModifiers, UnitActions
from com.unciv.logic.automation.unit import (
    SpecificUnitAutomation,
    ReligiousUnitAutomation,
    UnitAutomation
)

class CivilianUnitAutomation:
    """Handles AI automation for civilian units."""

    @staticmethod
    def should_clear_tile_for_add_in_capital_units(unit: MapUnit, tile: Tile) -> bool:
        """Check if a unit should clear a tile for AddInCapital units.
        
        Args:
            unit: The unit to check
            tile: The tile to check
            
        Returns:
            bool: Whether the unit should clear the tile
        """
        return (tile.is_city_center() 
                and tile.get_city().is_capital()
                and not unit.has_unique(UniqueType.AddInCapital)
                and any(unit.has_unique(UniqueType.AddInCapital) 
                       for unit in unit.civ.units.get_civ_units()))

    @staticmethod
    def automate_civilian_unit(unit: MapUnit, dangerous_tiles: Set[Tile]) -> None:
        """Automate the actions of a civilian unit.
        
        Args:
            unit: The unit to automate
            dangerous_tiles: Set of tiles that are dangerous to enter
        """
        # To allow "found city" actions that can only trigger a limited number of times
        settler_unique = (
            UnitActionModifiers.get_usable_unit_action_uniques(unit, UniqueType.FoundCity)[0]
            if UnitActionModifiers.get_usable_unit_action_uniques(unit, UniqueType.FoundCity)
            else UnitActionModifiers.get_usable_unit_action_uniques(unit, UniqueType.FoundPuppetCity)[0]
            if UnitActionModifiers.get_usable_unit_action_uniques(unit, UniqueType.FoundPuppetCity)
            else None
        )
        
        if settler_unique is not None:
            return SpecificUnitAutomation.automate_settler_actions(unit, dangerous_tiles)

        if CivilianUnitAutomation.try_run_away_if_necessary(unit):
            return

        if CivilianUnitAutomation.should_clear_tile_for_add_in_capital_units(unit, unit.current_tile):
            # First off get out of the way, then decide if you actually want to do something else
            tiles_can_move_to = {
                tile: dist for tile, dist in unit.movement.get_distance_to_tiles().items()
                if unit.movement.can_move_to(tile)
            }
            if tiles_can_move_to:
                unit.movement.move_to_tile(
                    min(tiles_can_move_to.items(), key=lambda x: x[1].total_movement)[0]
                )

        if unit.is_automating_road_connection():
            return unit.civ.get_worker_automation().road_to_automation.automate_connect_road(unit, dangerous_tiles)

        if unit.cache.has_unique_to_build_improvements:
            return unit.civ.get_worker_automation().automate_worker_action(unit, dangerous_tiles)

        if unit.cache.has_unique_to_create_water_improvements:
            if not unit.civ.get_worker_automation().automate_work_boats(unit):
                UnitAutomation.try_explore(unit)
            return

        if (unit.has_unique(UniqueType.MayFoundReligion)
            and unit.civ.religion_manager.religion_state < ReligionState.Religion
            and unit.civ.religion_manager.may_found_religion_at_all()):
            return ReligiousUnitAutomation.found_religion(unit)

        if (unit.has_unique(UniqueType.MayEnhanceReligion)
            and unit.civ.religion_manager.religion_state < ReligionState.EnhancedReligion
            and unit.civ.religion_manager.may_enhance_religion_at_all()):
            return ReligiousUnitAutomation.enhance_religion(unit)

        # We try to add any unit in the capital we can, though that might not always be desirable
        # For now its a simple option to allow AI to win a science victory again
        if unit.has_unique(UniqueType.AddInCapital):
            return SpecificUnitAutomation.automate_add_in_capital(unit)

        # todo this now supports "Great General"-like mod units not combining 'aura' and citadel
        # abilities, but not additional capabilities if automation finds no use for those two
        if (unit.cache.has_strength_bonus_in_radius_unique
            and SpecificUnitAutomation.automate_great_general(unit)):
            return
        if (unit.cache.has_citadel_placement_unique 
            and SpecificUnitAutomation.automate_citadel_placer(unit)):
            return
        if (unit.cache.has_citadel_placement_unique 
            or unit.cache.has_strength_bonus_in_radius_unique):
            return SpecificUnitAutomation.automate_great_general_fallback(unit)

        if unit.civ.religion_manager.may_spread_religion_at_all(unit):
            return ReligiousUnitAutomation.automate_missionary(unit)

        if (unit.has_unique(UniqueType.PreventSpreadingReligion) 
            or unit.has_unique(UniqueType.CanRemoveHeresy)):
            return ReligiousUnitAutomation.automate_inquisitor(unit)

        is_late_game = CivilianUnitAutomation._is_late_game(unit.civ)
        # Great scientist -> Hurry research if late game
        # Great writer -> Hurry policy if late game
        if is_late_game:
            if UnitActions.invoke_unit_action(unit, UnitActionType.HurryResearch):
                return
            if UnitActions.invoke_unit_action(unit, UnitActionType.HurryPolicy):
                return
            # TODO: save up great scientists/writers for late game (8 turns after research labs/broadcast towers resp.)

        # Great merchant -> Conduct trade mission if late game and if not at war.
        # TODO: This could be more complex to walk to the city state that is most beneficial to
        #  also have more influence.
        if (unit.has_unique(UniqueType.CanTradeWithCityStateForGoldAndInfluence)
            # Don't wander around with the great merchant when at war. Barbs might also be a
            # problem, but hopefully by the time we have a great merchant, they're under control.
            and not unit.civ.is_at_war()
            and is_late_game):
            if SpecificUnitAutomation.conduct_trade_mission(unit):
                return

        # Great engineer -> Try to speed up wonder construction
        if (unit.has_unique(UniqueType.CanSpeedupConstruction)
            or unit.has_unique(UniqueType.CanSpeedupWonderConstruction)):
            if SpecificUnitAutomation.speedup_wonder_construction(unit):
                return

        if unit.has_unique(UniqueType.GainFreeBuildings):
            unique = unit.get_matching_uniques(UniqueType.GainFreeBuildings)[0]
            building_name = unique.params[0]
            # Choose the city that is closest in distance and does not have the building constructed.
            city_to_gain_building = min(
                (city for city in unit.civ.cities
                 if (not city.city_constructions.contains_building_or_equivalent(building_name)
                     and (unit.movement.can_move_to(city.get_center_tile()) 
                          or unit.current_tile == city.get_center_tile()))),
                key=lambda city: len(unit.movement.get_shortest_path(city.get_center_tile())),
                default=None
            )

            if city_to_gain_building is not None:
                if unit.current_tile == city_to_gain_building.get_center_tile():
                    UniqueTriggerActivation.trigger_unique(
                        unique, unit.civ, unit=unit, tile=unit.current_tile
                    )
                    UnitActionModifiers.activate_side_effects(unit, unique)
                    return
                else:
                    unit.movement.head_towards(city_to_gain_building.get_center_tile())
                return

        # TODO: The AI tends to have a lot of great generals. Maybe there should be a cutoff
        #  (depending on number of cities) and after that they should just be used to start golden
        #  ages?

        if SpecificUnitAutomation.automate_improvement_placer(unit):
            return
        
        golden_age_action = next(
            (action for action in UnitActions.get_unit_actions(unit, UnitActionType.TriggerUnique)
             if (action.action is not None 
                 and action.associated_unique.type in [
                     UniqueType.OneTimeEnterGoldenAge,
                     UniqueType.OneTimeEnterGoldenAgeTurns
                 ])),
            None
        )
        if golden_age_action is not None:
            golden_age_action.action()
            return

        return  # The AI doesn't know how to handle unknown civilian units

    @staticmethod
    def _is_late_game(civ: Civilization) -> bool:
        """Check if the game is in the late stages.
        
        Args:
            civ: The civilization to check
            
        Returns:
            bool: Whether the game is in the late stages
        """
        research_complete_percent = (
            len(civ.tech.researched_technologies) / len(civ.game_info.ruleset.technologies)
        )
        return research_complete_percent >= 0.6

    @staticmethod
    def try_run_away_if_necessary(unit: MapUnit) -> bool:
        """Try to run away from danger if necessary.
        
        Args:
            unit: The unit to check
            
        Returns:
            bool: Whether the unit spent its turn running away
        """
        # This is a little 'Bugblatter Beast of Traal': Run if we can attack an enemy
        # Cheaper than determining which enemies could attack us next turn
        enemy_units_in_walking_distance = [
            tile for tile in unit.movement.get_distance_to_tiles().keys()
            if unit.civ.threat_manager.does_tile_have_military_enemy(tile)
        ]

        if (enemy_units_in_walking_distance 
            and not unit.base_unit.is_military
            and unit.get_tile().military_unit is None 
            and not unit.get_tile().is_city_center()):
            CivilianUnitAutomation._run_away(unit)
            return True

        return False

    @staticmethod
    def _run_away(unit: MapUnit) -> None:
        """Make the unit run away from danger.
        
        Args:
            unit: The unit to make run away
        """
        reachable_tiles = unit.movement.get_distance_to_tiles()
        
        # Try to enter a city
        enterable_city = next(
            (tile for tile in reachable_tiles.keys()
             if tile.is_city_center() and unit.movement.can_move_to(tile)),
            None
        )
        if enterable_city is not None:
            unit.movement.move_to_tile(enterable_city)
            return
            
        # Try to move to a tile with a friendly military unit
        defensive_unit = next(
            (tile for tile in reachable_tiles.keys()
             if (tile.military_unit is not None 
                 and tile.military_unit.civ == unit.civ 
                 and tile.civilian_unit is None)),
            None
        )
        if defensive_unit is not None:
            unit.movement.move_to_tile(defensive_unit)
            return
            
        # Try to move to the tile furthest from enemies
        tile_furthest_from_enemy = max(
            (tile for tile in reachable_tiles.keys()
             if (unit.movement.can_move_to(tile) 
                 and unit.get_damage_from_terrain(tile) < unit.health)),
            key=lambda tile: unit.civ.threat_manager.get_distance_to_closest_enemy_unit(
                unit.get_tile(), 4, False
            ),
            default=None
        )
        if tile_furthest_from_enemy is not None:
            unit.movement.move_to_tile(tile_furthest_from_enemy) 