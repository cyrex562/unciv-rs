from typing import List, Optional
import random

from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.diplomacy import RelationshipLevel
from com.unciv.models import Spy, SpyAction

class EspionageAutomation:
    """Handles AI automation for espionage operations."""

    def __init__(self, civ_info: Civilization):
        """Initialize the espionage automation.
        
        Args:
            civ_info: The civilization to automate espionage for
        """
        self.civ_info = civ_info
        self._civs_to_steal_from: Optional[List[Civilization]] = None
        self._civs_to_steal_from_sorted: Optional[List[Civilization]] = None
        self._city_states_to_rig: Optional[List[Civilization]] = None

    @property
    def civs_to_steal_from(self) -> List[Civilization]:
        """Get list of civilizations to steal from.
        
        Returns:
            List[Civilization]: List of civilizations that can be targeted for tech stealing
        """
        if self._civs_to_steal_from is None:
            self._civs_to_steal_from = [
                other_civ for other_civ in self.civ_info.get_known_civs()
                if (other_civ.is_major_civ()
                    and any(
                        city.get_center_tile().is_explored(self.civ_info)
                        and self.civ_info.espionage_manager.get_spy_assigned_to_city(city) is None
                        for city in other_civ.cities
                    )
                    and self.civ_info.espionage_manager.get_techs_to_steal(other_civ))
            ]
        return self._civs_to_steal_from

    @property
    def civs_to_steal_from_sorted(self) -> List[Civilization]:
        """Get sorted list of civilizations to steal from.
        
        Returns:
            List[Civilization]: List of civilizations sorted by number of active spies
        """
        if self._civs_to_steal_from_sorted is None:
            self._civs_to_steal_from_sorted = sorted(
                self.civs_to_steal_from,
                key=lambda other_civ: sum(
                    1 for spy in self.civ_info.espionage_manager.spy_list
                    if spy.is_doing_work() and spy.get_city_or_null()?.civ == other_civ
                )
            )
        return self._civs_to_steal_from_sorted

    @property
    def city_states_to_rig(self) -> List[Civilization]:
        """Get list of city states that can be rigged.
        
        Returns:
            List[Civilization]: List of city states that can be targeted for election rigging
        """
        if self._city_states_to_rig is None:
            self._city_states_to_rig = [
                other_civ for other_civ in self.civ_info.get_known_civs()
                if (other_civ.is_minor_civ()
                    and other_civ.knows(self.civ_info)
                    and not self.civ_info.is_at_war_with(other_civ))
            ]
        return self._city_states_to_rig

    def automate_spies(self) -> None:
        """Automate all spy operations."""
        spies = self.civ_info.espionage_manager.spy_list
        spies_to_move = [spy for spy in spies if spy.is_alive() and not spy.is_doing_work()]
        
        for spy in spies_to_move:
            random_seed = len(spies) + spies.index(spy) + self.civ_info.game_info.turns
            random.seed(random_seed)
            random_action = random.randint(0, 9)

            # Try each operation based on the random value and the success rate
            # If an operation was not successful try the next one
            if random_action <= 7 and self.automate_spy_steal_tech(spy):
                continue
            elif random_action <= 9 and self.automate_spy_rig_election(spy):
                continue
            elif self.automate_spy_counter_intelligence(spy):
                continue
            elif spy.is_doing_work():
                continue  # We might have been doing counter intelligence and wanted to look for something better
            else:
                # Retry all of the operations one more time
                if self.automate_spy_steal_tech(spy):
                    continue
                if self.automate_spy_rig_election(spy):
                    continue
                if self.automate_spy_counter_intelligence(spy):
                    continue

            # There is nothing for our spy to do, put it in a random city
            available_cities = [
                city for city in self.civ_info.game_info.get_cities()
                if spy.can_move_to(city)
            ]
            if available_cities:
                spy.move_to(random.choice(available_cities))

        for spy in spies:
            self._check_if_should_stage_coup(spy)

    def automate_spy_steal_tech(self, spy: Spy) -> bool:
        """Move the spy to a city that we can steal a tech from.
        
        Args:
            spy: The spy to automate
            
        Returns:
            bool: Whether the spy was successfully assigned a target
        """
        if not self.civs_to_steal_from:
            return False
            
        # We want to move the spy to the city with the highest science generation
        # Players can't usually figure this out so lets do highest population instead
        target_civ = self.civs_to_steal_from_sorted[0]
        city_to_move_to = max(
            (city for city in target_civ.cities if spy.can_move_to(city)),
            key=lambda city: city.population.population,
            default=None
        )
        
        if city_to_move_to:
            spy.move_to(city_to_move_to)
            return True
        return False

    def automate_spy_rig_election(self, spy: Spy) -> bool:
        """Move the spy to a random city-state for election rigging.
        
        Args:
            spy: The spy to automate
            
        Returns:
            bool: Whether the spy was successfully assigned a target
        """
        eligible_cities = [
            city for city_state in self.city_states_to_rig
            for city in city_state.cities
            if (not city.is_being_razed
                and spy.can_move_to(city)
                and (city.civ.get_diplomacy_manager(self.civ_info).get_influence() < 150
                     or city.civ.get_ally_civ() != self.civ_info.civ_name))
        ]
        
        if eligible_cities:
            city_to_move_to = max(
                eligible_cities,
                key=lambda city: city.civ.get_diplomacy_manager(self.civ_info).get_influence()
            )
            spy.move_to(city_to_move_to)
            return True
        return False

    def automate_spy_counter_intelligence(self, spy: Spy) -> bool:
        """Move the spy to a random city of ours for counter-intelligence.
        
        Args:
            spy: The spy to automate
            
        Returns:
            bool: Whether the spy was successfully assigned counter-intelligence duty
        """
        available_cities = [
            city for city in self.civ_info.cities
            if spy.can_move_to(city)
        ]
        
        if available_cities:
            spy.move_to(random.choice(available_cities))
            return spy.action == SpyAction.CounterIntelligence
        return False

    def _check_if_should_stage_coup(self, spy: Spy) -> None:
        """Check if a spy should stage a coup.
        
        Args:
            spy: The spy to check
        """
        if not spy.can_do_coup():
            return
            
        if spy.get_coup_chance_of_success(False) < 0.7:
            return
            
        ally_civ_name = spy.get_city().civ.get_ally_civ()
        if ally_civ_name:
            ally_civ = self.civ_info.game_info.get_civilization(ally_civ_name)
            # Don't coup city-states whose allies are our friends
            if (ally_civ
                and self.civ_info.get_diplomacy_manager(ally_civ)
                and self.civ_info.get_diplomacy_manager(ally_civ).is_relationship_level_ge(RelationshipLevel.Friend)):
                return

        random_seed = len(self.civ_info.espionage_manager.spy_list) + self.civ_info.espionage_manager.spy_list.index(spy) + self.civ_info.game_info.turns
        random.seed(random_seed)
        random_action = random.randint(0, 99)
        
        if random_action < 20:
            spy.set_action(SpyAction.Coup, 1) 