use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::f64::EPSILON;

use crate::constants::Constants;
use crate::civilization::Civilization;
use crate::civilization::diplomacy::{DiplomacyFlags, RelationshipLevel};
use crate::city::City;
use crate::game::game_info::GameInfo;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::trade::trade::Trade;
use crate::trade::trade_offer::TradeOffer;
use crate::trade::trade_offer_type::TradeOfferType;
use crate::automation::civilization::{
    DiplomacyAutomation, MotivationToAttackAutomation, DeclareWarPlanEvaluator,
};
use crate::ui::screens::victoryscreen::RankingType;

/// Evaluates trades between civilizations
pub struct TradeEvaluation;

impl TradeEvaluation {
    /// Check if a trade is valid
    ///
    /// # Parameters
    ///
    /// * `trade` - The trade to check
    /// * `offerer` - The civilization making the offer
    /// * `trade_partner` - The civilization receiving the offer
    ///
    /// # Returns
    ///
    /// True if the trade is valid, false otherwise
    pub fn is_trade_valid(trade: &Trade, offerer: &Civilization, trade_partner: &Civilization) -> bool {
        // Edge case time! Guess what happens if you offer a peace agreement to the AI for all their cities except for the capital,
        //  and then capture their capital THAT SAME TURN? It can agree, leading to the civilization getting instantly destroyed!
        // If a civ doen't has ever owned an original capital, which means it has not settle the first city yet,
        // it shouldn't be forbidden to trade with other civs owing to cities.size == 0.
        if (offerer.has_ever_owned_original_capital && trade.our_offers.iter().filter(|offer| offer.trade_offer_type == TradeOfferType::City).count() == offerer.cities.len())
            || (trade_partner.has_ever_owned_original_capital && trade.their_offers.iter().filter(|offer| offer.trade_offer_type == TradeOfferType::City).count() == trade_partner.cities.len()) {
            return false;
        }

        for offer in &trade.our_offers {
            if !Self::is_offer_valid(offer, offerer, trade_partner) {
                return false;
            }
        }

        for offer in &trade.their_offers {
            if !Self::is_offer_valid(offer, trade_partner, offerer) {
                return false;
            }
        }
        true
    }

    /// Check if an offer is valid
    ///
    /// # Parameters
    ///
    /// * `trade_offer` - The offer to check
    /// * `offerer` - The civilization making the offer
    /// * `trade_partner` - The civilization receiving the offer
    ///
    /// # Returns
    ///
    /// True if the offer is valid, false otherwise
    fn is_offer_valid(trade_offer: &TradeOffer, offerer: &Civilization, trade_partner: &Civilization) -> bool {
        let has_resource = |trade_offer: &TradeOffer| -> bool {
            let resources_by_name = offerer.get_civ_resources_by_name();
            resources_by_name.contains_key(&trade_offer.name) && resources_by_name[&trade_offer.name] >= trade_offer.amount
        };

        match trade_offer.trade_offer_type {
            // if they go a little negative it's okay, but don't allowing going overboard (promising same gold to many)
            TradeOfferType::Gold => (trade_offer.amount as f32 * 0.9) < offerer.gold as f32,
            TradeOfferType::Gold_Per_Turn => (trade_offer.amount as f32 * 0.9) < offerer.stats.stats_for_next_turn.gold as f32,
            TradeOfferType::Treaty => {
                // Current automation should prevent these from being offered anyway,
                //   these are a safeguard against future automation changes
                match trade_offer.name.as_str() {
                    Constants::peace_treaty => offerer.is_at_war_with(trade_partner),
                    Constants::research_agreement => !offerer.get_diplomacy_manager(trade_partner).unwrap().has_flag(DiplomacyFlags::ResearchAgreement),
                    Constants::defensive_pact => !offerer.get_diplomacy_manager(trade_partner).unwrap().has_flag(DiplomacyFlags::DefensivePact),
                    _ => true, // potential future treaties
                }
            }
            TradeOfferType::Agreement => true,
            TradeOfferType::Luxury_Resource => has_resource(trade_offer),
            TradeOfferType::Strategic_Resource => has_resource(trade_offer),
            TradeOfferType::Stockpiled_Resource => has_resource(trade_offer),
            TradeOfferType::Technology => true,
            TradeOfferType::Introduction => !trade_partner.knows(&trade_offer.name), // You can't introduce them to someone they already know!
            TradeOfferType::WarDeclaration => offerer.get_diplomacy_manager(&trade_offer.name).unwrap().can_declare_war(),
            TradeOfferType::City => offerer.cities.iter().any(|city| city.id == trade_offer.name),
        }
    }

