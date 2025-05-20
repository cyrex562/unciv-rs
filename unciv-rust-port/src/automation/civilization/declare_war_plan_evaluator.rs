use crate::{automation::civilization::motivation_to_attack_automation::MotivationToAttackAutomation, ranking_type::RankingType};
use crate::civilization::civilization::Civilization;
use crate::ai::personality::PersonalityValue;
use crate::diplomacy::flags::DiplomacyFlags;

use crate::diplomacy::status::DiplomaticStatus;
use crate::diplomacy::relationship_level::RelationshipLevel;
use serde::{Serialize, Deserialize};

/// Contains the logic for evaluating how we want to declare war on another civ.
pub struct DeclareWarPlanEvaluator;

impl DeclareWarPlanEvaluator {
    /// How much motivation [civ_info] has to do a team war with [team_civ] against [target].
    ///
    /// This style of declaring war favors fighting stronger civilizations.
    /// Returns the motivation of the plan. If it is > 0 then we can declare the war.
    pub fn evaluate_team_war_plan(
        civ_info: &Civilization,
        target: &Civilization,
        team_civ: &Civilization,
        given_motivation: Option<f32>,
    ) -> f32 {
        let team_civ_diplo = civ_info.get_diplomacy_manager(team_civ).unwrap();
        if civ_info.get_personality()[PersonalityValue::DeclareWar] == 0.0 {
            return -1000.0;
        }
        if team_civ_diplo.is_relationship_level_lt(RelationshipLevel::Neutral) {
            return -1000.0;
        }

        let motivation = given_motivation.unwrap_or_else(|| {
            MotivationToAttackAutomation::has_at_least_motivation_to_attack(civ_info, target, 0.0)
        });

        let mut motivation = motivation;

        if team_civ_diplo.is_relationship_level_eq(RelationshipLevel::Neutral) {
            motivation -= 5.0;
        }
        // Make sure that they can actually help us with the target
        if !team_civ.threat_manager.get_neighboring_civilizations().contains(target) {
            motivation -= 40.0;
        }

        let civ_force = civ_info.get_stat_for_ranking(RankingType::Force);
        let target_force = target.get_stat_for_ranking(RankingType::Force);
        let team_civ_force = (team_civ.get_stat_for_ranking(RankingType::Force)
            - 0.8 * team_civ.threat_manager.get_combined_force_of_warring_civs())
            .max(100.0);

        // A higher motivation means that we can be riskier
        let multiplier = if motivation < 5.0 {
            1.2
        } else if motivation < 10.0 {
            1.1
        } else if motivation < 20.0 {
            1.0
        } else {
            0.8
        };

        if civ_force + team_civ_force < target_force * multiplier {
            // We are weaker then them even with our combined forces
            // If they have twice our combined force we will have -30 motivation
            motivation -= 30.0 * ((target_force * multiplier) / (team_civ_force + civ_force) - 1.0);
        } else if civ_force + team_civ_force > target_force * 2.0 {
            // Why gang up on such a weaker enemy when we can declare war ourselves?
            // If our combined force is twice their force we will have -20 motivation
            motivation -= 20.0 * ((civ_force + team_civ_force) / target_force * 2.0 - 1.0);
        }

        let civ_score = civ_info.get_stat_for_ranking(RankingType::Score);
        let team_civ_score = team_civ.get_stat_for_ranking(RankingType::Score);
        let target_civ_score = target.get_stat_for_ranking(RankingType::Score);

        if team_civ_score > civ_score * 1.4 && team_civ_score >= target_civ_score {
            // If teamCiv has more score than us and the target they are likely in a good position already
            motivation -= 20.0 * ((team_civ_score / (civ_score * 1.4)) - 1.0);
        }
        motivation - 20.0
    }

    /// How much motivation [civ_info] has to join [civ_to_join] in their war against [target].
    ///
    /// Favors protecting allies.
    /// Returns the motivation of the plan. If it is > 0 then we can declare the war.
    pub fn evaluate_join_war_plan(
        civ_info: &Civilization,
        target: &Civilization,
        civ_to_join: &Civilization,
        given_motivation: Option<f32>,
    ) -> f32 {
        let third_civ_diplo = civ_info.get_diplomacy_manager(civ_to_join).unwrap();
        if civ_info.get_personality()[PersonalityValue::DeclareWar] == 0.0 {
            return -1000.0;
        }
        if third_civ_diplo.is_relationship_level_le(RelationshipLevel::Favorable) {
            return -1000.0;
        }

        let motivation = given_motivation.unwrap_or_else(|| {
            MotivationToAttackAutomation::has_at_least_motivation_to_attack(civ_info, target, 0.0)
        });

        let mut motivation = motivation;
        // We need to be able to trust the thirdCiv at least somewhat
        if !third_civ_diplo.has_defensive_pact()
            && third_civ_diplo.opinion_of_other_civ() + motivation * 2.0 < 80.0
        {
            motivation -= 80.0 - third_civ_diplo.opinion_of_other_civ() + motivation * 2.0;
        }
        if !civ_to_join.threat_manager.get_neighboring_civilizations().contains(target) {
            motivation -= 20.0;
        }

        let target_force = (target.get_stat_for_ranking(RankingType::Force)
            - 0.8 * target
                .get_civs_at_war_with()
                .iter()
                .map(|civ| civ.get_stat_for_ranking(RankingType::Force))
                .sum::<f32>())
            .max(100.0);
        let civ_force = civ_info.get_stat_for_ranking(RankingType::Force);

        // They need to be at least half the targets size, and we need to be stronger than the target together
        let civ_to_join_force = (civ_to_join.get_stat_for_ranking(RankingType::Force)
            - 0.8 * civ_to_join
                .get_civs_at_war_with()
                .iter()
                .map(|civ| civ.get_stat_for_ranking(RankingType::Force))
                .sum::<f32>())
            .max(100.0);
        if civ_to_join_force < target_force / 2.0 {
            // Make sure that there is no wrap around
            motivation -= 10.0 * (target_force / civ_to_join_force).clamp(-1000.0, 1000.0);
        }

        // A higher motivation means that we can be riskier
        let multiplier = if motivation < 10.0 {
            1.4
        } else if motivation < 15.0 {
            1.3
        } else if motivation < 20.0 {
            1.2
        } else if motivation < 25.0 {
            1.0
        } else {
            0.8
        };

        if civ_to_join_force + civ_force < target_force * multiplier {
            motivation -= 20.0
                * (target_force * multiplier)
                .clamp(-1000.0, 1000.0)
                / (civ_to_join_force + civ_force);
        }

        motivation - 15.0
    }

