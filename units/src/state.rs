use map::position::Position;
use serde::{Deserialize, Serialize};
use crate::action::UnitAction;
use crate::movement_profile::MovementProfile;
use crate::modifier::UnitModifier;
use crate::unit_move::UnitMove;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitState {
    pub health: u32,
    
    pub modifiers: Vec<UnitModifier>,
    pub position: Position,
    pub action_history: Vec<UnitAction>,
    pub experience_points: u32,
    pub level: u32,
    pub morale: u32,
}

impl UnitState {
    pub fn new(health: u32) -> Self {
        Self {
            health,
            modifiers: Vec::new(),
            position: Position::default(),
            experience_points: 0,
            level: 1,
            action_history: Vec::new(),
            morale: 100,
        }
    }
}