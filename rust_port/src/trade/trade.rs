use std::collections::HashMap;

use crate::constants::Constants;
use crate::game::game_info::GameInfo;
use crate::civilization::Civilization;
use crate::civilization::diplomacy::DiplomacyFlags;
use crate::models::ruleset::nation::PersonalityValue;
use crate::trade::trade_offers_list::TradeOffersList;
use crate::trade::trade_offer_type::TradeOfferType;

/// A trade between two civilizations
#[derive(Debug, Clone)]
pub struct Trade {
    /// The offers made by the other civilization
    pub their_offers: TradeOffersList,
    /// The offers made by our civilization
    pub our_offers: TradeOffersList,
}

impl Trade {
    /// Create a new empty trade
    pub fn new() -> Self {
        Self {
            their_offers: TradeOffersList::new(),
            our_offers: TradeOffersList::new(),
        }
    }

    /// Reverse the trade, swapping our offers and their offers
    ///
    /// # Returns
    ///
    /// A new trade with the offers reversed
    pub fn reverse(&self) -> Self {
        let mut new_trade = Self::new();
        new_trade.their_offers.extend(self.our_offers.iter().map(|offer| offer.clone()));
        new_trade.our_offers.extend(self.their_offers.iter().map(|offer| offer.clone()));
        new_trade
    }

    /// Check if this trade is equal to another trade
    ///
    /// # Parameters
    ///
    /// * `trade` - The trade to compare with
    ///
    /// # Returns
    ///
    /// True if the trades are equal, false otherwise
    pub fn equal_trade(&self, trade: &Trade) -> bool {
        if trade.our_offers.len() != self.our_offers.len() || trade.their_offers.len() != self.their_offers.len() {
            return false;
        }

        for offer in &trade.our_offers {
            if !self.our_offers.iter().any(|o| o.equals(offer)) {
                return false;
            }
        }

        for offer in &trade.their_offers {
            if !self.their_offers.iter().any(|o| o.equals(offer)) {
                return false;
            }
        }

        true
    }

    /// Clone this trade
    ///
    /// # Returns
    ///
    /// A new trade with the same offers
    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.their_offers.extend(self.their_offers.iter().cloned());
        to_return.our_offers.extend(self.our_offers.iter().cloned());
        to_return
    }

    /// Set this trade to match another trade
    ///
    /// # Parameters
    ///
    /// * `trade` - The trade to copy
    pub fn set(&mut self, trade: &Trade) {
        self.our_offers.clear();
        self.our_offers.extend(trade.our_offers.iter().cloned());
        self.their_offers.clear();
        self.their_offers.extend(trade.their_offers.iter().cloned());
    }

    /// Check if this trade is a peace treaty
    ///
    /// # Returns
    ///
    /// True if this trade is a peace treaty, false otherwise
    pub fn is_peace_treaty(&self) -> bool {
        self.our_offers.iter().any(|offer| {
            offer.trade_offer_type == TradeOfferType::Treaty && offer.name == Constants::peace_treaty
        })
    }
}

/// A trade request from one civilization to another
#[derive(Debug, Clone)]
pub struct TradeRequest {
    /// The civilization making the request
    pub requesting_civ: String,
    /// The trade being offered
    pub trade: Trade,
}

impl TradeRequest {
    /// Create a new empty trade request
    pub fn new() -> Self {
        Self {
            requesting_civ: String::new(),
            trade: Trade::new(),
        }
    }

    /// Create a new trade request
    ///
    /// # Parameters
    ///
    /// * `requesting_civ` - The civilization making the request
    /// * `trade` - The trade being offered
    ///
    /// # Returns
    ///
    /// A new trade request
    pub fn with_trade(requesting_civ: String, trade: Trade) -> Self {
        Self {
            requesting_civ,
            trade,
        }
    }

    /// Decline a trade request
    ///
    /// # Parameters
    ///
    /// * `declining_civ` - The civilization declining the request
    pub fn decline(&self, declining_civ: &Civilization) {
        let requesting_civ_info = declining_civ.game_info.get_civilization(&self.requesting_civ);
        let requesting_civ_diplo_manager = requesting_civ_info.get_diplomacy_manager(declining_civ).unwrap();

        // The numbers of the flags (20,5) are the amount of turns to wait until offering again
        if self.trade.our_offers.iter().all(|offer| offer.trade_offer_type == TradeOfferType::Luxury_Resource)
            && self.trade.their_offers.iter().all(|offer| offer.trade_offer_type == TradeOfferType::Luxury_Resource)
        {
            requesting_civ_diplo_manager.set_flag(
                DiplomacyFlags::DeclinedLuxExchange,
                5 - (requesting_civ_info.get_personality()[PersonalityValue::Commerce] / 2) as i32,
            );
        }

        if self.trade.our_offers.iter().any(|offer| offer.name == Constants::research_agreement) {
            requesting_civ_diplo_manager.set_flag(
                DiplomacyFlags::DeclinedResearchAgreement,
                15 - requesting_civ_info.get_personality()[PersonalityValue::Science] as i32,
            );
        }

        if self.trade.our_offers.iter().any(|offer| offer.name == Constants::defensive_pact) {
            requesting_civ_diplo_manager.set_flag(DiplomacyFlags::DeclinedDefensivePact, 10);
        }

        if self.trade.our_offers.iter().any(|offer| offer.name == Constants::open_borders) {
            requesting_civ_diplo_manager.set_flag(
                DiplomacyFlags::DeclinedOpenBorders,
                if declining_civ.is_ai() { 5 } else { 10 },
            );
        }

        if self.trade.their_offers.iter().any(|offer| offer.trade_offer_type == TradeOfferType::WarDeclaration) {
            requesting_civ_diplo_manager.set_flag(
                DiplomacyFlags::DeclinedJoinWarOffer,
                if declining_civ.is_ai() { 5 } else { 10 },
            );
        }

        if self.trade.our_offers.iter().any(|offer| offer.trade_offer_type == TradeOfferType::WarDeclaration) {
            requesting_civ_diplo_manager.other_civ_diplomacy().set_flag(
                DiplomacyFlags::DeclinedJoinWarOffer,
                if declining_civ.is_ai() { 5 } else { 10 },
            );
        }

        if self.trade.is_peace_treaty() {
            requesting_civ_diplo_manager.set_flag(DiplomacyFlags::DeclinedPeace, 3);
        }
    }
}