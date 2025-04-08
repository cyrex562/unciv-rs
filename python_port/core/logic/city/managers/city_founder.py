from typing import Optional, Set, List
from dataclasses import dataclass
import random
from enum import Enum

class ReligionState(Enum):
    PANTHEON = "Pantheon"

class UniqueType(Enum):
    BORROWS_CITY_NAMES = "BorrowsCityNames"
    TRIGGER_UPON_FOUNDING_CITY = "TriggerUponFoundingCity"

class DiplomacyFlags(Enum):
    SETTLED_CITIES_NEAR_US = "SettledCitiesNearUs"

class NamingConstants:
    # Prefixes to add when every base name is taken, ordered
    PREFIXES = ["New", "Neo", "Nova", "Altera"]
    FALLBACK = "City Without A Name"

class CityFounder:
    def found_city(self, civ_info, city_location, unit=None):
        city = City()

        city.founding_civ = civ_info.civ_name
        city.turn_acquired = civ_info.game_info.turns
        city.location = city_location
        city.set_transients(civ_info)

        city.name = self.generate_new_city_name(
            civ_info,
            {civ for civ in civ_info.game_info.civilizations if civ.is_alive()}
        ) or NamingConstants.FALLBACK

        city.is_original_capital = civ_info.cities_created == 0
        if city.is_original_capital:
            civ_info.has_ever_owned_original_capital = True
            # if you have some culture before the 1st city is founded, you may want to adopt the 1st policy
            civ_info.policies.should_open_policy_picker = True
        civ_info.cities_created += 1

        civ_info.cities.append(city)

        starting_era = civ_info.game_info.game_parameters.starting_era

        city.expansion.reset()
        city.try_update_road_status()

        tile = city.get_center_tile()
        for terrain_feature in [tf for tf in tile.terrain_features
                              if f"Remove {tf}" in city.get_ruleset().tile_improvements]:
            tile.remove_terrain_feature(terrain_feature)

        if "cityCenter" in civ_info.game_info.ruleset.tile_improvements:
            tile.set_improvement("cityCenter", civ_info)
        tile.stop_working_on_improvement()

        ruleset = civ_info.game_info.ruleset
        city.worked_tiles = set()  # reassign 1st working tile

        city.population.set_population(ruleset.eras[starting_era].settler_population)

        if civ_info.religion_manager.religion_state == ReligionState.PANTHEON:
            city.religion.add_pressure(
                civ_info.religion_manager.religion.name,
                200 * city.population.population
            )

        city.population.auto_assign_population()

        # Update proximity rankings for all civs
        for other_civ in civ_info.game_info.get_alive_major_civs():
            if civ_info.get_proximity(other_civ) != "Neighbors":  # unless already neighbors
                civ_info.cache.update_proximity(other_civ,
                    other_civ.cache.update_proximity(civ_info))

        for other_civ in civ_info.game_info.get_alive_city_states():
            if civ_info.get_proximity(other_civ) != "Neighbors":  # unless already neighbors
                civ_info.cache.update_proximity(other_civ,
                    other_civ.cache.update_proximity(civ_info))

        self.trigger_cities_settled_near_other_civ(city)
        civ_info.game_info.city_distances.set_dirty()

        self.add_starting_buildings(city, civ_info, starting_era)

        for unique in civ_info.get_triggered_uniques(
            UniqueType.TRIGGER_UPON_FOUNDING_CITY,
            StateForConditionals(civ_info, city, unit)
        ):
            UniqueTriggerActivation.trigger_unique(
                unique, civ_info, city, unit,
                trigger_notification_text="due to founding a city"
            )

        if unit is not None:
            for unique in unit.get_triggered_uniques(
                UniqueType.TRIGGER_UPON_FOUNDING_CITY,
                StateForConditionals(civ_info, city, unit)
            ):
                UniqueTriggerActivation.trigger_unique(
                    unique, civ_info, city, unit,
                    trigger_notification_text="due to founding a city"
                )

        return city

    def generate_new_city_name(
        self,
        founding_civ,
        alive_civs: Set
    ) -> Optional[str]:
        used_city_names = {
            city.name
            for civilization in alive_civs
            for city in civilization.cities
        }

        # Attempt to return the first missing name from the list of city names
        for city_name in founding_civ.nation.cities:
            if city_name not in used_city_names:
                return city_name

        # If all names are taken and this nation borrows city names,
        # return a random borrowed city name
        if founding_civ.has_unique(UniqueType.BORROWS_CITY_NAMES):
            return self.borrow_city_name(founding_civ, alive_civs, used_city_names)

        # If the nation doesn't have the unique above,
        # return the first missing name with an increasing number of prefixes attached
        for number in range(1, 11):
            for prefix in NamingConstants.PREFIXES:
                repeated_prefix = f"{prefix} [" * number
                suffix = "]" * number
                for base_name in founding_civ.nation.cities:
                    candidate = repeated_prefix + base_name + suffix
                    if candidate not in used_city_names:
                        return candidate

        # If all else fails (by using some sort of rule set mod without city names)
        return None

    def borrow_city_name(
        self,
        founding_civ,
        alive_civs: Set,
        used_city_names: Set[str]
    ) -> Optional[str]:
        alive_major_nations = {
            civ.nation for civ in alive_civs
            if civ.is_major_civ()
        }

        # We take the last unused city name for each other major nation in this game,
        # skipping nations whose names are exhausted,
        # and choose a random one from that pool if it's not empty.
        other_major_nations = {
            nation for nation in alive_major_nations
            if nation != founding_civ.nation
        }

        new_city_names = {
            city for nation in other_major_nations
            for city in reversed(nation.cities)
            if city not in used_city_names
        }

        if new_city_names:
            return random.choice(list(new_city_names))

        # As per fandom wiki, once the names from the other nations in the game are exhausted,
        # names are taken from the rest of the major nations in the rule set
        absent_major_nations = {
            nation for nation in founding_civ.game_info.ruleset.nations.values()
            if nation.is_major_civ and nation not in alive_major_nations
        }

        new_city_names = {
            city for nation in absent_major_nations
            for city in nation.cities
            if city not in used_city_names
        }

        if new_city_names:
            return random.choice(list(new_city_names))

        # If for some reason we have used every single city name in the game
        return None

    def add_starting_buildings(self, city, civ_info, starting_era: str):
        ruleset = civ_info.game_info.ruleset

        if len(civ_info.cities) == 1:
            capital_city_indicator = civ_info.capital_city_indicator(city)
            if capital_city_indicator is not None:
                city.city_constructions.add_building(
                    capital_city_indicator,
                    try_add_free_buildings=False
                )

        # Add buildings and pop we get from starting in this era
        for building_name in ruleset.eras[starting_era].settler_buildings:
            building = ruleset.buildings.get(building_name)
            if building is None:
                continue

            unique_building = civ_info.get_equivalent_building(building)
            if unique_building.is_buildable(city.city_constructions):
                city.city_constructions.add_building(
                    unique_building,
                    try_add_free_buildings=False
                )

        civ_info.civ_constructions.try_add_free_buildings()

    def trigger_cities_settled_near_other_civ(self, city):
        cities_within_6_tiles = [
            other_city for other_civ in city.civ.game_info.civilizations
            if other_civ.is_major_civ() and other_civ != city.civ
            for other_city in other_civ.cities
            if other_city.get_center_tile().aerial_distance_to(city.get_center_tile()) <= 6
        ]

        civs_with_close_cities = {
            other_city.civ for other_city in cities_within_6_tiles
            if other_city.civ.knows(city.civ) and other_city.civ.has_explored(city.get_center_tile())
        }

        for other_civ in civs_with_close_cities:
            other_civ.get_diplomacy_manager(city.civ).set_flag(
                DiplomacyFlags.SETTLED_CITIES_NEAR_US,
                30
            )