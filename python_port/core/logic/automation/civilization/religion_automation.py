from typing import List, Optional, Set, Dict, Tuple
from dataclasses import dataclass
import random
import math

from com.unciv.Constants import Constants
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.managers import ReligionState
from com.unciv.logic.map.tile import Tile
from com.unciv.models.Counter import Counter
from com.unciv.models.ruleset import Belief, BeliefType, Victory
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.models.stats import Stat

class ReligionAutomation:
    """Handles AI automation for religious decisions and actions."""

    @staticmethod
    def spend_faith_on_religion(civ_info: Civilization) -> None:
        """Decide how to spend faith points.
        
        Args:
            civ_info: The civilization making the decision
        """
        if not civ_info.cities:
            return

        # Save for great prophet
        if (civ_info.religion_manager.religion_state != ReligionState.EnhancedReligion
            and (civ_info.religion_manager.remaining_foundable_religions() != 0 
                 or civ_info.religion_manager.religion_state > ReligionState.Pantheon)):
            ReligionAutomation._buy_great_prophet_in_any_city(civ_info)
            return
        
        if civ_info.religion_manager.remaining_foundable_religions() == 0:
            ReligionAutomation._buy_great_person(civ_info)
            ReligionAutomation._try_buy_any_religious_building(civ_info)
            return

        # If we don't have majority in all our own cities, build missionaries and inquisitors
        cities_without_our_religion = [
            city for city in civ_info.cities 
            if city.religion.get_majority_religion() != civ_info.religion_manager.religion
        ]
        
        # The original had a cap at 4 missionaries total, but 1/4 * the number of cities should be more appropriate
        if len(cities_without_our_religion) > 4 * len([
            unit for unit in civ_info.units.get_civ_units()
            if (unit.has_unique(UniqueType.CanSpreadReligion)
                or unit.has_unique(UniqueType.CanRemoveHeresy))
        ]):
            city_pressure_pairs = [
                (city, city.religion.get_pressure_deficit(civ_info.religion_manager.religion.name))
                for city in cities_without_our_religion
            ]
            city, pressure_difference = max(city_pressure_pairs, key=lambda x: x[1])
            
            if pressure_difference >= Constants.aiPreferInquisitorOverMissionaryPressureDifference:
                ReligionAutomation._buy_inquisitor_near(civ_info, city)
            ReligionAutomation._buy_missionary_in_any_city(civ_info)
            return

        # Get an inquisitor to defend our holy city
        holy_city = civ_info.religion_manager.get_holy_city()
        if (holy_city is not None
            and holy_city in civ_info.cities
            and not any(unit.has_unique(UniqueType.PreventSpreadingReligion) 
                       for unit in civ_info.units.get_civ_units())
            and not holy_city.religion.is_protected_by_inquisitor()):
            ReligionAutomation._buy_inquisitor_near(civ_info, holy_city)
            return
        
        # Just buy missionaries to spread our religion outside of our civ
        if len([unit for unit in civ_info.units.get_civ_units() 
                if unit.has_unique(UniqueType.CanSpreadReligion)]) < 4:
            ReligionAutomation._buy_missionary_in_any_city(civ_info)
            return

    @staticmethod
    def _try_buy_any_religious_building(civ_info: Civilization) -> None:
        """Attempt to buy any available religious building.
        
        Args:
            civ_info: The civilization attempting to buy buildings
        """
        for city in civ_info.cities:
            if not city.religion.get_majority_religion():
                continue
                
            buildings = city.religion.get_majority_religion().buildings_purchasable_by_beliefs
            building_to_be_purchased = min(
                (building for building in buildings
                 if (building := civ_info.get_equivalent_building(building)) is not None
                 and building.is_purchasable(city.city_constructions)
                 and (cost := building.get_stat_buy_cost(city, Stat.Faith)) is not None
                 and cost <= civ_info.religion_manager.stored_faith),
                key=lambda b: b.get_stat_buy_cost(city, Stat.Faith),
                default=None
            )
            
            if building_to_be_purchased:
                city.city_constructions.purchase_construction(
                    building_to_be_purchased, -1, True, Stat.Faith
                )
                return

    @staticmethod
    def _buy_missionary_in_any_city(civ_info: Civilization) -> None:
        """Buy a missionary in any suitable city.
        
        Args:
            civ_info: The civilization buying the missionary
        """
        if civ_info.religion_manager.religion_state < ReligionState.Religion:
            return
            
        missionaries = [
            civ_info.get_equivalent_unit(unit)
            for unit in civ_info.game_info.ruleset.units.values()
            if unit.has_unique(UniqueType.CanSpreadReligion)
        ]

        missionary_construction = min(
            (unit for unit in missionaries
             if any(unit.is_purchasable(city.city_constructions) 
                   and unit.can_be_purchased_with_stat(city, Stat.Faith)
                   for city in civ_info.cities)),
            key=lambda unit: min(
                unit.get_stat_buy_cost(city, Stat.Faith)
                for city in civ_info.cities
                if unit.is_purchasable(city.city_constructions)
                and unit.can_be_purchased_with_stat(city, Stat.Faith)
            ),
            default=None
        )
        
        if not missionary_construction:
            return

        has_unique_to_take_civ_religion = missionary_construction.has_unique(
            UniqueType.TakeReligionOverBirthCity
        )

        valid_cities_to_buy = [
            city for city in civ_info.cities
            if ((has_unique_to_take_civ_religion 
                 or city.religion.get_majority_religion() == civ_info.religion_manager.religion)
                and (cost := missionary_construction.get_stat_buy_cost(city, Stat.Faith)) is not None
                and cost <= civ_info.religion_manager.stored_faith
                and missionary_construction.is_purchasable(city.city_constructions)
                and missionary_construction.can_be_purchased_with_stat(city, Stat.Faith))
        ]
        
        if not valid_cities_to_buy:
            return

        cities_with_bonus_charges = [
            city for city in valid_cities_to_buy
            if any(
                promotion := city.get_ruleset().unit_promotions.get(
                    unique.params[2]
                ),
                promotion.has_unique(UniqueType.CanSpreadReligion)
                for unique in city.get_matching_uniques(UniqueType.UnitStartingPromotions)
            )
        ]
        
        holy_city = next(
            (city for city in valid_cities_to_buy 
             if city.is_holy_city_of(civ_info.religion_manager.religion.name)),
            None
        )

        city_to_buy_missionary = (
            cities_with_bonus_charges[0] if cities_with_bonus_charges
            else holy_city if holy_city
            else valid_cities_to_buy[0]
        )

        city_to_buy_missionary.city_constructions.purchase_construction(
            missionary_construction, -1, True, Stat.Faith
        )

    @staticmethod
    def _buy_great_prophet_in_any_city(civ_info: Civilization) -> None:
        """Buy a great prophet in any suitable city.
        
        Args:
            civ_info: The civilization buying the great prophet
        """
        if civ_info.religion_manager.religion_state < ReligionState.Religion:
            return
            
        great_prophet_unit = civ_info.religion_manager.get_great_prophet_equivalent()
        if not great_prophet_unit:
            return
            
        great_prophet_unit = civ_info.get_equivalent_unit(great_prophet_unit)
        
        city_to_buy_great_prophet = min(
            (city for city in civ_info.cities
             if (great_prophet_unit.is_purchasable(city.city_constructions)
                 and great_prophet_unit.can_be_purchased_with_stat(city, Stat.Faith)
                 and (cost := great_prophet_unit.get_stat_buy_cost(city, Stat.Faith)) is not None
                 and cost <= civ_info.religion_manager.stored_faith)),
            key=lambda city: great_prophet_unit.get_stat_buy_cost(city, Stat.Faith),
            default=None
        )
        
        if city_to_buy_great_prophet:
            city_to_buy_great_prophet.city_constructions.purchase_construction(
                great_prophet_unit, -1, True, Stat.Faith
            )

    @staticmethod
    def _buy_inquisitor_near(civ_info: Civilization, city: City) -> None:
        """Buy an inquisitor near a specific city.
        
        Args:
            civ_info: The civilization buying the inquisitor
            city: The city to buy near
        """
        if civ_info.religion_manager.religion_state < ReligionState.Religion:
            return
            
        inquisitors = [
            civ_info.get_equivalent_unit(unit)
            for unit in civ_info.game_info.ruleset.units.values()
            if unit.has_unique(UniqueType.CanRemoveHeresy) 
            or unit.has_unique(UniqueType.PreventSpreadingReligion)
        ]

        inquisitor_construction = min(
            (unit for unit in inquisitors
             if any(unit.is_purchasable(city.city_constructions) 
                   and unit.can_be_purchased_with_stat(city, Stat.Faith)
                   for city in civ_info.cities)),
            key=lambda unit: min(
                unit.get_stat_buy_cost(city, Stat.Faith)
                for city in civ_info.cities
                if unit.is_purchasable(city.city_constructions)
                and unit.can_be_purchased_with_stat(city, Stat.Faith)
            ),
            default=None
        )
        
        if not inquisitor_construction:
            return

        has_unique_to_take_civ_religion = inquisitor_construction.has_unique(
            UniqueType.TakeReligionOverBirthCity
        )

        valid_cities_to_buy = [
            city for city in civ_info.cities
            if ((has_unique_to_take_civ_religion 
                 or city.religion.get_majority_religion() == civ_info.religion_manager.religion)
                and (cost := inquisitor_construction.get_stat_buy_cost(city, Stat.Faith)) is not None
                and cost <= civ_info.religion_manager.stored_faith
                and inquisitor_construction.is_purchasable(city.city_constructions)
                and inquisitor_construction.can_be_purchased_with_stat(city, Stat.Faith))
        ]
        
        if not valid_cities_to_buy:
            return

        city_to_buy = min(
            valid_cities_to_buy,
            key=lambda c: c.get_center_tile().aerial_distance_to(city.get_center_tile())
        )

        city_to_buy.city_constructions.purchase_construction(
            inquisitor_construction, -1, True, Stat.Faith
        )

    @staticmethod
    def _buy_great_person(civ_info: Civilization) -> None:
        """Buy a great person with faith.
        
        Args:
            civ_info: The civilization buying the great person
        """
        great_person_units = [
            unit for unit in civ_info.game_info.ruleset.units.values()
            if (unit.has_unique(UniqueType.GreatPerson) 
                and not unit.has_unique(UniqueType.MayFoundReligion))
        ]

        great_person_construction = min(
            (unit for unit in great_person_units
             if any(unit.is_purchasable(city.city_constructions) 
                   and unit.can_be_purchased_with_stat(city, Stat.Faith)
                   for city in civ_info.cities)),
            key=lambda unit: min(
                unit.get_stat_buy_cost(city, Stat.Faith)
                for city in civ_info.cities
                if unit.is_purchasable(city.city_constructions)
                and unit.can_be_purchased_with_stat(city, Stat.Faith)
            ),
            default=None
        )
        
        if not great_person_construction:
            return

        valid_cities_to_buy = [
            city for city in civ_info.cities
            if (cost := great_person_construction.get_stat_buy_cost(city, Stat.Faith)) is not None
            and cost <= civ_info.religion_manager.stored_faith
        ]
        
        if not valid_cities_to_buy:
            return

        city_to_buy = valid_cities_to_buy[0]
        city_to_buy.city_constructions.purchase_construction(
            great_person_construction, -1, True, Stat.Faith
        )

    @staticmethod
    def rate_belief(civ_info: Civilization, belief: Belief) -> float:
        """Rate a belief for selection.
        
        Args:
            civ_info: The civilization rating the belief
            belief: The belief to rate
            
        Returns:
            float: The rating score
        """
        score = 0.0  # Roughly equivalent to the sum of stats gained across all cities

        for city in civ_info.cities:
            for tile in city.get_center_tile().get_tiles_in_distance(city.get_work_range()):
                tile_score = ReligionAutomation._belief_bonus_for_tile(belief, tile, city)
                score += tile_score * (
                    1.0 if city.worked_tiles.contains(tile.position)  # worked
                    else 0.7 if tile.get_city() == city  # workable
                    else 0.5  # unavailable - for now
                ) * (random.random() * 0.05 + 0.975)

            score += ReligionAutomation._belief_bonus_for_city(
                civ_info, belief, city
            ) * (random.random() * 0.1 + 0.95)

        score += ReligionAutomation._belief_bonus_for_player(
            civ_info, belief
        ) * (random.random() * 0.3 + 0.85)

        # All of these random.random() don't exist in the original, but I've added them to make things a bit more random.
        if belief.type == BeliefType.Pantheon:
            score *= 0.9

        return score

    @staticmethod
    def _belief_bonus_for_tile(belief: Belief, tile: Tile, city: City) -> float:
        """Calculate belief bonus for a specific tile.
        
        Args:
            belief: The belief to calculate bonus for
            tile: The tile to calculate bonus for
            city: The city the tile belongs to
            
        Returns:
            float: The calculated bonus
        """
        bonus_yield = 0.0
        for unique in belief.unique_objects:
            if unique.type == UniqueType.StatsFromObject:
                if ((tile.matches_filter(unique.params[1])
                     and not (tile.last_terrain.has_unique(UniqueType.ProductionBonusWhenRemoved) 
                             and tile.last_terrain.matches_filter(unique.params[1]))  # forest pantheons are bad
                     or (tile.resource is not None 
                         and (tile.tile_resource.matches_filter(unique.params[1]) 
                              or tile.tile_resource.is_improved_by(unique.params[1]))))):  # resource pantheons are good
                    bonus_yield += sum(unique.stats.values())
            elif unique.type == UniqueType.StatsFromTilesWithout:
                if (city.matches_filter(unique.params[3])
                    and tile.matches_filter(unique.params[1])
                    and not tile.matches_filter(unique.params[2])):
                    bonus_yield += sum(unique.stats.values())
        return bonus_yield

    @staticmethod
    def _belief_bonus_for_city(civ_info: Civilization, belief: Belief, city: City) -> float:
        """Calculate belief bonus for a specific city.
        
        Args:
            civ_info: The civilization calculating the bonus
            belief: The belief to calculate bonus for
            city: The city to calculate bonus for
            
        Returns:
            float: The calculated bonus
        """
        score = 0.0
        rule_set = civ_info.game_info.ruleset
        
        for unique in belief.unique_objects:
            modifier = 0.5 ** len(unique.modifiers)
            # Multiply by 3/10 if has an obsoleted era
            # Multiply by 2 if enough pop/followers (best implemented with conditionals, so left open for now)
            # If obsoleted, continue
            
            score += modifier * {
                UniqueType.GrowthPercentBonus: lambda: float(unique.params[0]) / 3.0,
                UniqueType.BorderGrowthPercentage: lambda: -float(unique.params[0]) / 10.0,
                UniqueType.StrengthForCities: lambda: float(unique.params[0]) / 10.0,  # Modified by personality
                UniqueType.CityHealingUnits: lambda: float(unique.params[1]) / 10.0,
                UniqueType.PercentProductionBuildings: lambda: float(unique.params[0]) / 3.0,
                UniqueType.PercentProductionWonders: lambda: float(unique.params[0]) / 3.0,
                UniqueType.PercentProductionUnits: lambda: float(unique.params[0]) / 3.0,
                UniqueType.StatsFromCitiesOnSpecificTiles: lambda: (
                    sum(unique.stats.values()) if city.get_center_tile().matches_filter(unique.params[1])
                    else 0.0
                ),  # Modified by personality
                UniqueType.StatsFromObject: lambda: (
                    sum(unique.stats.values()) * (
                        0.25 if rule_set.buildings.get(unique.params[1]) and rule_set.buildings[unique.params[1]].is_national_wonder
                        else 2.0 if rule_set.specialists.get(unique.params[1]) and city.population.population > 8.0
                        else 1.0 if rule_set.buildings.get(unique.params[1])
                        else 0.0
                    )
                ),
                UniqueType.StatsFromTradeRoute: lambda: (
                    sum(unique.stats.values()) * (1.0 if city.is_connected_to_capital() else 0.0)
                ),
                UniqueType.StatPercentFromReligionFollowers: lambda: min(
                    float(unique.params[0]) * city.population.population,
                    float(unique.params[2])
                ),
                UniqueType.StatsPerCity: lambda: (
                    sum(unique.stats.values()) if city.matches_filter(unique.params[1]) else 0.0
                ),
            }.get(unique.type, lambda: 0.0)()

        return score

    @staticmethod
    def _belief_bonus_for_player(civ_info: Civilization, belief: Belief) -> float:
        """Calculate belief bonus for the player.
        
        Args:
            civ_info: The civilization calculating the bonus
            belief: The belief to calculate bonus for
            
        Returns:
            float: The calculated bonus
        """
        score = 0.0
        number_of_founded_religions = sum(
            1 for civ in civ_info.game_info.civilizations
            if civ.religion_manager.religion is not None 
            and civ.religion_manager.religion_state >= ReligionState.Religion
        )
        max_number_of_religions = (number_of_founded_religions 
                                 + civ_info.religion_manager.remaining_foundable_religions())

        # Adjusts scores of certain beliefs as game evolves
        game_time_scaling_percent = 100
        if civ_info.religion_manager.religion_state == ReligionState.FoundingReligion:
            game_time_scaling_percent = 100 - ((number_of_founded_religions * 100) 
                                             / max_number_of_religions)
        elif civ_info.religion_manager.religion_state == ReligionState.EnhancingReligion:
            amount_of_enhanced_religions = sum(
                1 for civ in civ_info.game_info.civilizations
                if (civ.religion_manager.religion is not None 
                    and civ.religion_manager.religion_state == ReligionState.EnhancedReligion)
            )
            game_time_scaling_percent = 100 - ((amount_of_enhanced_religions * 100) 
                                             / max_number_of_religions)

        good_early_modifier = (
            1.0 if game_time_scaling_percent < 33
            else 2.0 if game_time_scaling_percent < 66
            else 4.0
        )
        good_late_modifier = (
            2.0 if game_time_scaling_percent < 33
            else 1.0 if game_time_scaling_percent < 66
            else 0.5
        )

        for unique in belief.unique_objects:
            modifier = (
                0.5 if any(
                    unique.get_modifiers(UniqueType.ConditionalOurUnit)
                    and modifier.params[0] == civ_info.religion_manager.get_great_prophet_equivalent().name
                    for modifier in unique.get_modifiers(UniqueType.ConditionalOurUnit)
                )
                else 1.0
            )
            
            score += modifier * {
                UniqueType.KillUnitPlunderNearCity: lambda: (
                    float(unique.params[0]) * (
                        0.5 if civ_info.wants_to_focus_on(Victory.Focus.Military)
                        else 0.25
                    )
                ),
                UniqueType.BuyUnitsForAmountStat: lambda: (
                    0.0 if (civ_info.religion_manager.religion is not None
                           and any(unique.type in civ_info.religion_manager.religion.follower_belief_unique_map.get_uniques(unique.type)))
                    else civ_info.stats.stats_for_next_turn[Stat(unique.params[2])] * 300.0 / float(unique.params[1])
                ),
                UniqueType.BuyBuildingsForAmountStat: lambda: (
                    0.0 if (civ_info.religion_manager.religion is not None
                           and any(unique.type in civ_info.religion_manager.religion.follower_belief_unique_map.get_uniques(unique.type)))
                    else civ_info.stats.stats_for_next_turn[Stat(unique.params[2])] * 300.0 / float(unique.params[1])
                ),
                UniqueType.BuyUnitsWithStat: lambda: (
                    0.0 if (civ_info.religion_manager.religion is not None
                           and any(unique.type in civ_info.religion_manager.religion.follower_belief_unique_map.get_uniques(unique.type)))
                    else civ_info.stats.stats_for_next_turn[Stat(unique.params[1])] * 300.0 / civ_info.get_era().base_unit_buy_cost
                ),
                UniqueType.BuyBuildingsWithStat: lambda: (
                    0.0 if (civ_info.religion_manager.religion is not None
                           and any(unique.type in civ_info.religion_manager.religion.follower_belief_unique_map.get_uniques(unique.type)))
                    else civ_info.stats.stats_for_next_turn[Stat(unique.params[1])] * 300.0 / civ_info.get_era().base_unit_buy_cost
                ),
                UniqueType.BuyUnitsByProductionCost: lambda: 0.0,  # Holy Warriors is a waste
                UniqueType.StatsWhenSpreading: lambda: float(unique.params[0]) / 15.0,
                UniqueType.StatsWhenAdoptingReligion: lambda: sum(unique.stats.values()) / 50.0,
                UniqueType.RestingPointOfCityStatesFollowingReligionChange: lambda: (
                    float(unique.params[0]) / 4.0 if civ_info.wants_to_focus_on(Victory.Focus.CityStates)
                    else float(unique.params[0]) / 8.0
                ),
                UniqueType.StatsFromGlobalCitiesFollowingReligion: lambda: sum(unique.stats.values()) * 2.0,
                UniqueType.StatsFromGlobalFollowers: lambda: 10.0 * (sum(unique.stats.values()) / float(unique.params[1])),
                UniqueType.Strength: lambda: float(unique.params[0]) * 3.0,  # combat strength from beliefs is very strong
                UniqueType.ReligionSpreadDistance: lambda: (10.0 + float(unique.params[0])) * good_early_modifier,
                UniqueType.NaturalReligionSpreadStrength: lambda: float(unique.params[0]) * good_early_modifier / 10.0,
                UniqueType.SpreadReligionStrength: lambda: float(unique.params[0]) * good_late_modifier / 10.0,
                UniqueType.FaithCostOfGreatProphetChange: lambda: -float(unique.params[0]) * good_late_modifier / 10.0,
                UniqueType.BuyBuildingsDiscount: lambda: -float(unique.params[2]) * good_late_modifier / 5.0,
                UniqueType.BuyUnitsDiscount: lambda: -float(unique.params[2]) * good_late_modifier / 5.0,
                UniqueType.BuyItemsDiscount: lambda: -float(unique.params[1]) * good_late_modifier / 5.0,
            }.get(unique.type, lambda: 0.0)()

        return score

    @staticmethod
    def choose_religious_beliefs(civ_info: Civilization) -> None:
        """Choose all religious beliefs for a civilization.
        
        Args:
            civ_info: The civilization choosing beliefs
        """
        ReligionAutomation._choose_pantheon(civ_info)
        ReligionAutomation._found_religion(civ_info)
        ReligionAutomation._enhance_religion(civ_info)
        ReligionAutomation._choose_free_beliefs(civ_info)

    @staticmethod
    def _choose_pantheon(civ_info: Civilization) -> None:
        """Choose a pantheon belief.
        
        Args:
            civ_info: The civilization choosing the pantheon
        """
        if not civ_info.religion_manager.can_found_or_expand_pantheon():
            return
            
        chosen_pantheon = ReligionAutomation._choose_belief_of_type(civ_info, BeliefType.Pantheon)
        if not chosen_pantheon:
            return  # panic!
            
        civ_info.religion_manager.choose_beliefs(
            [chosen_pantheon],
            use_free_beliefs=civ_info.religion_manager.using_free_beliefs()
        )

    @staticmethod
    def _found_religion(civ_info: Civilization) -> None:
        """Found a religion.
        
        Args:
            civ_info: The civilization founding the religion
        """
        if civ_info.religion_manager.religion_state != ReligionState.FoundingReligion:
            return
            
        available_religion_icons = [
            religion for religion in civ_info.game_info.ruleset.religions
            if religion not in [religion.name for religion in civ_info.game_info.religions.values()]
        ]
        
        favored_religion = civ_info.nation.favored_religion
        religion_icon = (
            favored_religion if (favored_religion in available_religion_icons
                               and random.randint(1, 10) <= 5)
            else random.choice(available_religion_icons) if available_religion_icons
            else None
        )
        
        if not religion_icon:
            return  # Wait what? How did we pass the checking when using a great prophet but not this?

        civ_info.religion_manager.found_religion(religion_icon, religion_icon)

        chosen_beliefs = list(ReligionAutomation._choose_beliefs(
            civ_info,
            civ_info.religion_manager.get_beliefs_to_choose_at_founding()
        ))
        civ_info.religion_manager.choose_beliefs(chosen_beliefs)

    @staticmethod
    def _enhance_religion(civ_info: Civilization) -> None:
        """Enhance a religion.
        
        Args:
            civ_info: The civilization enhancing the religion
        """
        if civ_info.religion_manager.religion_state != ReligionState.EnhancingReligion:
            return
            
        civ_info.religion_manager.choose_beliefs(
            list(ReligionAutomation._choose_beliefs(
                civ_info,
                civ_info.religion_manager.get_beliefs_to_choose_at_enhancing()
            ))
        )

    @staticmethod
    def _choose_free_beliefs(civ_info: Civilization) -> None:
        """Choose free beliefs.
        
        Args:
            civ_info: The civilization choosing free beliefs
        """
        if not civ_info.religion_manager.has_free_beliefs():
            return
            
        civ_info.religion_manager.choose_beliefs(
            list(ReligionAutomation._choose_beliefs(
                civ_info,
                civ_info.religion_manager.free_beliefs_as_enums()
            )),
            use_free_beliefs=True
        )

    @staticmethod
    def _choose_beliefs(civ_info: Civilization, beliefs_to_choose: Counter[BeliefType]) -> Set[Belief]:
        """Choose beliefs of specific types.
        
        Args:
            civ_info: The civilization choosing beliefs
            beliefs_to_choose: Counter of belief types to choose
            
        Returns:
            Set[Belief]: The chosen beliefs
        """
        chosen_beliefs = set()
        
        for belief_type in BeliefType:
            if belief_type == BeliefType.None:
                continue
                
            for _ in range(beliefs_to_choose[belief_type]):
                if belief := ReligionAutomation._choose_belief_of_type(
                    civ_info, belief_type, chosen_beliefs
                ):
                    chosen_beliefs.add(belief)
                    
        return chosen_beliefs

    @staticmethod
    def _choose_belief_of_type(
        civ_info: Civilization,
        belief_type: BeliefType,
        additional_beliefs_to_exclude: Set[Belief] = None
    ) -> Optional[Belief]:
        """Choose a specific belief of a given type.
        
        Args:
            civ_info: The civilization choosing the belief
            belief_type: The type of belief to choose
            additional_beliefs_to_exclude: Additional beliefs to exclude from consideration
            
        Returns:
            Optional[Belief]: The chosen belief, if any
        """
        if additional_beliefs_to_exclude is None:
            additional_beliefs_to_exclude = set()
            
        return max(
            (belief for belief in civ_info.game_info.ruleset.beliefs.values()
             if ((belief.type == belief_type or belief_type == BeliefType.Any)
                 and belief not in additional_beliefs_to_exclude
                 and civ_info.religion_manager.get_religion_with_belief(belief) is None
                 and not any(
                     not unique.conditionals_apply(civ_info.state)
                     for unique in belief.get_matching_uniques(
                         UniqueType.OnlyAvailable,
                         StateForConditionals.IgnoreConditionals
                     )
                 ))),
            key=lambda b: ReligionAutomation.rate_belief(civ_info, b),
            default=None
        ) 