use crate::civilization::{Civilization, NotificationCategory, NotificationIcon};
use crate::models::ruleset::unique::UniqueType;
use crate::models::trade::{Trade, TradeOffer, TradeOfferType};
use crate::utils::color::Color;
use std::collections::HashMap;
use std::cmp::{max, min};
use std::f32;

/// Manages turn-based diplomatic actions and state changes
pub struct DiplomacyTurnManager;

impl DiplomacyTurnManager {
    /// Handles all turn-based diplomatic actions
    pub fn next_turn(diplomacy_manager: &mut DiplomacyManager) {
        Self::next_turn_trades(diplomacy_manager);
        Self::remove_untenable_trades(diplomacy_manager);
        Self::update_has_open_borders(diplomacy_manager);
        Self::next_turn_diplomatic_modifiers(diplomacy_manager);
        Self::next_turn_flags(diplomacy_manager);
        if diplomacy_manager.civ_info.is_city_state && diplomacy_manager.other_civ().is_major_civ() {
            Self::next_turn_city_state_influence(diplomacy_manager);
        }
    }

    /// Removes trades that are no longer valid
    fn remove_untenable_trades(diplomacy_manager: &mut DiplomacyManager) {
        let mut trades_to_remove = Vec::new();
        let negative_civ_resources = diplomacy_manager.civ_info.get_civ_resource_supply()
            .iter()
            .filter(|r| r.amount < 0 && !r.resource.is_stockpiled)
            .map(|r| r.resource.name.clone())
            .collect::<Vec<_>>();

        for trade in &diplomacy_manager.trades {
            for offer in &trade.our_offers {
                if (offer.trade_type == TradeOfferType::LuxuryResource ||
                    offer.trade_type == TradeOfferType::StrategicResource) &&
                    (negative_civ_resources.contains(&offer.name) ||
                     !diplomacy_manager.civ_info.game_info.ruleset.tile_resources.contains_key(&offer.name)) {
                    trades_to_remove.push(trade.clone());
                    let other_civ_trades = diplomacy_manager.other_civ_diplomacy().trades.clone();
                    other_civ_trades.retain(|t| !t.equal_trade(&trade.reverse()));

                    // Can't cut short peace treaties!
                    if trade.their_offers.iter().any(|o| o.name == "Peace Treaty") {
                        let duration = trade.their_offers.iter()
                            .find(|o| o.name == "Peace Treaty")
                            .unwrap()
                            .duration;
                        Self::remake_peace_treaty(diplomacy_manager, duration);
                    }

                    diplomacy_manager.civ_info.add_notification(
                        format!("One of our trades with [{}] has been cut short",
                            diplomacy_manager.other_civ_name),
                        NotificationCategory::Trade,
                        NotificationIcon::Trade,
                        &diplomacy_manager.other_civ_name
                    );
                    diplomacy_manager.other_civ().add_notification(
                        format!("One of our trades with [{}] has been cut short",
                            diplomacy_manager.civ_info.civ_name),
                        NotificationCategory::Trade,
                        NotificationIcon::Trade,
                        &diplomacy_manager.civ_info.civ_name
                    );

                    // If you cut a trade short, we're not going to trust you with per-turn trades for a while
                    diplomacy_manager.other_civ_diplomacy().set_flag(
                        DiplomacyFlags::ResourceTradesCutShort,
                        diplomacy_manager.civ_info.game_info.speed.deal_duration * 2
                    );
                    diplomacy_manager.civ_info.cache.update_civ_resources();
                }
            }
        }

        for trade in trades_to_remove {
            diplomacy_manager.trades.retain(|t| t != &trade);
        }
    }

    /// Creates a new peace treaty with the specified duration
    fn remake_peace_treaty(diplomacy_manager: &mut DiplomacyManager, duration_left: i32) {
        let mut treaty = Trade::new();
        treaty.our_offers.push(TradeOffer::new(
            "Peace Treaty",
            TradeOfferType::Treaty,
            duration_left
        ));
        treaty.their_offers.push(TradeOffer::new(
            "Peace Treaty",
            TradeOfferType::Treaty,
            duration_left
        ));
        diplomacy_manager.trades.push(treaty.clone());
        diplomacy_manager.other_civ_diplomacy().trades.push(treaty);
    }

