import random
from typing import List, Optional

from com.unciv import Constants
from com.unciv.logic.automation import Automation, ThreatLevel
from com.unciv.logic.automation.civilization.motivation_to_attack_automation import has_at_least_motivation_to_attack
from com.unciv.logic.civilization import AlertType, Civilization, PopupAlert
from com.unciv.logic.civilization.diplomacy import (
    DiplomacyFlags, DiplomaticModifiers, DiplomaticStatus, RelationshipLevel
)
from com.unciv.logic.trade import (
    TradeLogic, TradeOffer, TradeRequest, TradeOfferType, TradeEvaluation
)
from com.unciv.models.ruleset.nation import PersonalityValue
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.models.translations import tr
from com.unciv.ui.screens.victoryscreen import RankingType
from com.unciv.logic.automation.civilization.declare_war_target_automation import DeclareWarTargetAutomation
from com.unciv.logic.automation.civilization.declare_war_plan_evaluator import DeclareWarPlanEvaluator

class DiplomacyAutomation:
    """Contains logic for handling diplomatic actions and decisions."""
    
    @staticmethod
    def offer_declaration_of_friendship(civ_info: Civilization) -> None:
        """Offer declarations of friendship to other civilizations."""
        civs_that_we_can_declare_friendship_with = sorted(
            [
                civ for civ in civ_info.get_known_civs()
                if (civ_info.diplomacy_functions.can_sign_declaration_of_friendship_with(civ)
                    and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedDeclarationOfFriendship))
            ],
            key=lambda x: civ_info.get_diplomacy_manager(x).relationship_level(),
            reverse=True
        )

        for other_civ in civs_that_we_can_declare_friendship_with:
            # Default setting is 2, this will be changed according to different civ
            if (random.randint(1, 10) <= 2 * civ_info.get_personality().modifier_focus(PersonalityValue.Diplomacy, 0.5)
                and DiplomacyAutomation._wants_to_sign_declaration_of_friendship(civ_info, other_civ)):
                other_civ.popup_alerts.append(PopupAlert(AlertType.DeclarationOfFriendship, civ_info.civ_name))

    @staticmethod
    def _wants_to_sign_declaration_of_friendship(civ_info: Civilization, other_civ: Civilization) -> bool:
        """Determine if we want to sign a declaration of friendship with another civilization."""
        diplo_manager = civ_info.get_diplomacy_manager(other_civ)
        if diplo_manager.has_flag(DiplomacyFlags.DeclinedDeclarationOfFriendship):
            return False
        # Shortcut, if it is below favorable then don't consider it
        if diplo_manager.is_relationship_level_lt(RelationshipLevel.Favorable):
            return False

        num_of_friends = sum(1 for d in civ_info.diplomacy.values() 
                           if d.has_flag(DiplomacyFlags.DeclarationOfFriendship))
        other_civ_number_of_friends = sum(1 for d in other_civ.diplomacy.values() 
                                        if d.has_flag(DiplomacyFlags.DeclarationOfFriendship))
        known_civs = sum(1 for c in civ_info.get_known_civs() 
                        if c.is_major_civ() and c.is_alive())
        all_civs = sum(1 for c in civ_info.game_info.civilizations 
                      if c.is_major_civ()) - 1  # Don't include us
        dead_civs = sum(1 for c in civ_info.game_info.civilizations 
                       if c.is_major_civ() and not c.is_alive())
        all_alive_civs = all_civs - dead_civs

        # Motivation should be constant as the number of civs changes
        motivation = diplo_manager.opinion_of_other_civ() - 40.0

        # Warmongerers don't make good allies
        if diplo_manager.has_modifier(DiplomaticModifiers.WarMongerer):
            motivation -= (diplo_manager.get_modifier(DiplomaticModifiers.WarMongerer) * 
                         civ_info.get_personality().modifier_focus(PersonalityValue.Diplomacy, 0.5))

        # If the other civ is stronger than we are compelled to be nice to them
        # If they are too weak, then their friendship doesn't mean much to us
        threat_level = Automation.threat_assessment(civ_info, other_civ)
        motivation += {
            ThreatLevel.VeryHigh: 10,
            ThreatLevel.High: 5,
            ThreatLevel.VeryLow: -5,
            ThreatLevel.Low: 0,
            ThreatLevel.Neutral: 0
        }[threat_level]

        # Try to ally with a fourth of the civs in play
        civs_to_ally_with = 0.25 * all_alive_civs * civ_info.get_personality().modifier_focus(PersonalityValue.Diplomacy, 0.3)
        if num_of_friends < civs_to_ally_with:
            # Goes from 10 to 0 once the civ gets 1/4 of all alive civs as friends
            motivation += 10 - 10 * num_of_friends / civs_to_ally_with
        else:
            # Goes from 0 to -120 as the civ gets more friends, offset by civsToAllyWith
            motivation -= 120.0 * (num_of_friends - civs_to_ally_with) / (known_civs - civs_to_ally_with)

        # The more friends they have the less we should want to sign friendship (To promote teams)
        motivation -= other_civ_number_of_friends * 10

        # Goes from 0 to -50 as more civs die
        # this is meant to prevent the game from stalemating when a group of friends
        # conquers all opposition
        motivation -= dead_civs / all_civs * 50

        # Become more desperate as we have more wars
        motivation += sum(1 for d in civ_info.diplomacy.values() 
                         if d.other_civ().is_major_civ() and d.diplomatic_status == DiplomaticStatus.War) * 10

        # Wait to declare friendships until more civs
        # Goes from -30 to 0 when we know 75% of allCivs
        civs_to_know = 0.75 * all_alive_civs
        motivation -= max(0, (civs_to_know - known_civs) / civs_to_know * 30.0)

        # If they are the only non-friendly civ near us then they are the only civ to attack and expand into
        if not any(c.is_major_civ() and c != other_civ
                  and civ_info.get_diplomacy_manager(c).is_relationship_level_lt(RelationshipLevel.Favorable)
                  for c in civ_info.threat_manager.get_neighboring_civilizations()):
            motivation -= 20

        motivation -= has_at_least_motivation_to_attack(civ_info, other_civ, motivation / 2.0) * 2

        return motivation > 0

    @staticmethod
    def offer_open_borders(civ_info: Civilization) -> None:
        """Offer open borders agreements to other civilizations."""
        if not civ_info.has_unique(UniqueType.EnablesOpenBorders):
            return

        civs_that_we_can_open_borders_with = sorted(
            [
                civ for civ in civ_info.get_known_civs()
                if (civ.is_major_civ() and not civ_info.is_at_war_with(civ)
                    and civ.has_unique(UniqueType.EnablesOpenBorders)
                    and not civ_info.get_diplomacy_manager(civ).has_open_borders
                    and not civ.get_diplomacy_manager(civ_info).has_open_borders
                    and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedOpenBorders)
                    and not DiplomacyAutomation._are_we_offering_trade(civ_info, civ, Constants.open_borders))
            ],
            key=lambda x: civ_info.get_diplomacy_manager(x).relationship_level(),
            reverse=True
        )

        for other_civ in civs_that_we_can_open_borders_with:
            # Default setting is 3, this will be changed according to different civ
            if random.randint(1, 10) < 7:
                continue
            if DiplomacyAutomation._wants_to_open_borders(civ_info, other_civ):
                trade_logic = TradeLogic(civ_info, other_civ)
                trade_logic.current_trade.our_offers.append(
                    TradeOffer(Constants.open_borders, TradeOfferType.Agreement, speed=civ_info.game_info.speed)
                )
                trade_logic.current_trade.their_offers.append(
                    TradeOffer(Constants.open_borders, TradeOfferType.Agreement, speed=civ_info.game_info.speed)
                )

                other_civ.trade_requests.append(
                    TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
                )
            else:
                # Remember this for a few turns to save computation power
                civ_info.get_diplomacy_manager(other_civ).set_flag(DiplomacyFlags.DeclinedOpenBorders, 5)

    @staticmethod
    def _wants_to_open_borders(civ_info: Civilization, other_civ: Civilization) -> bool:
        """Determine if we want to open borders with another civilization."""
        diplo_manager = civ_info.get_diplomacy_manager(other_civ)
        if diplo_manager.has_flag(DiplomacyFlags.DeclinedOpenBorders):
            return False
        if diplo_manager.is_relationship_level_lt(RelationshipLevel.Favorable):
            return False
        # Don't accept if they are at war with our friends, they might use our land to attack them
        if any(d.is_relationship_level_ge(RelationshipLevel.Friend) and d.other_civ().is_at_war_with(other_civ)
               for d in civ_info.diplomacy.values()):
            return False
        # Being able to see their cities can give us an advantage later on, especially with espionage enabled
        if other_civ.cities.count(lambda city: not city.get_center_tile().is_visible(civ_info)) < other_civ.cities.count() * 0.8:
            return True
        if has_at_least_motivation_to_attack(
            civ_info, other_civ,
            diplo_manager.opinion_of_other_civ() * civ_info.get_personality().modifier_focus(PersonalityValue.Commerce, 0.3) / 2
        ) > 0:
            return False
        return True

    @staticmethod
    def offer_research_agreement(civ_info: Civilization) -> None:
        """Offer research agreements to other civilizations."""
        if not civ_info.diplomacy_functions.can_sign_research_agreement():
            return  # don't waste your time

        can_sign_research_agreement_civ = sorted(
            [
                civ for civ in civ_info.get_known_civs()
                if (civ_info.diplomacy_functions.can_sign_research_agreements_with(civ)
                    and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedResearchAgreement)
                    and not DiplomacyAutomation._are_we_offering_trade(civ_info, civ, Constants.researchAgreement))
            ],
            key=lambda x: x.stats.stats_for_next_turn.science,
            reverse=True
        )

        for other_civ in can_sign_research_agreement_civ:
            # Default setting is 5, this will be changed according to different civ
            if random.randint(1, 10) <= 5 * civ_info.get_personality().modifier_focus(PersonalityValue.Science, 0.3):
                continue
            trade_logic = TradeLogic(civ_info, other_civ)
            cost = civ_info.diplomacy_functions.get_research_agreement_cost(other_civ)
            trade_logic.current_trade.our_offers.append(
                TradeOffer(Constants.researchAgreement, TradeOfferType.Treaty, cost, civ_info.game_info.speed)
            )
            trade_logic.current_trade.their_offers.append(
                TradeOffer(Constants.researchAgreement, TradeOfferType.Treaty, cost, civ_info.game_info.speed)
            )

            other_civ.trade_requests.append(
                TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
            )

    @staticmethod
    def offer_defensive_pact(civ_info: Civilization) -> None:
        """Offer defensive pacts to other civilizations."""
        if not civ_info.diplomacy_functions.can_sign_defensive_pact():
            return  # don't waste your time

        can_sign_defensive_pact_civ = [
            civ for civ in civ_info.get_known_civs()
            if (civ_info.diplomacy_functions.can_sign_defensive_pact_with(civ)
                and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedDefensivePact)
                and civ_info.get_diplomacy_manager(civ).opinion_of_other_civ() < 70.0 * civ_info.get_personality().inverse_modifier_focus(PersonalityValue.Aggressive, 0.2)
                and not DiplomacyAutomation._are_we_offering_trade(civ_info, civ, Constants.defensivePact))
        ]

        for other_civ in can_sign_defensive_pact_civ:
            # Default setting is 3, this will be changed according to different civ
            if random.randint(1, 10) <= 7 * civ_info.get_personality().inverse_modifier_focus(PersonalityValue.Loyal, 0.3):
                continue
            if DiplomacyAutomation._wants_to_sign_defensive_pact(civ_info, other_civ):
                # TODO: Add more in depth evaluation here
                trade_logic = TradeLogic(civ_info, other_civ)
                trade_logic.current_trade.our_offers.append(
                    TradeOffer(Constants.defensivePact, TradeOfferType.Treaty, speed=civ_info.game_info.speed)
                )
                trade_logic.current_trade.their_offers.append(
                    TradeOffer(Constants.defensivePact, TradeOfferType.Treaty, speed=civ_info.game_info.speed)
                )

                other_civ.trade_requests.append(
                    TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
                )
            else:
                # Remember this for a few turns to save computation power
                civ_info.get_diplomacy_manager(other_civ).set_flag(DiplomacyFlags.DeclinedDefensivePact, 5)

    @staticmethod
    def _wants_to_sign_defensive_pact(civ_info: Civilization, other_civ: Civilization) -> bool:
        """Determine if we want to sign a defensive pact with another civilization."""
        diplo_manager = civ_info.get_diplomacy_manager(other_civ)
        if diplo_manager.has_flag(DiplomacyFlags.DeclinedDefensivePact):
            return False
        if diplo_manager.opinion_of_other_civ() < 65.0 * civ_info.get_personality().inverse_modifier_focus(PersonalityValue.Aggressive, 0.3):
            return False

        common_known_civs = diplo_manager.get_common_known_civs()
        for third_civ in common_known_civs:
            # If they have bad relations with any of our friends, don't consider it
            if (civ_info.get_diplomacy_manager(third_civ).has_flag(DiplomacyFlags.DeclarationOfFriendship)
                and third_civ.get_diplomacy_manager(other_civ).is_relationship_level_lt(RelationshipLevel.Favorable)):
                return False
            
            # If they have bad relations with any of our friends, don't consider it
            if (other_civ.get_diplomacy_manager(third_civ).has_flag(DiplomacyFlags.DeclarationOfFriendship)
                and third_civ.get_diplomacy_manager(civ_info).is_relationship_level_lt(RelationshipLevel.Neutral)):
                return False

        defensive_pacts = sum(1 for d in civ_info.diplomacy.values() 
                            if d.has_flag(DiplomacyFlags.DefensivePact))
        other_civ_non_overlapping_defensive_pacts = sum(1 for d in other_civ.diplomacy.values()
                                                      if d.has_flag(DiplomacyFlags.DefensivePact)
                                                      and not d.other_civ().get_diplomacy_manager(civ_info).has_flag(DiplomacyFlags.DefensivePact))
        all_civs = sum(1 for c in civ_info.game_info.civilizations 
                      if c.is_major_civ()) - 1  # Don't include us
        dead_civs = sum(1 for c in civ_info.game_info.civilizations 
                       if c.is_major_civ() and not c.is_alive())
        all_alive_civs = all_civs - dead_civs

        # We have to already be at RelationshipLevel.Ally, so we must have 80 opinion of them
        motivation = diplo_manager.opinion_of_other_civ() - 80.0

        # Warmongerers don't make good allies
        if diplo_manager.has_modifier(DiplomaticModifiers.WarMongerer):
            motivation -= (diplo_manager.get_modifier(DiplomaticModifiers.WarMongerer) * 
                         civ_info.get_personality().modifier_focus(PersonalityValue.Diplomacy, 0.5))

        # If they are stronger than us, then we value it a lot more
        # If they are weaker than us, then we don't value it
        threat_level = Automation.threat_assessment(civ_info, other_civ)
        motivation += {
            ThreatLevel.VeryHigh: 10,
            ThreatLevel.High: 5,
            ThreatLevel.Low: -3,
            ThreatLevel.VeryLow: -7,
            ThreatLevel.Neutral: 0
        }[threat_level]

        # If they have a defensive pact with another civ then we would get drawn into their battles as well
        motivation -= 15 * other_civ_non_overlapping_defensive_pacts

        # Become more desperate as we have more wars
        motivation += sum(1 for d in civ_info.diplomacy.values() 
                         if d.other_civ().is_major_civ() and d.diplomatic_status == DiplomaticStatus.War) * 5

        # Try to have a defensive pact with 1/5 of all civs
        civs_to_ally_with = 0.20 * all_alive_civs * civ_info.get_personality().modifier_focus(PersonalityValue.Diplomacy, 0.5)
        # Goes from 0 to -40 as the civ gets more allies, offset by civsToAllyWith
        motivation -= min(0, 40.0 * (defensive_pacts - civs_to_ally_with) / (all_alive_civs - civs_to_ally_with))

        return motivation > 0

    @staticmethod
    def declare_war(civ_info: Civilization) -> None:
        """Evaluate and potentially declare war on other civilizations."""
        if (civ_info.cities.is_empty() or civ_info.diplomacy.is_empty() or
            civ_info.get_personality()[PersonalityValue.DeclareWar] == 0.0 or
            civ_info.get_happiness() <= 0):
            return

        our_military_units = sum(1 for unit in civ_info.units.get_civ_units() 
                               if not unit.is_civilian())
        if (our_military_units < civ_info.cities.size or
            our_military_units < 4 or  # to stop AI declaring war at the beginning of games
            civ_info.cities.sum(lambda city: city.population.population) < 12):  # FAR too early for that
            return

        # evaluate war
        target_civs = [
            civ for civ in civ_info.get_known_civs()
            if (not civ.is_defeated() and civ != civ_info and not civ.cities.is_empty()
                and civ_info.get_diplomacy_manager(civ).can_declare_war()
                and any(civ_info.has_explored(city.get_center_tile()) 
                       for city in civ.cities))
        ]

        if not target_civs:
            return

        target_civs_with_motivation = [
            (civ, has_at_least_motivation_to_attack(civ_info, civ, 0.0))
            for civ in target_civs
            if has_at_least_motivation_to_attack(civ_info, civ, 0.0) > 0
        ]

        DeclareWarTargetAutomation.choose_declare_war_target(civ_info, target_civs_with_motivation)

    @staticmethod
    def offer_peace_treaty(civ_info: Civilization) -> None:
        """Offer peace treaties to enemy civilizations."""
        if (not civ_info.is_at_war() or civ_info.cities.is_empty() or 
            civ_info.diplomacy.is_empty()):
            return

        enemies_civ = [
            civ for civ in civ_info.diplomacy.items()
            if (civ[1].diplomatic_status == DiplomaticStatus.War and
                civ[0] != civ_info and not civ[0].is_barbarian and not civ[0].cities.is_empty()
                and not civ[1].has_flag(DiplomacyFlags.DeclaredWar)
                and not civ_info.get_diplomacy_manager(civ[0]).has_flag(DiplomacyFlags.DeclaredWar)
                and not civ_info.get_diplomacy_manager(civ[0]).has_flag(DiplomacyFlags.DeclinedPeace)
                and not (civ[0].is_city_state and civ[0].get_ally_civ() and 
                        civ_info.is_at_war_with(civ_info.game_info.get_civilization(civ[0].get_ally_civ())))
                and not any(trade_request.requesting_civ == civ_info.civ_name and 
                           trade_request.trade.is_peace_treaty() 
                           for trade_request in civ[0].trade_requests))
        ]

        for enemy in enemies_civ:
            if has_at_least_motivation_to_attack(civ_info, enemy, 10.0) >= 10:
                # We can still fight. Refuse peace.
                continue

            if (civ_info.get_stat_for_ranking(RankingType.Force) - 
                0.8 * civ_info.threat_manager.get_combined_force_of_warring_civs() > 0):
                random_seed = (civ_info.game_info.civilizations.index(enemy) + 
                             civ_info.get_civs_at_war_with().count() + 
                             123 * civ_info.game_info.turns)
                if random.Random(random_seed).randint(0, 99) > 80:
                    continue

            # pay for peace
            trade_logic = TradeLogic(civ_info, enemy)

            trade_logic.current_trade.our_offers.append(
                TradeOffer(Constants.peaceTreaty, TradeOfferType.Treaty, speed=civ_info.game_info.speed)
            )
            trade_logic.current_trade.their_offers.append(
                TradeOffer(Constants.peaceTreaty, TradeOfferType.Treaty, speed=civ_info.game_info.speed)
            )

            if enemy.is_major_civ():
                money_we_need_to_pay = -TradeEvaluation().evaluate_peace_cost_for_them(civ_info, enemy)

                if civ_info.gold > 0 and money_we_need_to_pay > 0:
                    if money_we_need_to_pay > civ_info.gold:
                        money_we_need_to_pay = civ_info.gold  # As much as possible
                    trade_logic.current_trade.our_offers.append(
                        TradeOffer(tr("Gold"), TradeOfferType.Gold, money_we_need_to_pay, civ_info.game_info.speed)
                    )
                elif money_we_need_to_pay < -100:
                    money_they_need_to_pay = min(abs(money_we_need_to_pay), enemy.gold)
                    if money_they_need_to_pay > 0:
                        trade_logic.current_trade.their_offers.append(
                            TradeOffer(tr("Gold"), TradeOfferType.Gold, money_they_need_to_pay, civ_info.game_info.speed)
                        )

            enemy.trade_requests.append(
                TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
            )

    @staticmethod
    def ask_for_help(civ_info: Civilization) -> None:
        """Ask other civilizations for help in ongoing wars."""
        if not civ_info.is_at_war() or civ_info.cities.is_empty() or civ_info.diplomacy.is_empty():
            return

        enemy_civs = sorted(
            [civ for civ in civ_info.get_civs_at_war_with() if civ.is_major_civ()],
            key=lambda x: x.get_stat_for_ranking(RankingType.Force),
            reverse=True
        )

        for enemy_civ in enemy_civs:
            potential_allies = sorted(
                [
                    civ for civ in enemy_civ.threat_manager.get_neighboring_civilizations()
                    if (civ_info.knows(civ) and not civ.is_at_war_with(enemy_civ)
                        and civ_info.get_diplomacy_manager(civ).is_relationship_level_ge(RelationshipLevel.Friend)
                        and not civ.get_diplomacy_manager(civ_info).has_flag(DiplomacyFlags.DeclinedJoinWarOffer))
                ],
                key=lambda x: x.get_stat_for_ranking(RankingType.Force),
                reverse=True
            )

            civ_to_ask = next(
                (civ for civ in potential_allies 
                 if DeclareWarPlanEvaluator.evaluate_join_our_war_plan(civ_info, enemy_civ, civ, None) > 0),
                None
            )
            if not civ_to_ask:
                continue

            trade_logic = TradeLogic(civ_info, civ_to_ask)
            # TODO: add gold offer here
            trade_logic.current_trade.their_offers.append(
                TradeOffer(enemy_civ.civ_name, TradeOfferType.WarDeclaration, speed=civ_info.game_info.speed)
            )
            civ_to_ask.trade_requests.append(
                TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
            )

    @staticmethod
    def _are_we_offering_trade(civ_info: Civilization, other_civ: Civilization, offer_name: str) -> bool:
        """Check if we are already offering a specific trade to another civilization."""
        return any(
            offer.name == offer_name
            for request in other_civ.trade_requests
            if request.requesting_civ == civ_info.civ_name
            for offer in request.trade.our_offers + request.trade.their_offers
        ) 