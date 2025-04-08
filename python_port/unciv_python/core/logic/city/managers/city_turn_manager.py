"""
CityTurnManager - Manages turn-based operations for a city
"""

import random
from typing import List, Optional
from enum import Enum

class CityFlags(Enum):
    """Flags that can be set on a city"""
    RESOURCE_DEMAND = "ResourceDemand"
    WE_LOVE_THE_KING = "WeLoveTheKing"
    RESISTANCE = "Resistance"

class CityFocus(Enum):
    """Focus types for a city"""
    GOLD_FOCUS = "GoldFocus"

class NotificationCategory(Enum):
    """Categories for notifications"""
    GENERAL = "General"

class NotificationIcon(Enum):
    """Icons for notifications"""
    CITY = "City"
    HAPPINESS = "Happiness"
    RESISTANCE = "StatIcons/Resistance"
    FIRE = "OtherIcons/Fire"

class ResourceType(Enum):
    """Types of resources"""
    LUXURY = "Luxury"

class UniqueType(Enum):
    """Types of unique abilities"""
    CITY_STATE_ONLY_RESOURCE = "CityStateOnlyResource"
    CITIES_ARE_RAZED_X_TIMES_FASTER = "CitiesAreRazedXTimesFaster"

class SpyFleeReason(Enum):
    """Reasons for spies to flee"""
    OTHER = "Other"

class EmpireOverviewCategories(Enum):
    """Categories for empire overview"""
    RESOURCES = "Resources"

class CityAction:
    """Action related to a city"""
    @staticmethod
    def with_location(city):
        """Create a city action with location"""
        return [LocationAction(city.location)]

class LocationAction:
    """Action related to a location"""
    def __init__(self, location):
        self.location = location

class OverviewAction:
    """Action related to overview"""
    def __init__(self, category):
        self.category = category