    /// Updates city-state influence for the next turn
    fn next_turn_city_state_influence(diplomacy_manager: &mut DiplomacyManager) {
        let initial_relationship_level = diplomacy_manager.relationship_ignore_afraid();

        let resting_point = diplomacy_manager.get_city_state_influence_resting_point();
        if diplomacy_manager.influence > resting_point {
            let decrement = diplomacy_manager.get_city_state_influence_degrade();
            diplomacy_manager.set_influence(max(resting_point, diplomacy_manager.influence - decrement));
        } else if diplomacy_manager.influence < resting_point {
            let increment = diplomacy_manager.get_city_state_influence_recovery();
            diplomacy_manager.set_influence(min(resting_point, diplomacy_manager.influence + increment));
        }

        if !diplomacy_manager.civ_info.is_defeated() {
            let notification_actions = diplomacy_manager.civ_info.city_state_functions.get_notification_actions();
            if diplomacy_manager.get_turns_to_relationship_change() == 1 {
                let text = format!("Your relationship with [{}] is about to degrade",
                    diplomacy_manager.civ_info.civ_name);
                diplomacy_manager.other_civ().add_notification(
                    text,
                    notification_actions,
                    NotificationCategory::Diplomacy,
                    &diplomacy_manager.civ_info.civ_name,
                    NotificationIcon::Diplomacy
                );
            }

            if initial_relationship_level >= RelationshipLevel::Friend &&
               initial_relationship_level != diplomacy_manager.relationship_ignore_afraid() {
                let text = format!("Your relationship with [{}] degraded",
                    diplomacy_manager.civ_info.civ_name);
                diplomacy_manager.other_civ().add_notification(
                    text,
                    notification_actions,
                    NotificationCategory::Diplomacy,
                    &diplomacy_manager.civ_info.civ_name,
                    NotificationIcon::Diplomacy
                );
            }

            // Potentially notify about afraid status
            if diplomacy_manager.get_influence() < 30.0 && // We usually don't want to bully our friends
               !diplomacy_manager.has_flag(DiplomacyFlags::NotifiedAfraid) &&
               diplomacy_manager.civ_info.city_state_functions.get_tribute_willingness(diplomacy_manager.other_civ()) > 0 &&
               diplomacy_manager.other_civ().is_major_civ() {
                diplomacy_manager.set_flag(DiplomacyFlags::NotifiedAfraid, 20); // Wait 20 turns until next reminder
                let text = format!("[{}] is afraid of your military power!",
                    diplomacy_manager.civ_info.civ_name);
                diplomacy_manager.other_civ().add_notification(
                    text,
                    notification_actions,
                    NotificationCategory::Diplomacy,
                    &diplomacy_manager.civ_info.civ_name,
                    NotificationIcon::Diplomacy
                );
            }
        }
    }

    /// Gets the city-state influence recovery rate
    fn get_city_state_influence_recovery(diplomacy_manager: &DiplomacyManager) -> f32 {
        if diplomacy_manager.get_influence() >= diplomacy_manager.get_city_state_influence_resting_point() {
            return 0.0;
        }

        let increment = 1.0; // sic: personality does not matter here
        let mut modifier_percent = 0.0;

        if diplomacy_manager.other_civ().has_unique(UniqueType::CityStateInfluenceRecoversTwiceNormalRate) {
            modifier_percent += 100.0;
        }

        let religion = if diplomacy_manager.civ_info.cities.is_empty() ||
                        diplomacy_manager.civ_info.get_capital().is_none() {
            None
        } else {
            diplomacy_manager.civ_info.get_capital()
                .map(|cap| cap.religion.get_majority_religion_name())
        };

        if let Some(religion) = religion {
            if let Some(other_religion) = &diplomacy_manager.other_civ().religion_manager.religion {
                if religion == other_religion.name {
                    modifier_percent += 50.0; // 50% quicker recovery when sharing a religion
                }
            }
        }

        max(0.0, increment) * max(0.0, modifier_percent).to_percent()
    }

