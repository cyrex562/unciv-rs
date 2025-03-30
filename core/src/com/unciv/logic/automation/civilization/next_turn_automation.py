from typing import List, Optional, Set, Tuple
from dataclasses import dataclass

from com.unciv.logic.automation import Automation, ThreatLevel
from com.unciv.logic.automation.unit import EspionageAutomation, UnitAutomation
from com.unciv.logic.battle import Battle, BattleDamage, CityCombatant, MapUnitCombatant
from com.unciv.logic.battle import TargetHelper
from com.unciv.logic.city import City
from com.unciv.logic.civilization import (
    AlertType, Civilization, NotificationCategory, NotificationIcon,
    PopupAlert
)
from com.unciv.logic.civilization.diplomacy import (
    DiplomacyFlags, DiplomaticModifiers, DiplomaticStatus,
    RelationshipLevel
)
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset import (
    MilestoneType, Policy, PolicyBranch, Victory
)
from com.unciv.models.ruleset.nation import PersonalityValue
from com.unciv.models.ruleset.tech import Technology
from com.unciv.models.ruleset.tile import ResourceType
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.models.ruleset.unit import BaseUnit
from com.unciv.models.stats import Stat
from com.unciv.ui.screens.victoryscreen import RankingType
from com.unciv.utils.random import random_weighted

@dataclass
class CityDistance:
    """Represents the distance between two cities."""
    city1: City
    city2: City
    aerial_distance: int

