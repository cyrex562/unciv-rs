use std::collections::HashMap;

use crate::trade::trade_offer::TradeOffer;

/// A list of trade offers with special handling for adding offers and calculating differences
#[derive(Clone, Default)]
pub struct TradeOffersList {
    /// The list of offers
    offers: Vec<TradeOffer>,
}

impl TradeOffersList {
    /// Create a new empty trade offers list
    pub fn new() -> Self {
        Self {
            offers: Vec::new(),
        }
    }

    /// Add an offer to the list
    ///
    /// If an equivalent offer (same name and type) already exists, the amounts are combined.
    /// If the combined amount is zero, the offer is removed.
    pub fn add(&mut self, element: TradeOffer) -> bool {
        // Find an equivalent offer (same name and type)
        if let Some(equivalent_offer) = self.offers.iter_mut().find(|it|
            it.name == element.name && it.trade_offer_type == element.trade_offer_type) {
            // Combine the amounts
            equivalent_offer.amount += element.amount;

            // Remove the offer if the amount is zero
            if equivalent_offer.amount == 0 {
                self.offers.retain(|it| it.name != element.name || it.trade_offer_type != element.trade_offer_type);
            }
        } else {
            // Add the new offer
            self.offers.push(element);
        }

        true
    }

    /// Calculate the difference between this list and another list
    ///
    /// Returns a new list containing all offers from this list and the negation of all offers from the other list
    pub fn without(&self, other_trade_offers_list: &TradeOffersList) -> TradeOffersList {
        let mut trade_offers_list_copy = TradeOffersList::new();

        // Add all offers from this list
        for offer in &self.offers {
            trade_offers_list_copy.add(offer.clone());
        }

        // Add the negation of all offers from the other list
        for offer in &other_trade_offers_list.offers {
            let mut negated_offer = offer.clone();
            negated_offer.amount = -negated_offer.amount;
            trade_offers_list_copy.add(negated_offer);
        }

        trade_offers_list_copy
    }

    /// Get all offers in the list
    pub fn get_offers(&self) -> &[TradeOffer] {
        &self.offers
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.offers.is_empty()
    }

    /// Get the number of offers in the list
    pub fn len(&self) -> usize {
        self.offers.len()
    }

    /// Iterate over the offers in the list
    pub fn iter(&self) -> std::slice::Iter<TradeOffer> {
        self.offers.iter()
    }

    /// Iterate over the offers in the list mutably
    pub fn iter_mut(&mut self) -> std::slice::IterMut<TradeOffer> {
        self.offers.iter_mut()
    }
}

impl From<Vec<TradeOffer>> for TradeOffersList {
    fn from(offers: Vec<TradeOffer>) -> Self {
        Self { offers }
    }
}

impl Into<Vec<TradeOffer>> for TradeOffersList {
    fn into(self) -> Vec<TradeOffer> {
        self.offers
    }
}

impl std::ops::Deref for TradeOffersList {
    type Target = Vec<TradeOffer>;

    fn deref(&self) -> &Self::Target {
        &self.offers
    }
}

impl std::ops::DerefMut for TradeOffersList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.offers
    }
}