    /// Updates diplomatic flags for the next turn
    fn next_turn_flags(diplomacy_manager: &mut DiplomacyManager) {
        let mut flags_to_remove = Vec::new();
        for (flag, countdown) in &mut diplomacy_manager.flags_countdown {
            // We want negative flags to keep on going negative to keep track of time
            *countdown -= 1;

            // If we have uniques that make city states grant military units faster when at war with a common enemy
            if flag == &DiplomacyFlags::ProvideMilitaryUnit.to_string() &&
               diplomacy_manager.civ_info.is_major_civ() &&
               diplomacy_manager.other_civ().is_city_state &&
               diplomacy_manager.civ_info.game_info.civilizations.iter()
                   .any(|c| diplomacy_manager.civ_info.is_at_war_with(c) &&
                           diplomacy_manager.other_civ().is_at_war_with(c)) {
                for unique in diplomacy_manager.civ_info.get_matching_uniques(UniqueType::CityStateMoreGiftedUnits) {
                    let countdown = diplomacy_manager.flags_countdown.get_mut(&DiplomacyFlags::ProvideMilitaryUnit.to_string()).unwrap();
                    *countdown = *countdown - unique.params[0].parse::<i32>().unwrap() + 1;
                    if *countdown <= 0 {
                        *countdown = 0;
                        break;
                    }
                }
            }

            // At the end of every turn
            if flag == &DiplomacyFlags::ResearchAgreement.to_string() {
                diplomacy_manager.total_of_science_during_ra +=
                    diplomacy_manager.civ_info.stats.stats_for_next_turn.science as i32;
            }

            // These modifiers decrease slightly @ 50
            if *countdown == 50 {
                match flag.as_str() {
                    "RememberAttackedProtectedMinor" => {
                        diplomacy_manager.add_modifier(DiplomaticModifiers::AttackedProtectedMinor, 5.0);
                    }
                    "RememberBulliedProtectedMinor" => {
                        diplomacy_manager.add_modifier(DiplomaticModifiers::BulliedProtectedMinor, 5.0);
                    }
                    _ => {}
                }
            }

            // Only when flag is expired
            if *countdown == 0 {
                match flag.as_str() {
                    "ResearchAgreement" => {
                        if !diplomacy_manager.other_civ_diplomacy().has_flag(DiplomacyFlags::ResearchAgreement) {
                            Self::science_from_research_agreement(diplomacy_manager);
                        }
                    }
                    "DefensivePact" => {
                        diplomacy_manager.diplomatic_status = DiplomaticStatus::Peace;
                    }
                    "ProvideMilitaryUnit" => {
                        if diplomacy_manager.civ_info.cities.is_empty() ||
                           diplomacy_manager.other_civ().cities.is_empty() {
                            continue;
                        } else {
                            diplomacy_manager.other_civ().city_state_functions
                                .give_military_unit_to_patron(&diplomacy_manager.civ_info);
                        }
                    }
                    "AgreedToNotSettleNearUs" => {
                        diplomacy_manager.add_modifier(
                            DiplomaticModifiers::FulfilledPromiseToNotSettleCitiesNearUs,
                            10.0
                        );
                    }
                    "RecentlyAttacked" => {
                        diplomacy_manager.civ_info.city_state_functions
                            .ask_for_unit_gifts(diplomacy_manager.other_civ());
                    }
                    // These modifiers don't tick down normally, instead there is a threshold number of turns
                    "RememberDestroyedProtectedMinor" => { // 125
                        diplomacy_manager.remove_modifier(DiplomaticModifiers::DestroyedProtectedMinor);
                    }
                    "RememberAttackedProtectedMinor" => { // 75
                        diplomacy_manager.remove_modifier(DiplomaticModifiers::AttackedProtectedMinor);
                    }
                    "RememberBulliedProtectedMinor" => { // 75
                        diplomacy_manager.remove_modifier(DiplomaticModifiers::BulliedProtectedMinor);
                    }
                    "RememberSidedWithProtectedMinor" => { // 25
                        diplomacy_manager.remove_modifier(DiplomaticModifiers::SidedWithProtectedMinor);
                    }
                    _ => {}
                }
                flags_to_remove.push(flag.clone());
            } else if flag == &DiplomacyFlags::WaryOf.to_string() && *countdown < -10 {
                // Used in DeclareWarTargetAutomation.declarePlannedWar to count the number of turns preparing
                // If we have been preparing for over 10 turns then cancel our attack plan
                flags_to_remove.push(flag.clone());
            }
        }

        for flag in flags_to_remove {
            diplomacy_manager.flags_countdown.remove(&flag);
        }
    }

