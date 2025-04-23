use std::collections::VecDeque;
use crate::models::game_info::Position;

/// Represents barbarian encampments and units in the game.
pub struct Barbarians {
    pub encampments: VecDeque<Encampment>,
}

/// Represents a barbarian encampment.
pub struct Encampment {
    pub position: Position,
}