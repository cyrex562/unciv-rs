use serde::{Serialize, Deserialize};

/// Represents the type of popup alert in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertType {
    /// Player has been defeated
    Defeated,
    /// A wonder has been built
    WonderBuilt,
    /// A technology has been researched
    TechResearched,
    /// A war has been declared
    WarDeclaration,
    /// First contact with another civilization
    FirstContact,
    /// A city has been conquered
    CityConquered,
    /// A city has been traded
    CityTraded,
    /// Border conflict with another civilization
    BorderConflict,
    /// Tiles have been stolen by another civilization
    TilesStolen,
    /// Demand to stop settling cities near another civilization
    DemandToStopSettlingCitiesNear,
    /// City settled near another civilization despite our promise
    CitySettledNearOtherCivDespiteOurPromise,
    /// Demand to stop spreading religion
    DemandToStopSpreadingReligion,
    /// Religion spread despite our promise
    ReligionSpreadDespiteOurPromise,
    /// Golden age has begun
    GoldenAge,
    /// Declaration of friendship with another civilization
    DeclarationOfFriendship,
    /// Start intro alert
    StartIntro,
    /// Diplomatic marriage with another civilization
    DiplomaticMarriage,
    /// Bullied a protected minor civilization
    BulliedProtectedMinor,
    /// Attacked a protected minor civilization
    AttackedProtectedMinor,
    /// Attacked an allied minor civilization
    AttackedAllyMinor,
    /// Recaptured a civilian unit
    RecapturedCivilian,
    /// Game has been won
    GameHasBeenWon,
    /// Event alert
    Event,
}

/// Represents a popup alert in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupAlert {
    /// The type of alert
    pub alert_type: AlertType,
    /// The value associated with the alert
    pub value: String,
}

impl PopupAlert {
    /// Creates a new popup alert with the specified type and value
    pub fn new(alert_type: AlertType, value: String) -> Self {
        Self {
            alert_type,
            value,
        }
    }
}

impl Default for PopupAlert {
    /// Creates a default popup alert (for serialization)
    fn default() -> Self {
        Self {
            alert_type: AlertType::Event,
            value: String::new(),
        }
    }
}