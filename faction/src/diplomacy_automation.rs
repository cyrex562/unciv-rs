use crate::ai::personality::PersonalityValue;
use crate::automation::civilization::declare_war_plan_evaluator::DeclareWarPlanEvaluator;
use crate::automation::civilization::declare_war_target_automation::DeclareWarTargetAutomation;
use crate::automation::civilization::motivation_to_attack_automation::MotivationToAttackAutomation;
use crate::civilization::civilization::Civilization;
use crate::civilization::popup_alert::PopupAlert;
use crate::diplomacy::relationship_level::RelationshipLevel;
use crate::models::trade::{Trade, TradeLogic, TradeOffer, TradeOfferType, TradeRequest};
use rand::Rng;
use std::collections::HashMap;
use crate::diplomacy::flags::DiplomacyFlags;

/// Contains logic for handling diplomatic actions between civilizations.
pub struct DiplomacyAutomation;

impl DiplomacyAutomation {
    /// Offers a declaration of friendship to civilizations that meet the criteria.
    pub fn offer_declaration_of_friendship(civ_info: &mut Civilization) {
        let civs_that_we_can_declare_friendship_with: Vec<_> = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| {
                civ_info
                    .diplomacy_functions
                    .can_sign_declaration_of_friendship_with(civ)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclinedDeclarationOfFriendship)
            })
            .sorted_by(|a, b| {
                b.get_diplomacy_manager(civ_info)
                    .unwrap()
                    .get_relationship_level()
                    .partial_cmp(
                        &a.get_diplomacy_manager(civ_info)
                            .unwrap()
                            .get_relationship_level(),
                    )
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .collect();

        for other_civ in civs_that_we_can_declare_friendship_with {
            // Default setting is 2, this will be changed according to different civ.
            let random_value = rand::thread_rng().gen_range(1..=10);
            let personality_modifier = civ_info
                .get_personality()
                .modifier_focus(PersonalityValue::Diplomacy, 0.5);

            if random_value <= (2.0 * personality_modifier) as i32
                && Self::wants_to_sign_declaration_of_friendship(civ_info, other_civ)
            {
                other_civ.popup_alerts.push(PopupAlert::new(
                    format!("Declaration of Friendship from {}", civ_info.name),
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Diplomacy,
                ));
            }
        }
    }

    /// Determines if a civilization wants to sign a declaration of friendship with another civilization.
    pub fn wants_to_sign_declaration_of_friendship(
        civ_info: &Civilization,
        other_civ: &Civilization,
    ) -> bool {
        let diplo_manager = civ_info.get_diplomacy_manager(other_civ).unwrap();
        if diplo_manager.has_flag(DiplomacyFlags::DeclinedDeclarationOfFriendship) {
            return false;
        }
        // Shortcut, if it is below favorable then don't consider it
        if diplo_manager.is_relationship_level_lt(RelationshipLevel::Favorable) {
            return false;
        }

        let num_of_friends = civ_info
            .diplomacy_managers
            .values()
            .filter(|manager| manager.has_flag(DiplomacyFlags::DeclarationOfFriendship))
            .count();

        let other_civ_number_of_friends = other_civ
            .diplomacy_managers
            .values()
            .filter(|manager| manager.has_flag(DiplomacyFlags::DeclarationOfFriendship))
            .count();

        let known_civs = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| civ.is_major_civ() && !civ.is_defeated())
            .count();

        let all_civs = civ_info
            .game_info
            .civilizations
            .iter()
            .filter(|civ| civ.is_major_civ())
            .count()
            - 1; // Don't include us

        let dead_civs = civ_info
            .game_info
            .civilizations
            .iter()
            .filter(|civ| civ.is_major_civ() && civ.is_defeated())
            .count();

        let all_alive_civs = all_civs - dead_civs;

        // Motivation should be constant as the number of civs changes
        let mut motivation = diplo_manager.opinion_of_other_civ() - 40.0;

        // Warmongerers don't make good allies
        if diplo_manager.has_modifier(DiplomaticModifiers::WarMongerer) {
            motivation -= diplo_manager.get_modifier(DiplomaticModifiers::WarMongerer)
                * civ_info
                    .get_personality()
                    .modifier_focus(PersonalityValue::Diplomacy, 0.5);
        }

        // If the other civ is stronger than we are compelled to be nice to them
        // If they are too weak, then their friendship doesn't mean much to us
        match ThreatLevel::assess_threat(civ_info, other_civ) {
            ThreatLevel::VeryHigh => motivation += 10.0,
            ThreatLevel::High => motivation += 5.0,
            ThreatLevel::VeryLow => motivation -= 5.0,
            _ => {}
        }

        // Try to ally with a fourth of the civs in play
        let civs_to_ally_with = 0.25
            * all_alive_civs as f32
            * civ_info
                .get_personality()
                .modifier_focus(PersonalityValue::Diplomacy, 0.3);

        if num_of_friends < civs_to_ally_with as usize {
            // Goes from 10 to 0 once the civ gets 1/4 of all alive civs as friends
            motivation += 10.0 - 10.0 * num_of_friends as f32 / civs_to_ally_with;
        } else {
            // Goes from 0 to -120 as the civ gets more friends, offset by civsToAllyWith
            motivation -= 120.0 * (num_of_friends as f32 - civs_to_ally_with)
                / (known_civs as f32 - civs_to_ally_with);
        }

        // The more friends they have the less we should want to sign friendship (To promote teams)
        motivation -= other_civ_number_of_friends as f32 * 10.0;

        // Goes from 0 to -50 as more civs die
        // this is meant to prevent the game from stalemating when a group of friends
        // conquers all opposition
        motivation -= (dead_civs as f32 / all_civs as f32) * 50.0;

        // Become more desperate as we have more wars
        let war_count = civ_info
            .diplomacy_managers
            .values()
            .filter(|manager| {
                let other_civ = civ_info
                    .game_info
                    .get_civilization_by_name(&manager.other_civ_name);
                other_civ.is_major_civ() && manager.get_diplomatic_status() == DiplomaticStatus::War
            })
            .count();
        motivation += war_count as f32 * 10.0;

        // Wait to declare friendships until more civs
        // Goes from -30 to 0 when we know 75% of allCivs
        let civs_to_know = 0.75 * all_alive_civs as f32;
        motivation -= ((civs_to_know - known_civs as f32) / civs_to_know * 30.0).max(0.0);

        // If they are the only non-friendly civ near us then they are the only civ to attack and expand into
        let has_non_friendly_neighbors = civ_info
            .threat_manager
            .get_neighboring_civilizations()
            .iter()
            .any(|civ| {
                civ.is_major_civ()
                    && civ.name != other_civ.name
                    && civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .is_relationship_level_lt(RelationshipLevel::Favorable)
            });

        if !has_non_friendly_neighbors {
            motivation -= 20.0;
        }

        if MotivationToAttackAutomation::has_at_least_motivation_to_attack(
            civ_info,
            other_civ,
            motivation / 2.0,
        ) > 0.0
        {
            motivation -= 2.0;
        }

        motivation > 0.0
    }

    /// Offers open borders to civilizations that meet the criteria.
    pub fn offer_open_borders(civ_info: &mut Civilization) {
        if !civ_info.has_unique(UniqueType::EnablesOpenBorders) {
            return;
        }

        let civs_that_we_can_open_borders_with: Vec<_> = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| {
                civ.is_major_civ()
                    && !civ_info.is_at_war_with(civ)
                    && civ.has_unique(UniqueType::EnablesOpenBorders)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_open_borders
                    && !civ
                        .get_diplomacy_manager(civ_info)
                        .unwrap()
                        .has_open_borders
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclinedOpenBorders)
                    && !Self::are_we_offering_trade(civ_info, civ, Constants::OPEN_BORDERS)
            })
            .sorted_by(|a, b| {
                b.get_diplomacy_manager(civ_info)
                    .unwrap()
                    .get_relationship_level()
                    .partial_cmp(
                        &a.get_diplomacy_manager(civ_info)
                            .unwrap()
                            .get_relationship_level(),
                    )
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .collect();

        for other_civ in civs_that_we_can_open_borders_with {
            // Default setting is 3, this will be changed according to different civ.
            let random_value = rand::thread_rng().gen_range(1..=10);
            if random_value < 7 {
                continue;
            }

            if Self::wants_to_open_borders(civ_info, other_civ) {
                let mut trade_logic = TradeLogic::new(civ_info, other_civ);
                trade_logic.current_trade.our_offers.push(TradeOffer::new(
                    Constants::OPEN_BORDERS.to_string(),
                    TradeOfferType::Agreement,
                    civ_info.game_info.speed,
                ));
                trade_logic.current_trade.their_offers.push(TradeOffer::new(
                    Constants::OPEN_BORDERS.to_string(),
                    TradeOfferType::Agreement,
                    civ_info.game_info.speed,
                ));

                other_civ.trade_requests.push(TradeRequest::new(
                    civ_info.name.clone(),
                    trade_logic.current_trade.reverse(),
                ));
            } else {
                // Remember this for a few turns to save computation power
                civ_info
                    .get_diplomacy_manager_mut(other_civ)
                    .unwrap()
                    .set_flag(DiplomacyFlags::DeclinedOpenBorders, 5);
            }
        }
    }

    /// Determines if a civilization wants to open borders with another civilization.
    pub fn wants_to_open_borders(civ_info: &Civilization, other_civ: &Civilization) -> bool {
        let diplo_manager = civ_info.get_diplomacy_manager(other_civ).unwrap();
        if diplo_manager.has_flag(DiplomacyFlags::DeclinedOpenBorders) {
            return false;
        }
        if diplo_manager.is_relationship_level_lt(RelationshipLevel::Favorable) {
            return false;
        }

        // Don't accept if they are at war with our friends, they might use our land to attack them
        let has_friends_at_war = civ_info.diplomacy_managers.values().any(|manager| {
            let other_civ_name = &manager.other_civ_name;
            let other_civ = civ_info.game_info.get_civilization_by_name(other_civ_name);
            manager.is_relationship_level_ge(RelationshipLevel::Friend)
                && other_civ.is_at_war_with(other_civ)
        });

        if has_friends_at_war {
            return false;
        }

        // Being able to see their cities can give us an advantage later on, especially with espionage enabled
        let visible_cities = other_civ
            .cities
            .iter()
            .filter(|city| !city.get_center_tile().is_visible(civ_info))
            .count();

        if visible_cities < (other_civ.cities.len() as f32 * 0.8) as usize {
            return true;
        }

        if MotivationToAttackAutomation::has_at_least_motivation_to_attack(
            civ_info,
            other_civ,
            diplo_manager.opinion_of_other_civ()
                * civ_info
                    .get_personality()
                    .modifier_focus(PersonalityValue::Commerce, 0.3)
                / 2.0,
        ) > 0.0
        {
            return false;
        }

        true
    }

    /// Offers research agreements to civilizations that meet the criteria.
    pub fn offer_research_agreement(civ_info: &mut Civilization) {
        if !civ_info.diplomacy_functions.can_sign_research_agreement() {
            return; // don't waste your time
        }

        let can_sign_research_agreement_civ: Vec<_> = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| {
                civ_info
                    .diplomacy_functions
                    .can_sign_research_agreements_with(civ)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclinedResearchAgreement)
                    && !Self::are_we_offering_trade(civ_info, civ, Constants::RESEARCH_AGREEMENT)
            })
            .sorted_by(|a, b| {
                b.stats
                    .stats_for_next_turn
                    .science
                    .partial_cmp(&a.stats.stats_for_next_turn.science)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .collect();

        for other_civ in can_sign_research_agreement_civ {
            // Default setting is 5, this will be changed according to different civ.
            let random_value = rand::thread_rng().gen_range(1..=10);
            let personality_modifier = civ_info
                .get_personality()
                .modifier_focus(PersonalityValue::Science, 0.3);

            if random_value <= (5.0 * personality_modifier) as i32 {
                continue;
            }

            let mut trade_logic = TradeLogic::new(civ_info, other_civ);
            let cost = civ_info
                .diplomacy_functions
                .get_research_agreement_cost(other_civ);

            trade_logic.current_trade.our_offers.push(TradeOffer::new(
                Constants::RESEARCH_AGREEMENT.to_string(),
                TradeOfferType::Treaty,
                cost,
                civ_info.game_info.speed,
            ));

            trade_logic.current_trade.their_offers.push(TradeOffer::new(
                Constants::RESEARCH_AGREEMENT.to_string(),
                TradeOfferType::Treaty,
                cost,
                civ_info.game_info.speed,
            ));

            other_civ.trade_requests.push(TradeRequest::new(
                civ_info.name.clone(),
                trade_logic.current_trade.reverse(),
            ));
        }
    }

    /// Offers defensive pacts to civilizations that meet the criteria.
    pub fn offer_defensive_pact(civ_info: &mut Civilization) {
        if !civ_info.diplomacy_functions.can_sign_defensive_pact() {
            return; // don't waste your time
        }

        let can_sign_defensive_pact_civ: Vec<_> = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| {
                civ_info
                    .diplomacy_functions
                    .can_sign_defensive_pact_with(civ)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclinedDefensivePact)
                    && civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .opinion_of_other_civ()
                        < 70.0
                            * civ_info
                                .get_personality()
                                .inverse_modifier_focus(PersonalityValue::Aggressive, 0.2)
                    && !Self::are_we_offering_trade(civ_info, civ, Constants::DEFENSIVE_PACT)
            })
            .collect();

        for other_civ in can_sign_defensive_pact_civ {
            // Default setting is 3, this will be changed according to different civ.
            let random_value = rand::thread_rng().gen_range(1..=10);
            let personality_modifier = civ_info
                .get_personality()
                .inverse_modifier_focus(PersonalityValue::Loyal, 0.3);

            if random_value <= (7.0 * personality_modifier) as i32 {
                continue;
            }

            if Self::wants_to_sign_defensive_pact(civ_info, other_civ) {
                //todo: Add more in depth evaluation here
                let mut trade_logic = TradeLogic::new(civ_info, other_civ);
                trade_logic.current_trade.our_offers.push(TradeOffer::new(
                    Constants::DEFENSIVE_PACT.to_string(),
                    TradeOfferType::Treaty,
                    civ_info.game_info.speed,
                ));
                trade_logic.current_trade.their_offers.push(TradeOffer::new(
                    Constants::DEFENSIVE_PACT.to_string(),
                    TradeOfferType::Treaty,
                    civ_info.game_info.speed,
                ));

                other_civ.trade_requests.push(TradeRequest::new(
                    civ_info.name.clone(),
                    trade_logic.current_trade.reverse(),
                ));
            } else {
                // Remember this for a few turns to save computation power
                civ_info
                    .get_diplomacy_manager_mut(other_civ)
                    .unwrap()
                    .set_flag(DiplomacyFlags::DeclinedDefensivePact, 5);
            }
        }
    }

    /// Determines if a civilization wants to sign a defensive pact with another civilization.
    pub fn wants_to_sign_defensive_pact(civ_info: &Civilization, other_civ: &Civilization) -> bool {
        let diplo_manager = civ_info.get_diplomacy_manager(other_civ).unwrap();
        if diplo_manager.has_flag(DiplomacyFlags::DeclinedDefensivePact) {
            return false;
        }

        let personality_modifier = civ_info
            .get_personality()
            .inverse_modifier_focus(PersonalityValue::Aggressive, 0.3);
        if diplo_manager.opinion_of_other_civ() < 65.0 * personality_modifier {
            return false;
        }

        let common_known_civs = diplo_manager.get_common_known_civs();
        for third_civ in common_known_civs {
            // If they have bad relations with any of our friends, don't consider it
            if civ_info
                .get_diplomacy_manager(third_civ)
                .unwrap()
                .has_flag(DiplomacyFlags::DeclarationOfFriendship)
                && third_civ
                    .get_diplomacy_manager(other_civ)
                    .unwrap()
                    .is_relationship_level_lt(RelationshipLevel::Favorable)
            {
                return false;
            }

            // If they have bad relations with any of our friends, don't consider it
            if other_civ
                .get_diplomacy_manager(third_civ)
                .unwrap()
                .has_flag(DiplomacyFlags::DeclarationOfFriendship)
                && third_civ
                    .get_diplomacy_manager(civ_info)
                    .unwrap()
                    .is_relationship_level_lt(RelationshipLevel::Neutral)
            {
                return false;
            }
        }

        let defensive_pacts = civ_info
            .diplomacy_managers
            .values()
            .filter(|manager| manager.has_flag(DiplomacyFlags::DefensivePact))
            .count();

        let other_civ_non_overlapping_defensive_pacts = other_civ
            .diplomacy_managers
            .values()
            .filter(|manager| {
                manager.has_flag(DiplomacyFlags::DefensivePact)
                    && !manager.other_civ_name.eq(&civ_info.name)
                    && !civ_info
                        .get_diplomacy_manager_by_name(&manager.other_civ_name)
                        .map_or(false, |m| m.has_flag(DiplomacyFlags::DefensivePact))
            })
            .count();

        let all_civs = civ_info
            .game_info
            .civilizations
            .iter()
            .filter(|civ| civ.is_major_civ())
            .count()
            - 1; // Don't include us

        let dead_civs = civ_info
            .game_info
            .civilizations
            .iter()
            .filter(|civ| civ.is_major_civ() && civ.is_defeated())
            .count();

        let all_alive_civs = all_civs - dead_civs;

        // We have to already be at RelationshipLevel.Ally, so we must have 80 opinion of them
        let mut motivation = diplo_manager.opinion_of_other_civ() - 80.0;

        // Warmongerers don't make good allies
        if diplo_manager.has_modifier(DiplomaticModifiers::WarMongerer) {
            motivation -= diplo_manager.get_modifier(DiplomaticModifiers::WarMongerer)
                * civ_info
                    .get_personality()
                    .modifier_focus(PersonalityValue::Diplomacy, 0.5);
        }

        // If they are stronger than us, then we value it a lot more
        // If they are weaker than us, then we don't value it
        match ThreatLevel::assess_threat(civ_info, other_civ) {
            ThreatLevel::VeryHigh => motivation += 10.0,
            ThreatLevel::High => motivation += 5.0,
            ThreatLevel::Low => motivation -= 3.0,
            ThreatLevel::VeryLow => motivation -= 7.0,
            _ => {}
        }

        // If they have a defensive pact with another civ then we would get drawn into their battles as well
        motivation -= 15.0 * other_civ_non_overlapping_defensive_pacts as f32;

        // Become more desperate as we have more wars
        let war_count = civ_info
            .diplomacy_managers
            .values()
            .filter(|manager| {
                let other_civ = civ_info
                    .game_info
                    .get_civilization_by_name(&manager.other_civ_name);
                other_civ.is_major_civ() && manager.get_diplomatic_status() == DiplomaticStatus::War
            })
            .count();
        motivation += war_count as f32 * 5.0;

        // Try to have a defensive pact with 1/5 of all civs
        let civs_to_ally_with = 0.20
            * all_alive_civs as f32
            * civ_info
                .get_personality()
                .modifier_focus(PersonalityValue::Diplomacy, 0.5);

        // Goes from 0 to -40 as the civ gets more allies, offset by civsToAllyWith
        motivation -= (40.0 * (defensive_pacts as f32 - civs_to_ally_with)
            / (all_alive_civs as f32 - civs_to_ally_with))
            .min(0.0);

        motivation > 0.0
    }

    /// Declares war on civilizations that meet the criteria.
    pub fn declare_war(civ_info: &mut Civilization) {
        if civ_info.cities.is_empty() || civ_info.diplomacy_managers.is_empty() {
            return;
        }

        if civ_info
            .get_personality()
            .get(&PersonalityValue::DeclareWar)
            .unwrap_or(&0.0)
            == &0.0
        {
            return;
        }

        if civ_info.get_happiness() <= 0 {
            return;
        }

        let our_military_units = civ_info
            .units
            .get_civ_units()
            .iter()
            .filter(|unit| !unit.is_civilian())
            .count();

        if our_military_units < civ_info.cities.len() {
            return;
        }

        if our_military_units < 4 {
            return; // to stop AI declaring war at the beginning of games when everyone isn't set up well enough
        }

        // For mods we can't check the number of cities, so we will check the population instead.
        let total_population: i32 = civ_info
            .cities
            .iter()
            .map(|city| city.population.population)
            .sum();

        if total_population < 12 {
            return; // FAR too early for that what are you thinking!
        }

        //evaluate war
        let target_civs: Vec<_> = civ_info
            .get_known_civs()
            .iter()
            .filter(|civ| {
                !civ.is_defeated()
                    && civ.name != civ_info.name
                    && !civ.cities.is_empty()
                    && civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .can_declare_war()
                    && civ
                        .cities
                        .iter()
                        .any(|city| civ_info.has_explored(&city.get_center_tile()))
            })
            .collect();

        // If the AI declares war on a civ without knowing the location of any cities,
        // it'll just keep amassing an army and not sending it anywhere, and end up at a massive disadvantage.
        if target_civs.is_empty() {
            return;
        }

        let target_civs_with_motivation: Vec<_> = target_civs
            .iter()
            .map(|civ| {
                (
                    civ,
                    MotivationToAttackAutomation::has_at_least_motivation_to_attack(
                        civ_info, civ, 0.0,
                    ),
                )
            })
            .filter(|(_, motivation)| *motivation > 0.0)
            .collect();

        DeclareWarTargetAutomation::choose_declare_war_target(
            civ_info,
            &target_civs_with_motivation,
        );
    }

    /// Offers peace treaties to civilizations that meet the criteria.
    pub fn offer_peace_treaty(civ_info: &mut Civilization) {
        if !civ_info.is_at_war()
            || civ_info.cities.is_empty()
            || civ_info.diplomacy_managers.is_empty()
        {
            return;
        }

        let enemies_civ: Vec<_> = civ_info
            .diplomacy_managers
            .values()
            .filter(|manager| manager.get_diplomatic_status() == DiplomaticStatus::War)
            .map(|manager| {
                civ_info
                    .game_info
                    .get_civilization_by_name(&manager.other_civ_name)
            })
            .filter(|civ| {
                civ.name != civ_info.name
                    && !civ.is_barbarian
                    && !civ.cities.is_empty()
                    && !civ
                        .get_diplomacy_manager(civ_info)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclaredWar)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclaredWar)
                    && !civ_info
                        .get_diplomacy_manager(civ)
                        .unwrap()
                        .has_flag(DiplomacyFlags::DeclinedPeace)
            })
            .filter(|civ| {
                // Don't allow AIs to offer peace to city states allied with their enemies
                !(civ.is_city_state
                    && civ.get_ally_civ().is_some()
                    && civ_info.is_at_war_with(
                        civ_info
                            .game_info
                            .get_civilization_by_name(civ.get_ally_civ().unwrap()),
                    ))
            })
            .filter(|civ| {
                // ignore civs that we have already offered peace this turn as a counteroffer to another civ's peace offer
                !civ.trade_requests.iter().any(|request| {
                    request.from_civ == civ_info.name && request.trade.is_peace_treaty()
                })
            })
            .collect();

        for enemy in enemies_civ {
            if MotivationToAttackAutomation::has_at_least_motivation_to_attack(
                civ_info, enemy, 10.0,
            ) >= 10.0
            {
                // We can still fight. Refuse peace.
                continue;
            }

            if civ_info.get_stat_for_ranking(RankingType::Force)
                - 0.8 * civ_info.threat_manager.get_combined_force_of_warring_civs()
                > 0.0
            {
                let random_seed = civ_info
                    .game_info
                    .civilizations
                    .iter()
                    .position(|c| c.name == enemy.name)
                    .unwrap_or(0)
                    + civ_info.get_civs_at_war_with().len()
                    + 123 * civ_info.game_info.turns;
                let mut rng = rand::thread_rng();
                rng.seed(rand::SeedableRng::seed_from_u64(random_seed as u64));
                if rng.gen_range(0..100) > 80 {
                    continue;
                }
            }

            // pay for peace
            let mut trade_logic = TradeLogic::new(civ_info, enemy);

            trade_logic.current_trade.our_offers.push(TradeOffer::new(
                Constants::PEACE_TREATY.to_string(),
                TradeOfferType::Treaty,
                civ_info.game_info.speed,
            ));
            trade_logic.current_trade.their_offers.push(TradeOffer::new(
                Constants::PEACE_TREATY.to_string(),
                TradeOfferType::Treaty,
                civ_info.game_info.speed,
            ));

            if enemy.is_major_civ() {
                let mut money_we_need_to_pay =
                    -TradeEvaluation::evaluate_peace_cost_for_them(civ_info, enemy);

                if civ_info.gold > 0 && money_we_need_to_pay > 0 {
                    if money_we_need_to_pay > civ_info.gold {
                        money_we_need_to_pay = civ_info.gold; // As much as possible
                    }
                    trade_logic.current_trade.our_offers.push(TradeOffer::new(
                        "Gold".to_string(),
                        TradeOfferType::Gold,
                        money_we_need_to_pay,
                        civ_info.game_info.speed,
                    ));
                } else if money_we_need_to_pay < -100 {
                    let money_they_need_to_pay = money_we_need_to_pay.abs().min(enemy.gold as f32);
                    if money_they_need_to_pay > 0 {
                        trade_logic.current_trade.their_offers.push(TradeOffer::new(
                            "Gold".to_string(),
                            TradeOfferType::Gold,
                            money_they_need_to_pay as i32,
                            civ_info.game_info.speed,
                        ));
                    }
                }
            }

            enemy.trade_requests.push(TradeRequest::new(
                civ_info.name.clone(),
                trade_logic.current_trade.reverse(),
            ));
        }
    }

    /// Asks for help from civilizations that meet the criteria.
    pub fn ask_for_help(civ_info: &mut Civilization) {
        if !civ_info.is_at_war()
            || civ_info.cities.is_empty()
            || civ_info.diplomacy_managers.is_empty()
        {
            return;
        }

        let enemy_civs: Vec<_> = civ_info
            .get_civs_at_war_with()
            .iter()
            .filter(|civ| civ.is_major_civ())
            .sorted_by(|a, b| {
                b.get_stat_for_ranking(RankingType::Force)
                    .partial_cmp(&a.get_stat_for_ranking(RankingType::Force))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .collect();

        for enemy_civ in enemy_civs {
            let potential_allies: Vec<_> = enemy_civ
                .threat_manager
                .get_neighboring_civilizations()
                .iter()
                .filter(|civ| {
                    civ_info.knows(civ)
                        && !civ.is_at_war_with(enemy_civ)
                        && civ_info
                            .get_diplomacy_manager(civ)
                            .unwrap()
                            .is_relationship_level_ge(RelationshipLevel::Friend)
                        && !civ
                            .get_diplomacy_manager(civ_info)
                            .unwrap()
                            .has_flag(DiplomacyFlags::DeclinedJoinWarOffer)
                })
                .sorted_by(|a, b| {
                    b.get_stat_for_ranking(RankingType::Force)
                        .partial_cmp(&a.get_stat_for_ranking(RankingType::Force))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .collect();

            let civ_to_ask = potential_allies.iter().find(|civ| {
                DeclareWarPlanEvaluator::evaluate_join_our_war_plan(civ_info, enemy_civ, civ, None)
                    > 0.0
            });

            if let Some(civ_to_ask) = civ_to_ask {
                let mut trade_logic = TradeLogic::new(civ_info, civ_to_ask);
                // TODO: add gold offer here
                trade_logic.current_trade.their_offers.push(TradeOffer::new(
                    enemy_civ.name.clone(),
                    TradeOfferType::WarDeclaration,
                    civ_info.game_info.speed,
                ));
                civ_to_ask.trade_requests.push(TradeRequest::new(
                    civ_info.name.clone(),
                    trade_logic.current_trade.reverse(),
                ));
            }
        }
    }

    /// Checks if we are offering a trade to a civilization.
    fn are_we_offering_trade(
        civ_info: &Civilization,
        other_civ: &Civilization,
        offer_name: &str,
    ) -> bool {
        other_civ
            .trade_requests
            .iter()
            .filter(|request| request.from_civ == civ_info.name)
            .any(|trade| {
                trade
                    .trade
                    .our_offers
                    .iter()
                    .any(|offer| offer.target == offer_name)
                    || trade
                        .trade
                        .their_offers
                        .iter()
                        .any(|offer| offer.target == offer_name)
            })
    }
}
