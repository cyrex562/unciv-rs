
use std::collections::{HashMap, HashSet};
use std::cmp::min;

/// Contains logic for automating trade decisions and actions
pub struct TradeAutomation;

impl TradeAutomation {
    /// Responds to trade requests from other civilizations
    pub fn respond_to_trade_requests(civ_info: &mut Civilization, trade_and_change_state: bool) {
        let trade_requests: Vec<_> = civ_info.trade_requests.iter().cloned().collect();

        for trade_request in trade_requests {
            let other_civ = civ_info.game_info.get_civilization(&trade_request.requesting_civ);

            // Treat 'no trade' state as if all trades are invalid - thus AIs will not update its "turns to offer"
            if !trade_and_change_state || !TradeEvaluation::is_trade_valid(&trade_request.trade, civ_info, &other_civ) {
                continue;
            }

            let mut trade_logic = TradeLogic::new(civ_info, &other_civ);
            trade_logic.current_trade.set(trade_request.trade.clone());

            // We need to remove this here, so that if the trade is accepted, the updateDetailedCivResources()
            // in tradeLogic.acceptTrade() will not consider *both* the trade *and the trade offer as decreasing the
            // amount of available resources, since that will lead to "Our proposed trade is no longer valid" if we try to offer
            // the same resource to ANOTHER civ in this turn. Complicated!
            civ_info.trade_requests.retain(|r| r != &trade_request);

            if TradeEvaluation::is_trade_acceptable(&trade_logic.current_trade, civ_info, &other_civ) {
                trade_logic.accept_trade();
                other_civ.add_notification(
                    format!("[{}] has accepted your trade request", civ_info.civ_name),
                    NotificationCategory::Trade,
                    NotificationIcon::Trade,
                    civ_info.civ_name.clone()
                );
            } else {
                if let Some(counteroffer) = Self::get_counteroffer(civ_info, &trade_request) {
                    other_civ.add_notification(
                        format!("[{}] has made a counteroffer to your trade request", civ_info.civ_name),
                        NotificationCategory::Trade,
                        NotificationIcon::Trade,
                        civ_info.civ_name.clone()
                    );
                    other_civ.trade_requests.push(counteroffer);
                } else {
                    other_civ.add_notification(
                        format!("[{}] has denied your trade request", civ_info.civ_name),
                        NotificationCategory::Trade,
                        civ_info.civ_name.clone(),
                        NotificationIcon::Trade
                    );
                    trade_request.decline(civ_info);
                }
            }
        }

        civ_info.trade_requests.clear();
    }

