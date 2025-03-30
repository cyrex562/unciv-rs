from typing import List, Dict, Set, Optional
from dataclasses import dataclass
import math
from sortedcontainers import SortedDict

from com.unciv.logic.automation.unit import UnitAutomation
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map import BFS
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset import INonPerpetualConstruction
from com.unciv.models.ruleset.tile import ResourceType
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.models.stats import Stat
from com.unciv.logic.automation.civilization.motivation_to_attack_automation import MotivationToAttackAutomation
from com.unciv.logic.automation.civilization.next_turn_automation import NextTurnAutomation

class UseGoldAutomation:
    """Handles AI automation for gold spending decisions."""

    @staticmethod
    def use_gold(civ: Civilization) -> None:
        """Allow AI to spend money to purchase city-state friendship, buildings & units.
        
        Args:
            civ: The civilization spending gold
        """
        for unit in civ.units.get_civ_units():
            UnitAutomation.try_upgrade_unit(unit)
        
        if civ.is_major_civ():
            UseGoldAutomation._use_gold_for_city_states(civ)

        for city in sorted(civ.cities, key=lambda c: c.population.population, reverse=True):
            construction = city.city_constructions.get_current_construction()
            if not isinstance(construction, INonPerpetualConstruction):
                continue
                
            stat_buy_cost = construction.get_stat_buy_cost(city, Stat.Gold)
            if stat_buy_cost is None:
                continue
                
            if not city.city_constructions.is_construction_purchase_allowed(
                construction, Stat.Gold, stat_buy_cost
            ):
                continue
                
            if civ.gold < stat_buy_cost * 3:
                continue
                
            city.city_constructions.purchase_construction(construction, 0, True)

        UseGoldAutomation._maybe_buy_city_tiles(civ)

    @staticmethod
    def _use_gold_for_city_states(civ: Civilization) -> None:
        """Use gold to influence city states.
        
        Args:
            civ: The civilization using gold for city states
        """
        # RARE EDGE CASE: If you ally with a city-state, you may reveal more map that includes ANOTHER civ!
        # So if we don't lock this list, we may later discover that there are more known civs, concurrent modification exception!
        known_city_states = [
            other_civ for other_civ in civ.get_known_civs()
            if (other_civ.is_city_state 
                and MotivationToAttackAutomation.has_at_least_motivation_to_attack(civ, other_civ, 0.0) <= 0)
        ]

        # canBeMarriedBy checks actual cost, but it can't be below 500*speedmodifier, and the later check is expensive
        if (civ.gold >= 330 
            and civ.get_happiness() > 0 
            and (civ.has_unique(UniqueType.CityStateCanBeBoughtForGold) 
                 or civ.has_unique(UniqueType.CityStateCanBeBoughtForGoldOld))):
            for city_state in list(known_city_states):  # Materialize sequence as diplomaticMarriage may kill a CS
                if city_state.city_state_functions.can_be_married_by(civ):
                    city_state.city_state_functions.diplomatic_marriage(civ)
                if civ.get_happiness() <= 0:
                    break  # Stop marrying if happiness is getting too low

        if civ.gold < 500 or not known_city_states:
            return  # skip checks if tryGainInfluence will bail anyway
            
        city_state = max(
            ((cs, NextTurnAutomation.value_city_state_alliance(civ, cs, True))
             for cs in known_city_states
             if cs.get_ally_civ() != civ.civ_name),
            key=lambda x: x[1],
            default=None
        )
        
        if city_state and city_state[1] > 0:
            UseGoldAutomation._try_gain_influence(civ, city_state[0])

    @staticmethod
    def _maybe_buy_city_tiles(civ_info: Civilization) -> None:
        """Attempt to buy city tiles if conditions are met.
        
        Args:
            civ_info: The civilization attempting to buy tiles
        """
        if civ_info.gold <= 0:
            return
            
        # Don't buy tiles in the very early game. It is unlikely that we already have the required
        # tech, the necessary worker and that there is a reasonable threat from another player to
        # grab the tile. We could also check all that, but it would require a lot of cycles each
        # turn and this is probably a good approximation.
        if civ_info.game_info.turns < int(civ_info.game_info.speed.science_cost_modifier * 20):
            return

        highly_desirable_tiles = UseGoldAutomation._get_highly_desirable_tiles_to_city_map(civ_info)

        # Always try to buy highly desirable tiles if it can be afforded.
        for tile, cities in highly_desirable_tiles.items():
            city_with_least_cost = min(
                cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(tile)
            )
            
            bfs = BFS(city_with_least_cost.get_center_tile())
            bfs.set_condition(
                lambda t: t.get_owner() is None or t.owning_city == city_with_least_cost
            )
            bfs.step_until_destination(tile)
            
            tiles_that_need_buying = list(reversed(  # getPathTo is from destination to source
                [t for t in bfs.get_path_to(tile) if t.get_owner() is None]
            ))

            # We're trying to acquire everything and revert if it fails, because of the difficult
            # way how tile acquisition cost is calculated. Everytime you buy a tile, the next one
            # gets more expensive and by how much depends on other things such as game speed. To
            # not introduce hidden dependencies on that and duplicate that logic here to calculate
            # the price of the whole path, this is probably simpler.
            ran_out_of_money = False
            gold_spent = 0
            
            for tile_to_buy in tiles_that_need_buying:
                gold_cost = city_with_least_cost.expansion.get_gold_cost_of_tile(tile_to_buy)
                if civ_info.gold >= gold_cost:
                    city_with_least_cost.expansion.buy_tile(tile_to_buy)
                    gold_spent += gold_cost
                else:
                    ran_out_of_money = True
                    break
                    
            if ran_out_of_money:
                for tile_to_buy in tiles_that_need_buying:
                    city_with_least_cost.expansion.relinquish_ownership(tile_to_buy)
                civ_info.add_gold(gold_spent)

    @staticmethod
    def _get_highly_desirable_tiles_to_city_map(civ_info: Civilization) -> Dict[Tile, Set[City]]:
        """Get a map of highly desirable tiles to cities that want them.
        
        Args:
            civ_info: The civilization getting the map
            
        Returns:
            Dict[Tile, Set[City]]: Map of tiles to cities that want them
        """
        def tile_comparator(t1: Optional[Tile], t2: Optional[Tile]) -> int:
            if t1 is None or t2 is None:
                return 0
                
            # Compare by natural wonder first
            if t1.natural_wonder is not None and t2.natural_wonder is None:
                return -1
            if t1.natural_wonder is None and t2.natural_wonder is not None:
                return 1
                
            # Then by luxury resources
            if (t1.resource is not None and t1.tile_resource.resource_type == ResourceType.Luxury
                and (t2.resource is None or t2.tile_resource.resource_type != ResourceType.Luxury)):
                return -1
            if (t2.resource is not None and t2.tile_resource.resource_type == ResourceType.Luxury
                and (t1.resource is None or t1.tile_resource.resource_type != ResourceType.Luxury)):
                return 1
                
            # Then by strategic resources
            if (t1.resource is not None and t1.tile_resource.resource_type == ResourceType.Strategic
                and (t2.resource is None or t2.tile_resource.resource_type != ResourceType.Strategic)):
                return -1
            if (t2.resource is not None and t2.tile_resource.resource_type == ResourceType.Strategic
                and (t1.resource is None or t1.tile_resource.resource_type != ResourceType.Strategic)):
                return 1
                
            # Finally by hash code to maintain consistent ordering
            return hash(t1) - hash(t2)

        highly_desirable_tiles = SortedDict(tile_comparator)

        for city in [c for c in civ_info.cities if not c.is_puppet and not c.is_being_razed]:
            highly_desirable_tiles_in_city = [
                tile for tile in city.tiles_in_range
                if UseGoldAutomation._is_highly_desirable_tile(tile, civ_info, city)
            ]
            for tile in highly_desirable_tiles_in_city:
                if tile not in highly_desirable_tiles:
                    highly_desirable_tiles[tile] = set()
                highly_desirable_tiles[tile].add(city)
                
        return highly_desirable_tiles

    @staticmethod
    def _is_highly_desirable_tile(tile: Tile, civ_info: Civilization, city: City) -> bool:
        """Check if a tile is highly desirable for a city.
        
        Args:
            tile: The tile to check
            civ_info: The civilization checking the tile
            city: The city checking the tile
            
        Returns:
            bool: Whether the tile is highly desirable
        """
        if not tile.is_visible(civ_info):
            return False
        if tile.get_owner() is not None:
            return False
        if not any(neighbor.get_city() == city for neighbor in tile.neighbors):
            return False

        def has_natural_wonder() -> bool:
            return tile.natural_wonder is not None

        def has_luxury_civ_doesnt_own() -> bool:
            return (tile.has_viewable_resource(civ_info)
                   and tile.tile_resource.resource_type == ResourceType.Luxury
                   and not civ_info.has_resource(tile.resource))

        def has_resource_civ_has_none_or_little() -> bool:
            return (tile.has_viewable_resource(civ_info)
                   and tile.tile_resource.resource_type == ResourceType.Strategic
                   and civ_info.get_resource_amount(tile.resource) <= 3)

        return (has_natural_wonder() 
                or has_luxury_civ_doesnt_own() 
                or has_resource_civ_has_none_or_little())

    @staticmethod
    def _try_gain_influence(civ_info: Civilization, city_state: Civilization) -> None:
        """Attempt to gain influence with a city state through gold gifts.
        
        Args:
            civ_info: The civilization trying to gain influence
            city_state: The city state to gain influence with
        """
        if civ_info.gold < 500:
            return  # Save up, giving 500 gold in one go typically grants +5 influence compared to giving 2×250 gold
            
        influence = city_state.get_diplomacy_manager(civ_info).get_influence()
        stop_spending = influence > 60 + 2 * NextTurnAutomation.value_city_state_alliance(
            civ_info, city_state, True
        )
        
        # Don't go into a gold gift race: be content with friendship for cheap, or use the gold on more productive uses,
        # for example upgrading an army to conquer the player who's contesting our city states
        if influence < 10 or stop_spending:
            return
            
        # Only make an investment if we got our Pledge to Protect influence at the highest level
        if civ_info.gold >= 1000:
            city_state.city_state_functions.receive_gold_gift(civ_info, 1000)
        else:
            city_state.city_state_functions.receive_gold_gift(civ_info, 500) 