    /// Calculates and applies science from research agreements
    fn science_from_research_agreement(diplomacy_manager: &mut DiplomacyManager) {
        let science_from_research_agreement = min(
            diplomacy_manager.total_of_science_during_ra,
            diplomacy_manager.other_civ_diplomacy().total_of_science_during_ra
        );
        diplomacy_manager.civ_info.tech.science_from_research_agreements += science_from_research_agreement;
        diplomacy_manager.other_civ().tech.science_from_research_agreements += science_from_research_agreement;
        diplomacy_manager.total_of_science_during_ra = 0;
        diplomacy_manager.other_civ_diplomacy().total_of_science_during_ra = 0;
    }

    /// Updates trades for the next turn
    fn next_turn_trades(diplomacy_manager: &mut DiplomacyManager) {
        let mut trades_to_remove = Vec::new();
        for trade in &mut diplomacy_manager.trades {
            for offer in trade.our_offers.iter_mut().chain(trade.their_offers.iter_mut())
                .filter(|o| o.duration > 0) {
                offer.duration -= 1;
            }

            if trade.our_offers.iter().all(|o| o.duration <= 0) &&
               trade.their_offers.iter().all(|o| o.duration <= 0) {
                trades_to_remove.push(trade.clone());
                for offer in trade.our_offers.iter().chain(trade.their_offers.iter())
                    .filter(|o| o.duration == 0) {
                    let direction = if trade.their_offers.contains(offer) { "from" } else { "to" };
                    diplomacy_manager.civ_info.add_notification(
                        format!("[{}] {} [{}] has ended",
                            offer.name, direction, diplomacy_manager.other_civ_name),
                        NotificationCategory::Trade,
                        &diplomacy_manager.other_civ_name,
                        NotificationIcon::Trade
                    );

                    diplomacy_manager.civ_info.update_stats_for_next_turn();
                    if trade.their_offers.iter().chain(trade.our_offers.iter())
                        .any(|o| o.trade_type == TradeOfferType::LuxuryResource ||
                                o.trade_type == TradeOfferType::StrategicResource) {
                        diplomacy_manager.civ_info.cache.update_civ_resources();
                    }
                }
            }

            for offer in trade.their_offers.iter().filter(|o| o.duration <= 3) {
                if offer.duration == 3 {
                    diplomacy_manager.civ_info.add_notification(
                        format!("[{}] from [{}] will end in [3] turns",
                            offer.name, diplomacy_manager.other_civ_name),
                        NotificationCategory::Trade,
                        &diplomacy_manager.other_civ_name,
                        NotificationIcon::Trade
                    );
                } else if offer.duration == 1 {
                    diplomacy_manager.civ_info.add_notification(
                        format!("[{}] from [{}] will end next turn",
                            offer.name, diplomacy_manager.other_civ_name),
                        NotificationCategory::Trade,
                        &diplomacy_manager.other_civ_name,
                        NotificationIcon::Trade
                    );
                }
            }
        }

        for trade in trades_to_remove {
            diplomacy_manager.trades.retain(|t| t != &trade);
        }
    }

    /// Updates diplomatic modifiers for the next turn
    fn next_turn_diplomatic_modifiers(diplomacy_manager: &mut DiplomacyManager) {
        if diplomacy_manager.diplomatic_status == DiplomaticStatus::Peace {
            if diplomacy_manager.get_modifier(DiplomaticModifiers::YearsOfPeace) < 30.0 {
                diplomacy_manager.add_modifier(DiplomaticModifiers::YearsOfPeace, 0.5);
            }
        } else {
            Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::YearsOfPeace, 0.5);
        }