    /// Returns a TradeRequest with the same ourOffers as trade_request but with enough theirOffers
    /// added to make the deal acceptable. Will find a valid counteroffer if any exist, but is not
    /// guaranteed to find the best or closest one.
    fn get_counteroffer(civ_info: &Civilization, trade_request: &TradeRequest) -> Option<TradeRequest> {
        let other_civ = civ_info.game_info.get_civilization(&trade_request.requesting_civ);

        // AIs counteroffering each other could be problematic if they ping-pong back and forth forever
        // If this happens, that means our trade automation doesn't settle into an equilibrium that's favourable to both parties, so that should be updated when observed
        let evaluation = TradeEvaluation::new();
        let mut delta_in_our_favor = evaluation.get_trade_acceptability(&trade_request.trade, civ_info, &other_civ, true);

        if delta_in_our_favor > 0 {
            delta_in_our_favor = (delta_in_our_favor / 1.1) as i32; // They seem very interested in this deal, let's push it a bit.
        }

        let mut trade_logic = TradeLogic::new(civ_info, &other_civ);
        trade_logic.current_trade.set(trade_request.trade.clone());

        // What do they have that we would want?
        let mut potential_asks = HashMap::new();
        let mut counteroffer_asks = HashMap::new();
        let mut counteroffer_gifts = Vec::new();

        for offer in &trade_logic.their_available_offers {
            if (offer.offer_type == TradeOfferType::Gold || offer.offer_type == TradeOfferType::GoldPerTurn)
                && trade_request.trade.our_offers.iter().any(|o| o.offer_type == offer.offer_type) {
                continue; // Don't want to counteroffer straight gold for gold, that's silly
            }

            if !offer.is_tradable() {
                continue; // For example resources gained by trade or CS
            }

            if offer.offer_type == TradeOfferType::City {
                continue; // Players generally don't want to give up their cities, and they might misclick
            }

            if offer.offer_type == TradeOfferType::LuxuryResource {
                continue; // Don't ask for luxuries as counteroffer, players likely don't want to sell them if they didn't offer them already
            }

            if trade_logic.current_trade.their_offers.iter().any(|o| o.offer_type == offer.offer_type && o.name == offer.name) {
                continue; // So you don't get double offers of open borders declarations of war etc.
            }

            if offer.offer_type == TradeOfferType::Treaty {
                continue; // Don't try to counter with a defensive pact or research pact
            }

            let value = evaluation.evaluate_buy_cost_with_inflation(offer, civ_info, &other_civ, &trade_request.trade);
            if value > 0 {
                potential_asks.insert(offer.clone(), value);
            }
        }

        while !potential_asks.is_empty() && delta_in_our_favor < 0 {
            // Keep adding their worst offer until we get above the threshold
            let (offer_to_add, value) = potential_asks.iter()
                .min_by_key(|(_, v)| *v)
                .map(|(k, v)| (k.clone(), *v))
                .unwrap();

            delta_in_our_favor += value;
            counteroffer_asks.insert(offer_to_add, value);
            potential_asks.remove(&offer_to_add);
        }

        if delta_in_our_favor < 0 {
            return None; // We couldn't get a good enough deal
        }

        // At this point we are sure to find a good counteroffer
        while delta_in_our_favor > 0 {
            // Now remove the best offer valued below delta until the deal is barely acceptable
            let offer_to_remove = counteroffer_asks.iter()
                .filter(|(_, v)| **v <= delta_in_our_favor)
                .max_by_key(|(_, v)| *v)
                .map(|(k, v)| (k.clone(), *v));

            if let Some((offer, value)) = offer_to_remove {
                delta_in_our_favor -= value;
                counteroffer_asks.remove(&offer);
            } else {
                break; // Nothing more can be removed, at least en bloc
            }
        }

        // Only ask for enough of each resource to get maximum price
        for ask in counteroffer_asks.keys()
            .filter(|o| o.offer_type == TradeOfferType::LuxuryResource || o.offer_type == TradeOfferType::StrategicResource)
            .cloned()
            .collect::<Vec<_>>()
        {
            // Remove 1 amount as long as doing so does not change the price
            let original_value = counteroffer_asks[&ask];
            let mut modified_ask = ask.clone();

            while modified_ask.amount > 1 &&
                original_value == evaluation.evaluate_buy_cost_with_inflation(
                    &TradeOffer::new(
                        modified_ask.name.clone(),
                        modified_ask.offer_type,
                        modified_ask.amount - 1,
                        modified_ask.duration
                    ),
                    civ_info,
                    &other_civ,
                    &trade_request.trade
                )
            {
                modified_ask.amount -= 1;
            }

            if modified_ask.amount != ask.amount {
                counteroffer_asks.remove(&ask);
                counteroffer_asks.insert(modified_ask, original_value);
            }
        }

        // Adjust any gold asked for
        let mut to_remove = Vec::new();
        let gold_asks: Vec<_> = counteroffer_asks.keys()
            .filter(|o| o.offer_type == TradeOfferType::GoldPerTurn || o.offer_type == TradeOfferType::Gold)
            .sorted_by(|a, b| b.offer_type.cmp(&a.offer_type)) // Do GPT first
            .cloned()
            .collect();

        for gold_ask in gold_asks {
            let value_of_one = evaluation.evaluate_buy_cost_with_inflation(
                &TradeOffer::new(gold_ask.name.clone(), gold_ask.offer_type, 1, gold_ask.duration),
                civ_info,
                &other_civ,
                &trade_request.trade
            );

            let amount_can_be_removed = delta_in_our_favor / value_of_one;
            let mut modified_ask = gold_ask.clone();

            if amount_can_be_removed >= gold_ask.amount {
                delta_in_our_favor -= counteroffer_asks[&gold_ask];
                to_remove.push(gold_ask);
            } else {
                delta_in_our_favor -= value_of_one * amount_can_be_removed;
                modified_ask.amount -= amount_can_be_removed;
                counteroffer_asks.remove(&gold_ask);
                counteroffer_asks.insert(modified_ask, counteroffer_asks[&gold_ask]);
            }
        }

        for ask in to_remove {
            counteroffer_asks.remove(&ask);
        }

        // If the delta is still very in our favor consider sweetening the pot with some gold
        if delta_in_our_favor >= 100 {
            delta_in_our_favor = (delta_in_our_favor * 2) / 3; // Only compensate some of it though, they're the ones asking us

            // First give some GPT, then lump sum - but only if they're not already offering the same
            for our_gold in trade_logic.our_available_offers.iter()
                .filter(|o| o.is_tradable() && (o.offer_type == TradeOfferType::Gold || o.offer_type == TradeOfferType::GoldPerTurn))
                .sorted_by(|a, b| b.offer_type.cmp(&a.offer_type))
            {
                if !trade_logic.current_trade.their_offers.iter().any(|o| o.offer_type == our_gold.offer_type) &&
                    !counteroffer_asks.keys().any(|o| o.offer_type == our_gold.offer_type)
                {
                    let value_of_one = evaluation.evaluate_sell_cost_with_inflation(
                        &TradeOffer::new(our_gold.name.clone(), our_gold.offer_type, 1, our_gold.duration),
                        civ_info,
                        &other_civ,
                        &trade_request.trade
                    );

                    let amount_to_give = min(delta_in_our_favor / value_of_one, our_gold.amount);
                    delta_in_our_favor -= amount_to_give * value_of_one;

                    if amount_to_give > 0 {
                        counteroffer_gifts.push(
                            TradeOffer::new(
                                our_gold.name.clone(),
                                our_gold.offer_type,
                                amount_to_give,
                                our_gold.duration
                            )
                        );
                    }
                }
            }
        }

        trade_logic.current_trade.their_offers.extend(counteroffer_asks.keys().cloned());
        trade_logic.current_trade.our_offers.extend(counteroffer_gifts);

        // Trades reversed, because when *they* get it then the 'ouroffers' become 'theiroffers'
        Some(TradeRequest::new(civ_info.civ_name.clone(), trade_logic.current_trade.reverse()))
    }

