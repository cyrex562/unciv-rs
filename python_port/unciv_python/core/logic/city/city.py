"""
City - Represents a city in the game
"""

import uuid
from enum import Enum
from typing import Dict, List, Optional, Set, Sequence, Any, Callable, Union
from dataclasses import dataclass, field

# Import necessary modules
# These would be created as part of the port
from unciv_python.core.logic.civilization import Civilization
from unciv_python.core.logic.map.tile_map import TileMap
from unciv_python.core.logic.map.tile import Tile
from unciv_python.core.logic.map.mapunit import MapUnit, UnitPromotions
from unciv_python.core.logic.city.managers.city_population_manager import CityPopulationManager
from unciv_python.core.logic.city.managers.city_constructions import CityConstructions
from unciv_python.core.logic/city.managers.city_expansion_manager import CityExpansionManager
from unciv_python.core.logic/city.managers.city_religion_manager import CityReligionManager
from unciv_python.core.logic/city.managers.city_espionage_manager import CityEspionageManager
from unciv_python.core.logic.city.managers.city_conquest_functions import CityConquestFunctions
from unciv_python.core.logic.city.managers.spy_flee_reason import SpyFleeReason
from unciv_python.core.logic.city.city_stats import CityStats
from unciv_python.core.logic.city.city_resources import CityResources
from unciv_python.core.logic.city.great_person_points_breakdown import GreatPersonPointsBreakdown
from unciv_python.core.models.counter import Counter
from unciv_python.core.models.ruleset.building import Building
from unciv_python.core.models.ruleset.tile.tile_resource import TileResource
from unciv_python.core.models.ruleset.unique.state_for_conditionals import StateForConditionals
from unciv_python.core.models.ruleset.unique.unique import Unique
from unciv_python.core.models.ruleset.unique.unique_type import UniqueType
from unciv_python.core.models.ruleset.unit.base_unit import BaseUnit
from unciv_python.core.models.stats.game_resource import GameResource
from unciv_python.core.models.stats.stat import Stat, SubStat
from unciv_python.core.utils.vector2 import Vector2
from unciv_python.core.utils.multi_filter import MultiFilter
from unciv_python.core.utils.constants import Constants


class CityFlags(Enum):
    """Flags that can be set on a city"""
    WE_LOVE_THE_KING = "WeLoveTheKing"
    RESOURCE_DEMAND = "ResourceDemand"
    RESISTANCE = "Resistance"


class CityFocus(Enum):
    """Focus types for a city"""
    NO_FOCUS = "NoFocus"
    GOLD_FOCUS = "GoldFocus"
    MANUAL = "Manual"


class ConnectedToCapitalStatus(Enum):
    """Status of connection to capital"""
    UNKNOWN = "Unknown"
    FALSE = "false"
    TRUE = "true"