    /// How much motivation [civ_info] has for [civ_to_join] to join them in their war against [target].
    ///
    /// Returns the motivation of the plan. If it is >= 0 then we can accept their war offer.
    pub fn evaluate_join_our_war_plan(
        civ_info: &Civilization,
        target: &Civilization,
        civ_to_join: &Civilization,
        given_motivation: Option<f32>,
    ) -> f32 {
        if civ_info
            .get_diplomacy_manager(civ_to_join)
            .unwrap()
            .is_relationship_level_lt(RelationshipLevel::Favorable)
        {
            return -1000.0;
        }
        let mut motivation = given_motivation.unwrap_or(0.0);
        if !civ_to_join
            .threat_manager
            .get_neighboring_civilizations()
            .contains(target)
        {
            motivation -= 50.0;
        }

        let target_force = target.get_stat_for_ranking(RankingType::Force);
        let civ_force = civ_info.get_stat_for_ranking(RankingType::Force);

        // If we have more force than all enemies and overpower this enemy then we don't need help
        if civ_force - civ_info.threat_manager.get_combined_force_of_warring_civs() > target_force * 2.0
        {
            return 0.0;
        }

        // They should to be at least half the targets size
        let third_civ_force = (civ_to_join.get_stat_for_ranking(RankingType::Force)
            - 0.8 * civ_to_join
                .get_civs_at_war_with()
                .iter()
                .map(|civ| civ.get_stat_for_ranking(RankingType::Force))
                .sum::<f32>())
            .max(100.0);
        motivation += (20.0 * (1.0 - third_civ_force / target_force)).min(40.0);

        // If we have less relative force then the target then we have more motivation to accept
        motivation += 20.0 * (1.0 - civ_force / target_force).clamp(-40.0, 40.0);

        motivation - 20.0
    }

    /// How much motivation [civ_info] has to declare war against [target] this turn.
    /// This can be through a prepared war or a surprise war.
    ///
    /// Returns the motivation of the plan. If it is > 0 then we can declare the war.
    pub fn evaluate_declare_war_plan(
        civ_info: &Civilization,
        target: &Civilization,
        given_motivation: Option<f32>,
    ) -> f32 {
        if civ_info.get_personality()[PersonalityValue::DeclareWar] == 0.0 {
            return -1000.0;
        }
        let motivation = given_motivation.unwrap_or_else(|| {
            MotivationToAttackAutomation::has_at_least_motivation_to_attack(civ_info, target, 0.0)
        });

        let diplo_manager = civ_info.get_diplomacy_manager(target).unwrap();

        if diplo_manager.has_flag(DiplomacyFlags::WaryOf) && diplo_manager.get_flag(DiplomacyFlags::WaryOf) < 0 {
            let turns_to_plan = (10.0 - (motivation / 10.0)).max(3.0);
            let turns_to_wait = turns_to_plan + diplo_manager.get_flag(DiplomacyFlags::WaryOf) as f32;
            return motivation - turns_to_wait * 3.0;
        }

        motivation - 40.0
    }

    /// How much motivation [civ_info] has to start preparing for a war against [target].
    ///
    /// Returns the motivation of the plan. If it is > 0 then we can start planning the war.
    pub fn evaluate_start_preparing_war_plan(
        civ_info: &Civilization,
        target: &Civilization,
        given_motivation: Option<f32>,
    ) -> f32 {
        let motivation = given_motivation.unwrap_or_else(|| {
            MotivationToAttackAutomation::has_at_least_motivation_to_attack(civ_info, target, 0.0)
        });

        // TODO: We use negative values in WaryOf for now so that we aren't adding any extra fields to the save file
        // This will very likely change in the future and we will want to build upon it
        let diplo_manager = civ_info.get_diplomacy_manager(target).unwrap();
        if diplo_manager.has_flag(DiplomacyFlags::WaryOf) {
            return 0.0;
        }

        motivation - 15.0
    }
}