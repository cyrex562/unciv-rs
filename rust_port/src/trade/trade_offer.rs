use std::fmt;

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::models::ruleset::Speed;
use crate::models::translations::tr;
use crate::ui::components::fonts::Fonts;

/// Represents a single offer in a trade between civilizations
#[derive(Clone, Debug)]
pub struct TradeOffer {
    /// The name of the offer (resource name, city ID, etc.)
    pub name: String,
    /// The type of offer
    pub trade_offer_type: TradeOfferType,
    /// The amount of the offer (for resources, gold, etc.)
    pub amount: i32,
    /// The duration of the offer (for per-turn offers)
    pub duration: i32,
}

impl TradeOffer {
    /// Create a new trade offer with the given parameters
    pub fn new(name: String, trade_offer_type: TradeOfferType, amount: i32, speed: &Speed) -> Self {
        let duration = if trade_offer_type.is_immediate() {
            -1 // -1 for offers that are immediate (e.g. gold transfer)
        } else if name == Constants::peace_treaty {
            speed.peace_deal_duration
        } else {
            speed.deal_duration
        };

        Self {
            name,
            trade_offer_type,
            amount,
            duration,
        }
    }

    /// Create a new empty trade offer (for JSON deserialization)
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            trade_offer_type: TradeOfferType::Gold,
            amount: 1,
            duration: -1,
        }
    }

    /// Check if this offer is equal to another offer
    pub fn equals(&self, offer: &TradeOffer) -> bool {
        offer.name == self.name
            && offer.trade_offer_type == self.trade_offer_type
            && offer.amount == self.amount
    }

    /// Check if this offer is tradable (amount > 0)
    pub fn is_tradable(&self) -> bool {
        self.amount > 0
    }

    /// Get the text representation of this offer
    pub fn get_offer_text(&self, untradable: i32) -> String {
        let mut offer_text = match self.trade_offer_type {
            TradeOfferType::WarDeclaration => format!("Declare war on [{}]", self.name),
            TradeOfferType::Introduction => format!("Introduction to [{}]", self.name),
            TradeOfferType::City => {
                let city = UncivGame::current()
                    .game_info
                    .get_cities()
                    .iter()
                    .find(|it| it.id == self.name);

                match city {
                    Some(city) => format!("{{{}}} ({})", city.id, city.population.population),
                    None => "Non-existent city".to_string(),
                }
            },
            _ => self.name.clone(),
        };

        // Translate the offer text
        offer_text = tr(&offer_text, true);

        // Add amount information
        if self.trade_offer_type.number_type() == TradeTypeNumberType::Simple
            || self.trade_offer_type.number_type() == TradeTypeNumberType::Gold {
            offer_text.push_str(&format!(" ({})", self.amount));
        } else if self.name == Constants::research_agreement {
            offer_text.push_str(&format!(" (-{}{})", self.amount, Fonts::gold()));
        }

        // Add duration information
        if self.duration > 0 {
            offer_text.push_str(&format!("\n{}{}", self.duration, Fonts::turn()));
        }

        // Add untradable information
        if untradable == 1 {
            offer_text.push_str(&format!("\n+[{}] untradable copy", untradable));
        } else if untradable > 1 {
            offer_text.push_str(&format!("\n+[{}] untradable copies", untradable));
        }

        offer_text
    }
}

impl Default for TradeOffer {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for TradeOffer {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Eq for TradeOffer {}

/// The type of trade offer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeOfferType {
    /// Gold
    Gold,
    /// Gold per turn
    Gold_Per_Turn,
    /// Technology
    Technology,
    /// City
    City,
    /// Treaty (peace, research agreement, defensive pact)
    Treaty,
    /// Agreement (open borders)
    Agreement,
    /// Luxury resource
    Luxury_Resource,
    /// Strategic resource
    Strategic_Resource,
    /// Stockpiled resource
    Stockpiled_Resource,
    /// Introduction to another civilization
    Introduction,
    /// Declaration of war
    WarDeclaration,
}

impl TradeOfferType {
    /// Check if this offer type is immediate (not a per-turn offer)
    pub fn is_immediate(&self) -> bool {
        match self {
            TradeOfferType::Gold | TradeOfferType::Technology | TradeOfferType::City => true,
            _ => false,
        }
    }

    /// Get the number type for this offer type
    pub fn number_type(&self) -> TradeTypeNumberType {
        match self {
            TradeOfferType::Gold | TradeOfferType::Gold_Per_Turn => TradeTypeNumberType::Gold,
            TradeOfferType::Luxury_Resource | TradeOfferType::Strategic_Resource | TradeOfferType::Stockpiled_Resource => TradeTypeNumberType::Resource,
            _ => TradeTypeNumberType::Simple,
        }
    }
}

/// The type of number used for a trade offer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeTypeNumberType {
    /// Simple number (e.g. 1 city)
    Simple,
    /// Gold amount
    Gold,
    /// Resource amount
    Resource,
}