        let mut open_borders = 0;
        if diplomacy_manager.has_open_borders {
            open_borders += 1;
        }
        if diplomacy_manager.other_civ_diplomacy().has_open_borders {
            open_borders += 1;
        }
        if open_borders > 0 {
            diplomacy_manager.add_modifier(DiplomaticModifiers::OpenBorders, open_borders as f32 / 8.0);
        } else {
            Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::OpenBorders, 1.0 / 8.0);
        }

        // Negatives
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::DeclaredWarOnUs, 1.0 / 8.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::WarMongerer, 1.0 / 2.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::CapturedOurCities, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::BetrayedDeclarationOfFriendship, 1.0 / 8.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::BetrayedDefensivePact, 1.0 / 16.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::RefusedToNotSettleCitiesNearUs, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::BetrayedPromiseToNotSettleCitiesNearUs, 1.0 / 8.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::BetrayedPromiseToNotSpreadReligionToUs, 1.0 / 8.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::UnacceptableDemands, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::StealingTerritory, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::DenouncedOurAllies, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::DenouncedOurEnemies, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::Denunciation, 1.0 / 8.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::SpiedOnUs, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::StoleOurAlly, 1.0 / 2.0);

        // Positives
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::GaveUsUnits, 1.0 / 4.0);
        Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::LiberatedCity, 1.0 / 8.0);
        if diplomacy_manager.has_modifier(DiplomaticModifiers::GaveUsGifts) {
            let gift_loss = match diplomacy_manager.relationship_level() {
                RelationshipLevel::Ally => 1.0,
                RelationshipLevel::Friend => 1.5,
                RelationshipLevel::Favorable => 2.0,
                RelationshipLevel::Neutral => 2.5,
                RelationshipLevel::Competitor => 5.0,
                RelationshipLevel::Enemy => 7.5,
                RelationshipLevel::Unforgivable => 10.0,
                _ => 2.5,
            } * diplomacy_manager.civ_info.game_info.ruleset.mod_options.constants.gold_gift_degradation_multiplier;

            let amount_lost = (diplomacy_manager.get_modifier(DiplomaticModifiers::GaveUsGifts).abs() * gift_loss / 100.0)
                .max(gift_loss / 5.0);
            Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::GaveUsGifts, amount_lost);
        }

        diplomacy_manager.set_friendship_based_modifier();
        diplomacy_manager.set_defensive_pact_based_modifier();
        diplomacy_manager.set_religion_based_modifier();

        if !diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
            Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::DeclarationOfFriendship, 1.0 / 2.0);
        }

        if !diplomacy_manager.has_flag(DiplomacyFlags::DefensivePact) {
            Self::revert_to_zero(diplomacy_manager, DiplomaticModifiers::DefensivePact, 1.0);
        }

        if !diplomacy_manager.other_civ().is_city_state {
            return;
        }

        if diplomacy_manager.is_relationship_level_lt(RelationshipLevel::Friend) {
            if diplomacy_manager.has_flag(DiplomacyFlags::ProvideMilitaryUnit) {
                diplomacy_manager.remove_flag(DiplomacyFlags::ProvideMilitaryUnit);
            }
            return;
        }

        let variance = [-1, 0, 1][rand::random::<usize>() % 3];

        let provide_military_unit_uniques = diplomacy_manager.civ_info.city_state_functions
            .get_city_state_bonuses(
                diplomacy_manager.other_civ().city_state_type,
                diplomacy_manager.relationship_ignore_afraid(),
                UniqueType::CityStateMilitaryUnits
            )
            .iter()
            .filter(|u| u.conditionals_apply(&diplomacy_manager.civ_info.state))
            .collect::<Vec<_>>();

        if provide_military_unit_uniques.is_empty() {
            diplomacy_manager.remove_flag(DiplomacyFlags::ProvideMilitaryUnit);
        }

        for unique in provide_military_unit_uniques {
            if !diplomacy_manager.has_flag(DiplomacyFlags::ProvideMilitaryUnit) ||
               diplomacy_manager.get_flag(DiplomacyFlags::ProvideMilitaryUnit) > unique.params[0].parse::<i32>().unwrap() {
                diplomacy_manager.set_flag(
                    DiplomacyFlags::ProvideMilitaryUnit,
                    unique.params[0].parse::<i32>().unwrap() + variance
                );
            }
        }
    }

    /// Reverts a modifier towards zero by the given amount
    fn revert_to_zero(diplomacy_manager: &mut DiplomacyManager, modifier: DiplomaticModifiers, amount: f32) {
        if !diplomacy_manager.has_modifier(modifier) {
            return;
        }
        let current_amount = diplomacy_manager.get_modifier(modifier);
        if amount >= current_amount.abs() {
            diplomacy_manager.remove_modifier(modifier);
        } else if current_amount > 0.0 {
            diplomacy_manager.add_modifier(modifier, -amount);
        } else {
            diplomacy_manager.add_modifier(modifier, amount);
        }
    }
}