    /// Check if a trade is acceptable
    ///
    /// # Parameters
    ///
    /// * `trade` - The trade to check
    /// * `evaluator` - The civilization evaluating the trade
    /// * `trade_partner` - The civilization being traded with
    ///
    /// # Returns
    ///
    /// True if the trade is acceptable, false otherwise
    pub fn is_trade_acceptable(trade: &Trade, evaluator: &Civilization, trade_partner: &Civilization) -> bool {
        Self::get_trade_acceptability(trade, evaluator, trade_partner, true) >= 0
    }

    /// Get the acceptability of a trade
    ///
    /// # Parameters
    ///
    /// * `trade` - The trade to evaluate
    /// * `evaluator` - The civilization evaluating the trade
    /// * `trade_partner` - The civilization being traded with
    /// * `include_diplomatic_gifts` - Whether to include diplomatic gifts in the evaluation
    ///
    /// # Returns
    ///
    /// The acceptability score of the trade
    pub fn get_trade_acceptability(trade: &Trade, evaluator: &Civilization, trade_partner: &Civilization, include_diplomatic_gifts: bool) -> i32 {
        let cities_asked_to_surrender = trade.our_offers.iter().filter(|offer| offer.trade_offer_type == TradeOfferType::City).count();
        let max_cities_to_surrender = (evaluator.cities.len() as f32 / 5.0).ceil() as usize;
        if cities_asked_to_surrender > max_cities_to_surrender {
            return i32::MIN;
        }

        let sum_of_their_offers = trade.their_offers.iter()
            .filter(|offer| offer.trade_offer_type != TradeOfferType::Treaty) // since treaties should only be evaluated once for 2 sides
            .map(|offer| Self::evaluate_buy_cost_with_inflation(offer, evaluator, trade_partner, trade))
            .sum::<i32>();

        let mut sum_of_our_offers = trade.our_offers.iter()
            .map(|offer| Self::evaluate_sell_cost_with_inflation(offer, evaluator, trade_partner, trade))
            .sum::<i32>();

        let relationship_level = evaluator.get_diplomacy_manager(trade_partner).unwrap().relationship_ignore_afraid();
        // If we're making a peace treaty, don't try to up the bargain for people you don't like.
        // Leads to spartan behaviour where you demand more, the more you hate the enemy...unhelpful
        if !trade.our_offers.iter().any(|offer| offer.name == Constants::peace_treaty || offer.name == Constants::research_agreement) {
            match relationship_level {
                RelationshipLevel::Enemy => sum_of_our_offers = (sum_of_our_offers as f32 * 1.5) as i32,
                RelationshipLevel::Unforgivable => sum_of_our_offers *= 2,
                _ => {}
            }
        }
        if trade.our_offers.iter().any(|offer| offer.name == Constants::defensive_pact) {
            match relationship_level {
                RelationshipLevel::Ally => {
                    //todo: Add more in depth evaluation here
                }
                _ => {
                    return i32::MIN;
                }
            }
        }
        let diplomatic_gifts = if include_diplomatic_gifts {
            evaluator.get_diplomacy_manager(trade_partner).unwrap().get_gold_gifts()
        } else {
            0
        };
        sum_of_their_offers - sum_of_our_offers + diplomatic_gifts
    }

    /// Evaluate the buy cost of an offer with inflation
    ///
    /// # Parameters
    ///
    /// * `offer` - The offer to evaluate
    /// * `civ_info` - The civilization evaluating the offer
    /// * `trade_partner` - The civilization being traded with
    /// * `trade` - The trade containing the offer
    ///
    /// # Returns
    ///
    /// The buy cost of the offer with inflation
    pub fn evaluate_buy_cost_with_inflation(offer: &TradeOffer, civ_info: &Civilization, trade_partner: &Civilization, trade: &Trade) -> i32 {
        if offer.trade_offer_type != TradeOfferType::Gold && offer.trade_offer_type != TradeOfferType::Gold_Per_Turn {
            (Self::evaluate_buy_cost(offer, civ_info, trade_partner, trade) as f32 / Self::get_gold_inflation(civ_info)) as i32
        } else {
            Self::evaluate_buy_cost(offer, civ_info, trade_partner, trade)
        }
    }

