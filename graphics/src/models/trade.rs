
/// Represents a trade offer in the game.
pub struct TradeOffer {
    pub target: String,
    pub offer_type: TradeOfferType,
    pub speed: i32,
}

impl TradeOffer {
    /// Creates a new trade offer.
    pub fn new(target: String, offer_type: TradeOfferType, speed: i32) -> Self {
        TradeOffer {
            target,
            offer_type,
            speed,
        }
    }
}

/// Represents a trade request in the game.
pub struct TradeRequest {
    pub from_civ: String,
    pub trade: Trade,
}

impl TradeRequest {
    /// Creates a new trade request.
    pub fn new(from_civ: String, trade: Trade) -> Self {
        TradeRequest {
            from_civ,
            trade,
        }
    }
}

/// Represents a trade in the game.
pub struct Trade {
    pub our_offers: Vec<TradeOffer>,
    pub their_offers: Vec<TradeOffer>,
}

impl Trade {
    /// Creates a new trade.
    pub fn new() -> Self {
        Trade {
            our_offers: Vec::new(),
            their_offers: Vec::new(),
        }
    }

    /// Reverses the trade, swapping our offers with their offers.
    pub fn reverse(&self) -> Trade {
        Trade {
            our_offers: self.their_offers.clone(),
            their_offers: self.our_offers.clone(),
        }
    }
}

/// Represents the type of a trade offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeOfferType {
    /// Declaration of war
    WarDeclaration,
    /// Gold payment
    Gold,
    /// Technology
    Technology,
    /// Resource
    Resource,
    /// City
    City,
    /// Peace treaty
    PeaceTreaty,
    /// Open borders
    OpenBorders,
    /// Defensive pact
    DefensivePact,
    /// Research agreement
    ResearchAgreement,
    /// Declaration of friendship
    DeclarationOfFriendship,
}

/// Contains logic for handling trades between civilizations.
pub struct TradeLogic<'a> {
    pub civ_info: &'a Civilization,
    pub other_civ: &'a Civilization,
    pub current_trade: Trade,
}

impl<'a> TradeLogic<'a> {
    /// Creates a new TradeLogic instance.
    pub fn new(civ_info: &'a Civilization, other_civ: &'a Civilization) -> Self {
        TradeLogic {
            civ_info,
            other_civ,
            current_trade: Trade::new(),
        }
    }
}