from typing import List, Tuple

from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags, RelationshipLevel
from com.unciv.logic.trade import TradeLogic, TradeOffer, TradeRequest, TradeOfferType
from com.unciv.ui.screens.victoryscreen import RankingType
from com.unciv.logic.automation.civilization.declare_war_plan_evaluator import DeclareWarPlanEvaluator

class DeclareWarTargetAutomation:
    """Contains logic for choosing war targets and executing war plans."""
    
    @staticmethod
    def choose_declare_war_target(
        civ_info: Civilization,
        civ_attack_motivations: List[Tuple[Civilization, float]]
    ) -> None:
        """Choose a target civilization along with a plan of attack.
        
        Note that this doesn't guarantee that we will declare war on them immediately,
        or that we will end up declaring war at all.
        
        Args:
            civ_info: The civilization choosing the target
            civ_attack_motivations: List of tuples containing (target_civ, motivation)
        """
        # Sort targets by score in descending order
        highest_value_targets = sorted(
            civ_attack_motivations,
            key=lambda x: x[0].get_stat_for_ranking(RankingType.Score),
            reverse=True
        )

        for target, motivation in highest_value_targets:
            if DeclareWarTargetAutomation._try_declare_war_with_plan(civ_info, target, motivation):
                return  # We have successfully found a plan and started executing it!

    @staticmethod
    def _try_declare_war_with_plan(
        civ_info: Civilization,
        target: Civilization,
        motivation: float
    ) -> bool:
        """Determine a war plan against this target and execute it if able.
        
        Args:
            civ_info: The civilization planning the war
            target: The civilization to declare war against
            motivation: The motivation to declare war
            
        Returns:
            True if a plan was successfully executed, False otherwise
        """
        if not target.is_city_state:
            if motivation > 5 and DeclareWarTargetAutomation._try_team_war(civ_info, target, motivation):
                return True

            if motivation >= 15 and DeclareWarTargetAutomation._try_join_war(civ_info, target, motivation):
                return True

        if motivation >= 20 and DeclareWarTargetAutomation._declare_war(civ_info, target, motivation):
            return True

        if motivation >= 15 and DeclareWarTargetAutomation._prepare_war(civ_info, target, motivation):
            return True

        return False

    @staticmethod
    def _try_team_war(
        civ_info: Civilization,
        target: Civilization,
        motivation: float
    ) -> bool:
        """Attempt to form a team war against the target.
        
        The safest option for war is to invite a new ally to join the war with us.
        Together we are stronger and are more likely to take down bigger threats.
        
        Args:
            civ_info: The civilization initiating the team war
            target: The civilization to declare war against
            motivation: The motivation to declare war
            
        Returns:
            True if a team war was successfully initiated, False otherwise
        """
        potential_allies = sorted(
            [
                civ for civ in civ_info.get_diplomacy_manager(target).get_common_known_civs()
                if (civ.is_major_civ()
                    and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedJoinWarOffer)
                    and civ_info.get_diplomacy_manager(civ).is_relationship_level_ge(RelationshipLevel.Neutral)
                    and not civ.is_at_war_with(target))
            ],
            key=lambda x: x.get_stat_for_ranking(RankingType.Force),
            reverse=True
        )

        for third_civ in potential_allies:
            if DeclareWarPlanEvaluator.evaluate_team_war_plan(civ_info, target, third_civ, motivation) <= 0:
                continue

            # Send them an offer
            trade_logic = TradeLogic(civ_info, third_civ)
            trade_logic.current_trade.our_offers.append(
                TradeOffer(target.civ_name, TradeOfferType.WarDeclaration, speed=civ_info.game_info.speed)
            )
            trade_logic.current_trade.their_offers.append(
                TradeOffer(target.civ_name, TradeOfferType.WarDeclaration, speed=civ_info.game_info.speed)
            )

            third_civ.trade_requests.append(
                TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
            )

            return True

        return False

    @staticmethod
    def _try_join_war(
        civ_info: Civilization,
        target: Civilization,
        motivation: float
    ) -> bool:
        """Attempt to join an existing war against the target.
        
        The next safest approach is to join an existing war on the side of an ally
        that is already at war with the target.
        
        Args:
            civ_info: The civilization joining the war
            target: The civilization to declare war against
            motivation: The motivation to declare war
            
        Returns:
            True if successfully joined an existing war, False otherwise
        """
        potential_allies = sorted(
            [
                civ for civ in civ_info.get_diplomacy_manager(target).get_common_known_civs()
                if (civ.is_major_civ()
                    and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedJoinWarOffer)
                    and civ_info.get_diplomacy_manager(civ).is_relationship_level_ge(RelationshipLevel.Favorable)
                    and civ.is_at_war_with(target))
            ],
            key=lambda x: x.get_stat_for_ranking(RankingType.Force),
            reverse=True
        )

        for third_civ in potential_allies:
            if DeclareWarPlanEvaluator.evaluate_join_war_plan(civ_info, target, third_civ, motivation) <= 0:
                continue

            # Send them an offer
            trade_logic = TradeLogic(civ_info, third_civ)
            trade_logic.current_trade.our_offers.append(
                TradeOffer(target.civ_name, TradeOfferType.WarDeclaration, speed=civ_info.game_info.speed)
            )
            # TODO: Maybe add in payment requests in some situations
            third_civ.trade_requests.append(
                TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())
            )

            return True

        return False

    @staticmethod
    def _declare_war(
        civ_info: Civilization,
        target: Civilization,
        motivation: float
    ) -> bool:
        """Declare war directly against the target.
        
        If our motivation is high enough and we don't have any better plans then
        we just declare war.
        
        Args:
            civ_info: The civilization declaring war
            target: The civilization to declare war against
            motivation: The motivation to declare war
            
        Returns:
            True if war was successfully declared, False otherwise
        """
        if DeclareWarPlanEvaluator.evaluate_declare_war_plan(civ_info, target, motivation) > 0:
            civ_info.get_diplomacy_manager(target).declare_war()
            return True
        return False

    @staticmethod
    def _prepare_war(
        civ_info: Civilization,
        target: Civilization,
        motivation: float
    ) -> bool:
        """Start preparing for war against the target.
        
        Slightly safer is to silently plan an invasion and declare war later.
        
        Args:
            civ_info: The civilization preparing for war
            target: The civilization to prepare war against
            motivation: The motivation to declare war
            
        Returns:
            True if war preparation was successfully initiated, False otherwise
        """
        # TODO: We use negative values in WaryOf for now so that we aren't adding any extra fields to the save file
        # This will very likely change in the future and we will want to build upon it
        diplo_manager = civ_info.get_diplomacy_manager(target)
        if DeclareWarPlanEvaluator.evaluate_start_preparing_war_plan(civ_info, target, motivation) > 0:
            diplo_manager.set_flag(DiplomacyFlags.WaryOf, -1)
            return True
        return False 