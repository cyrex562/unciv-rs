use map::position::Position;
use serde::{Deserialize, Serialize};
use crate::action::UnitAction;
use crate::movement_profile::MovementProfile;
use crate::modifier::UnitModifier;
use crate::unit_move::UnitMove;

pub const INITIAL_EXPERIENCE_POINTS: u32 = 0;
pub const INITIAL_LEVEL: u32 = 1;
pub const INITIAL_MORALE: u32 = 100;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitState {
    pub health: u32,
    pub modifiers: Vec<UnitModifier>,
    pub position: Position,
    pub action_history: Vec<String>,
    pub experience_points: u32,
    pub level: u32,
    pub morale: u32,
    pub owning_faction_history: Vec<String>,
}

impl UnitState {
    pub fn new(health: u32, owner: &str) -> Self {
        let owning_faction_history = vec![owner.to_string()];
        Self {
            health,
            modifiers: Vec::new(),
            position: Position::default(),
            experience_points: INITIAL_EXPERIENCE_POINTS,
            level: INITIAL_LEVEL,
            action_history: Vec::new(),
            morale: INITIAL_MORALE,
            owning_faction_history,
            
        }
    }

    pub fn transported(&self) -> bool {
        unimplemented!()
    }

    pub fn automated(&self) -> bool {
        unimplemented!()
        // check whether the unit is currently performing automated actions.
    }

    pub fn escorting(&self) -> bool {
        unimplemented!()
        // check whether the unit is currently escorting another unit.
    }
}