    /// Exchanges luxury resources with other civilizations
    pub fn exchange_luxuries(civ_info: &mut Civilization) {
        let known_civs = civ_info.get_known_civs();

        // Player trades are... more complicated.
        // When the AI offers a trade, it's not immediately accepted,
        // so what if it thinks that it has a spare luxury and offers it to two human players?
        // What's to stop the AI "nagging" the player to accept a luxury trade?
        // We should A. add some sort of timer (20? 30 turns?) between luxury trade requests if they're denied - see DeclinedLuxExchange
        // B. have a way for the AI to keep track of the "pending offers" - see DiplomacyManager.resourcesFromTrade

        for other_civ in known_civs.iter()
            .filter(|c| c.is_major_civ() && !c.is_at_war_with(civ_info) &&
                !civ_info.get_diplomacy_manager(c).has_flag(DiplomacyFlags::DeclinedLuxExchange))
        {
            let is_enemy = civ_info.get_diplomacy_manager(other_civ).is_relationship_level_le(RelationshipLevel::Enemy);
            if is_enemy || other_civ.trade_requests.iter().any(|r| r.requesting_civ == civ_info.civ_name) {
                continue;
            }

            let trades = Self::potential_luxury_trades(civ_info, other_civ);
            for trade in trades {
                let trade_request = TradeRequest::new(civ_info.civ_name.clone(), trade.reverse());
                other_civ.trade_requests.push(trade_request);
            }
        }
    }

    /// Finds potential luxury trades between two civilizations
    fn potential_luxury_trades(civ_info: &Civilization, other_civ_info: &Civilization) -> Vec<Trade> {
        let trade_logic = TradeLogic::new(civ_info, other_civ_info);

        let our_tradable_luxury_resources: Vec<_> = trade_logic.our_available_offers.iter()
            .filter(|o| o.offer_type == TradeOfferType::LuxuryResource && o.amount > 1)
            .cloned()
            .collect();

        let their_tradable_luxury_resources: Vec<_> = trade_logic.their_available_offers.iter()
            .filter(|o| o.offer_type == TradeOfferType::LuxuryResource && o.amount > 1)
            .cloned()
            .collect();

        let we_have_they_dont: Vec<_> = our_tradable_luxury_resources.iter()
            .filter(|resource| {
                !trade_logic.their_available_offers.iter()
                    .any(|o| o.name == resource.name && o.offer_type == TradeOfferType::LuxuryResource)
            })
            .cloned()
            .collect();

        let they_have_we_dont: Vec<_> = their_tradable_luxury_resources.iter()
            .filter(|resource| {
                !trade_logic.our_available_offers.iter()
                    .any(|o| o.name == resource.name && o.offer_type == TradeOfferType::LuxuryResource)
            })
            .sorted_by(|a, b| {
                let a_count = civ_info.cities.iter()
                    .filter(|city| city.demanded_resource == Some(a.name.clone()))
                    .count();
                let b_count = civ_info.cities.iter()
                    .filter(|city| city.demanded_resource == Some(b.name.clone()))
                    .count();
                b_count.cmp(&a_count) // Prioritize resources that get WLTKD
            })
            .cloned()
            .collect();

        let mut trades = Vec::new();

        for i in 0..min(we_have_they_dont.len(), they_have_we_dont.len()) {
            let mut trade = Trade::new();
            trade.our_offers.push(we_have_they_dont[i].clone_with_amount(1));
            trade.their_offers.push(they_have_we_dont[i].clone_with_amount(1));
            trades.push(trade);
        }

        trades
    }
}

/// Helper trait for sorting collections
trait SortedBy<T> {
    fn sorted_by<F>(self, compare: F) -> Vec<T> where
        F: FnMut(&T, &T) -> std::cmp::Ordering;
}

impl<T> SortedBy<T> for Vec<T> where T: Clone {
    fn sorted_by<F>(self, mut compare: F) -> Vec<T> where
        F: FnMut(&T, &T) -> std::cmp::Ordering {
        let mut result = self;
        result.sort_by(compare);
        result
    }
}

/// Helper trait for collections
trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<K, V> IsEmpty for HashMap<K, V> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

/// Helper trait for collections
trait LastIndex {
    fn last_index(&self) -> usize;
}

impl<T> LastIndex for Vec<T> {
    fn last_index(&self) -> usize {
        self.len().saturating_sub(1)
    }
}