class CityTurnManager:
    """Manages turn-based operations for a city"""

    def __init__(self, city):
        """Initialize the turn manager with a city"""
        self.city = city

    def start_turn(self):
        """Process start of turn actions"""
        # Construct units at the beginning of the turn,
        # so they won't be generated out in the open and vulnerable to enemy attacks before you can control them
        self.city.city_constructions.construct_if_enough()

        self.city.try_update_road_status()
        self.city.attacked_this_turn = False

        # The ordering is intentional - you get a turn without WLTKD even if you have the next resource already
        # Also resolve end of resistance before update_citizens
        if not self.city.has_flag(CityFlags.WE_LOVE_THE_KING):
            self.try_we_love_the_king()
        self.next_turn_flags()

        if self.city.is_puppet:
            self.city.set_city_focus(CityFocus.GOLD_FOCUS)
            self.city.reassign_all_population()
        elif self.city.should_reassign_population:
            self.city.reassign_population()  # includes city_stats.update
        else:
            self.city.city_stats.update()

        # Seed resource demand countdown
        if self.city.demanded_resource == "" and not self.city.has_flag(CityFlags.RESOURCE_DEMAND):
            self.city.set_flag(
                CityFlags.RESOURCE_DEMAND,
                (25 if self.city.is_capital() else 15) + random.randint(0, 9))

    def try_we_love_the_king(self):
        """Try to trigger We Love The King Day"""
        if self.city.demanded_resource == "":
            return
        if self.city.get_available_resource_amount(self.city.demanded_resource) > 0:
            self.city.set_flag(CityFlags.WE_LOVE_THE_KING, 20 + 1)  # +1 because it will be decremented by 1 in the same start_turn()
            self.city.civ.add_notification(
                f"Because they have [{self.city.demanded_resource}], the citizens of [{self.city.name}] are celebrating We Love The King Day!",
                CityAction.with_location(self.city),
                NotificationCategory.GENERAL,
                NotificationIcon.CITY,
                NotificationIcon.HAPPINESS)

    def next_turn_flags(self):
        """Process flags for the next turn"""
        for flag in list(self.city.flags_countdown.keys()):
            if self.city.flags_countdown[flag] > 0:
                self.city.flags_countdown[flag] = self.city.flags_countdown[flag] - 1

            if self.city.flags_countdown[flag] == 0:
                del self.city.flags_countdown[flag]

                if flag == CityFlags.RESOURCE_DEMAND.name:
                    self.demand_new_resource()
                elif flag == CityFlags.WE_LOVE_THE_KING.name:
                    self.city.civ.add_notification(
                        f"We Love The King Day in [{self.city.name}] has ended.",
                        CityAction.with_location(self.city),
                        NotificationCategory.GENERAL,
                        NotificationIcon.CITY)
                    self.demand_new_resource()
                elif flag == CityFlags.RESISTANCE.name:
                    self.city.should_reassign_population = True
                    self.city.civ.add_notification(
                        f"The resistance in [{self.city.name}] has ended!",
                        CityAction.with_location(self.city),
                        NotificationCategory.GENERAL,
                        NotificationIcon.RESISTANCE)

    def demand_new_resource(self):
        """Demand a new resource from the city"""
        candidates = [
            resource for resource in self.city.get_ruleset().tile_resources.values()
            if resource.resource_type == ResourceType.LUXURY and  # Must be luxury
            not resource.has_unique(UniqueType.CITY_STATE_ONLY_RESOURCE) and  # Not a city-state only resource eg jewelry
            resource.name != self.city.demanded_resource and  # Not same as last time
            resource.name in self.city.tile_map.resources and  # Must exist somewhere on the map
            not any(near_tile.resource == resource.name
                   for near_tile in self.city.get_center_tile().get_tiles_in_distance(self.city.get_work_range()))  # Not in this city's radius
        ]

        missing_resources = [resource for resource in candidates if not self.city.civ.has_resource(resource.name)]

        if not missing_resources:  # hooray happy day forever!
            self.city.demanded_resource = random.choice(candidates).name if candidates else ""
            return  # actually triggering "wtlk" is done in try_we_love_the_king(), *next turn*

        chosen_resource = random.choice(missing_resources) if missing_resources else None

        self.city.demanded_resource = chosen_resource.name if chosen_resource else ""  # mods may have no resources as candidates even
        if self.city.demanded_resource == "":  # Failed to get a valid resource, try again some time later
            self.city.set_flag(CityFlags.RESOURCE_DEMAND, 15 + random.randint(0, 9))
        else:
            self.city.civ.add_notification(
                f"[{self.city.name}] demands [{self.city.demanded_resource}]!",
                [LocationAction(self.city.location), OverviewAction(EmpireOverviewCategories.RESOURCES)],
                NotificationCategory.GENERAL,
                NotificationIcon.CITY,
                f"ResourceIcons/{self.city.demanded_resource}")

    def end_turn(self):
        """Process end of turn actions"""
        stats = self.city.city_stats.current_city_stats

        self.city.city_constructions.end_turn(stats)
        self.city.expansion.next_turn(stats.culture)

        if self.city.is_being_razed:
            removed_population = 1 + sum(
                int(unique.params[0]) - 1
                for unique in self.city.civ.get_matching_uniques(UniqueType.CITIES_ARE_RAZED_X_TIMES_FASTER)
            )

            if self.city.population.population <= removed_population:
                self.city.espionage.remove_all_present_spies(SpyFleeReason.OTHER)
                self.city.civ.add_notification(
                    f"[{self.city.name}] has been razed to the ground!",
                    self.city.location,
                    NotificationCategory.GENERAL,
                    NotificationIcon.FIRE
                )
                self.city.destroy_city()
            else:  # if not razed yet:
                self.city.population.add_population(-removed_population)
                if self.city.population.food_stored >= self.city.population.get_food_to_next_population():  # if surplus in the granary...
                    self.city.population.food_stored = self.city.population.get_food_to_next_population() - 1  # ...reduce below the new growth threshold
        else:
            self.city.population.next_turn(self.city.food_for_next_turn())

        # This should go after the population change, as that might impact the amount of followers in this city
        if self.city.civ.game_info.is_religion_enabled():
            self.city.religion.end_turn()

        if self.city in self.city.civ.cities:  # city was not destroyed
            self.city.health = min(self.city.health + 20, self.city.get_max_health())
            self.city.population.unassign_extra_population()