    /// Evaluate the buy cost of an offer
    ///
    /// # Parameters
    ///
    /// * `offer` - The offer to evaluate
    /// * `civ_info` - The civilization evaluating the offer
    /// * `trade_partner` - The civilization being traded with
    /// * `trade` - The trade containing the offer
    ///
    /// # Returns
    ///
    /// The buy cost of the offer
    fn evaluate_buy_cost(offer: &TradeOffer, civ_info: &Civilization, trade_partner: &Civilization, trade: &Trade) -> i32 {
        match offer.trade_offer_type {
            TradeOfferType::Gold => offer.amount,
            // GPT loses value for each 'future' turn, meaning: gold now is more valuable than gold in the future
            // Empire-wide production tends to grow at roughly 2% per turn (quick speed), so let's take that as a base line
            // Formula could be more sophisticated by taking into account game speed and estimated chance of the gpt-giver cancelling the trade after X amount of turns
            TradeOfferType::Gold_Per_Turn => {
                let mut sum = 0;
                for i in 1..=offer.duration {
                    sum += (offer.amount as f32 * 0.98_f32.powi(i as i32)) as i32;
                }
                sum
            }
            TradeOfferType::Treaty => {
                match offer.name.as_str() {
                    // Since it will be evaluated twice, once when they evaluate our offer and once when they evaluate theirs
                    Constants::peace_treaty => Self::evaluate_peace_cost_for_them(civ_info, trade_partner),
                    Constants::defensive_pact => 0,
                    Constants::research_agreement => -offer.amount,
                    _ => 1000,
                }
            }
            TradeOfferType::Luxury_Resource => {
                if civ_info.get_diplomacy_manager(trade_partner).unwrap().has_flag(DiplomacyFlags::ResourceTradesCutShort) {
                    return 0; // We don't trust you for resources
                }

                let lowest_explicit_buy_cost = civ_info.game_info.ruleset.tile_resources[&offer.name]
                    .get_matching_uniques(UniqueType::AiWillBuyAt, StateForConditionals::new(civ_info))
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .min();

                if let Some(cost) = lowest_explicit_buy_cost {
                    return cost;
                }

                let we_love_the_king_potential = civ_info.cities.iter().filter(|city| city.demanded_resource == offer.name).count() * 50;
                if !civ_info.has_resource(&offer.name) { // we can't trade on resources, so we are only interested in 1 copy for ourselves
                    we_love_the_king_potential as i32 + match civ_info.get_happiness() {
                        h if h < 0 => 450,
                        h if h < 10 => 350,
                        _ => 300, // Higher than corresponding sell cost since a trade is mutually beneficial!
                    }
                } else {
                    0
                }
            }
            TradeOfferType::Strategic_Resource => {
                if civ_info.get_diplomacy_manager(trade_partner).unwrap().has_flag(DiplomacyFlags::ResourceTradesCutShort) {
                    return 0; // We don't trust you for resources
                }

                let amount_willing_to_buy = 2 - civ_info.get_resource_amount(&offer.name);
                if amount_willing_to_buy <= 0 {
                    return 0; // we already have enough.
                }
                let amount_to_buy_in_offer = amount_willing_to_buy.min(offer.amount);

                let lowest_explicit_buy_cost = civ_info.game_info.ruleset.tile_resources[&offer.name]
                    .get_matching_uniques(UniqueType::AiWillBuyAt, StateForConditionals::new(civ_info))
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .min();

                if let Some(cost) = lowest_explicit_buy_cost {
                    return cost;
                }

                let can_use_for_buildings = civ_info.cities.iter().any(|city| {
                    city.city_constructions.get_buildable_buildings().iter().any(|building| {
                        building.get_resource_requirements_per_turn(&city.state).contains_key(&offer.name)
                    })
                });

                let can_use_for_units = civ_info.cities.iter().any(|city| {
                    city.city_constructions.get_constructable_units().iter().any(|unit| {
                        unit.get_resource_requirements_per_turn(&civ_info.state).contains_key(&offer.name)
                    })
                });

                if !can_use_for_buildings && !can_use_for_units {
                    return 0;
                }

                50 * amount_to_buy_in_offer
            }
            TradeOfferType::Stockpiled_Resource => {
                if let Some(resource) = civ_info.game_info.ruleset.tile_resources.get(&offer.name) {
                    let lowest_buy_cost = resource
                        .get_matching_uniques(UniqueType::AiWillBuyAt, StateForConditionals::new(civ_info))
                        .iter()
                        .map(|unique| unique.params[0].parse::<i32>().unwrap())
                        .min();
                    lowest_buy_cost.unwrap_or(0)
                } else {
                    0
                }
            }
            TradeOfferType::Technology => {
                // Currently unused
                (civ_info.game_info.ruleset.technologies[&offer.name].cost as f64).sqrt() as i32 * 20
            }
            TradeOfferType::Introduction => Self::introduction_value(&civ_info.game_info.ruleset),
            TradeOfferType::WarDeclaration => {
                let civ_to_declare_war_on = civ_info.game_info.get_civilization(&offer.name);
                if trade.their_offers.iter().any(|o| o.trade_offer_type == TradeOfferType::WarDeclaration && o.name == offer.name)
                    && trade.our_offers.iter().any(|o| o.trade_offer_type == TradeOfferType::WarDeclaration && o.name == offer.name) {
                    // Team war is handled in the selling method
                    0
                } else if civ_info.is_at_war_with(&civ_to_declare_war_on) {
                    // We shouldn't require them to pay us to join our war (no negative values)
                    (20.0 * DeclareWarPlanEvaluator::evaluate_join_our_war_plan(civ_info, &civ_to_declare_war_on, trade_partner, None)).max(0.0) as i32
                } else {
                    // Why should we pay you to go fight someone else?
                    0
                }
            }
            TradeOfferType::City => {
                let city = trade_partner.cities.iter().find(|city| city.id == offer.name)
                    .expect(&format!("Got an offer for city id {} which doesn't seem to exist for this civ!", offer.name));
                let surrounded = Self::surrounded_by_our_cities(city, civ_info);
                if civ_info.get_happiness() + city.city_stats.happiness_list.values.iter().sum::<i32>() < 0 {
                    return 0; // we can't really afford to go into negative happiness because of buying a city
                }
                let sum_of_pop = city.population.population;
                let sum_of_buildings = city.city_constructions.get_built_buildings().len();
                (sum_of_pop * 4 + sum_of_buildings as i32 + 4 + surrounded) * 100
            }
            TradeOfferType::Agreement => {
                if offer.name == Constants::open_borders {
                    100
                } else {
                    panic!("Invalid agreement type!");
                }
            }
        }
    }