class NextTurnAutomation:
    """Handles AI turn automation for civilizations."""

    @staticmethod
    def automate_civ_moves(civ_info: Civilization, trade_and_change_state: bool = True) -> None:
        """Top-level AI turn task list.
        
        Args:
            civ_info: The civilization to automate
            trade_and_change_state: Set false for 'forced' automation, such as skip turn
        """
        if civ_info.is_barbarian:
            BarbarianAutomation(civ_info).automate()
            return
        if civ_info.is_spectator():
            return  # When there's a spectator in multiplayer games, it's processed automatically, but shouldn't be able to actually do anything

        NextTurnAutomation._respond_to_popup_alerts(civ_info)
        TradeAutomation.respond_to_trade_requests(civ_info, trade_and_change_state)

        if trade_and_change_state and civ_info.is_major_civ():
            if not civ_info.game_info.ruleset.mod_options.has_unique(UniqueType.DiplomaticRelationshipsCannotChange):
                DiplomacyAutomation.declare_war(civ_info)
                DiplomacyAutomation.offer_peace_treaty(civ_info)
                DiplomacyAutomation.ask_for_help(civ_info)
                DiplomacyAutomation.offer_declaration_of_friendship(civ_info)
            if civ_info.game_info.is_religion_enabled():
                ReligionAutomation.spend_faith_on_religion(civ_info)
            
            DiplomacyAutomation.offer_open_borders(civ_info)
            DiplomacyAutomation.offer_research_agreement(civ_info)
            DiplomacyAutomation.offer_defensive_pact(civ_info)
            TradeAutomation.exchange_luxuries(civ_info)
            
            NextTurnAutomation._issue_requests(civ_info)
            NextTurnAutomation._adopt_policy(civ_info)
            NextTurnAutomation._free_up_space_resources(civ_info)
        elif civ_info.is_city_state:
            civ_info.city_state_functions.get_free_tech_for_city_state()
            civ_info.city_state_functions.update_diplomatic_relationship_for_city_state()

        NextTurnAutomation._choose_tech_to_research(civ_info)
        NextTurnAutomation.automate_city_bombardment(civ_info)
        if trade_and_change_state:
            UseGoldAutomation.use_gold(civ_info)
        if trade_and_change_state and not civ_info.is_city_state:
            NextTurnAutomation._protect_city_states(civ_info)
            NextTurnAutomation._bully_city_states(civ_info)
        NextTurnAutomation._automate_units(civ_info)

        if trade_and_change_state and civ_info.is_major_civ():
            if civ_info.game_info.is_religion_enabled():
                ReligionAutomation.choose_religious_beliefs(civ_info)
            if civ_info.game_info.is_espionage_enabled():
                EspionageAutomation(civ_info).automate_spies()

        NextTurnAutomation._automate_cities(civ_info)
        if trade_and_change_state:
            NextTurnAutomation._train_settler(civ_info)
        NextTurnAutomation._try_vote_for_diplomatic_victory(civ_info)

    @staticmethod
    def automate_gold_to_science_percentage(civ_info: Civilization) -> None:
        """Automate the conversion of gold to science percentage.
        
        Args:
            civ_info: The civilization to automate
        """
        estimated_income = int(civ_info.stats.stats_for_next_turn.gold)
        projected_gold = civ_info.gold + estimated_income
        
        piss_poor = civ_info.tech.era.base_unit_buy_cost
        stinking_rich = civ_info.tech.era.starting_gold * 10 + civ_info.cities.size * 2 * piss_poor
        max_percent = 0.8
        
        if civ_info.gold <= 0:
            civ_info.tech.gold_percent_converted_to_science = 0.0
        elif projected_gold <= piss_poor:
            civ_info.tech.gold_percent_converted_to_science = 0.0
        else:
            civ_info.tech.gold_percent_converted_to_science = min(
                max_percent,
                (projected_gold - piss_poor) * max_percent / stinking_rich
            )

    @staticmethod
    def _respond_to_popup_alerts(civ_info: Civilization) -> None:
        """Handle popup alerts for the civilization.
        
        Args:
            civ_info: The civilization to handle alerts for
        """
        for popup_alert in list(civ_info.popup_alerts):  # toList because this can trigger other things that give alerts, like Golden Age
            if popup_alert.type == AlertType.DemandToStopSettlingCitiesNear:
                demanding_civ = civ_info.game_info.get_civilization(popup_alert.value)
                diplo_manager = civ_info.get_diplomacy_manager(demanding_civ)
                if Automation.threat_assessment(civ_info, demanding_civ) >= ThreatLevel.High:
                    diplo_manager.agree_not_to_settle_near()
                else:
                    diplo_manager.refuse_demand_not_to_settle_near()

            if popup_alert.type == AlertType.DemandToStopSpreadingReligion:
                demanding_civ = civ_info.game_info.get_civilization(popup_alert.value)
                diplo_manager = civ_info.get_diplomacy_manager(demanding_civ)
                if (Automation.threat_assessment(civ_info, demanding_civ) >= ThreatLevel.High
                    or diplo_manager.is_relationship_level_gt(RelationshipLevel.Ally)):
                    diplo_manager.agree_not_to_spread_religion_to()
                else:
                    diplo_manager.refuse_not_to_spread_religion_to()

            if popup_alert.type == AlertType.DeclarationOfFriendship:
                requesting_civ = civ_info.game_info.get_civilization(popup_alert.value)
                diplo_manager = civ_info.get_diplomacy_manager(requesting_civ)
                if (civ_info.diplomacy_functions.can_sign_declaration_of_friendship_with(requesting_civ)
                    and DiplomacyAutomation.wants_to_sign_declaration_of_friendship(civ_info, requesting_civ)):
                    diplo_manager.sign_declaration_of_friendship()
                    requesting_civ.add_notification(
                        f"We have signed a Declaration of Friendship with [{civ_info.civ_name}]!",
                        NotificationCategory.Diplomacy,
                        NotificationIcon.Diplomacy,
                        civ_info.civ_name
                    )
                else:
                    diplo_manager.other_civ_diplomacy().set_flag(DiplomacyFlags.DeclinedDeclarationOfFriendship, 10)
                    requesting_civ.add_notification(
                        f"[{civ_info.civ_name}] has denied our Declaration of Friendship!",
                        NotificationCategory.Diplomacy,
                        NotificationIcon.Diplomacy,
                        civ_info.civ_name
                    )

        civ_info.popup_alerts.clear()  # AIs don't care about popups.

    @staticmethod
    def value_city_state_alliance(civ_info: Civilization, city_state: Civilization, include_quests: bool = False) -> int:
        """Calculate the value of an alliance with a city-state.
        
        Args:
            civ_info: The civilization evaluating the alliance
            city_state: The city-state being evaluated
            include_quests: Whether to include quest values in the calculation
            
        Returns:
            int: The calculated value of the alliance
        """
        value = 0
        civ_personality = civ_info.get_personality()

        if city_state.city_state_functions.can_provide_stat(Stat.Culture):
            if civ_info.wants_to_focus_on(Victory.Focus.Culture):
                value += 10
            value += civ_personality[PersonalityValue.Culture] - 5

        if city_state.city_state_functions.can_provide_stat(Stat.Faith):
            if civ_info.wants_to_focus_on(Victory.Focus.Faith):
                value += 10
            value += civ_personality[PersonalityValue.Faith] - 5

        if city_state.city_state_functions.can_provide_stat(Stat.Production):
            if civ_info.wants_to_focus_on(Victory.Focus.Production):
                value += 10
            value += civ_personality[PersonalityValue.Production] - 5

        if city_state.city_state_functions.can_provide_stat(Stat.Science):
            if civ_info.wants_to_focus_on(Victory.Focus.Science):
                value += 10
            value += civ_personality[PersonalityValue.Science] - 5

        if civ_info.wants_to_focus_on(Victory.Focus.Military):
            if not city_state.is_alive():
                value -= 5
            else:
                # Don't ally close city-states, conquer them instead
                distance = NextTurnAutomation.get_min_distance_between_cities(civ_info, city_state)
                if distance < 20:
                    value -= (20 - distance) // 4

        if civ_info.wants_to_focus_on(Victory.Focus.CityStates):
            value += 5  # Generally be friendly

        if city_state.city_state_functions.can_provide_stat(Stat.Happiness):
            if civ_info.get_happiness() < 10:
                value += 10 - civ_info.get_happiness()
            value += civ_personality[PersonalityValue.Happiness] - 5

        if city_state.city_state_functions.can_provide_stat(Stat.Food):
            value += 5
            value += civ_personality[PersonalityValue.Food] - 5

        if not city_state.is_alive() or not city_state.cities or not civ_info.cities:
            return value

        # The more we have invested into the city-state the more the alliance is worth
        our_influence = (city_state.get_diplomacy_manager(civ_info).get_influence()
                        if civ_info.knows(city_state) else 0)
        value += min(10, our_influence // 10)  # don't let this spiral out of control

        if our_influence < 30:
            # Consider bullying for cash
            value -= 5

        if city_state.get_ally_civ() and city_state.get_ally_civ() != civ_info.civ_name:
            # easier not to compete if a third civ has this locked down
            third_civ_influence = city_state.get_diplomacy_manager(city_state.get_ally_civ()).get_influence()
            value -= (third_civ_influence - 30) // 10

        # Bonus for luxury resources we can get from them
        value += sum(1 for resource in city_state.detailed_civ_resources
                    if resource.resource.resource_type == ResourceType.Luxury
                    and resource.resource not in [supply.resource for supply in civ_info.detailed_civ_resources])

        if include_quests:
            # Investing is better if there is an investment bonus quest active.
            value += int(city_state.quest_manager.get_investment_multiplier(civ_info.civ_name) * 10) - 10

        return value

    @staticmethod
    def _protect_city_states(civ_info: Civilization) -> None:
        """Protect city-states that are threatened.
        
        Args:
            civ_info: The civilization to protect city-states
        """
        for state in [civ for civ in civ_info.get_known_civs() 
                     if not civ.is_defeated() and civ.is_city_state]:
            if state.city_state_functions.other_civ_can_pledge_protection(civ_info):
                state.city_state_functions.add_protector_civ(civ_info)
                # Always pledge to protect, as it makes it harder for others to demand tribute, and grants +10 resting Influence

    @staticmethod
    def _bully_city_states(civ_info: Civilization) -> None:
        """Bully city-states for resources when possible.
        
        Args:
            civ_info: The civilization to bully city-states
        """
        for state in [civ for civ in civ_info.get_known_civs() 
                     if not civ.is_defeated() and civ.is_city_state]:
            diplomacy_manager = state.get_diplomacy_manager(civ_info.civ_name)
            if (diplomacy_manager.is_relationship_level_lt(RelationshipLevel.Friend)
                    and diplomacy_manager.diplomatic_status == DiplomaticStatus.Peace
                    and NextTurnAutomation.value_city_state_alliance(civ_info, state) <= 0
                    and state.city_state_functions.get_tribute_willingness(civ_info) >= 0):
                if state.city_state_functions.get_tribute_willingness(civ_info, demanding_worker=True) > 0:
                    state.city_state_functions.tribute_worker(civ_info)
                else:
                    state.city_state_functions.tribute_gold(civ_info)

    @staticmethod
    def _choose_tech_to_research(civ_info: Civilization) -> None:
        """Choose which technology to research next.
        
        Args:
            civ_info: The civilization choosing technology
        """
        def get_grouped_researchable_techs() -> List[List[Technology]]:
            researchable_techs = {
                cost: [tech for tech in civ_info.game_info.ruleset.technologies.values()
                      if civ_info.tech.can_be_researched(tech.name)]
                for cost in {tech.cost for tech in civ_info.game_info.ruleset.technologies.values()}
            }
            return list(researchable_techs.values())

        state_for_conditionals = civ_info.state
        while civ_info.tech.free_techs > 0:
            costs = get_grouped_researchable_techs()
            if not costs:
                return
            
            most_expensive_techs = next(
                (techs for techs in reversed(costs)
                 if any(tech.get_weight_for_ai_decision(state_for_conditionals) > 0 for tech in techs)),
                costs[-1]
            )
            chosen_tech = random_weighted(
                most_expensive_techs,
                lambda tech: tech.get_weight_for_ai_decision(state_for_conditionals)
            )
            civ_info.tech.get_free_technology(chosen_tech.name)

        if not civ_info.tech.techs_to_research:
            costs = get_grouped_researchable_techs()
            if not costs:
                return

            cheapest_techs = next(
                (techs for techs in costs
                 if any(tech.get_weight_for_ai_decision(state_for_conditionals) > 0 for tech in techs)),
                costs[0]
            )

            # Do not consider advanced techs if only one tech left in cheapest group
            tech_to_research = (
                random_weighted(
                    cheapest_techs,
                    lambda tech: tech.get_weight_for_ai_decision(state_for_conditionals)
                )
                if len(cheapest_techs) == 1 or len(costs) == 1
                else random_weighted(
                    cheapest_techs + costs[1],
                    lambda tech: tech.get_weight_for_ai_decision(state_for_conditionals)
                )
            )

            civ_info.tech.techs_to_research.append(tech_to_research.name)

    @staticmethod
    def _adopt_policy(civ_info: Civilization) -> None:
        """Adopt policies based on priorities and completion status.
        
        Args:
            civ_info: The civilization adopting policies
        """
        while civ_info.policies.can_adopt_policy():
            incomplete_branches: Set[PolicyBranch] = civ_info.policies.incomplete_branches
            adoptable_branches: Set[PolicyBranch] = civ_info.policies.adoptable_branches

            # Skip the whole thing if all branches are completed
            if not incomplete_branches and not adoptable_branches:
                return

            priority_map = civ_info.policies.priority_map
            max_incomplete_priority = civ_info.policies.get_max_priority(incomplete_branches)
            max_adoptable_priority = civ_info.policies.get_max_priority(adoptable_branches)

            # This here is a (probably dirty) code to bypass NoSuchElementException error
            # when one of the priority variables is None
            if max_incomplete_priority is None:
                max_incomplete_priority = max_adoptable_priority - 1
            if max_adoptable_priority is None:
                max_adoptable_priority = max_incomplete_priority - 1

            # Candidate branches to adopt
            candidates: Set[PolicyBranch] = (
                # If incomplete branches have higher priorities than any newly adoptable branch,
                {branch for branch in incomplete_branches
                 if priority_map[branch] == max_incomplete_priority}
                if max_adoptable_priority <= max_incomplete_priority
                # If newly adoptable branches have higher priorities than any incomplete branch,
                else {branch for branch in adoptable_branches
                      if priority_map[branch] == max_adoptable_priority}
            )

            # branchCompletionMap but keys are only candidates
            candidate_completion_map = {
                branch: completion
                for branch, completion in civ_info.policies.branch_completion_map.items()
                if branch in candidates
            }

            # Choose the branch with the LEAST REMAINING policies, not the MOST ADOPTED ones
            target_branch = min(
                candidate_completion_map.items(),
                key=lambda x: len(x[0].policies) - x[1]
            )[0]

            policy_to_adopt = (
                target_branch
                if civ_info.policies.is_adoptable(target_branch)
                else random_weighted(
                    [policy for policy in target_branch.policies
                     if civ_info.policies.is_adoptable(policy)],
                    lambda policy: policy.get_weight_for_ai_decision(civ_info.state)
                )
            )

            civ_info.policies.adopt(policy_to_adopt)

    @staticmethod
    def choose_great_person(civ_info: Civilization) -> None:
        """Choose and add a great person to the civilization.
        
        Args:
            civ_info: The civilization choosing a great person
        """
        if civ_info.great_people.free_great_people == 0:
            return

        mayan_great_person = civ_info.great_people.maya_limited_free_gp > 0
        great_people = (
            [gp for gp in civ_info.great_people.get_great_people()
             if gp.name in civ_info.great_people.long_count_gp_pool]
            if mayan_great_person
            else civ_info.great_people.get_great_people()
        )

        if not great_people:
            return

        great_person = great_people[0]
        science_gp = next(
            (gp for gp in great_people
             if "Great Person - [Science]" in gp.uniques),
            None
        )
        if science_gp:
            great_person = science_gp
            # Humans would pick a prophet or engineer, but it'd require more sophistication on part of the AI - a scientist is the safest option for now

        civ_info.units.add_unit(great_person, next((city for city in civ_info.cities if city.is_capital()), None))

        civ_info.great_people.free_great_people -= 1
        if mayan_great_person:
            civ_info.great_people.long_count_gp_pool.remove(great_person.name)
            civ_info.great_people.maya_limited_free_gp -= 1

    @staticmethod
    def _free_up_space_resources(civ_info: Civilization) -> None:
        """Free up resources needed for spaceship construction.
        
        Args:
            civ_info: The civilization freeing up resources
        """
        # No need to build spaceship parts just yet
        if not any(civ_info.victory_manager.get_next_milestone(victory).type == MilestoneType.AddedSSPartsInCapital
                  for victory in civ_info.game_info.ruleset.victories.values()):
            return

        for resource in civ_info.game_info.space_resources:
            # Have enough resources already
            if civ_info.get_resource_amount(resource) >= Automation.get_reserved_space_resource_amount(civ_info):
                continue

            unit_to_disband = min(
                (unit for unit in civ_info.units.get_civ_units()
                 if unit.requires_resource(resource)),
                key=lambda unit: unit.get_force_evaluation(),
                default=None
            )
            if unit_to_disband:
                unit_to_disband.disband()

            for city in civ_info.cities:
                if city.has_sold_building_this_turn:
                    continue
                building_to_sell = next(
                    (building for building in civ_info.game_info.ruleset.buildings.values()
                     if (city.city_constructions.is_built(building.name)
                         and resource in building.required_resources(city.state)
                         and building.is_sellable()
                         and not civ_info.civ_constructions.has_free_building(city, building))),
                    None
                )
                if building_to_sell:
                    city.sell_building(building_to_sell)
                    break

    @staticmethod
    def _automate_units(civ_info: Civilization) -> None:
        """Automate unit movements and actions.
        
        Args:
            civ_info: The civilization automating units
        """
        is_at_war = civ_info.is_at_war()
        sorted_units = sorted(
            civ_info.units.get_civ_units(),
            key=lambda unit: NextTurnAutomation.get_unit_priority(unit, is_at_war)
        )
        
        cities_requiring_manual_placement = [
            city for city in [
                city for civ in civ_info.get_known_civs()
                if civ.is_at_war_with(civ_info)
                for city in civ.cities
            ]
            if len([tile for tile in city.get_center_tile().get_tiles_in_distance(4)
                   if tile.military_unit and tile.military_unit.civ == civ_info]) > 4
        ]
        
        for city in cities_requiring_manual_placement:
            NextTurnAutomation.automate_city_conquer(civ_info, city)
        
        for unit in sorted_units:
            UnitAutomation.automate_unit_moves(unit)

    @staticmethod
    def automate_city_conquer(civ_info: Civilization, city: City) -> None:
        """Automate the conquest of a specific city.
        
        Args:
            civ_info: The civilization conquering the city
            city: The city to conquer
        """
        def our_units_in_range(range: int) -> List[MapUnit]:
            return [
                unit for tile in city.get_center_tile().get_tiles_in_distance(range)
                if tile.military_unit and tile.military_unit.civ == civ_info
                for unit in [tile.military_unit]
            ]
        
        def attack_if_possible(unit: MapUnit, tile: Tile) -> None:
            attackable_tile = next(
                (t for t in TargetHelper.get_attackable_enemies(
                    unit, unit.movement.get_distance_to_tiles(), [tile]
                )),
                None
            )
            if attackable_tile:
                Battle.move_and_attack(MapUnitCombatant(unit), attackable_tile)
        
        # Air units should do their thing before any of this
        for unit in our_units_in_range(7):
            if unit.base_unit.is_air_unit():
                UnitAutomation.automate_unit_moves(unit)
        
        # First off, any siege unit that can attack the city, should
        siege_units = [unit for unit in our_units_in_range(4)
                      if unit.base_unit.is_probably_siege_unit()]
        for unit in siege_units:
            if not unit.has_unique(UniqueType.MustSetUp) or unit.is_set_up_for_siege():
                attack_if_possible(unit, city.get_center_tile())
        
        # Melee units should focus on getting rid of enemy units that threaten the siege units
        # If there are no units, this means attacking the city
        melee_units = [unit for unit in our_units_in_range(5)
                      if unit.base_unit.is_melee()]
        for unit in sorted(melee_units, key=lambda u: u.base_unit.get_force_evaluation(), reverse=True):
            # We're so close, full speed ahead!
            if city.health < city.get_max_health() / 5:
                attack_if_possible(unit, city.get_center_tile())
            
            tiles_to_target = city.get_center_tile().get_tiles_in_distance(4)
            
            attackable_enemies = TargetHelper.get_attackable_enemies(
                unit, unit.movement.get_distance_to_tiles(), tiles_to_target
            )
            if not attackable_enemies:
                continue
            enemy_we_will_damage_most = max(
                attackable_enemies,
                key=lambda enemy: BattleDamage.calculate_damage_to_defender(
                    MapUnitCombatant(unit), enemy.combatant, enemy.tile_to_attack_from, 0.5
                )
            )
            
            Battle.move_and_attack(MapUnitCombatant(unit), enemy_we_will_damage_most)

    @staticmethod
    def get_unit_priority(unit: MapUnit, is_at_war: bool) -> int:
        """Get the priority of a unit for automation.
        
        Args:
            unit: The unit to get priority for
            is_at_war: Whether the civilization is at war
            
        Returns:
            int: The unit's priority (lower is higher priority)
        """
        if unit.is_civilian() and not unit.is_great_person_of_type("War"):
            return 1  # Civilian
        if unit.base_unit.is_air_unit():
            if unit.can_intercept():
                return 2  # Fighters first
            if unit.is_nuclear_weapon():
                return 3  # Then Nukes (area damage)
            if not unit.has_unique(UniqueType.SelfDestructs):
                return 4  # Then Bombers (reusable)
            return 5  # Missiles

        distance = (0 if not is_at_war
                   else unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6))
        # Lower health units should move earlier to swap with higher health units
        return (distance + (unit.health / 10) +
                (10 if unit.base_unit.is_ranged()
                 else 30 if unit.base_unit.is_melee()
                 else 100 if unit.is_great_person_of_type("War")  # Generals move after military units
                 else 1))

    @staticmethod
    def automate_city_bombardment(civ_info: Civilization) -> None:
        """Automate city bombardment.
        
        Args:
            civ_info: The civilization automating bombardment
        """
        for city in civ_info.cities:
            UnitAutomation.try_bombard_enemy(city)

    @staticmethod
    def _automate_cities(civ_info: Civilization) -> None:
        """Automate city management.
        
        Args:
            civ_info: The civilization automating cities
        """
        own_military_strength = civ_info.get_stat_for_ranking(RankingType.Force)
        sum_of_enemies_military_strength = sum(
            civ.get_stat_for_ranking(RankingType.Force)
            for civ in civ_info.game_info.civilizations
            if civ != civ_info and not civ.is_barbarian and civ_info.is_at_war_with(civ)
        )
        civ_has_significantly_weaker_military_than_enemies = (
            own_military_strength < sum_of_enemies_military_strength * 0.66
        )

        for city in civ_info.cities:
            if (city.is_puppet and city.population.population > 9
                    and not city.is_in_resistance()
                    and not civ_info.has_unique(UniqueType.MayNotAnnexCities)):
                city.annex_city()

            city.reassign_all_population()

            if (city.health < city.get_max_health()
                    or civ_has_significantly_weaker_military_than_enemies):
                Automation.try_train_military_unit(city)  # need defenses if city is under attack
                if city.city_constructions.construction_queue:
                    continue  # found a unit to build so move on

            city.city_constructions.choose_next_construction()

    @staticmethod
    def _train_settler(civ_info: Civilization) -> None:
        """Train settlers when appropriate.
        
        Args:
            civ_info: The civilization training settlers
        """
        personality = civ_info.get_personality()
        if (civ_info.is_city_state or civ_info.is_one_city_challenger()
                or civ_info.is_at_war()  # don't train settlers when you could be training troops
                or not civ_info.cities
                or civ_info.get_happiness() <= civ_info.cities.size):
            return

        if any(unit.has_unique(UniqueType.FoundCity)
               for unit in civ_info.units.get_civ_units()):
            return
        if any(
            isinstance(city.city_constructions.get_current_construction(), BaseUnit)
            and city.city_constructions.get_current_construction().is_city_founder()
            for city in civ_info.cities
        ):
            return

        settler_units = [
            unit for unit in civ_info.game_info.ruleset.units.values()
            if (unit.is_city_founder() and unit.is_buildable(civ_info)
                and not any(
                    unit.matches_filter(unique.params[0], civ_info.state)
                    for unique in personality.get_matching_uniques(UniqueType.WillNotBuild, civ_info.state)
                ))
        ]
        if not settler_units:
            return

        if len([unit for unit in civ_info.units.get_civ_units() if unit.is_military()]) < civ_info.cities.size:
            return  # We need someone to defend them first

        workers_buildable_for_this_civ = any(
            unit.has_unique(UniqueType.BuildImprovements) and unit.is_buildable(civ_info)
            for unit in civ_info.game_info.ruleset.units.values()
        )

        best_city = max(
            (city for city in civ_info.cities if not city.is_puppet
             if (not workers_buildable_for_this_civ
                 or len([tile for tile in city.get_center_tile().get_tiles_in_distance(civ_info.mod_constants.city_work_range - 1)
                        if tile.improvement]) > 1
                 or any(tile.civilian_unit and tile.civilian_unit.has_unique(UniqueType.BuildImprovements)
                       for tile in city.get_center_tile().get_tiles_in_distance(civ_info.mod_constants.city_work_range))),
             key=lambda city: city.city_stats.current_city_stats.production
        )
        if not best_city:
            return

        if len(best_city.city_constructions.get_built_buildings()) > 1:  # 2 buildings or more, otherwise focus on self first
            best_city.city_constructions.current_construction_from_queue = min(
                settler_units, key=lambda unit: unit.cost
            ).name

    @staticmethod
    def _try_vote_for_diplomatic_victory(civ: Civilization) -> None:
        """Try to vote for diplomatic victory.
        
        Args:
            civ: The civilization voting
        """
        if not civ.may_vote_for_diplomatic_victory():
            return

        chosen_civ = None
        if civ.is_major_civ():
            known_major_civs = [c for c in civ.get_known_civs() if c.is_major_civ()]
            highest_opinion = max(
                (civ.get_diplomacy_manager(other_civ).opinion_of_other_civ()
                 for other_civ in known_major_civs),
                default=None
            )

            if highest_opinion is None:  # Abstain if we know nobody
                pass
            elif highest_opinion < -80 or (highest_opinion < -40 and highest_opinion + random.randint(0, 39) < -40):
                pass  # Abstain if we hate everybody (proportional chance in the RelationshipLevel.Enemy range - lesser evil)
            else:
                chosen_civ = random.choice([
                    other_civ for other_civ in known_major_civs
                    if civ.get_diplomacy_manager(other_civ).opinion_of_other_civ() == highest_opinion
                ]).civ_name
        else:
            chosen_civ = civ.get_ally_civ()

        civ.diplomatic_vote_for_civ(chosen_civ)

    @staticmethod
    def _issue_requests(civ_info: Civilization) -> None:
        """Issue diplomatic requests to other civilizations.
        
        Args:
            civ_info: The civilization issuing requests
        """
        for other_civ in [c for c in civ_info.get_known_civs()
                         if c.is_major_civ() and not civ_info.is_at_war_with(c)]:
            diplo_manager = civ_info.get_diplomacy_manager(other_civ)
            if diplo_manager.has_flag(DiplomacyFlags.SettledCitiesNearUs):
                NextTurnAutomation._on_city_settled_near_borders(civ_info, other_civ)
            if diplo_manager.has_flag(DiplomacyFlags.SpreadReligionInOurCities):
                NextTurnAutomation._on_religion_spread_in_our_city(civ_info, other_civ)

    @staticmethod
    def _on_city_settled_near_borders(civ_info: Civilization, other_civ: Civilization) -> None:
        """Handle cities being settled near borders.
        
        Args:
            civ_info: The civilization making the request
            other_civ: The civilization being requested
        """
        diplomacy_manager = civ_info.get_diplomacy_manager(other_civ)
        if diplomacy_manager.has_flag(DiplomacyFlags.IgnoreThemSettlingNearUs):
            pass
        elif diplomacy_manager.has_flag(DiplomacyFlags.AgreedToNotSettleNearUs):
            other_civ.popup_alerts.append(PopupAlert(
                AlertType.CitySettledNearOtherCivDespiteOurPromise,
                civ_info.civ_name
            ))
            diplomacy_manager.set_flag(DiplomacyFlags.IgnoreThemSettlingNearUs, 100)
            diplomacy_manager.set_modifier(DiplomaticModifiers.BetrayedPromiseToNotSettleCitiesNearUs, -20.0)
            diplomacy_manager.remove_flag(DiplomacyFlags.AgreedToNotSettleNearUs)
        else:
            threat_level = Automation.threat_assessment(civ_info, other_civ)
            if threat_level < ThreatLevel.High:  # don't piss them off for no reason please.
                other_civ.popup_alerts.append(PopupAlert(
                    AlertType.DemandToStopSettlingCitiesNear,
                    civ_info.civ_name
                ))
        diplomacy_manager.remove_flag(DiplomacyFlags.SettledCitiesNearUs)

    @staticmethod
    def _on_religion_spread_in_our_city(civ_info: Civilization, other_civ: Civilization) -> None:
        """Handle religion being spread in our cities.
        
        Args:
            civ_info: The civilization making the request
            other_civ: The civilization being requested
        """
        diplomacy_manager = civ_info.get_diplomacy_manager(other_civ)
        if diplomacy_manager.has_flag(DiplomacyFlags.IgnoreThemSpreadingReligion):
            pass
        elif diplomacy_manager.has_flag(DiplomacyFlags.AgreedToNotSpreadReligion):
            other_civ.popup_alerts.append(PopupAlert(
                AlertType.ReligionSpreadDespiteOurPromise,
                civ_info.civ_name
            ))
            diplomacy_manager.set_flag(DiplomacyFlags.IgnoreThemSpreadingReligion, 100)
            diplomacy_manager.set_modifier(DiplomaticModifiers.BetrayedPromiseToNotSpreadReligionToUs, -20.0)
            diplomacy_manager.remove_flag(DiplomacyFlags.AgreedToNotSpreadReligion)
        else:
            threat_level = Automation.threat_assessment(civ_info, other_civ)
            if threat_level < ThreatLevel.High:  # don't piss them off for no reason please.
                other_civ.popup_alerts.append(PopupAlert(
                    AlertType.DemandToStopSpreadingReligion,
                    civ_info.civ_name
                ))
        diplomacy_manager.remove_flag(DiplomacyFlags.SpreadReligionInOurCities)

    @staticmethod
    def get_min_distance_between_cities(civ1: Civilization, civ2: Civilization) -> int:
        """Get the minimum distance between cities of two civilizations.
        
        Args:
            civ1: First civilization
            civ2: Second civilization
            
        Returns:
            int: The minimum distance between cities
        """
        closest_cities = NextTurnAutomation.get_closest_cities(civ1, civ2)
        return closest_cities.aerial_distance if closest_cities else float('inf')

    @staticmethod
    def get_closest_cities(civ1: Civilization, civ2: Civilization) -> Optional[CityDistance]:
        """Get the closest cities between two civilizations.
        
        Args:
            civ1: First civilization
            civ2: Second civilization
            
        Returns:
            Optional[CityDistance]: The closest cities and their distance
        """
        if not civ1.cities or not civ2.cities:
            return None
        
        min_distance = None
        for civ1_city in civ1.cities:
            for civ2_city in civ2.cities:
                current_distance = civ1_city.get_center_tile().aerial_distance_to(civ2_city.get_center_tile())
                if min_distance is None or current_distance < min_distance.aerial_distance:
                    min_distance = CityDistance(civ1_city, civ2_city, current_distance)

        return min_distance 