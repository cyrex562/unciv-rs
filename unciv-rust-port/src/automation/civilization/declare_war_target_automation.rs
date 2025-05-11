
use crate::models::trade::{TradeLogic, TradeOffer, TradeRequest, TradeOfferType};
use crate::automation::civilization::declare_war_plan_evaluator::DeclareWarPlanEvaluator;

/// Contains logic for choosing war targets and executing different war declaration strategies.
pub struct DeclareWarTargetAutomation;

impl DeclareWarTargetAutomation {
    /// Chooses a target civilization along with a plan of attack.
    /// Note that this doesn't guarantee that we will declare war on them immediately, or that we will end up declaring war at all.
    pub fn choose_declare_war_target(civ_info: &Civilization, civ_attack_motivations: &[(Civilization, f32)]) {
        // Sort targets by score in descending order
        let highest_value_targets: Vec<_> = civ_attack_motivations.iter()
            .sorted_by(|a, b| b.0.get_stat_for_ranking(RankingType::Score)
                .partial_cmp(&a.0.get_stat_for_ranking(RankingType::Score))
                .unwrap_or(std::cmp::Ordering::Equal))
            .collect();

        for (target, motivation) in highest_value_targets {
            if Self::try_declare_war_with_plan(civ_info, target, *motivation) {
                return; // We have successfully found a plan and started executing it!
            }
        }
    }

    /// Determines a war plan against this target and executes it if able.
    fn try_declare_war_with_plan(civ_info: &Civilization, target: &Civilization, motivation: f32) -> bool {
        if !target.is_city_state {
            if motivation > 5.0 && Self::try_team_war(civ_info, target, motivation) {
                return true;
            }

            if motivation >= 15.0 && Self::try_join_war(civ_info, target, motivation) {
                return true;
            }
        }

        if motivation >= 20.0 && Self::declare_war(civ_info, target, motivation) {
            return true;
        }

        if motivation >= 15.0 && Self::prepare_war(civ_info, target, motivation) {
            return true;
        }

        false
    }

    /// The safest option for war is to invite a new ally to join the war with us.
    /// Together we are stronger and are more likely to take down bigger threats.
    fn try_team_war(civ_info: &Civilization, target: &Civilization, motivation: f32) -> bool {
        let potential_allies = civ_info.get_diplomacy_manager(target).unwrap()
            .get_common_known_civs()
            .iter()
            .filter(|civ| {
                civ.is_major_civ()
                    && !civ_info.get_diplomacy_manager(civ).unwrap().has_flag(DiplomacyFlags::DeclinedJoinWarOffer)
                    && civ_info.get_diplomacy_manager(civ).unwrap().is_relationship_level_ge(RelationshipLevel::Neutral)
                    && !civ.is_at_war_with(target)
            })
            .sorted_by(|a, b| b.get_stat_for_ranking(RankingType::Force)
                .partial_cmp(&a.get_stat_for_ranking(RankingType::Force))
                .unwrap_or(std::cmp::Ordering::Equal))
            .collect::<Vec<_>>();

        for third_civ in potential_allies {
            if DeclareWarPlanEvaluator::evaluate_team_war_plan(civ_info, target, third_civ, Some(motivation)) <= 0.0 {
                continue;
            }

            // Send them an offer
            let mut trade_logic = TradeLogic::new(civ_info, third_civ);
            trade_logic.current_trade.our_offers.push(TradeOffer::new(
                target.name.clone(),
                TradeOfferType::WarDeclaration,
                civ_info.game_info.speed,
            ));
            trade_logic.current_trade.their_offers.push(TradeOffer::new(
                target.name.clone(),
                TradeOfferType::WarDeclaration,
                civ_info.game_info.speed,
            ));

            third_civ.trade_requests.push(TradeRequest::new(
                civ_info.name.clone(),
                trade_logic.current_trade.reverse(),
            ));

            return true;
        }

        false
    }

    /// The next safest approach is to join an existing war on the side of an ally that is already at war with target.
    fn try_join_war(civ_info: &Civilization, target: &Civilization, motivation: f32) -> bool {
        let potential_allies = civ_info.get_diplomacy_manager(target).unwrap()
            .get_common_known_civs()
            .iter()
            .filter(|civ| {
                civ.is_major_civ()
                    && !civ_info.get_diplomacy_manager(civ).unwrap().has_flag(DiplomacyFlags::DeclinedJoinWarOffer)
                    && civ_info.get_diplomacy_manager(civ).unwrap().is_relationship_level_ge(RelationshipLevel::Favorable)
                    && civ.is_at_war_with(target)
            })
            .sorted_by(|a, b| b.get_stat_for_ranking(RankingType::Force)
                .partial_cmp(&a.get_stat_for_ranking(RankingType::Force))
                .unwrap_or(std::cmp::Ordering::Equal))
            .collect::<Vec<_>>();

        for third_civ in potential_allies {
            if DeclareWarPlanEvaluator::evaluate_join_war_plan(civ_info, target, third_civ, Some(motivation)) <= 0.0 {
                continue;
            }

            // Send them an offer
            let mut trade_logic = TradeLogic::new(civ_info, third_civ);
            trade_logic.current_trade.our_offers.push(TradeOffer::new(
                target.name.clone(),
                TradeOfferType::WarDeclaration,
                civ_info.game_info.speed,
            ));
            // TODO: Maybe add in payment requests in some situations
            third_civ.trade_requests.push(TradeRequest::new(
                civ_info.name.clone(),
                trade_logic.current_trade.reverse(),
            ));

            return true;
        }

        false
    }

    /// Lastly, if our motivation is high enough and we don't have any better plans then lets just declare war.
    fn declare_war(civ_info: &Civilization, target: &Civilization, motivation: f32) -> bool {
        if DeclareWarPlanEvaluator::evaluate_declare_war_plan(civ_info, target, Some(motivation)) > 0.0 {
            civ_info.get_diplomacy_manager(target).unwrap().declare_war();
            return true;
        }
        false
    }

    /// Slightly safer is to silently plan an invasion and declare war later.
    fn prepare_war(civ_info: &Civilization, target: &Civilization, motivation: f32) -> bool {
        // TODO: We use negative values in WaryOf for now so that we aren't adding any extra fields to the save file
        // This will very likely change in the future and we will want to build upon it
        let diplo_manager = civ_info.get_diplomacy_manager(target).unwrap();
        if DeclareWarPlanEvaluator::evaluate_start_preparing_war_plan(civ_info, target, Some(motivation)) > 0.0 {
            diplo_manager.set_flag(DiplomacyFlags::WaryOf, -1);
            return true;
        }
        false
    }
}