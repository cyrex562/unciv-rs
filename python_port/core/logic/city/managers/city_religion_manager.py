from typing import Optional, Dict, Set, List
from dataclasses import dataclass
import math
from enum import Enum

class NotificationCategory(Enum):
    RELIGION = "Religion"

class NotificationIcon(Enum):
    FAITH = "Faith"

class UniqueType(Enum):
    STATS_WHEN_ADOPTING_RELIGION = "StatsWhenAdoptingReligion"
    RELIGION_SPREAD_DISTANCE = "ReligionSpreadDistance"
    NATURAL_RELIGION_SPREAD_STRENGTH = "NaturalReligionSpreadStrength"
    PREVENT_SPREADING_RELIGION = "PreventSpreadingReligion"

class Counter:
    def __init__(self):
        self._data: Dict[str, int] = {}

    def add(self, key: str, amount: int = 1):
        self._data[key] = self._data.get(key, 0) + amount

    def __getitem__(self, key: str) -> int:
        return self._data.get(key, 0)

    def __setitem__(self, key: str, value: int):
        self._data[key] = value

    def keys(self):
        return self._data.keys()

    def values(self):
        return self._data.values()

    def items(self):
        return self._data.items()

    def sum(self) -> int:
        return sum(self.values())

    def clone(self) -> 'Counter':
        counter = Counter()
        for key, value in self._data.items():
            counter._data[key] = value
        return counter

    def clear(self):
        self._data.clear()

    def remove(self, key: str):
        if key in self._data:
            del self._data[key]

    def put_all(self, other: 'Counter'):
        for key, value in other._data.items():
            self._data[key] = value

