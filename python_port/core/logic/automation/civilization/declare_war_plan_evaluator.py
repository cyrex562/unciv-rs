from typing import Optional

from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.diplomacy import (
    DiplomacyFlags, DiplomaticStatus, RelationshipLevel
)
from com.unciv.models.ruleset.nation import PersonalityValue
from com.unciv.ui.screens.victoryscreen import RankingType
from com.unciv.logic.automation import MotivationToAttackAutomation

class DeclareWarPlanEvaluator:
    """Contains the logic for evaluating how we want to declare war on another civ."""
    
    @staticmethod
    def evaluate_team_war_plan(
        civ_info: Civilization,
        target: Civilization,
        team_civ: Civilization,
        given_motivation: Optional[float] = None
    ) -> float:
        """Evaluate motivation for a team war against a target.
        
        This style of declaring war favors fighting stronger civilizations.
        
        Args:
            civ_info: The civilization evaluating the war plan
            target: The civilization to declare war against
            team_civ: The civilization to team up with
            given_motivation: Optional pre-calculated motivation value
            
        Returns:
            The motivation of the plan. If > 0 then we can declare the war.
        """
        team_civ_diplo = civ_info.get_diplomacy_manager(team_civ)
        if civ_info.get_personality()[PersonalityValue.DeclareWar] == 0:
            return -1000.0
        if team_civ_diplo.is_relationship_level_lt(RelationshipLevel.Neutral):
            return -1000.0

        motivation = (given_motivation or 
                     MotivationToAttackAutomation.has_at_least_motivation_to_attack(
                         civ_info, target, 0.0))

        if team_civ_diplo.is_relationship_level_eq(RelationshipLevel.Neutral):
            motivation -= 5.0
            
        # Make sure that they can actually help us with the target
        if target not in team_civ.threat_manager.get_neighboring_civilizations():
            motivation -= 40.0

        civ_force = civ_info.get_stat_for_ranking(RankingType.Force)
        target_force = target.get_stat_for_ranking(RankingType.Force)
        team_civ_force = max(
            100.0,
            team_civ.get_stat_for_ranking(RankingType.Force) - 
            0.8 * team_civ.threat_manager.get_combined_force_of_warring_civs()
        )

        # A higher motivation means that we can be riskier
        multiplier = {
            motivation < 5: 1.2,
            motivation < 10: 1.1,
            motivation < 20: 1.0,
            True: 0.8
        }[True]

        if civ_force + team_civ_force < target_force * multiplier:
            # We are weaker then them even with our combined forces
            # If they have twice our combined force we will have -30 motivation
            motivation -= 30 * ((target_force * multiplier) / (team_civ_force + civ_force) - 1)
        elif civ_force + team_civ_force > target_force * 2:
            # Why gang up on such a weaker enemy when we can declare war ourselves?
            # If our combined force is twice their force we will have -20 motivation
            motivation -= 20 * ((civ_force + team_civ_force) / target_force * 2 - 1)

        civ_score = civ_info.get_stat_for_ranking(RankingType.Score)
        team_civ_score = team_civ.get_stat_for_ranking(RankingType.Score)
        target_civ_score = target.get_stat_for_ranking(RankingType.Score)

        if team_civ_score > civ_score * 1.4 and team_civ_score >= target_civ_score:
            # If teamCiv has more score than us and the target they are likely in a good position already
            motivation -= 20 * ((team_civ_score / (civ_score * 1.4)) - 1)
            
        return motivation - 20.0

    @staticmethod
    def evaluate_join_war_plan(
        civ_info: Civilization,
        target: Civilization,
        civ_to_join: Civilization,
        given_motivation: Optional[float] = None
    ) -> float:
        """Evaluate motivation for joining a civilization in their war against a target.
        
        Favors protecting allies.
        
        Args:
            civ_info: The civilization evaluating the war plan
            target: The civilization to declare war against
            civ_to_join: The civilization to join in war
            given_motivation: Optional pre-calculated motivation value
            
        Returns:
            The motivation of the plan. If > 0 then we can declare the war.
        """
        third_civ_diplo = civ_info.get_diplomacy_manager(civ_to_join)
        if civ_info.get_personality()[PersonalityValue.DeclareWar] == 0:
            return -1000.0
        if third_civ_diplo.is_relationship_level_le(RelationshipLevel.Favorable):
            return -1000.0

        motivation = (given_motivation or 
                     MotivationToAttackAutomation.has_at_least_motivation_to_attack(
                         civ_info, target, 0.0))

        # We need to be able to trust the thirdCiv at least somewhat
        if (third_civ_diplo.diplomatic_status != DiplomaticStatus.DefensivePact and
            third_civ_diplo.opinion_of_other_civ() + motivation * 2 < 80):
            motivation -= 80 - third_civ_diplo.opinion_of_other_civ() + motivation * 2

        if target not in civ_to_join.threat_manager.get_neighboring_civilizations():
            motivation -= 20.0

        target_force = max(
            100.0,
            target.get_stat_for_ranking(RankingType.Force) - 
            0.8 * sum(civ.get_stat_for_ranking(RankingType.Force) 
                     for civ in target.get_civs_at_war_with())
        )
        civ_force = civ_info.get_stat_for_ranking(RankingType.Force)

        # They need to be at least half the targets size, and we need to be stronger than the target together
        civ_to_join_force = max(
            100.0,
            civ_to_join.get_stat_for_ranking(RankingType.Force) - 
            0.8 * sum(civ.get_stat_for_ranking(RankingType.Force) 
                     for civ in civ_to_join.get_civs_at_war_with())
        )

        if civ_to_join_force < target_force / 2:
            # Make sure that there is no wrap around
            motivation -= 10 * max(min(target_force / civ_to_join_force, 1000.0), -1000.0)

        # A higher motivation means that we can be riskier
        multiplier = {
            motivation < 10: 1.4,
            motivation < 15: 1.3,
            motivation < 20: 1.2,
            motivation < 25: 1.0,
            True: 0.8
        }[True]

        if civ_to_join_force + civ_force < target_force * multiplier:
            motivation -= 20 * max(min(
                (target_force * multiplier) / (civ_to_join_force + civ_force),
                1000.0
            ), -1000.0)

        return motivation - 15.0

    @staticmethod
    def evaluate_join_our_war_plan(
        civ_info: Civilization,
        target: Civilization,
        civ_to_join: Civilization,
        given_motivation: Optional[float] = None
    ) -> float:
        """Evaluate motivation for having another civilization join our war.
        
        Args:
            civ_info: The civilization evaluating the war plan
            target: The civilization to declare war against
            civ_to_join: The civilization to invite to war
            given_motivation: Optional pre-calculated motivation value
            
        Returns:
            The motivation of the plan. If >= 0 then we can accept their war offer.
        """
        if civ_info.get_diplomacy_manager(civ_to_join).is_relationship_level_lt(RelationshipLevel.Favorable):
            return -1000.0
            
        motivation = given_motivation or 0.0
        
        if target not in civ_to_join.threat_manager.get_neighboring_civilizations():
            motivation -= 50.0

        target_force = target.get_stat_for_ranking(RankingType.Force)
        civ_force = civ_info.get_stat_for_ranking(RankingType.Force)

        # If we have more force than all enemies and overpower this enemy then we don't need help
        if (civ_force - civ_info.threat_manager.get_combined_force_of_warring_civs() > 
            target_force * 2):
            return 0.0

        # They should to be at least half the targets size
        third_civ_force = max(
            100.0,
            civ_to_join.get_stat_for_ranking(RankingType.Force) - 
            0.8 * sum(civ.get_stat_for_ranking(RankingType.Force) 
                     for civ in civ_to_join.get_civs_at_war_with())
        )
        motivation += min(40.0, 20 * (1 - third_civ_force / target_force))

        # If we have less relative force then the target then we have more motivation to accept
        motivation += max(min(40.0, 20 * (1 - civ_force / target_force)), -40.0)

        return motivation - 20.0

    @staticmethod
    def evaluate_declare_war_plan(
        civ_info: Civilization,
        target: Civilization,
        given_motivation: Optional[float] = None
    ) -> float:
        """Evaluate motivation for declaring war against a target.
        
        This can be through a prepared war or a surprise war.
        
        Args:
            civ_info: The civilization evaluating the war plan
            target: The civilization to declare war against
            given_motivation: Optional pre-calculated motivation value
            
        Returns:
            The motivation of the plan. If > 0 then we can declare the war.
        """
        if civ_info.get_personality()[PersonalityValue.DeclareWar] == 0:
            return -1000.0
            
        motivation = (given_motivation or 
                     MotivationToAttackAutomation.has_at_least_motivation_to_attack(
                         civ_info, target, 0.0))

        diplo_manager = civ_info.get_diplomacy_manager(target)

        if diplo_manager.has_flag(DiplomacyFlags.WaryOf):
            wary_value = diplo_manager.get_flag(DiplomacyFlags.WaryOf)
            if wary_value < 0:
                turns_to_plan = max(3.0, 10 - (motivation / 10))
                turns_to_wait = turns_to_plan + wary_value
                return motivation - turns_to_wait * 3

        return motivation - 40.0

    @staticmethod
    def evaluate_start_preparing_war_plan(
        civ_info: Civilization,
        target: Civilization,
        given_motivation: Optional[float] = None
    ) -> float:
        """Evaluate motivation for starting to prepare for war against a target.
        
        Args:
            civ_info: The civilization evaluating the war plan
            target: The civilization to prepare war against
            given_motivation: Optional pre-calculated motivation value
            
        Returns:
            The motivation of the plan. If > 0 then we can start planning the war.
        """
        motivation = (given_motivation or 
                     MotivationToAttackAutomation.has_at_least_motivation_to_attack(
                         civ_info, target, 0.0))

        # TODO: We use negative values in WaryOf for now so that we aren't adding any extra fields to the save file
        # This will very likely change in the future and we will want to build upon it
        diplo_manager = civ_info.get_diplomacy_manager(target)
        if diplo_manager.has_flag(DiplomacyFlags.WaryOf):
            return 0.0

        return motivation - 15.0 