    /// Check how many of our cities surround a city
    ///
    /// # Parameters
    ///
    /// * `city` - The city to check
    /// * `civ_info` - The civilization to check against
    ///
    /// # Returns
    ///
    /// The number of our cities surrounding the city
    fn surrounded_by_our_cities(city: &City, civ_info: &Civilization) -> i32 {
        let bordering_civs = Self::get_neighbouring_civs(city);
        if bordering_civs.len() == 1 && bordering_civs.contains(&civ_info.civ_name) {
            return 10 * civ_info.get_era_number(); // if the city is surrounded only by trading civ
        }
        if bordering_civs.contains(&civ_info.civ_name) {
            return 2 * civ_info.get_era_number(); // if the city has a border with trading civ
        }
        0
    }

    /// Get the neighboring civilizations of a city
    ///
    /// # Parameters
    ///
    /// * `city` - The city to check
    ///
    /// # Returns
    ///
    /// The set of neighboring civilization names
    fn get_neighbouring_civs(city: &City) -> HashSet<String> {
        let tiles_list: HashSet<&Tile> = city.get_tiles().iter().collect();
        let mut city_position_list = Vec::new();

        for tile in &tiles_list {
            for neighbor in &tile.neighbors {
                if !tiles_list.contains(neighbor) {
                    city_position_list.push(neighbor);
                }
            }
        }

        city_position_list.iter()
            .filter_map(|tile| tile.get_owner().map(|civ| civ.civ_name.clone()))
            .collect()
    }