class CityReligionManager:
    def __init__(self):
        self.city = None  # Will be set via set_transients

        # This needs to be kept track of for the
        # "[Stats] when a city adopts this religion for the first time" unique
        self.religions_at_some_point_adopted: Set[str] = set()

        self.pressures = Counter()
        # Cached because using `updateNumberOfFollowers` to get this value resulted in many calls
        self.followers = Counter()

        self.religion_this_is_the_holy_city_of: Optional[str] = None
        self.is_blocked_holy_city = False

        self.clear_all_pressures()

    @property
    def pressure_from_adjacent_cities(self) -> int:
        return self.city.civ.game_info.speed.religious_pressure_adjacent_city

    def clone(self) -> 'CityReligionManager':
        to_return = CityReligionManager()
        to_return.city = self.city
        to_return.religions_at_some_point_adopted.update(self.religions_at_some_point_adopted)
        to_return.pressures.put_all(self.pressures)
        to_return.followers.put_all(self.followers)
        to_return.religion_this_is_the_holy_city_of = self.religion_this_is_the_holy_city_of
        to_return.is_blocked_holy_city = self.is_blocked_holy_city
        return to_return

    def set_transients(self, city):
        self.city = city
        # We don't need to check for changes in the majority religion, and as this
        # loads in the religion, _of course_ the religion changes, but it shouldn't
        # have any effect
        self.update_number_of_followers(False)

    def end_turn(self):
        self.get_affected_by_surrounding_cities()

    def get_uniques(self, unique_type: UniqueType) -> List['Unique']:
        majority_religion = self.get_majority_religion()
        if majority_religion is None:
            return []
        return majority_religion.follower_belief_unique_map.get_uniques(unique_type)

    def get_pressures(self) -> Counter:
        return self.pressures.clone()

    def clear_all_pressures(self):
        self.pressures.clear()
        # We add pressure for following no religion
        # Basically used as a failsafe so that there is always some religion,
        # and we don't suddenly divide by 0 somewhere
        # Should be removed when updating the followers so it never becomes the majority religion,
        # `None` is used for that instead.
        self.pressures.add("No Religion", 100)

    def add_pressure(self, religion_name: str, amount: int, should_update_followers: bool = True):
        if not self.city.civ.game_info.is_religion_enabled():
            return  # No religion, no pressures
        self.pressures.add(religion_name, amount)

        if should_update_followers:
            self.update_number_of_followers()

    def remove_all_pressures_except_for(self, religion: str):
        pressure_from_this_religion = self.pressures[religion]
        # Atheism is never removed
        pressure_from_atheism = self.pressures["No Religion"]
        self.clear_all_pressures()
        self.pressures.add(religion, pressure_from_this_religion)
        if pressure_from_atheism != 0:
            self.pressures["No Religion"] = pressure_from_atheism
        self.update_number_of_followers()

    def update_pressure_on_population_change(self, population_change_amount: int):
        majority_religion = (
            self.get_majority_religion_name() if self.get_majority_religion_name() is not None
            else "No Religion"
        )

        if population_change_amount > 0:
            self.add_pressure(majority_religion, 100 * population_change_amount)
        else:
            self.update_number_of_followers()

    def trigger_religion_adoption(self, new_majority_religion: str):
        new_majority_religion_object = self.city.civ.game_info.religions[new_majority_religion]
        self.city.civ.add_notification(
            f"Your city [{self.city.name}] was converted to [{new_majority_religion_object.get_religion_display_name()}]!",
            self.city.location,
            NotificationCategory.RELIGION,
            NotificationIcon.FAITH
        )

        if new_majority_religion in self.religions_at_some_point_adopted:
            return

        religion_owning_civ = new_majority_religion_object.get_founder()
        if religion_owning_civ.has_unique(UniqueType.STATS_WHEN_ADOPTING_RELIGION):
            stats_granted = {}
            for unique in religion_owning_civ.get_matching_uniques(UniqueType.STATS_WHEN_ADOPTING_RELIGION):
                stats = unique.stats
                if not unique.is_modified_by_game_speed():
                    multiplier = 1.0
                else:
                    multiplier = self.city.civ.game_info.speed.modifier

                for key, value in stats.items():
                    stats_granted[key] = stats_granted.get(key, 0) + int(value * multiplier)

            for key, value in stats_granted.items():
                religion_owning_civ.add_stat(key, value)

            if religion_owning_civ.has_explored(self.city.get_center_tile()):
                religion_owning_civ.add_notification(
                    f"You gained [{stats_granted}] as your religion was spread to [{self.city.name}]",
                    self.city.location,
                    NotificationCategory.RELIGION,
                    NotificationIcon.FAITH
                )
            else:
                religion_owning_civ.add_notification(
                    f"You gained [{stats_granted}] as your religion was spread to an unknown city",
                    NotificationCategory.RELIGION,
                    NotificationIcon.FAITH
                )

        self.religions_at_some_point_adopted.add(new_majority_religion)

    def update_number_of_followers(self, check_for_religion_adoption: bool = True):
        old_majority_religion = (
            self.get_majority_religion_name() if check_for_religion_adoption
            else None
        )

        previous_followers = self.followers.clone()
        self.followers.clear()

        if self.city.population.population <= 0:
            return

        remainders = {}
        pressure_per_follower = self.pressures.values.sum() / self.city.population.population

        # First give each religion an approximate share based on pressure
        for religion, pressure in self.pressures.items():
            followers_of_this_religion = int(pressure / pressure_per_follower)
            self.followers.add(religion, followers_of_this_religion)
            remainders[religion] = float(pressure) - followers_of_this_religion * pressure_per_follower

        unallocated_population = self.city.population.population - self.followers.values.sum()

        # Divide up the remaining population
        while unallocated_population > 0:
            largest_remainder = max(remainders.items(), key=lambda x: x[1], default=None)
            if largest_remainder is None:
                self.followers.add("No Religion", unallocated_population)
                break
            self.followers.add(largest_remainder[0], 1)
            remainders[largest_remainder[0]] = 0.0
            unallocated_population -= 1

        self.followers.remove("No Religion")

        if check_for_religion_adoption:
            new_majority_religion = self.get_majority_religion_name()
            if (old_majority_religion != new_majority_religion and
                new_majority_religion is not None):
                self.trigger_religion_adoption(new_majority_religion)

            if old_majority_religion != new_majority_religion:
                self.city.civ.cache.update_civ_resources()  # follower uniques can provide resources

            if self.followers._data != previous_followers._data:
                self.city.city_stats.update()

    def get_number_of_followers(self) -> Counter:
        return self.followers.clone()

    def get_followers_of(self, religion: str) -> int:
        return self.followers[religion]

    def get_followers_of_majority_religion(self) -> int:
        majority_religion = self.get_majority_religion_name()
        if majority_religion is None:
            return 0
        return self.followers[majority_religion]

    def get_followers_of_our_religion(self) -> int:
        our_religion = self.city.civ.religion_manager.religion
        if our_religion is None:
            return 0
        return self.followers[our_religion.name]

    def get_followers_of_other_religions_than(self, religion: str) -> int:
        return sum(
            count for rel, count in self.followers.items()
            if rel != religion
        )

    def remove_unknown_pantheons(self):
        """Removes all pantheons except for the one founded by the current owner of the city
         Should be called whenever a city changes hands, e.g. conquering and trading"""
        for pressure in list(self.pressures.keys()):  # Copy the keys because we might modify
            if pressure == "No Religion":
                continue
            corresponding_religion = self.city.civ.game_info.religions[pressure]
            if (corresponding_religion.is_pantheon() and
                corresponding_religion.founding_civ_name != self.city.civ.civ_name):
                self.pressures.remove(pressure)
        self.update_number_of_followers()

    def get_majority_religion_name(self) -> Optional[str]:
        if not self.followers._data:
            return None
        religion_with_max_pressure = max(self.followers.items(), key=lambda x: x[1])[0]
        if religion_with_max_pressure == "No Religion":
            return None
        if self.followers[religion_with_max_pressure] >= self.city.population.population / 2:
            return religion_with_max_pressure
        return None

    def get_majority_religion(self) -> Optional['Religion']:
        majority_religion_name = self.get_majority_religion_name()
        if majority_religion_name is None:
            return None
        return self.city.civ.game_info.religions[majority_religion_name]

    def get_affected_by_surrounding_cities(self):
        if not self.city.civ.game_info.is_religion_enabled():
            return  # No religion, no spreading
        # We don't update the amount of followers yet, as only the end result should matter
        # If multiple religions would become the majority religion due to pressure,
        # this will make it so we only receive a notification for the last one.
        # Also, doing it like this increases performance :D
        if self.city.is_holy_city():
            self.add_pressure(
                self.religion_this_is_the_holy_city_of,
                5 * self.pressure_from_adjacent_cities,
                False
            )

        for other_city in self.city.civ.game_info.get_cities():
            if other_city == self.city:
                continue
            majority_religion_of_city = other_city.religion.get_majority_religion_name()
            if majority_religion_of_city is None:
                continue
            if not self.city.civ.game_info.religions[majority_religion_of_city].is_major_religion():
                continue
            if (other_city.get_center_tile().aerial_distance_to(self.city.get_center_tile()) >
                    other_city.religion.get_spread_range()):
                continue
            self.add_pressure(
                majority_religion_of_city,
                other_city.religion.pressure_amount_to_adjacent_cities(self.city),
                False
            )

        self.update_number_of_followers()

    def get_spread_range(self) -> int:
        spread_range = 10

        for unique in self.city.get_matching_uniques(UniqueType.RELIGION_SPREAD_DISTANCE):
            spread_range += int(unique.params[0])

        majority_religion = self.get_majority_religion()
        if majority_religion is not None:
            for unique in majority_religion.get_founder().get_matching_uniques(UniqueType.RELIGION_SPREAD_DISTANCE):
                spread_range += int(unique.params[0])

        return spread_range

    def get_pressures_from_surrounding_cities(self) -> Counter:
        """Doesn't update the pressures, only returns what they are if the update were to happen right now"""
        added_pressure = Counter()
        if self.city.is_holy_city():
            added_pressure[self.religion_this_is_the_holy_city_of] = 5 * self.pressure_from_adjacent_cities

        all_cities_within_10_tiles = [
            city for city in self.city.civ.game_info.get_cities()
            if (city != self.city and
                city.get_center_tile().aerial_distance_to(self.city.get_center_tile()) <=
                city.religion.get_spread_range())
        ]

        for city in all_cities_within_10_tiles:
            majority_religion_of_city = city.religion.get_majority_religion()
            if majority_religion_of_city is None:
                continue
            if not majority_religion_of_city.is_major_religion():
                continue
            added_pressure.add(
                majority_religion_of_city.name,
                city.religion.pressure_amount_to_adjacent_cities(self.city)
            )

        return added_pressure

    def is_protected_by_inquisitor(self, from_religion: Optional[str] = None) -> bool:
        for tile in self.city.get_center_tile().get_tiles_in_distance(1):
            for unit in [tile.civilian_unit, tile.military_unit]:
                if (unit is not None and
                    unit.religion is not None and
                    (from_religion is None or unit.religion != from_religion) and
                    unit.has_unique(UniqueType.PREVENT_SPREADING_RELIGION)):
                    return True
        return False

    def pressure_amount_to_adjacent_cities(self, pressured_city: 'City') -> int:
        pressure = float(self.pressure_from_adjacent_cities)

        # Follower beliefs of this religion
        for unique in self.city.get_matching_uniques(UniqueType.NATURAL_RELIGION_SPREAD_STRENGTH):
            if pressured_city.matches_filter(unique.params[1]):
                pressure *= float(unique.params[0]) / 100

        # Founder beliefs of this religion
        majority_religion = self.get_majority_religion()
        if majority_religion is not None:
            for unique in majority_religion.get_founder().get_matching_uniques(UniqueType.NATURAL_RELIGION_SPREAD_STRENGTH):
                if pressured_city.matches_filter(unique.params[1]):
                    pressure *= float(unique.params[0]) / 100

        return int(pressure)

    def get_pressure_deficit(self, other_religion: Optional[str]) -> int:
        return (self.get_pressures()[self.get_majority_religion_name() or ""] or 0) - (self.get_pressures()[other_religion or ""] or 0)