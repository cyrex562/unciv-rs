from typing import Optional, Dict, List
from dataclasses import dataclass
import math
from enum import Enum

class NotificationCategory(Enum):
    CITIES = "Cities"

class NotificationIcon(Enum):
    GROWTH = "Growth"
    DEATH = "Death"

class UniqueType(Enum):
    NULLIFIES_GROWTH = "NullifiesGrowth"
    CARRY_OVER_FOOD = "CarryOverFood"
    FOOD_CONSUMPTION_BY_SPECIALISTS = "FoodConsumptionBySpecialists"

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

class CityPopulationManager:
    def __init__(self):
        self.city = None  # Will be set via set_transients
        self._population = 1
        self.food_stored = 0
        self.specialist_allocations = Counter()

    @property
    def population(self) -> int:
        return self._population

    def get_new_specialists(self) -> Counter:
        return self.specialist_allocations

    def clone(self) -> 'CityPopulationManager':
        to_return = CityPopulationManager()
        for key, value in self.specialist_allocations.items():
            to_return.specialist_allocations.add(key, value)
        to_return._population = self._population
        to_return.food_stored = self.food_stored
        return to_return

    def get_number_of_specialists(self) -> int:
        return self.get_new_specialists().sum()

    def get_free_population(self) -> int:
        working_population = len(self.city.worked_tiles)
        return self.population - working_population - self.get_number_of_specialists()

    def get_food_to_next_population(self) -> int:
        # civ v math, civilization.wikia
        food_required = 15 + 6 * (self.population - 1) + math.floor((self.population - 1) ** 1.8)

        food_required *= self.city.civ.game_info.speed.modifier

        if self.city.civ.is_city_state:
            food_required *= 1.5
        if not self.city.civ.is_human():
            food_required *= self.city.civ.game_info.get_difficulty().ai_city_growth_modifier
        return int(food_required)

    def get_num_turns_to_starvation(self) -> Optional[int]:
        """Take None to mean infinity."""
        if not self.city.is_starving():
            return None
        return self.food_stored // -self.city.food_for_next_turn() + 1

    def get_num_turns_to_new_population(self) -> Optional[int]:
        """Take None to mean infinity."""
        if not self.city.is_growing():
            return None
        rounded_food_per_turn = float(self.city.food_for_next_turn())
        remaining_food = self.get_food_to_next_population() - self.food_stored
        turns_to_growth = math.ceil(remaining_food / rounded_food_per_turn)
        if turns_to_growth < 1:
            turns_to_growth = 1
        return int(turns_to_growth)

    def get_population_filter_amount(self, filter_name: str) -> int:
        """Implements PopulationFilter"""
        if filter_name == "Specialists":
            return self.get_number_of_specialists()
        elif filter_name == "Population":
            return self.population
        elif filter_name in ["Followers of the Majority Religion", "Followers of this Religion"]:
            return self.city.religion.get_followers_of_majority_religion()
        elif filter_name == "Unemployed":
            return self.get_free_population()
        else:
            return self.specialist_allocations[filter_name]

    def next_turn(self, food: int):
        self.food_stored += food
        if food < 0:
            self.city.civ.add_notification(
                f"[{self.city.name}] is starving!",
                self.city.location,
                NotificationCategory.CITIES,
                NotificationIcon.GROWTH,
                NotificationIcon.DEATH
            )
        if self.food_stored < 0:  # starvation!
            if self.population > 1:
                self.add_population(-1)
            self.food_stored = 0
        food_needed_to_grow = self.get_food_to_next_population()
        if self.food_stored < food_needed_to_grow:
            return

        # What if the stores are already over foodNeededToGrow but NullifiesGrowth is in effect?
        # We could simply test food==0 - but this way NullifiesStat(food) will still allow growth:
        if any(self.city.get_matching_uniques(UniqueType.NULLIFIES_GROWTH)):
            return

        # Hard block growth when using Avoid Growth, cap stored food
        if self.city.avoid_growth:
            self.food_stored = food_needed_to_grow
            return

        # growth!
        self.food_stored -= food_needed_to_grow
        percent_of_food_carried_over = min(
            sum(
                int(unique.params[0])
                for unique in self.city.get_matching_uniques(UniqueType.CARRY_OVER_FOOD)
                if self.city.matches_filter(unique.params[1])
            ),
            95  # Try to avoid runaway food gain in mods, just in case
        )
        self.food_stored += int(food_needed_to_grow * percent_of_food_carried_over / 100)
        self.add_population(1)
        self.city.should_reassign_population = True
        self.city.civ.add_notification(
            f"[{self.city.name}] has grown!",
            self.city.location,
            NotificationCategory.CITIES,
            NotificationIcon.GROWTH
        )

    def add_population(self, count: int):
        changed_amount = max(count, 1 - self.population)
        self._population += changed_amount
        free_population = self.get_free_population()
        if free_population < 0:
            self.unassign_extra_population()
            self.city.city_stats.update()
        else:
            self.auto_assign_population()

        if self.city.civ.game_info.is_religion_enabled():
            self.city.religion.update_pressure_on_population_change(changed_amount)

    def set_population(self, count: int):
        self.add_population(-self.population + count)

    def auto_assign_population(self):
        """Only assigns free population"""
        self.city.city_stats.update()  # calculate current stats with current assignments
        free_population = self.get_free_population()
        if free_population <= 0:
            return

        city_stats = self.city.city_stats.current_city_stats
        self.city.current_gpp_bonus = self.city.get_great_person_percentage_bonus()  # pre-calculate for use in Automation.rankSpecialist
        specialist_food_bonus = 2.0  # See CityStats.calcFoodEaten()
        for unique in self.city.get_matching_uniques(UniqueType.FOOD_CONSUMPTION_BY_SPECIALISTS):
            if self.city.matches_filter(unique.params[1]):
                specialist_food_bonus *= float(unique.params[0]) / 100
        specialist_food_bonus = 2.0 - specialist_food_bonus

        tiles_to_evaluate = [
            tile for tile in self.city.get_workable_tiles()
            if not tile.is_blockaded()
        ]

        local_unique_cache = LocalUniqueCache()
        # Calculate stats once - but the *ranking of those stats* is dynamic and depends on what the city needs
        tile_stats = {
            tile: tile.stats.get_tile_stats(self.city, self.city.civ, local_unique_cache)
            for tile in tiles_to_evaluate
            if not tile.provides_yield()
        }

        max_specialists = self.get_max_specialists()

        for _ in range(free_population):
            # evaluate tiles
            best_tile_and_rank = max(
                (
                    (tile, Automation.rank_stats_for_city_work(tile_stats[tile], self.city, False, local_unique_cache))
                    for tile in tiles_to_evaluate
                    if not tile.provides_yield()  # Changes with every tile assigned
                ),
                key=lambda x: (x[1], x[0].longitude, x[0].latitude),
                default=(None, 0.0)
            )
            best_tile = best_tile_and_rank[0]
            value_best_tile = best_tile_and_rank[1]

            best_job_and_rank = None
            if not self.city.manual_specialists:
                best_job_and_rank = max(
                    (
                        (specialist, Automation.rank_specialist(specialist, self.city, local_unique_cache))
                        for specialist, max_amount in max_specialists.items()
                        if self.specialist_allocations[specialist] < max_amount
                    ),
                    key=lambda x: x[1],
                    default=(None, 0.0)
                )
            best_job = best_job_and_rank[0] if best_job_and_rank else None
            value_best_specialist = best_job_and_rank[1] if best_job_and_rank else 0.0

            # assign population
            if value_best_tile > value_best_specialist:
                if best_tile is not None:
                    self.city.worked_tiles.add(best_tile.position)
                    city_stats.food += tile_stats[best_tile].food
            elif best_job is not None:
                self.specialist_allocations.add(best_job, 1)
                city_stats.food += specialist_food_bonus

        self.city.city_stats.update()

    def stop_working_tile(self, position):
        self.city.worked_tiles.discard(position)
        self.city.locked_tiles.discard(position)

    def unassign_extra_population(self):
        for tile in [self.city.tile_map[pos] for pos in self.city.worked_tiles]:
            if (tile.get_owner() != self.city.civ or
                tile.get_working_city() != self.city or
                tile.aerial_distance_to(self.city.get_center_tile()) > self.city.get_work_range()):
                self.stop_working_tile(tile.position)

        # unassign specialists that cannot be (e.g. the city was captured and one of the specialist buildings was destroyed)
        for specialist_name, max_amount in self.get_max_specialists().items():
            if self.specialist_allocations[specialist_name] > max_amount:
                self.specialist_allocations[specialist_name] = max_amount

        local_unique_cache = LocalUniqueCache()

        while self.get_free_population() < 0:
            # evaluate tiles
            worst_worked_tile = None
            if self.city.worked_tiles:
                worst_worked_tile = min(
                    (self.city.tile_map[pos] for pos in self.city.worked_tiles),
                    key=lambda tile: (
                        Automation.rank_tile_for_city_work(tile, self.city, local_unique_cache) +
                        (10 if tile.is_locked() else 0)
                    )
                )
            value_worst_tile = (
                Automation.rank_tile_for_city_work(worst_worked_tile, self.city, local_unique_cache)
                if worst_worked_tile else 0.0
            )

            # evaluate specialists
            worst_auto_job = None
            if not self.city.manual_specialists:
                worst_auto_job = min(
                    self.specialist_allocations.keys(),
                    key=lambda specialist: Automation.rank_specialist(specialist, self.city, local_unique_cache),
                    default=None
                )
            value_worst_specialist = (
                Automation.rank_specialist(worst_auto_job, self.city, local_unique_cache)
                if worst_auto_job else 0.0
            )

            # un-assign population
            if worst_auto_job is not None and worst_worked_tile is not None:
                # choose between removing a specialist and removing a tile
                if value_worst_tile < value_worst_specialist:
                    self.stop_working_tile(worst_worked_tile.position)
                else:
                    self.specialist_allocations.add(worst_auto_job, -1)
            elif worst_auto_job is not None:
                self.specialist_allocations.add(worst_auto_job, -1)
            elif worst_worked_tile is not None:
                self.stop_working_tile(worst_worked_tile.position)
            else:
                # It happens when "city.manualSpecialists == true"
                #  and population goes below the number of specialists, e.g. city is razing.
                # Let's give a chance to do the work automatically at least.
                worst_job = min(
                    self.specialist_allocations.keys(),
                    key=lambda specialist: Automation.rank_specialist(specialist, self.city, local_unique_cache),
                    default=None
                )
                if worst_job is None:
                    break  # sorry, we can do nothing about that
                self.specialist_allocations.add(worst_job, -1)

    def get_max_specialists(self) -> Counter:
        counter = Counter()
        for building in self.city.city_constructions.get_built_buildings():
            counter.add(building.new_specialists())
        return counter