    /// Evaluate the sell cost of an offer with inflation
    ///
    /// # Parameters
    ///
    /// * `offer` - The offer to evaluate
    /// * `civ_info` - The civilization evaluating the offer
    /// * `trade_partner` - The civilization being traded with
    /// * `trade` - The trade containing the offer
    ///
    /// # Returns
    ///
    /// The sell cost of the offer with inflation
    pub fn evaluate_sell_cost_with_inflation(offer: &TradeOffer, civ_info: &Civilization, trade_partner: &Civilization, trade: &Trade) -> i32 {
        if offer.trade_offer_type != TradeOfferType::Gold && offer.trade_offer_type != TradeOfferType::Gold_Per_Turn {
            (Self::evaluate_sell_cost(offer, civ_info, trade_partner, trade) as f32 / Self::get_gold_inflation(civ_info)) as i32
        } else {
            Self::evaluate_sell_cost(offer, civ_info, trade_partner, trade)
        }
    }

    /// Evaluate the sell cost of an offer
    ///
    /// # Parameters
    ///
    /// * `offer` - The offer to evaluate
    /// * `civ_info` - The civilization evaluating the offer
    /// * `trade_partner` - The civilization being traded with
    /// * `trade` - The trade containing the offer
    ///
    /// # Returns
    ///
    /// The sell cost of the offer
    fn evaluate_sell_cost(offer: &TradeOffer, civ_info: &Civilization, trade_partner: &Civilization, trade: &Trade) -> i32 {
        match offer.trade_offer_type {
            TradeOfferType::Gold => offer.amount,
            TradeOfferType::Gold_Per_Turn => offer.amount * offer.duration,
            TradeOfferType::Treaty => {
                match offer.name.as_str() {
                    // Since it will be evaluated twice, once when they evaluate our offer and once when they evaluate theirs
                    Constants::peace_treaty => Self::evaluate_peace_cost_for_them(civ_info, trade_partner),
                    Constants::defensive_pact => {
                        if DiplomacyAutomation::wants_to_sign_defensive_pact(civ_info, trade_partner) {
                            0
                        } else {
                            100000
                        }
                    }
                    Constants::research_agreement => -offer.amount,
                    _ => 1000,
                    //Todo:AddDefensiveTreatyHere
                }
            }
            TradeOfferType::Luxury_Resource => {
                let lowest_explicit_sell_cost = civ_info.game_info.ruleset.tile_resources[&offer.name]
                    .get_matching_uniques(UniqueType::AiWillSellAt, StateForConditionals::new(civ_info))
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .min();

                if let Some(cost) = lowest_explicit_sell_cost {
                    return cost;
                }

                if civ_info.get_resource_amount(&offer.name) > 1 {
                    250 // fair price
                } else if civ_info.has_unique(UniqueType::RetainHappinessFromLuxury) {
                    // If we retain 100% happiness, value it as a duplicate lux
                    600 - (civ_info.get_matching_uniques(UniqueType::RetainHappinessFromLuxury)
                        .first().unwrap().params[0].parse::<f32>().unwrap() * 350.0) as i32
                } else {
                    600 // you want to take away our last lux of this type?!
                }
            }
            TradeOfferType::Strategic_Resource => {
                if civ_info.game_info.space_resources.contains(&offer.name) &&
                    (civ_info.has_unique(UniqueType::EnablesConstructionOfSpaceshipParts) ||
                        trade_partner.has_unique(UniqueType::EnablesConstructionOfSpaceshipParts))
                {
                    return i32::MAX; // We'd rather win the game, thanks
                }

                let lowest_explicit_sell_cost = civ_info.game_info.ruleset.tile_resources[&offer.name]
                    .get_matching_uniques(UniqueType::AiWillSellAt, StateForConditionals::new(civ_info))
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .min();

                if let Some(cost) = lowest_explicit_sell_cost {
                    return cost;
                }

                if !civ_info.is_at_war() {
                    return 50 * offer.amount;
                }

                let can_use_for_units = civ_info.game_info.ruleset.units.values
                    .iter()
                    .any(|unit| unit.get_resource_requirements_per_turn(&civ_info.state).contains_key(&offer.name)
                        && unit.is_buildable(civ_info));
                if !can_use_for_units {
                    return 50 * offer.amount;
                }

                let amount_left = civ_info.get_resource_amount(&offer.name);

                // Each strategic resource starts costing 100 more when we ass the 5 resources baseline
                // That is to say, if I have 4 and you take one away, that's 200
                // take away the third, that's 300, 2nd 400, 1st 500

                // So if he had 5 left, and we want to buy 2, then we want to buy his 5th and 4th last resources,
                // So we'll calculate how much he'll sell his 4th for (200) and his 5th for (100)
                let mut total_cost = 0;

                // I know it's confusing, you're welcome to change to a more understandable way of counting if you can think of one...
                for number_of_resource in (amount_left - offer.amount + 1)..=amount_left {
                    total_cost += if number_of_resource > 5 {
                        100
                    } else {
                        (6 - number_of_resource) * 100
                    };
                }
                total_cost
            }
            TradeOfferType::Stockpiled_Resource => {
                if let Some(resource) = civ_info.game_info.ruleset.tile_resources.get(&offer.name) {
                    let lowest_sell_cost = resource
                        .get_matching_uniques(UniqueType::AiWillSellAt, StateForConditionals::new(civ_info))
                        .iter()
                        .map(|unique| unique.params[0].parse::<i32>().unwrap())
                        .min();
                    lowest_sell_cost.unwrap_or(i32::MAX)
                } else {
                    0
                }
            }
            TradeOfferType::Technology => (civ_info.game_info.ruleset.technologies[&offer.name].cost as f64).sqrt() as i32 * 20,
            TradeOfferType::Introduction => Self::introduction_value(&civ_info.game_info.ruleset),
            TradeOfferType::WarDeclaration => {
                let civ_to_declare_war_on = civ_info.game_info.get_civilization(&offer.name);
                if trade.their_offers.iter().any(|o| o.trade_offer_type == TradeOfferType::WarDeclaration && o.name == offer.name)
                    && trade.our_offers.iter().any(|o| o.trade_offer_type == TradeOfferType::WarDeclaration && o.name == offer.name) {
                    // Only accept if the war will benefit us, or if they pay us enough
                    // We shouldn't want to pay them for us to declare war (no negative values)
                    (-20.0 * DeclareWarPlanEvaluator::evaluate_team_war_plan(civ_info, &civ_to_declare_war_on, trade_partner, None)).max(0.0) as i32
                } else if trade_partner.is_at_war_with(&civ_to_declare_war_on) {
                    // We might want them to pay us to join them in war (no negative values)
                    (-20.0 * DeclareWarPlanEvaluator::evaluate_join_war_plan(civ_info, &civ_to_declare_war_on, trade_partner, None)).max(0.0) as i32
                } else {
                    // We might want them to pay us to declare war (no negative values)
                    (-25.0 * DeclareWarPlanEvaluator::evaluate_declare_war_plan(civ_info, &civ_to_declare_war_on, None)).max(0.0) as i32
                }
            }
            TradeOfferType::City => {
                let city = civ_info.cities.iter().find(|city| city.id == offer.name)
                    .expect(&format!("Got an offer to sell city id {} which doesn't seem to exist for this civ!", offer.name));

                let distance_bonus = Self::distance_city_trade_modifier(civ_info, city);
                let sum_of_pop = city.population.population;
                let sum_of_buildings = city.city_constructions.get_built_buildings().len();
                ((sum_of_pop * 4 + sum_of_buildings as i32 + 4 + distance_bonus) * 100).max(1000)
            }
            TradeOfferType::Agreement => {
                if offer.name == Constants::open_borders {
                    match civ_info.get_diplomacy_manager(trade_partner).unwrap().relationship_ignore_afraid() {
                        RelationshipLevel::Unforgivable => 10000,
                        RelationshipLevel::Enemy => 2000,
                        RelationshipLevel::Competitor => 500,
                        RelationshipLevel::Neutral | RelationshipLevel::Afraid => 200,
                        RelationshipLevel::Favorable | RelationshipLevel::Friend | RelationshipLevel::Ally => 100,
                    }
                } else {
                    panic!("Invalid agreement type!");
                }
            }
        }
    }

