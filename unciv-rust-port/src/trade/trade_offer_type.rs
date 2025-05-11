/// Enum that classifies Trade Types
///
/// # Parameters
/// * `number_type` - How the value number is formatted - None, Simple number or with a Gold symbol
/// * `is_immediate` - Trade is a one-time effect without duration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeOfferType {
    /// Gold
    Gold,
    /// Gold per turn
    Gold_Per_Turn,
    /// Treaties are shared by both sides - like peace treaty and defensive pact
    Treaty,
    /// Agreements are one-sided, like open borders
    Agreement,
    /// Luxury resource
    Luxury_Resource,
    /// Strategic resource
    Strategic_Resource,
    /// Stockpiled resource
    Stockpiled_Resource,
    /// Technology
    Technology,
    /// Introduction to another civilization
    Introduction,
    /// Declaration of war
    WarDeclaration,
    /// City
    City,
}

impl TradeOfferType {
    /// Get the number type for this trade offer type
    pub fn number_type(&self) -> TradeTypeNumberType {
        match self {
            TradeOfferType::Gold | TradeOfferType::Gold_Per_Turn => TradeTypeNumberType::Gold,
            TradeOfferType::Treaty | TradeOfferType::Technology | TradeOfferType::Introduction |
            TradeOfferType::WarDeclaration | TradeOfferType::City => TradeTypeNumberType::None,
            _ => TradeTypeNumberType::Simple,
        }
    }

    /// Check if this trade offer type is immediate (one-time effect without duration)
    pub fn is_immediate(&self) -> bool {
        match self {
            TradeOfferType::Gold | TradeOfferType::Stockpiled_Resource |
            TradeOfferType::Technology | TradeOfferType::Introduction |
            TradeOfferType::WarDeclaration | TradeOfferType::City => true,
            _ => false,
        }
    }
}

/// How the value number is formatted for a trade offer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeTypeNumberType {
    /// No number is displayed
    None,
    /// Simple number is displayed
    Simple,
    /// Gold symbol is displayed with the number
    Gold,
}