class City:
    """Represents a city in the game"""

    def __init__(self):
        """Initialize a new city"""
        # Transient properties (not serialized)
        self.civ = None
        self.center_tile = None
        self.tile_map = None
        self.tiles_in_range = set()
        self.state = StateForConditionals.EMPTY_STATE
        self.has_just_been_conquered = False
        self.city_stats = None

        # Serialized properties
        self.location = Vector2.ZERO
        self.id = str(uuid.uuid4())
        self.name = ""
        self.founding_civ = ""
        self.previous_owner = ""
        self.turn_acquired = 0
        self.health = 200

        # Managers
        self.population = CityPopulationManager()
        self.city_constructions = CityConstructions()
        self.expansion = CityExpansionManager()
        self.religion = CityReligionManager()
        self.espionage = CityEspionageManager()

        # Resources
        self.resource_stockpiles = Counter()

        # Tiles
        self.tiles = set()
        self.worked_tiles = set()
        self.locked_tiles = set()

        # City state
        self.manual_specialists = False
        self.is_being_razed = False
        self.attacked_this_turn = False
        self.has_sold_building_this_turn = False
        self.is_puppet = False
        self.should_reassign_population = False

        # Units
        self.unit_should_use_saved_promotion = {}
        self.unit_to_promotions = {}

        # City focus
        self.city_ai_focus = CityFocus.NO_FOCUS.name

        # Growth
        self.avoid_growth = False
        self.current_gpp_bonus = 0

        # Capital status
        self.is_original_capital = False

        # We Love the King Day
        self.demanded_resource = ""

        # Flags
        self.flags_countdown = {}

        # Connection status
        self.connected_to_capital_status = ConnectedToCapitalStatus.UNKNOWN

    def has_diplomatic_marriage(self) -> bool:
        """Check if the city has a diplomatic marriage"""
        return self.founding_civ == ""

    def clone(self) -> 'City':
        """Create a clone of this city"""
        to_return = City()
        to_return.location = self.location
        to_return.id = self.id
        to_return.name = self.name
        to_return.health = self.health
        to_return.population = self.population.clone()
        to_return.city_constructions = self.city_constructions.clone()
        to_return.expansion = self.expansion.clone()
        to_return.religion = self.religion.clone()
        to_return.tiles = self.tiles.copy()
        to_return.worked_tiles = self.worked_tiles.copy()
        to_return.locked_tiles = self.locked_tiles.copy()
        to_return.resource_stockpiles = self.resource_stockpiles.clone()
        to_return.is_being_razed = self.is_being_razed
        to_return.attacked_this_turn = self.attacked_this_turn
        to_return.founding_civ = self.founding_civ
        to_return.turn_acquired = self.turn_acquired
        to_return.is_puppet = self.is_puppet
        to_return.is_original_capital = self.is_original_capital
        to_return.flags_countdown = self.flags_countdown.copy()
        to_return.demanded_resource = self.demanded_resource
        to_return.should_reassign_population = self.should_reassign_population
        to_return.city_ai_focus = self.city_ai_focus
        to_return.avoid_growth = self.avoid_growth
        to_return.manual_specialists = self.manual_specialists
        to_return.connected_to_capital_status = self.connected_to_capital_status
        to_return.unit_should_use_saved_promotion = self.unit_should_use_saved_promotion.copy()
        to_return.unit_to_promotions = self.unit_to_promotions.copy()
        return to_return

    def can_bombard(self) -> bool:
        """Check if the city can bombard"""
        return not self.attacked_this_turn and not self.is_in_resistance()

    def get_center_tile(self) -> Tile:
        """Get the center tile of the city"""
        return self.center_tile

    def get_center_tile_or_null(self) -> Optional[Tile]:
        """Get the center tile of the city or None if not initialized"""
        return self.center_tile if hasattr(self, 'center_tile') else None

    def get_tiles(self) -> Sequence[Tile]:
        """Get all tiles controlled by the city"""
        return [self.tile_map[tile_pos] for tile_pos in self.tiles]

    def get_workable_tiles(self) -> Sequence[Tile]:
        """Get all workable tiles for the city"""
        return [tile for tile in self.tiles_in_range if tile.get_owner() == self.civ]

    def is_worked(self, tile: Tile) -> bool:
        """Check if a tile is being worked"""
        return tile.position in self.worked_tiles

    def is_capital(self) -> bool:
        """Check if the city is a capital"""
        return self.city_constructions.built_building_unique_map.has_unique(UniqueType.INDICATES_CAPITAL, self.state)

    def is_coastal(self) -> bool:
        """Check if the city is coastal"""
        return self.center_tile.is_coastal_tile()

    def get_bombard_range(self) -> int:
        """Get the bombard range of the city"""
        return self.civ.game_info.ruleset.mod_options.constants.base_city_bombard_range

    def get_work_range(self) -> int:
        """Get the work range of the city"""
        return self.civ.game_info.ruleset.mod_options.constants.city_work_range

    def get_expand_range(self) -> int:
        """Get the expansion range of the city"""
        return self.civ.game_info.ruleset.mod_options.constants.city_expand_range

    def is_connected_to_capital(self, connection_type_predicate: Callable[[Set[str]], bool] = lambda _: True) -> bool:
        """Check if the city is connected to the capital"""
        medium_types = self.civ.cache.cities_connected_to_capital_to_mediums.get(self)
        if medium_types is None:
            return False
        return connection_type_predicate(medium_types)

    def is_garrisoned(self) -> bool:
        """Check if the city is garrisoned"""
        return self.get_garrison() is not None

    def get_garrison(self) -> Optional[MapUnit]:
        """Get the garrison unit of the city"""
        center_tile = self.get_center_tile()
        if center_tile.military_unit is None:
            return None
        if center_tile.military_unit.civ == self.civ and center_tile.military_unit.can_garrison():
            return center_tile.military_unit
        return None

    def has_flag(self, flag: CityFlags) -> bool:
        """Check if the city has a flag"""
        return flag.name in self.flags_countdown

    def get_flag(self, flag: CityFlags) -> int:
        """Get the value of a flag"""
        return self.flags_countdown[flag.name]

    def is_we_love_the_king_day_active(self) -> bool:
        """Check if We Love The King Day is active"""
        return self.has_flag(CityFlags.WE_LOVE_THE_KING)

    def is_in_resistance(self) -> bool:
        """Check if the city is in resistance"""
        return self.has_flag(CityFlags.RESISTANCE)

    def is_blockaded(self) -> bool:
        """Check if the city is blockaded"""
        if not self.is_coastal():
            return False
        return all(tile.is_blockaded() for tile in self.get_center_tile().neighbors if tile.is_water)

    def get_ruleset(self):
        """Get the ruleset of the game"""
        return self.civ.game_info.ruleset

    def get_resources_generated_by_city(self, civ_resource_modifiers: Dict[str, float]) -> Dict[str, float]:
        """Get resources generated by the city"""
        return CityResources.get_resources_generated_by_city(self, civ_resource_modifiers)

    def get_available_resource_amount(self, resource_name: str) -> int:
        """Get the available amount of a resource"""
        return CityResources.get_available_resource_amount(self, resource_name)

    def is_growing(self) -> bool:
        """Check if the city is growing"""
        return self.food_for_next_turn() > 0

    def is_starving(self) -> bool:
        """Check if the city is starving"""
        return self.food_for_next_turn() < 0

    def food_for_next_turn(self) -> int:
        """Get the food for the next turn"""
        return round(self.city_stats.current_city_stats.food)

    def contains_building_unique(self, unique_type: UniqueType, state: StateForConditionals = None) -> bool:
        """Check if the city contains a building with a unique"""
        if state is None:
            state = self.state
        return any(self.city_constructions.built_building_unique_map.get_matching_uniques(unique_type, state))

    def get_great_person_percentage_bonus(self) -> float:
        """Get the great person percentage bonus"""
        return GreatPersonPointsBreakdown.get_great_person_percentage_bonus(this)

    def get_great_person_points(self) -> Dict[str, float]:
        """Get the great person points"""
        return GreatPersonPointsBreakdown(self).sum()

    def gain_stockpiled_resource(self, resource: TileResource, amount: int):
        """Gain a stockpiled resource"""
        if resource.is_city_wide:
            self.resource_stockpiles.add(resource.name, amount)
        else:
            self.civ.resource_stockpiles.add(resource.name, amount)

    def add_stat(self, stat: Stat, amount: int):
        """Add a stat to the city"""
        if stat == Stat.PRODUCTION:
            self.city_constructions.add_production_points(amount)
        elif stat == Stat.FOOD:
            self.population.food_stored += amount
        else:
            self.civ.add_stat(stat, amount)

    def add_game_resource(self, stat: GameResource, amount: int):
        """Add a game resource to the city"""
        if isinstance(stat, TileResource):
            if not stat.is_stockpiled:
                return
            self.gain_stockpiled_resource(stat, amount)
            return

        if stat == Stat.PRODUCTION:
            self.city_constructions.add_production_points(amount)
        elif stat in (Stat.FOOD, SubStat.STORED_FOOD):
            self.population.food_stored += amount
        else:
            self.civ.add_game_resource(stat, amount)

    def get_stat_reserve(self, stat: Stat) -> int:
        """Get the reserve of a stat"""
        if stat == Stat.PRODUCTION:
            return self.city_constructions.get_work_done(self.city_constructions.get_current_construction().name)
        elif stat == Stat.FOOD:
            return self.population.food_stored
        else:
            return self.civ.get_stat_reserve(stat)

    def get_reserve(self, stat: GameResource) -> int:
        """Get the reserve of a game resource"""
        if isinstance(stat, TileResource):
            if not stat.is_stockpiled:
                return 0
            if stat.is_city_wide:
                return self.resource_stockpiles[stat.name]
            return self.civ.resource_stockpiles[stat.name]

        if stat == Stat.PRODUCTION:
            return self.city_constructions.get_work_done(self.city_constructions.get_current_construction().name)
        elif stat in (Stat.FOOD, SubStat.STORED_FOOD):
            return self.population.food_stored
        else:
            return self.civ.get_reserve(stat)

    def has_stat_to_buy(self, stat: Stat, price: int) -> bool:
        """Check if the city has enough of a stat to buy something"""
        if self.civ.game_info.game_parameters.god_mode:
            return True
        if price == 0:
            return True
        return self.get_stat_reserve(stat) >= price

    def get_max_health(self) -> int:
        """Get the maximum health of the city"""
        return 200 + sum(building.city_health for building in self.city_constructions.get_built_buildings())

    def get_strength(self) -> float:
        """Get the strength of the city"""
        return sum(building.city_strength for building in self.city_constructions.get_built_buildings())

    def get_max_air_units(self) -> int:
        """Get the maximum number of air units in the city"""
        return 6  # This should probably be configurable

    def __str__(self) -> str:
        """Get a string representation of the city"""
        return self.name  # for debug

    def is_holy_city(self) -> bool:
        """Check if the city is a holy city"""
        return self.religion.religion_this_is_the_holy_city_of is not None and not self.religion.is_blocked_holy_city

    def is_holy_city_of(self, religion_name: Optional[str]) -> bool:
        """Check if the city is a holy city of a specific religion"""
        return self.is_holy_city() and self.religion.religion_this_is_the_holy_city_of == religion_name

    def can_be_destroyed(self, just_captured: bool = False) -> bool:
        """Check if the city can be destroyed"""
        if self.civ.game_info.game_parameters.no_city_razing:
            return False

        allow_raze_capital = self.civ.game_info.ruleset.mod_options.has_unique(UniqueType.ALLOW_RAZE_CAPITAL)
        allow_raze_holy_city = self.civ.game_info.ruleset.mod_options.has_unique(UniqueType.ALLOW_RAZE_HOLY_CITY)

        if self.is_original_capital and not allow_raze_capital:
            return False
        if self.is_holy_city() and not allow_raze_holy_city:
            return False
        if self.is_capital() and not just_captured and not allow_raze_capital:
            return False

        return True

    def set_transients(self, civ_info: Civilization):
        """Set transient properties of the city"""
        self.civ = civ_info
        self.tile_map = civ_info.game_info.tile_map
        self.center_tile = self.tile_map[self.location]
        self.state = StateForConditionals(self)
        self.tiles_in_range = set(self.get_center_tile().get_tiles_in_distance(self.get_work_range()))
        self.population.city = self
        self.expansion.city = self
        self.expansion.set_transients()
        self.city_constructions.city = this
        self.religion.set_transients(self)
        self.city_constructions.set_transients()
        self.espionage.set_transients(self)

    def set_flag(self, flag: CityFlags, amount: int):
        """Set a flag on the city"""
        self.flags_countdown[flag.name] = amount

    def remove_flag(self, flag: CityFlags):
        """Remove a flag from the city"""
        if flag.name in self.flags_countdown:
            del self.flags_countdown[flag.name]

    def reset_wltkd(self):
        """Reset We Love The King Day"""
        # Removes the flags for we love the king & resource demand
        # The resource demand flag will automatically be readded with 15 turns remaining, see start_turn()
        this.remove_flag(CityFlags.WE_LOVE_THE_KING)
        this.remove_flag(CityFlags.RESOURCE_DEMAND)
        this.demanded_resource = ""

    def reassign_all_population(self):
        """Reassign all population"""
        # Reassign all Specialists and Unlock all tiles
        # Mainly for automated cities, Puppets, just captured
        this.manual_specialists = False
        this.reassign_population(reset_locked=True)

    def reassign_population(self, reset_locked: bool = False):
        """Reassign population"""
        # Apply worked tiles optimization (aka CityFocus) - Expensive!
        # If the next City.start_turn is soon enough, then use reassign_population_deferred() instead.
        if reset_locked:
            this.worked_tiles = set()
            this.locked_tiles = set()
        elif this.city_ai_focus != CityFocus.MANUAL.name:
            this.worked_tiles = this.locked_tiles.copy()

        if not this.manual_specialists:
            this.population.specialist_allocations.clear()

        this.should_reassign_population = False
        this.population.auto_assign_population()

    def reassign_population_deferred(self):
        """Reassign population deferred"""
        # Apply worked tiles optimization (aka CityFocus) -
        # immediately for a human player whose turn it is (interactive),
        # or deferred to the next start_turn while next_turn is running (for AI)
        # TODO - is this the best (or even correct) way to detect "interactive" UI calls?
        from unciv_python.core.gui import GUI
        if GUI.is_my_turn() and GUI.get_viewing_player() == this.civ:
            this.reassign_population()
        else:
            this.should_reassign_population = True

    def destroy_city(self, override_safeties: bool = False):
        """Destroy the city"""
        # Original capitals and holy cities cannot be destroyed,
        # unless, of course, they are captured by a one-city-challenger.
        if not this.can_be_destroyed() and not override_safeties:
            return

        # Destroy planes stationed in city
        for air_unit in list(this.get_center_tile().air_units):
            air_unit.destroy()

        # The relinquish ownership MUST come before removing the city,
        # because it updates the city stats which assumes there is a capital, so if you remove the capital it crashes
        for tile in this.get_tiles():
            this.expansion.relinquish_ownership(tile)

        # Move the capital if destroyed (by a nuke or by razing)
        # Must be before removing existing capital because we may be annexing a puppet which means city stats update - see #8337
        if this.is_capital():
            this.civ.move_capital_to_next_largest(None)

        this.civ.cities = [city for city in this.civ.cities if city != this]

        if this.get_ruleset().tile_improvements.get("City ruins"):
            this.get_center_tile().set_improvement("City ruins")

        # Edge case! What if a water unit is in a city, and you raze the city?
        # Well, the water unit has to return to the water!
        for unit in list(this.get_center_tile().get_units()):
            if not unit.movement.can_pass_through(this.get_center_tile()):
                unit.movement.teleport_to_closest_moveable_tile()

        this.espionage.remove_all_present_spies(SpyFleeReason.CITY_DESTROYED)

        # Update proximity rankings for all civs
        for other_civ in this.civ.game_info.get_alive_major_civs():
            this.civ.update_proximity(other_civ, other_civ.update_proximity(this.civ))

        for other_civ in this.civ.game_info.get_alive_city_states():
            this.civ.update_proximity(other_civ, other_civ.update_proximity(this.civ))

        this.civ.game_info.city_distances.set_dirty()

    def annex_city(self):
        """Annex the city"""
        return CityConquestFunctions(this).annex_city()

    def puppet_city(self, conquering_civ: Civilization):
        """Puppet the city"""
        # This happens when we either puppet OR annex, basically whenever we conquer a city and don't liberate it
        return CityConquestFunctions(this).puppet_city(conquering_civ)

    def liberate_city(self, conquering_civ: Civilization):
        """Liberate the city"""
        # Liberating is returning a city to its founder - makes you LOSE warmongering points
        return CityConquestFunctions(this).liberate_city(conquering_civ)

    def move_to_civ(self, new_civ_info: Civilization):
        """Move the city to a new civilization"""
        return CityConquestFunctions(this).move_to_civ(new_civ_info)

    def try_update_road_status(self):
        """Try to update the road status"""
        required_road = None
        if (this.get_ruleset().railroad_improvement and
            (this.get_ruleset().railroad_improvement.tech_required is None or
             this.get_ruleset().railroad_improvement.tech_required in this.civ.tech.techs_researched)):
            required_road = "Railroad"
        elif (this.get_ruleset().road_improvement and
              (this.get_ruleset().road_improvement.tech_required is None or
               this.get_ruleset().road_improvement.tech_required in this.civ.tech.techs_researched)):
            required_road = "Road"
        else:
            required_road = "None"

        this.get_center_tile().set_road_status(required_road, this.civ)

    def get_gold_for_selling_building(self, building_name: str) -> int:
        """Get the gold for selling a building"""
        return this.get_ruleset().buildings[building_name].cost // 10

    def sell_building(self, building_name: str):
        """Sell a building by name"""
        this.sell_building(this.get_ruleset().buildings[building_name])

    def sell_building(self, building: Building):
        """Sell a building"""
        this.city_constructions.remove_building(building)
        this.civ.add_gold(this.get_gold_for_selling_building(building.name))
        this.has_sold_building_this_turn = True

        this.population.unassign_extra_population()  # If the building provided specialists, release them to other work
        this.population.auto_assign_population()  # also updates city stats
        this.civ.cache.update_civ_resources()  # this building could be a resource-requiring one

    def can_place_new_unit(self, construction: BaseUnit) -> bool:
        """Check if a new unit can be placed in the city"""
        tile = this.get_center_tile()
        if construction.is_civilian():
            return tile.civilian_unit is None
        elif construction.moves_like_air_units:
            return True  # Dealt with in MapUnit.get_rejection_reasons
        else:
            return tile.military_unit is None

    def matches_filter(self, filter_str: str, viewing_civ: Optional[Civilization] = None, multi_filter: bool = True) -> bool:
        """Check if the city matches a filter"""
        # Implements UniqueParameterType.CityFilter
        if multi_filter:
            return MultiFilter.multi_filter(filter_str, lambda f: this.matches_single_filter(f, viewing_civ))
        else:
            return this.matches_single_filter(filter_str, viewing_civ)

    def matches_single_filter(self, filter_str: str, viewing_civ: Optional[Civilization] = None) -> bool:
        """Check if the city matches a single filter"""
        if viewing_civ is None:
            viewing_civ = this.civ

        if filter_str == "in this city":
            return True  # Filtered by the way uniques are found
        elif filter_str == "in all cities":
            return True
        elif filter_str in Constants.ALL:
            return True
        elif filter_str in ("in your cities", "Your"):
            return viewing_civ == this.civ
        elif filter_str in ("in all coastal cities", "Coastal"):
            return this.is_coastal()
        elif filter_str in ("in capital", "Capital"):
            return this.is_capital()
        elif filter_str in ("in all non-occupied cities", "Non-occupied"):
            return not this.city_stats.has_extra_annex_unhappiness() or this.is_puppet
        elif filter_str == "in all cities with a world wonder":
            return any(building.is_wonder for building in this.city_constructions.get_built_buildings())
        elif filter_str == "in all cities connected to capital":
            return this.is_connected_to_capital()
        elif filter_str in ("in all cities with a garrison", "Garrisoned"):
            return this.is_garrisoned()
        elif filter_str == "in all cities in which the majority religion is a major religion":
            return (this.religion.get_majority_religion_name() is not None and
                    this.religion.get_majority_religion().is_major_religion())
        elif filter_str == "in all cities in which the majority religion is an enhanced religion":
            return (this.religion.get_majority_religion_name() is not None and
                    this.religion.get_majority_religion().is_enhanced_religion())
        elif filter_str == "in non-enemy foreign cities":
            return (viewing_civ is not None and viewing_civ != this.civ and
                    not this.civ.is_at_war_with(viewing_civ))
        elif filter_str in ("in enemy cities", "Enemy"):
            return this.civ.is_at_war_with(viewing_civ)
        elif filter_str in ("in foreign cities", "Foreign"):
            return viewing_civ is not None and viewing_civ != this.civ
        elif filter_str in ("in annexed cities", "Annexed"):
            return this.founding_civ != this.civ.civ_name and not this.is_puppet
        elif filter_str in ("in puppeted cities", "Puppeted"):
            return this.is_puppet
        elif filter_str in ("in resisting cities", "Resisting"):
            return this.is_in_resistance()
        elif filter_str in ("in cities being razed", "Razing"):
            return this.is_being_razed
        elif filter_str in ("in holy cities", "Holy"):
            return this.is_holy_city()
        elif filter_str == "in City-State cities":
            return this.civ.is_city_state
        elif filter_str == "in cities following this religion":
            # This is only used in communication to the user indicating that only in cities with this
            # religion a unique is active. However, since religion uniques only come from the city itself,
            # this will always be true when checked.
            return True
        elif filter_str == "in cities following our religion":
            return (viewing_civ is not None and
                    viewing_civ.religion_manager.religion == this.religion.get_majority_religion())
        else:
            return this.civ.matches_filter(filter_str, this.state, False)

    def get_matching_uniques(self, unique_type: UniqueType, state_for_conditionals: StateForConditionals = None,
                            include_civ_uniques: bool = True) -> Sequence[Unique]:
        """Get matching uniques"""
        # Finds matching uniques provided from both local and non-local sources.
        if state_for_conditionals is None:
            state_for_conditionals = this.state

        if include_civ_uniques:
            return list(this.civ.get_matching_uniques(unique_type, state_for_conditionals)) + \
                   list(this.get_local_matching_uniques(unique_type, state_for_conditionals))
        else:
            uniques = list(this.city_constructions.built_building_unique_map.get_uniques(unique_type)) + \
                     list(this.religion.get_uniques(unique_type))
            return [unique for unique in uniques
                   if not unique.is_timed_triggerable and unique.conditionals_apply(state_for_conditionals)]

    def get_local_matching_uniques(self, unique_type: UniqueType, state_for_conditionals: StateForConditionals = None) -> Sequence[Unique]:
        """Get local matching uniques"""
        # Uniques special to this city
        if state_for_conditionals is None:
            state_for_conditionals = this.state

        uniques = list(this.city_constructions.built_building_unique_map.get_uniques(unique_type)) + \
                 list(this.religion.get_uniques(unique_type))
        return [unique for unique in uniques
                if unique.is_local_effect and not unique.is_timed_triggerable and unique.conditionals_apply(state_for_conditionals)]

    def get_matching_uniques_with_non_local_effects(self, unique_type: UniqueType, state_for_conditionals: StateForConditionals = None) -> Sequence[Unique]:
        """Get matching uniques with non-local effects"""
        # Uniques coming from this city, but that should be provided globally
        if state_for_conditionals is None:
            state_for_conditionals = this.state

        uniques = list(this.city_constructions.built_building_unique_map.get_uniques(unique_type))
        # Memory performance showed that this function was very memory intensive, thus we only create the filter if needed
        if uniques:
            return [unique for unique in uniques
                    if not unique.is_local_effect and not unique.is_timed_triggerable and unique.conditionals_apply(state_for_conditionals)]
        else:
            return uniques