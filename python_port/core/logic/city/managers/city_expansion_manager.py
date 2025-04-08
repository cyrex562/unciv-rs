from typing import List, Optional
from dataclasses import dataclass
import math
from enum import Enum

class NotificationCategory(Enum):
    CITIES = "Cities"

class NotificationIcon(Enum):
    CULTURE = "Culture"

@dataclass
class LocationAction:
    location: tuple
    source_location: tuple

class CityExpansionManager:
    def __init__(self):
        self.city = None  # Will be set via set_transients
        self.culture_stored: int = 0

    def clone(self) -> 'CityExpansionManager':
        to_return = CityExpansionManager()
        to_return.culture_stored = self.culture_stored
        return to_return

    def tiles_claimed(self) -> int:
        tiles_around_city = [tile.position for tile in self.city.get_center_tile().neighbors]
        return sum(1 for tile in self.city.tiles
                  if tile != self.city.location and tile not in tiles_around_city)

    def get_culture_to_next_tile(self) -> int:
        # Formula from Civ VI: 6*(t+0.4813)^1.3
        culture_to_next_tile = 6 * (max(0, self.tiles_claimed()) + 1.4813) ** 1.3

        culture_to_next_tile *= self.city.civ.game_info.speed.culture_cost_modifier

        if self.city.civ.is_city_state:
            culture_to_next_tile *= 1.5  # City states grow slower, perhaps 150% cost?

        for unique in self.city.get_matching_uniques("BorderGrowthPercentage"):
            if self.city.matches_filter(unique.params[1]):
                culture_to_next_tile *= float(unique.params[0]) / 100

        return round(culture_to_next_tile)

    def can_buy_tile(self, tile) -> bool:
        if (self.city.is_puppet or
            self.city.is_being_razed or
            tile.get_owner() is not None or
            self.city.is_in_resistance() or
            tile not in self.city.tiles_in_range):
            return False
        return any(neighbor.get_city() == self.city for neighbor in tile.neighbors)

    def buy_tile(self, tile):
        gold_cost = self.get_gold_cost_of_tile(tile)

        if not any(neighbor.get_city() == self.city for neighbor in tile.neighbors):
            raise Exception(f"{self.city} tried to buy {tile}, but it owns none of the neighbors")

        if (self.city.civ.gold < gold_cost and
            not self.city.civ.game_info.game_parameters.god_mode):
            raise Exception(f"{self.city} tried to buy {tile}, but lacks gold "
                          f"(cost {gold_cost}, has {self.city.civ.gold})")

        self.city.civ.add_gold(-gold_cost)
        self.take_ownership(tile)

        # Reapply worked tiles optimization (aka CityFocus)
        self.city.reassign_population_deferred()

    def get_gold_cost_of_tile(self, tile) -> int:
        base_cost = 50
        distance_from_center = tile.aerial_distance_to(self.city.get_center_tile())
        cost = base_cost * (distance_from_center - 1) + self.tiles_claimed() * 5.0

        cost *= self.city.civ.game_info.speed.gold_cost_modifier

        for unique in self.city.get_matching_uniques("TileCostPercentage"):
            if self.city.matches_filter(unique.params[1]):
                cost *= float(unique.params[0]) / 100

        return round(cost)

    def get_choosable_tiles(self):
        return [tile for tile in self.city.get_center_tile().get_tiles_in_distance(self.city.get_expand_range())
                if tile.get_owner() is None]

    def choose_new_tile_to_own(self) -> Optional['Tile']:
        local_unique_cache = LocalUniqueCache()
        choosable_tiles = self.get_choosable_tiles()
        if not choosable_tiles:
            return None
        return min(choosable_tiles,
                  key=lambda tile: Automation.rank_tile_for_expansion(tile, self.city, local_unique_cache))

    def reset(self):
        for tile in self.city.get_tiles():
            self.relinquish_ownership(tile)

        # The only way to create a city inside an owned tile is if it's in your territory
        # In this case, if you don't assign control of the central tile to the city,
        # It becomes an invisible city and weird shit starts happening
        self.take_ownership(self.city.get_center_tile())

        for tile in self.city.get_center_tile().get_tiles_in_distance(1):
            if tile.get_city() is None:  # can't take ownership of owned tiles (by other cities)
                self.take_ownership(tile)

    def add_new_tile_with_culture(self) -> Optional[tuple]:
        chosen_tile = self.choose_new_tile_to_own()
        if chosen_tile is not None:
            self.culture_stored -= self.get_culture_to_next_tile()
            self.take_ownership(chosen_tile)
            return chosen_tile.position
        return None

    def relinquish_ownership(self, tile):
        self.city.tiles = [t for t in self.city.tiles if t != tile.position]

        for city in self.city.civ.cities:
            if city.is_worked(tile):
                city.population.stop_working_tile(tile.position)
                city.population.auto_assign_population()

        tile.improvement_functions.remove_creates_one_improvement_marker()
        tile.set_owning_city(None)
        self.city.civ.cache.update_our_tiles()
        self.city.city_stats.update()
        tile.history.record_relinquish_ownership(tile)

    def take_ownership(self, tile):
        if tile.is_city_center():
            raise Exception("Trying to found a city in a tile that already has one")

        if tile.get_city() is not None:
            tile.get_city().expansion.relinquish_ownership(tile)

        self.city.tiles.append(tile.position)
        tile.set_owning_city(self.city)
        self.city.population.auto_assign_population()
        self.city.civ.cache.update_our_tiles()
        self.city.city_stats.update()

        for unit in list(tile.get_units()):  # list() because we're modifying
            if not unit.civ.diplomacy_functions.can_pass_through_tiles(self.city.civ):
                unit.movement.teleport_to_closest_moveable_tile()
            elif unit.civ == self.city.civ and unit.is_sleeping():
                # If the unit is sleeping and is a worker, it might want to build on this tile
                # So lets try to wake it up for the player to notice it
                if (unit.cache.has_unique_to_build_improvements or
                    unit.cache.has_unique_to_create_water_improvements):
                    unit.due = True
                    unit.action = None

        tile.history.record_take_ownership(tile)

    def next_turn(self, culture: float):
        self.culture_stored += int(culture)
        if self.culture_stored >= self.get_culture_to_next_tile():
            location = self.add_new_tile_with_culture()
            if location is not None:
                locations = LocationAction(location, self.city.location)
                self.city.civ.add_notification(
                    f"[{self.city.name}] has expanded its borders!",
                    locations,
                    NotificationCategory.CITIES,
                    NotificationIcon.CULTURE
                )

    def set_transients(self):
        tiles = self.city.get_tiles()
        for tile in tiles:
            tile.set_owning_city(self.city)