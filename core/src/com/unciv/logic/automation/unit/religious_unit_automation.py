from typing import Optional, List

from com.unciv import Constants
from com.unciv.logic.automation import Automation, ThreatLevel
from com.unciv.logic.city import City
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags, RelationshipLevel
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.models import UnitActionType
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActions
from com.unciv.logic.automation.unit import UnitAutomation

class ReligiousUnitAutomation:
    """Handles AI automation for religious units."""

    @staticmethod
    def automate_missionary(unit: MapUnit) -> None:
        """Automate missionary unit actions.
        
        Args:
            unit: The missionary unit to automate
        """
        if (unit.religion != unit.civ.religion_manager.religion?.name 
            or unit.religion is None):
            return unit.disband()

        our_cities_without_religion = [
            city for city in unit.civ.cities
            if city.religion.get_majority_religion() != unit.civ.religion_manager.religion
        ]
        
        def is_valid_spread_religion_target(city: City) -> bool:
            """Check if a city is a valid target for spreading religion.
            
            Args:
                city: The city to check
                
            Returns:
                bool: Whether the city is a valid target
            """
            diplomacy_manager = unit.civ.get_diplomacy_manager(city.civ)
            if diplomacy_manager?.has_flag(DiplomacyFlags.AgreedToNotSpreadReligion):
                # See NextTurnAutomation - these are the conditions under which AI agrees to religious demands
                # If they still hold, keep the agreement, otherwise we can renege
                if diplomacy_manager.relationship_level() == RelationshipLevel.Ally:
                    return False
                if Automation.threat_assessment(unit.civ, city.civ) >= ThreatLevel.High:
                    return False
            return True

        def rank_city_for_religion_spread(city: City) -> int:
            """Rank a city for religion spread priority.
            
            Args:
                city: The city to rank
                
            Returns:
                int: Lower value means higher priority
            """
            rank = city.get_center_tile().aerial_distance_to(unit.get_tile())
            
            diplomacy_manager = unit.civ.get_diplomacy_manager(city.civ)
            if diplomacy_manager?.has_flag(DiplomacyFlags.AgreedToNotSpreadReligion):
                rank += 10  # Greatly discourage, but if the other options are too far away we'll take it anyway
                
            return rank
        
        city = None
        if our_cities_without_religion:
            city = min(
                our_cities_without_religion,
                key=lambda city: city.get_center_tile().aerial_distance_to(unit.get_tile())
            )
        else:
            valid_cities = [
                city for city in unit.civ.game_info.get_cities()
                if (city.religion.get_majority_religion() != unit.civ.religion_manager.religion
                    and city.civ.knows(unit.civ)
                    and not city.civ.is_at_war_with(unit.civ)
                    and not city.religion.is_protected_by_inquisitor(unit.religion)
                    and is_valid_spread_religion_target(city))
            ]
            if valid_cities:
                city = min(valid_cities, key=rank_city_for_religion_spread)

        if not city:
            return

        destination = next(
            (tile for tile in sorted(
                city.get_tiles(),
                key=lambda tile: tile.aerial_distance_to(unit.get_tile())
            )
            if (unit.movement.can_move_to(tile) or tile == unit.get_tile())
            and unit.movement.can_reach(tile)),
            None
        )
        
        if not destination:
            return

        unit.movement.head_towards(destination)

        if (unit.get_tile() in city.get_tiles() 
            and unit.civ.religion_manager.may_spread_religion_now(unit)):
            UnitActions.invoke_unit_action(unit, UnitActionType.SpreadReligion)

    @staticmethod
    def automate_inquisitor(unit: MapUnit) -> None:
        """Automate inquisitor unit actions.
        
        Args:
            unit: The inquisitor unit to automate
        """
        civ_religion = unit.civ.religion_manager.religion

        if (unit.religion != civ_religion?.name 
            or unit.religion is None):
            return unit.disband()  # No need to keep a unit we can't use, as it only blocks religion spreads of religions other that its own

        holy_city = unit.civ.religion_manager.get_holy_city()
        city_to_convert = ReligiousUnitAutomation._determine_best_inquisitor_city_to_convert(unit)
        pressure_deficit = (
            city_to_convert.religion.get_pressure_deficit(civ_religion?.name)
            if city_to_convert else 0
        )

        cities_to_protect = [
            city for city in unit.civ.cities
            if (city.religion.get_majority_religion() == civ_religion
                # We only look at cities that are not currently protected or are protected by us
                and (not city.religion.is_protected_by_inquisitor()
                     or unit.get_tile() in city.get_center_tile().get_tiles_in_distance(1)))
        ]

        # Cities with most populations will be prioritized by the AI
        city_to_protect = max(
            cities_to_protect,
            key=lambda city: city.population.population,
            default=None
        )

        destination_city = None
        if (city_to_convert
            and (city_to_convert == holy_city
                 or pressure_deficit > Constants.ai_prefer_inquisitor_over_missionary_pressure_difference
                 or (city_to_convert.religion.is_blocked_holy_city 
                     and city_to_convert.religion.religion_this_is_the_holy_city_of == civ_religion?.name))
            and unit.has_unique(UniqueType.CanRemoveHeresy)):
            destination_city = city_to_convert
        elif (city_to_protect 
              and unit.has_unique(UniqueType.PreventSpreadingReligion)):
            if holy_city and not holy_city.religion.is_protected_by_inquisitor():
                destination_city = holy_city
            else:
                destination_city = city_to_protect
        elif city_to_convert:
            destination_city = city_to_convert

        if not destination_city:
            return

        destination_tile = next(
            (tile for tile in sorted(
                destination_city.get_center_tile().neighbors,
                key=lambda tile: tile.aerial_distance_to(unit.current_tile)
            )
            if (unit.movement.can_move_to(tile) or tile == unit.get_tile())
            and unit.movement.can_reach(tile)),
            None
        )
        
        if not destination_tile:
            return

        unit.movement.head_towards(destination_tile)

        if (city_to_convert 
            and unit.get_tile().get_city() == destination_city):
            UnitActions.invoke_unit_action(unit, UnitActionType.RemoveHeresy)

    @staticmethod
    def _determine_best_inquisitor_city_to_convert(unit: MapUnit) -> Optional[City]:
        """Determine the best city for an inquisitor to convert.
        
        Args:
            unit: The inquisitor unit
            
        Returns:
            Optional[City]: The best city to convert, or None if no valid targets
        """
        if (unit.religion != unit.civ.religion_manager.religion?.name 
            or not unit.has_unique(UniqueType.CanRemoveHeresy)):
            return None

        holy_city = unit.civ.religion_manager.get_holy_city()
        if (holy_city 
            and holy_city.religion.get_majority_religion() != unit.civ.religion_manager.religion):
            return holy_city

        blocked_holy_city = next(
            (city for city in unit.civ.cities
             if (city.religion.is_blocked_holy_city 
                 and city.religion.religion_this_is_the_holy_city_of == unit.religion)),
            None
        )
        if blocked_holy_city:
            return blocked_holy_city

        return max(
            (city for city in unit.civ.cities
             if (city.religion.get_majority_religion()
                 and city.religion.get_majority_religion() != unit.civ.religion_manager.religion
                 and city.get_center_tile().aerial_distance_to(unit.current_tile) <= 20)),
            key=lambda city: city.religion.get_pressure_deficit(
                unit.civ.religion_manager.religion?.name
            ),
            default=None
        )

    @staticmethod
    def found_religion(unit: MapUnit) -> None:
        """Automate founding a religion.
        
        Args:
            unit: The unit to use for founding religion
        """
        city_to_found_religion_at = None
        if (unit.get_tile().is_city_center() 
            and not unit.get_tile().owning_city.is_holy_city()):
            city_to_found_religion_at = unit.get_tile().owning_city
        else:
            city_to_found_religion_at = next(
                (city for city in unit.civ.cities
                 if (not city.is_holy_city()
                     and unit.movement.can_move_to(city.get_center_tile())
                     and unit.movement.can_reach(city.get_center_tile()))),
                None
            )

        if not city_to_found_religion_at:
            return

        if unit.get_tile() != city_to_found_religion_at.get_center_tile():
            unit.movement.head_towards(city_to_found_religion_at.get_center_tile())
            return

        UnitActions.invoke_unit_action(unit, UnitActionType.FoundReligion)

    @staticmethod
    def enhance_religion(unit: MapUnit) -> None:
        """Automate enhancing a religion.
        
        Args:
            unit: The unit to use for enhancing religion
        """
        # Try go to a nearby city
        if not unit.get_tile().is_city_center():
            UnitAutomation.try_enter_own_closest_city(unit)

        # If we were unable to go there this turn, unable to do anything else
        if not unit.get_tile().is_city_center():
            return

        UnitActions.invoke_unit_action(unit, UnitActionType.EnhanceReligion) 