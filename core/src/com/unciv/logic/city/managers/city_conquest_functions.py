from typing import List, Optional
import random
from com.unciv import Constants, GUI
from com.unciv.logic.battle import Battle
from com.unciv.logic.city import City, CityFlags, CityFocus
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization import NotificationCategory, NotificationIcon
from com.unciv.logic.civilization.diplomacy import DiplomaticModifiers, DiplomaticStatus
from com.unciv.logic.map.mapunit import UnitPromotions
from com.unciv.logic.trade import TradeLogic, TradeOffer, TradeOfferType
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.logic.city.managers.spy_flee_reason import SpyFleeReason

class CityConquestFunctions:
    def __init__(self, city: City):
        self.city = city
        # Use city position as seed for deterministic random numbers
        self.tile_based_random = random.Random(str(city.get_center_tile().position).__hash__())

    def _get_gold_for_capturing_city(self, conquering_civ: Civilization) -> int:
        base_gold = 20 + 10 * self.city.population.population + self.tile_based_random.randint(0, 39)
        turn_modifier = max(0, min(50, self.city.civ.game_info.turns - self.city.turn_acquired)) / 50.0
        city_modifier = 2.0 if self.city.contains_building_unique(UniqueType.DoublesGoldFromCapturingCity) else 1.0
        conquering_civ_modifier = 3.0 if conquering_civ.has_unique(UniqueType.TripleGoldFromEncampmentsAndCities) else 1.0

        gold_plundered = base_gold * turn_modifier * city_modifier * conquering_civ_modifier
        return int(gold_plundered)

    def _destroy_buildings_on_capture(self) -> None:
        # Possibly remove other buildings
        for building in self.city.city_constructions.get_built_buildings():
            if (building.has_unique(UniqueType.NotDestroyedWhenCityCaptured)
                or building.is_wonder
                or building.has_unique(UniqueType.IndicatesCapital, self.city.state)):
                continue
            if building.has_unique(UniqueType.DestroyedWhenCityCaptured):
                self.city.city_constructions.remove_building(building)
                continue
            # Regular buildings have a 34% chance of removal
            if self.tile_based_random.randint(0, 99) < 34:
                self.city.city_constructions.remove_building(building)

    def _remove_auto_promotion(self) -> None:
        self.city.unit_should_use_saved_promotion = {}
        self.city.unit_to_promotions = {}

    def _remove_buildings_on_move_to_civ(self) -> None:
        # Remove all buildings provided for free to this city
        # At this point, the city has *not* yet moved to the new civ
        for building in self.city.civ.civ_constructions.get_free_building_names(self.city):
            self.city.city_constructions.remove_building(building)
        self.city.city_constructions.free_buildings_provided_from_this_city.clear()

        for building in self.city.city_constructions.get_built_buildings():
            # Remove national wonders
            if building.is_national_wonder and not building.has_unique(UniqueType.NotDestroyedWhenCityCaptured):
                self.city.city_constructions.remove_building(building)

            # Check if we exceed MaxNumberBuildable for any buildings
            for unique in building.get_matching_uniques(UniqueType.MaxNumberBuildable):
                if (len([
                    city for city in self.city.civ.cities
                    if (city.city_constructions.contains_building_or_equivalent(building.name)
                        or city.city_constructions.is_being_constructed_or_enqueued(building.name))
                ]) >= int(unique.params[0])):
                    # For now, just destroy in new city. Even if constructing in own cities
                    self.city.city_constructions.remove_building(building)

    def _conquer_city(self, conquering_civ: Civilization, conquered_civ: Civilization, receiving_civ: Civilization) -> None:
        self.city.espionage.remove_all_present_spies(SpyFleeReason.CityCaptured)

        # Gain gold for plundering city
        gold_plundered = self._get_gold_for_capturing_city(conquering_civ)
        conquering_civ.add_gold(gold_plundered)
        conquering_civ.add_notification(
            f"Received [{gold_plundered}] Gold for capturing [{self.city.name}]",
            self.city.get_center_tile().position,
            NotificationCategory.General,
            NotificationIcon.Gold
        )

        reconquered_city_while_still_in_resistance = (
            self.city.previous_owner == receiving_civ.civ_name
            and self.city.is_in_resistance()
        )

        self._destroy_buildings_on_capture()
        self.move_to_civ(receiving_civ)
        Battle.destroy_if_defeated(conquered_civ, conquering_civ, self.city.location)

        self.city.health = self.city.get_max_health() // 2  # I think that cities recover to half health when conquered?
        self.city.avoid_growth = False  # reset settings
        self.city.set_city_focus(CityFocus.NoFocus)  # reset settings
        if self.city.population.population > 1:
            self.city.population.add_population(-1 - self.city.population.population // 4)  # so from 2-4 population, remove 1, from 5-8, remove 2, etc.
        self.city.reassign_all_population()

        if not reconquered_city_while_still_in_resistance and self.city.founding_civ != receiving_civ.civ_name:
            # add resistance
            # I checked, and even if you puppet there's resistance for conquering
            self.city.set_flag(CityFlags.Resistance, self.city.population.population)
        else:
            # reconquering or liberating city in resistance so eliminate it
            self.city.remove_flag(CityFlags.Resistance)

    def puppet_city(self, conquering_civ: Civilization) -> None:
        old_civ = self.city.civ

        # must be before moving the city to the conquering civ,
        # so the repercussions are properly checked
        self._diplomatic_repercussions_for_conquering_city(old_civ, conquering_civ)

        self._conquer_city(conquering_civ, old_civ, conquering_civ)

        self.city.is_puppet = True
        self.city.city_stats.update()
        # The city could be producing something that puppets shouldn't, like units
        self.city.city_constructions.current_construction_is_user_set = False
        self.city.city_constructions.in_progress_constructions.clear()  # undo all progress of the previous civ on units etc.
        self.city.city_constructions.construction_queue.clear()
        self.city.city_constructions.choose_next_construction()

    def annex_city(self) -> None:
        self.city.is_puppet = False
        if not self.city.is_in_resistance():
            self.city.should_reassign_population = True
        self.city.avoid_growth = False
        self.city.set_city_focus(CityFocus.NoFocus)
        self.city.city_stats.update()
        GUI.set_update_world_on_next_render()

    def _diplomatic_repercussions_for_conquering_city(self, old_civ: Civilization, conquering_civ: Civilization) -> None:
        current_population = self.city.population.population
        percentage_of_civ_population_in_that_city = current_population * 100.0 / sum(city.population.population for city in old_civ.cities)
        aggro_generated = 10.0 + round(percentage_of_civ_population_in_that_city)

        # How can you conquer a city but not know the civ you conquered it from?!
        # I don't know either, but some of our players have managed this, and crashed their game!
        if not conquering_civ.knows(old_civ):
            conquering_civ.diplomacy_functions.make_civilizations_meet(old_civ)

        old_civ.get_diplomacy_manager(conquering_civ).add_modifier(
            DiplomaticModifiers.CapturedOurCities,
            -aggro_generated
        )

        for third_party_civ in [civ for civ in conquering_civ.get_known_civs() if civ.is_major_civ()]:
            aggro_generated_for_other_civs = round(aggro_generated / 10.0)
            if third_party_civ.is_at_war_with(old_civ):  # Shared Enemies should like us more
                third_party_civ.get_diplomacy_manager(conquering_civ).add_modifier(
                    DiplomaticModifiers.SharedEnemy,
                    aggro_generated_for_other_civs
                )  # Cool, keep at it! =D
            else:
                third_party_civ.get_diplomacy_manager(conquering_civ).add_modifier(
                    DiplomaticModifiers.WarMongerer,
                    -aggro_generated_for_other_civs
                )  # Uncool bro.

    def liberate_city(self, conquering_civ: Civilization) -> None:
        if not self.city.founding_civ:  # this should never happen but just in case...
            self.puppet_city(conquering_civ)
            self.annex_city()
            return

        founding_civ = self.city.civ.game_info.get_civilization(self.city.founding_civ)
        if founding_civ.is_defeated():  # resurrected civ
            for diplo_manager in founding_civ.diplomacy.values():
                if diplo_manager.diplomatic_status == DiplomaticStatus.War:
                    diplo_manager.make_peace()

        old_civ = self.city.civ
        self._diplomatic_repercussions_for_liberating_city(conquering_civ, old_civ)
        self._conquer_city(conquering_civ, old_civ, founding_civ)

        if len(founding_civ.cities) == 1:
            # Resurrection!
            capital_city_indicator = conquering_civ.capital_city_indicator(self.city)
            if capital_city_indicator:
                self.city.city_constructions.add_building(capital_city_indicator)
            for civ in self.city.civ.game_info.civilizations:
                if civ in (founding_civ, conquering_civ):
                    continue  # don't need to notify these civs
                if civ.knows(conquering_civ) and civ.knows(founding_civ):
                    civ.add_notification(
                        f"[{conquering_civ}] has liberated [{founding_civ}]",
                        NotificationCategory.Diplomacy,
                        founding_civ.civ_name,
                        NotificationIcon.Diplomacy,
                        conquering_civ.civ_name
                    )
                elif civ.knows(conquering_civ):
                    civ.add_notification(
                        f"[{conquering_civ}] has liberated an unknown civilization",
                        NotificationCategory.Diplomacy,
                        NotificationIcon.Diplomacy,
                        conquering_civ.civ_name
                    )
                elif civ.knows(founding_civ):
                    civ.add_notification(
                        f"An unknown civilization has liberated [{founding_civ}]",
                        NotificationCategory.Diplomacy,
                        NotificationIcon.Diplomacy,
                        founding_civ.civ_name
                    )

        self.city.is_puppet = False
        self.city.city_stats.update()

        # Move units out of the city when liberated
        for unit in self.city.get_center_tile().get_units():
            unit.movement.teleport_to_closest_moveable_tile()
        for unit in [unit for tile in self.city.get_tiles() for unit in tile.get_units()]:
            if not unit.movement.can_pass_through(unit.current_tile):
                unit.movement.teleport_to_closest_moveable_tile()

    def _diplomatic_repercussions_for_liberating_city(self, conquering_civ: Civilization, conquered_civ: Civilization) -> None:
        founding_civ = next(civ for civ in conquered_civ.game_info.civilizations if civ.civ_name == self.city.founding_civ)
        percentage_of_civ_population_in_that_city = (
            self.city.population.population * 100.0 /
            (sum(city.population.population for city in founding_civ.cities) + self.city.population.population)
        )
        respect_for_liberating_our_city = 10.0 + round(percentage_of_civ_population_in_that_city)

        if founding_civ.is_major_civ():
            # In order to get "plus points" in Diplomacy, you have to establish diplomatic relations if you haven't yet
            founding_civ.get_diplomacy_manager_or_meet(conquering_civ).add_modifier(
                DiplomaticModifiers.CapturedOurCities,
                respect_for_liberating_our_city
            )
            open_borders_trade = TradeLogic(founding_civ, conquering_civ)
            open_borders_trade.current_trade.our_offers.append(
                TradeOffer(Constants.open_borders, TradeOfferType.Agreement, speed=conquering_civ.game_info.speed)
            )
            open_borders_trade.accept_trade(False)
        else:
            # Liberating a city state gives a large amount of influence, and peace
            founding_civ.get_diplomacy_manager_or_meet(conquering_civ).set_influence(90.0)
            if founding_civ.is_at_war_with(conquering_civ):
                trade_logic = TradeLogic(founding_civ, conquering_civ)
                trade_logic.current_trade.our_offers.append(
                    TradeOffer(Constants.peace_treaty, TradeOfferType.Treaty, speed=conquering_civ.game_info.speed)
                )
                trade_logic.current_trade.their_offers.append(
                    TradeOffer(Constants.peace_treaty, TradeOfferType.Treaty, speed=conquering_civ.game_info.speed)
                )
                trade_logic.accept_trade(False)

        other_civs_respect_for_liberating = round(respect_for_liberating_our_city / 10.0)
        for third_party_civ in [civ for civ in conquering_civ.get_known_civs() if civ.is_major_civ() and civ != conquered_civ]:
            third_party_civ.get_diplomacy_manager(conquering_civ).add_modifier(
                DiplomaticModifiers.LiberatedCity,
                other_civs_respect_for_liberating
            )  # Cool, keep at at! =D

    def move_to_civ(self, new_civ: Civilization) -> None:
        old_civ = self.city.civ

        # Remove/relocate palace for old Civ - need to do this BEFORE we move the cities between
        #  civs so the capitalCityIndicator recognizes the unique buildings of the conquered civ
        if self.city.is_capital():
            old_civ.move_capital_to_next_largest(self.city)

        old_civ.cities = [city for city in old_civ.cities if city != self.city]
        new_civ.cities.append(self.city)
        self.city.civ = new_civ
        self.city.state = StateForConditionals(self.city)
        self.city.has_just_been_conquered = False
        self.city.turn_acquired = self.city.civ.game_info.turns
        self.city.previous_owner = old_civ.civ_name

        # now that the tiles have changed, we need to reassign population
        for worked_tile in [tile for tile in self.city.worked_tiles if tile not in self.city.tiles]:
            self.city.population.stop_working_tile(worked_tile)
            self.city.population.auto_assign_population()

        # Stop WLTKD if it's still going
        self.city.reset_wltkd()

        # Remove their free buildings from this city and remove free buildings provided by the city from their cities
        self._remove_buildings_on_move_to_civ()

        # Remove auto promotion from city that is being moved
        self._remove_auto_promotion()

        # catch-all - should ideally not happen as we catch the individual cases with an appropriate notification
        self.city.espionage.remove_all_present_spies(SpyFleeReason.Other)

        # Place palace for newCiv if this is the only city they have.
        if len(new_civ.cities) == 1:
            new_civ.move_capital_to(self.city, None)

        # Add our free buildings to this city and add free buildings provided by the city to other cities
        self.city.civ.civ_constructions.try_add_free_buildings()

        self.city.is_being_razed = False

        # Transfer unique buildings
        for building in self.city.city_constructions.get_built_buildings():
            civ_equivalent_building = new_civ.get_equivalent_building(building)
            if building != civ_equivalent_building:
                self.city.city_constructions.remove_building(building)
                self.city.city_constructions.add_building(civ_equivalent_building)

        if self.city.civ.game_info.is_religion_enabled():
            self.city.religion.remove_unknown_pantheons()

        if new_civ.has_unique(UniqueType.MayNotAnnexCities):
            self.city.is_puppet = True
            self.city.city_constructions.current_construction_is_user_set = False
            self.city.city_constructions.construction_queue.clear()
            self.city.city_constructions.choose_next_construction()

        self.city.try_update_road_status()
        self.city.city_stats.update()

        # Update proximity rankings
        self.city.civ.update_proximity(old_civ, old_civ.update_proximity(self.city.civ))

        # Update history
        for tile in self.city.get_tiles():
            tile.history.record_take_ownership(tile)

        new_civ.cache.update_our_tiles()
        old_civ.cache.update_our_tiles()