    /// Get the gold inflation for a civilization
    ///
    /// # Parameters
    ///
    /// * `civ_info` - The civilization to check
    ///
    /// # Returns
    ///
    /// The gold inflation value
    pub fn get_gold_inflation(civ_info: &Civilization) -> f32 {
        let modifier = 1000.0;
        let gold_per_turn = civ_info.stats.stats_for_next_turn.gold as f32;
        // To visualise the function, plug this into a 2d graphing calculator \frac{1000}{x^{1.2}+1.66*1000}
        // Goes from 1 at GPT = 0 to .923 at GPT = 100, .577 at GPT = 1000 and 0.415 at GPT = 10000
        // The current value of gold will never go below 40%, or the .4f that it is set to (being roughly the efficiency ratio between purchasing and upgrading units)
        // So this does not scale off to infinity
        modifier / (gold_per_turn.max(1.0).powf(1.2) + (1.66 * modifier)) + 0.4
    }

    /// Get the distance city trade modifier
    ///
    /// # Parameters
    ///
    /// * `civ_info` - The civilization to check
    /// * `city` - The city to check
    ///
    /// # Returns
    ///
    /// The distance city trade modifier
    fn distance_city_trade_modifier(civ_info: &Civilization, city: &City) -> i32 {
        let distance_to_capital = civ_info.get_capital().unwrap().get_center_tile().aerial_distance_to(city.get_center_tile());

        if distance_to_capital < 500 {
            0
        } else {
            (distance_to_capital - 500) * civ_info.get_era_number()
        }
    }

