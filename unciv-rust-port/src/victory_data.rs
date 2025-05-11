/// Data about a victory in the game
#[derive(Clone, Debug)]
pub struct VictoryData {
    /// The name of the winning civilization
    pub winning_civ: String,
    /// The type of victory achieved
    pub victory_type: String,
    /// The turn on which the victory was achieved
    pub victory_turn: i32,
}

impl VictoryData {
    /// Create a new VictoryData
    pub fn new(winning_civ: String, victory_type: String, victory_turn: i32) -> Self {
        Self {
            winning_civ,
            victory_type,
            victory_turn,
        }
    }

    /// Default constructor for serialization
    pub fn default() -> Self {
        Self {
            winning_civ: String::new(),
            victory_type: String::new(),
            victory_turn: 0,
        }
    }
}