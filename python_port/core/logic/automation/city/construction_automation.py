from typing import List, Set, Dict, Optional, Sequence, Any
from dataclasses import dataclass
import math
from enum import Enum

from com.unciv.logic.automation import Automation
from com.unciv.logic.automation.civilization import NextTurnAutomation
from com.unciv.logic.automation.unit import WorkerAutomation
from com.unciv.logic.city import CityConstructions
from com.unciv.logic.civilization import CityAction, NotificationCategory, NotificationIcon
from com.unciv.logic.map import BFS
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset import (
    Building, IConstruction, INonPerpetualConstruction,
    MilestoneType, PerpetualConstruction, Victory,
    PersonalityValue, LocalUniqueCache, UniqueType,
    BaseUnit, Stat, Stats
)
from com.unciv.ui.screens.cityscreen import CityScreen
from com.unciv.GUI import GUI
from com.unciv.UncivGame import UncivGame

@dataclass
class ConstructionChoice:
    choice: str
    choice_modifier: float
    remaining_work: int
    production: int

class ConstructionAutomation:
    def __init__(self, city_constructions: CityConstructions):
        self.city_constructions = city_constructions
        self.city = city_constructions.city
        self.civ_info = self.city.civ
        
        self.relative_cost_effectiveness: List[ConstructionChoice] = []
        self.city_state = self.city.state
        self.city_stats = self.city.city_stats
        
        self.personality = self.civ_info.get_personality()
        
        self.constructions_to_avoid = [
            unique.params[0] for unique in 
            self.personality.get_matching_uniques(UniqueType.WillNotBuild, self.city_state)
        ]
        
        self.disabled_auto_assign_constructions: Set[str] = (
            GUI.get_settings().disabled_auto_assign_constructions 
            if self.civ_info.is_human() 
            else set()
        )
        
        self.buildable_buildings: Dict[str, bool] = {}
        self.buildable_units: Dict[str, bool] = {}
        
        # Initialize sequences
        self._initialize_sequences()
        
        # Calculate derived values
        self._calculate_derived_values()
        
    def _initialize_sequences(self):
        """Initialize the building and unit sequences."""
        buildings = self.city.get_ruleset().buildings.values
        self.buildings = (
            building for building in buildings
            if building.name not in self.disabled_auto_assign_constructions 
            and not self._should_avoid_construction(building)
        )
        
        self.non_wonders = (
            building for building in self.buildings
            if not building.is_any_wonder()
            and self.buildable_buildings.get(building.name, True)
        )
        
        units = self.city.get_ruleset().units.values
        self.units = (
            unit for unit in units
            if self.buildable_units.get(unit.name, True)
            and unit.name not in self.disabled_auto_assign_constructions
            and not self._should_avoid_construction(unit)
        )
    
    def _calculate_derived_values(self):
        """Calculate derived values used in decision making."""
        self.civ_units = self.civ_info.units.get_civ_units()
        self.military_units = sum(1 for unit in self.civ_units if unit.base_unit.is_military)
        self.workers = sum(1 for unit in self.civ_units if unit.cache.has_unique_to_build_improvements)
        self.cities = len(self.civ_info.cities)
        self.all_techs_are_researched = self.civ_info.tech.all_techs_are_researched()
        
        self.is_at_war = self.civ_info.is_at_war()
        self.buildings_for_victory = [
            milestone.params[0] for milestone in
            (civ_info.victory_manager.get_next_milestone(victory) 
             for victory in self.civ_info.game_info.get_enabled_victories().values())
            if milestone and milestone.type in (MilestoneType.BuiltBuilding, MilestoneType.BuildingBuiltGlobally)
        ]
        
        self.spaceship_parts = self.civ_info.game_info.space_resources
        
        self.average_production = sum(
            city.city_stats.current_city_stats.production 
            for city in self.civ_info.cities
        ) / len(self.civ_info.cities)
        
        self.city_is_over_average_production = (
            self.city.city_stats.current_city_stats.production >= self.average_production
        )
    
    def _should_avoid_construction(self, construction: IConstruction) -> bool:
        """Check if a construction should be avoided based on personality settings."""
        state_for_conditionals = self.city_state
        for to_avoid in self.constructions_to_avoid:
            if isinstance(construction, Building) and construction.matches_filter(to_avoid, state_for_conditionals):
                return True
            if isinstance(construction, BaseUnit) and construction.matches_filter(to_avoid, state_for_conditionals):
                return True
        return False
    
    def _add_choice(self, choice: str, choice_modifier: float):
        """Add a construction choice to the list of choices."""
        self.relative_cost_effectiveness.append(ConstructionChoice(
            choice=choice,
            choice_modifier=choice_modifier,
            remaining_work=self.city_constructions.get_remaining_work(choice),
            production=self.city_constructions.production_for_construction(choice)
        ))
    
    def _filter_buildable(self, sequence: Sequence[INonPerpetualConstruction]) -> Sequence[INonPerpetualConstruction]:
        """Filter a sequence of constructions to only include buildable ones."""
        return (
            item for item in sequence
            if self._is_buildable(item)
        )
    
    def _is_buildable(self, item: INonPerpetualConstruction) -> bool:
        """Check if an item is buildable in the current city."""
        cache = self.buildable_buildings if isinstance(item, Building) else self.buildable_units
        if item.name not in cache:
            cache[item.name] = item.is_buildable(self.city_constructions)
        return cache[item.name]
    
    def choose_next_construction(self):
        """Choose the next construction for the city."""
        if not isinstance(self.city_constructions.get_current_construction(), PerpetualConstruction):
            return
            
        self._add_building_choices()
        
        if not self.city.is_puppet:
            self._add_spaceship_part_choice()
            self._add_worker_choice()
            self._add_work_boat_choice()
            self._add_military_unit_choice()
            
        chosen_construction = self._select_chosen_construction()
        self._update_construction_and_notify(chosen_construction)
    
    def _select_chosen_construction(self) -> str:
        """Select the most appropriate construction from available choices."""
        if not self.relative_cost_effectiveness:
            return self._select_special_construction()
            
        if any(choice.remaining_work < choice.production * 30 
               for choice in self.relative_cost_effectiveness):
            return self._select_from_short_term_choices()
            
        return self._select_from_long_term_choices()
    
    def _select_special_construction(self) -> str:
        """Select a special construction when no regular choices are available."""
        if (PerpetualConstruction.science.is_buildable(self.city_constructions) 
            and not self.all_techs_are_researched):
            return PerpetualConstruction.science.name
        if PerpetualConstruction.gold.is_buildable(self.city_constructions):
            return PerpetualConstruction.gold.name
        if (PerpetualConstruction.culture.is_buildable(self.city_constructions) 
            and not self.civ_info.policies.all_policies_adopted(True)):
            return PerpetualConstruction.culture.name
        if PerpetualConstruction.faith.is_buildable(self.city_constructions):
            return PerpetualConstruction.faith.name
        return PerpetualConstruction.idle.name
    
    def _select_from_short_term_choices(self) -> str:
        """Select from choices that can be completed within 30 turns."""
        self.relative_cost_effectiveness = [
            choice for choice in self.relative_cost_effectiveness
            if choice.remaining_work < choice.production * 30
        ]
        
        if not any(choice.choice_modifier >= 0 for choice in self.relative_cost_effectiveness):
            return max(
                self.relative_cost_effectiveness,
                key=lambda c: (c.remaining_work / c.choice_modifier) / max(c.production, 1)
            ).choice
            
        self.relative_cost_effectiveness = [
            choice for choice in self.relative_cost_effectiveness
            if choice.choice_modifier >= 0
        ]
        return min(
            self.relative_cost_effectiveness,
            key=lambda c: (c.remaining_work / c.choice_modifier) / max(c.production, 1)
        ).choice
    
    def _select_from_long_term_choices(self) -> str:
        """Select from long-term choices when no short-term options are available."""
        return min(
            self.relative_cost_effectiveness,
            key=lambda c: c.remaining_work / max(c.production, 1)
        ).choice
    
    def _update_construction_and_notify(self, chosen_construction: str):
        """Update the current construction and notify if necessary."""
        no_notification = (
            self.city.is_in_resistance()
            or self.civ_info.is_ai()
            or self.city_constructions.current_construction_from_queue == chosen_construction
            or isinstance(UncivGame.Current.screen, CityScreen)
        )
        
        self.city_constructions.current_construction_from_queue = chosen_construction
        if no_notification:
            return
            
        self.civ_info.add_notification(
            f"[{self.city.name}] has started working on [{chosen_construction}]",
            CityAction.with_location(self.city),
            NotificationCategory.Production,
            NotificationIcon.Construction
        )
    
    def _add_military_unit_choice(self):
        """Add military unit choices based on current game state."""
        if not self.is_at_war and not self.city_is_over_average_production:
            return  # don't make any military units here. Infrastructure first!
            
        if self.civ_info.stats.get_unit_supply_deficit() > 0:
            return  # we don't want more units if it's already hurting our empire
            
        if (not self.is_at_war and 
            (self.civ_info.stats.stats_for_next_turn.gold < 0 or 
             self.military_units > max(7, self.cities * 5))):
            return
            
        if self.civ_info.gold < -50:
            return

        military_unit = Automation.choose_military_unit(self.city, self.units)
        if not military_unit:
            return
            
        units_to_cities_ratio = self.cities / (self.military_units + 1)
        modifier = 1 + math.sqrt(units_to_cities_ratio) / 2
        
        if (self.civ_info.wants_to_focus_on(Victory.Focus.Military) or 
            self.is_at_war):
            modifier *= 2

        if Automation.afraid_of_barbarians(self.civ_info):
            modifier = 2.0  # military units are pro-growth if pressured by barbs
            
        if not self.city_is_over_average_production:
            modifier /= 5  # higher production cities will deal with this

        civilian_unit = self.city.get_center_tile().civilian_unit
        if (civilian_unit and civilian_unit.has_unique(UniqueType.FoundCity) and
            not any(tile.military_unit and tile.military_unit.civ == self.civ_info
                   for tile in self.city.get_center_tile().get_tiles_in_distance(
                       self.city.get_expand_range()))):
            modifier = 5.0  # there's a settler just sitting here, doing nothing - BAD

        if not self.civ_info.is_ai_or_auto_playing():
            modifier /= 2  # Players prefer to make their own unit choices usually
            
        modifier *= self.personality.modifier_focus(PersonalityValue.Military, 0.3)
        self._add_choice(military_unit, modifier)

    def _add_work_boat_choice(self):
        """Add work boat choices based on available water resources."""
        buildable_workboat_units = {
            unit for unit in self.units
            if (unit.has_unique(UniqueType.CreateWaterImprovements) and
                Automation.allow_automated_construction(self.civ_info, self.city, unit))
        }
        if not buildable_workboat_units:
            return

        # Check for existing workboats
        two_turns_movement = max(unit.movement for unit in buildable_workboat_units) * 2
        
        def is_our_workboat(unit: MapUnit) -> bool:
            return (unit.cache.has_unique_to_create_water_improvements and 
                   unit.civ == self.civ_info)
                    
        already_has_workboat = any(
            tile.civilian_unit and is_our_workboat(tile.civilian_unit)
            for tile in self.city.get_center_tile().get_tiles_in_distance(two_turns_movement)
        )
        if already_has_workboat:
            return

        def is_worth_improving(tile: Tile) -> bool:
            if tile.get_owner() != self.civ_info:
                return False
            if not WorkerAutomation.has_workable_sea_resource(tile, self.civ_info):
                return False
            return WorkerAutomation.is_not_bonus_resource_or_workable(tile, self.civ_info)

        def find_tile_worth_improving() -> bool:
            search_max_tiles = self.civ_info.game_info.ruleset.mod_options.constants.workboat_automation_search_max_tiles
            bfs = BFS(self.city.get_center_tile())
            bfs.set_condition(lambda tile: (
                tile.is_water or tile.is_city_center() and
                (tile.get_owner() is None or tile.is_friendly_territory(self.civ_info)) and
                tile.is_explored(self.civ_info)
            ))
            
            while bfs.size() < search_max_tiles:
                tile = bfs.next_step()
                if not tile:
                    break
                if is_worth_improving(tile):
                    return True
            return False

        if not find_tile_worth_improving():
            return

        self._add_choice(
            min(buildable_workboat_units, key=lambda u: u.cost).name,
            0.6
        )

    def _add_worker_choice(self):
        """Add worker choices based on current worker count and city count."""
        worker_equivalents = [
            unit for unit in self.units
            if (unit.has_unique(UniqueType.BuildImprovements) and
                Automation.allow_automated_construction(self.civ_info, self.city, unit))
        ]
        if not worker_equivalents:
            return

        # Calculate desired worker count
        if self.cities <= 1:
            workers_wanted = 1.0
        elif self.cities <= 5:
            workers_wanted = self.cities * 1.5
        else:
            workers_wanted = 7.5 + (self.cities - 5)

        if self.workers < workers_wanted:
            modifier = workers_wanted / (self.workers + 0.17)
            self._add_choice(
                min(worker_equivalents, key=lambda u: u.cost).name,
                modifier
            )

    def _add_spaceship_part_choice(self):
        """Add spaceship part choices if conditions are met."""
        if not self.city_is_over_average_production:
            return
            
        if not self.civ_info.has_unique(UniqueType.EnablesConstructionOfSpaceshipParts):
            return
            
        spaceship_part = next(
            (item for item in self.non_wonders if item.name in self.spaceship_parts),
            None
        )
        if not spaceship_part:
            return
            
        modifier = 20.0  # We're weighing Apollo program according to personality
        self._add_choice(spaceship_part.name, modifier)

    def _add_building_choices(self):
        """Add building choices based on various factors."""
        local_unique_cache = LocalUniqueCache()
        for building in self.buildings:
            if building.is_wonder and self.city.is_puppet:
                continue
            if (building.is_wonder and 
                (not self.city_is_over_average_production or 
                 sum(city.population.population for city in self.civ_info.cities) < 12)):
                continue
                
            self._add_choice(
                building.name,
                self._get_value_of_building(building, local_unique_cache)
            )

    def _get_value_of_building(self, building: Building, local_unique_cache: LocalUniqueCache) -> float:
        """Calculate the value of a building based on various factors."""
        value = 0.0
        value += self._apply_building_stats(building, local_unique_cache)
        value += self._apply_military_building_value(building)
        value += self._apply_victory_building_value(building)
        value += self._apply_onetime_unique_bonuses(building)
        return value

    def _apply_onetime_unique_bonuses(self, building: Building) -> float:
        """Apply value from one-time unique bonuses."""
        value = 0.0
        # TODO: Add specific Uniques here
        return value

    def _apply_victory_building_value(self, building: Building) -> float:
        """Apply value from victory-related building effects."""
        value = 0.0
        if not self.city_is_over_average_production:
            return value
            
        if building.is_wonder:
            value += 2.0
            
        if (building.has_unique(UniqueType.TriggersCulturalVictory) or
            building.has_unique(UniqueType.TriggersVictory)):
            value += 20.0  # if we're this close to actually winning, we don't care what your preferred victory type is
            
        if building.has_unique(UniqueType.EnablesConstructionOfSpaceshipParts):
            value += 10.0 * self.personality.modifier_focus(PersonalityValue.Science, 0.3)
            
        return value

    def _apply_military_building_value(self, building: Building) -> float:
        """Apply value from military-related building effects."""
        value = 0.0
        war_modifier = 1.0 if self.is_at_war else 0.5
        
        # If this city is the closest city to another civ, that makes it a likely candidate for attack
        if any(
            NextTurnAutomation.get_closest_cities(self.civ_info, civ)[0] == self.city
            for civ in self.civ_info.get_known_civs()
        ):
            war_modifier *= 2.0
            
        value += (war_modifier * building.city_health / self.city.get_max_health() * 
                 self.personality.inverse_modifier_focus(PersonalityValue.Aggressive, 0.3))
                 
        value += (war_modifier * building.city_strength / (self.city.get_strength() + 3) * 
                 self.personality.inverse_modifier_focus(PersonalityValue.Aggressive, 0.3))

        for experience_unique in (
            building.get_matching_uniques(UniqueType.UnitStartingExperience, self.city_state) +
            building.get_matching_uniques(UniqueType.UnitStartingExperienceOld, self.city_state)
        ):
            modifier = float(experience_unique.params[1]) / 5
            modifier *= 1.0 if self.city_is_over_average_production else 0.2
            modifier *= self.personality.modifier_focus(PersonalityValue.Military, 0.3)
            modifier *= max(1.0, self.personality.modifier_focus(PersonalityValue.Aggressive, 0.2))
            value += modifier
            
        if (building.has_unique(UniqueType.EnablesNuclearWeapons) and 
            not self.civ_info.has_unique(UniqueType.EnablesNuclearWeapons)):
            value += 4.0 * self.personality.modifier_focus(PersonalityValue.Military, 0.3)
            
        return value

    def _apply_building_stats(self, building: Building, local_unique_cache: LocalUniqueCache) -> float:
        """Apply value from building stats and unique effects."""
        building_stats = self._get_stat_difference_from_building(building.name, local_unique_cache)
        self._get_building_stats_from_uniques(building, building_stats)

        surplus_food = self.city.city_stats.current_city_stats[Stat.Food]
        if surplus_food < 0:
            building_stats.food *= 8  # Starving, need Food, get to 0
        else:
            building_stats.food *= 3

        if self.civ_info.stats.stats_for_next_turn.gold < 10:
            building_stats.gold *= 2  # We have a gold problem and need to adjust build queue accordingly

        if (self.civ_info.get_happiness() < 10 or 
            self.civ_info.get_happiness() < self.civ_info.cities.size):
            building_stats.happiness *= 5

        if self.city.city_stats.current_city_stats.culture < 2:
            building_stats.culture *= 2  # We need to start growing borders

        for stat in Stat:
            if self.civ_info.wants_to_focus_on(stat):
                building_stats[stat] *= 2.0

            building_stats[stat] *= self.personality.modifier_focus(
                PersonalityValue[stat], 0.5
            )

        return Automation.rank_stats_value(
            self.civ_info.get_personality().scale_stats(building_stats.clone(), 0.3),
            self.civ_info
        )

    def _get_stat_difference_from_building(self, building: str, local_unique_cache: LocalUniqueCache) -> Stats:
        """Calculate the stat difference that would result from building a building."""
        new_city = self.city.clone()
        new_city.set_transients(self.city.civ)
        new_city.city_constructions.built_buildings.add(building)
        new_city.city_constructions.set_transients()
        new_city.city_stats.update(update_civ_stats=False, local_unique_cache=local_unique_cache)
        self.city.expansion.set_transients()
        return new_city.city_stats.current_city_stats - self.city.city_stats.current_city_stats

    def _get_building_stats_from_uniques(self, building: Building, building_stats: Stats):
        """Apply stat effects from building uniques."""
        for unique in building.get_matching_uniques(UniqueType.StatPercentBonusCities, self.city_state):
            stat_type = Stat[unique.params[1]]
            relative_amount = float(unique.params[0]) / 100.0
            amount = self.civ_info.stats.stats_for_next_turn[stat_type] * relative_amount
            building_stats[stat_type] += amount

        for unique in building.get_matching_uniques(UniqueType.CarryOverFood, self.city_state):
            if (self.city.matches_filter(unique.params[1]) and 
                int(unique.params[0]) != 0):
                food_gain = self.city_stats.current_city_stats.food + building_stats.food
                relative_amount = float(unique.params[0]) / 100.0
                building_stats[Stat.Food] += food_gain * relative_amount 