    /// Evaluate the peace cost for them
    ///
    /// # Parameters
    ///
    /// * `our_civ` - Our civilization
    /// * `other_civ` - The other civilization
    ///
    /// # Returns
    ///
    /// The peace cost for them
    pub fn evaluate_peace_cost_for_them(our_civ: &Civilization, other_civ: &Civilization) -> i32 {
        let our_combat_strength = our_civ.get_stat_for_ranking(RankingType::Force);
        let their_combat_strength = other_civ.get_stat_for_ranking(RankingType::Force);
        if our_combat_strength as f32 * 1.5 >= their_combat_strength as f32 && their_combat_strength as f32 * 1.5 >= our_combat_strength as f32 {
            return 0; // we're roughly equal, there's no huge power imbalance
        }
        if our_combat_strength > their_combat_strength {
            if MotivationToAttackAutomation::has_at_least_motivation_to_attack(our_civ, other_civ, 0.0) <= 0 {
                return 0;
            }
            let absolute_advantage = our_combat_strength - their_combat_strength;
            let percentage_advantage = absolute_advantage as f32 / their_combat_strength as f32;
            // We don't add the same constraint here. We should not make peace easily if we're
            // heavily advantaged.
            let total_advantage = (absolute_advantage as f32 * percentage_advantage) as i32;
            if total_advantage < 0 {
                // May be a negative number if strength disparity is such that it leads to integer overflow
                return 10000; // in that rare case, the AI would accept peace against a defeated foe.
            }
            (total_advantage as f32 / (Self::get_gold_inflation(other_civ) * 2.0)) as i32
        } else {
            // This results in huge values for large power imbalances. However, we should not give
            // up everything just because there is a big power imbalance. There's a better chance to
            // recover if you don't give away all your cities for example.
            //
            // Example A (this would probably give us away everything):
            // absoluteAdvantage = 10000 - 100 = 9500
            // percentageAdvantage = 9500 / 100 = 95
            // return -(9500 * 95) * 10 = -9025000
            //
            // Example B (this is borderline)
            // absoluteAdvantage = 10000 - 2500 = 7500
            // percentageAdvantage = 7500 / 2500 = 3
            // return -(7500 * 3) * 10 = -225000
            //
            // Example C (this is fine):
            // absoluteAdvantage = 10000 - 5000 = 5000
            // percentageAdvantage = 5000 / 5000 = 1
            // return -(5000 * 1) * 10 = -50000
            //
            // Hence we cap the max cost at 100k which equals about 2 or 3 cities in the mid game
            // (stats ~30 each)
            let absolute_advantage = their_combat_strength - our_combat_strength;
            let percentage_advantage = absolute_advantage as f32 / our_combat_strength as f32;
            (-(absolute_advantage as f32 * percentage_advantage / (Self::get_gold_inflation(our_civ) * 2.0)).min(10000.0)) as i32
        }
    }

    /// Get the introduction value
    ///
    /// # Parameters
    ///
    /// * `rule_set` - The ruleset to check
    ///
    /// # Returns
    ///
    /// The introduction value
    fn introduction_value(rule_set: &Ruleset) -> i32 {
        if let Some(unique) = rule_set.mod_options.get_matching_uniques(UniqueType::TradeCivIntroductions).first() {
            unique.params[0].parse::<i32>().unwrap()
        } else {
            0
        }
    }
}