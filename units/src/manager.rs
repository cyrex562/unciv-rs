use crate::unit_move::UnitMove;

pub struct MovementOrder {
    pub unit_id: String,
    pub from_tile: String,
    pub to_tile: String,
}

pub struct UnitMovesRemainingReport {
    pub unit_id: String,
    pub land_moves_left: u32,
    pub water_surface_moves_left: u32,
    pub water_subsurface_moves_left: u32,
    pub air_moves_left: u32,
}

pub struct PlannedUnitMove {
    pub unit_id: String,
    pub from_tile: String,
    pub to_tile: String,
}

pub struct UnitManager {
    // stores the moves made by units this turn
    pub current_moves: Vec<UnitMove>,
    // stores the moves made by units in previous turns
    pub past_moves: Vec<UnitMove>,
    // stores planned moves for units, which may take multiple turns to complete
    pub planned_moves: Vec<PlannedUnitMove>
}

impl UnitManager {
    pub fn new() -> Self {
        Self {
            current_moves: Vec::new(),
            past_moves: Vec::new(),
            planned_moves: Vec::new(),
        }
    }



    /// Checks how many movement points are left for units in the manager.
    pub fn unit_moves_left(&self, unit_id: &str) -> UnitMovesRemainingReport {
        unimplemented!()
        // TODO: find all the moves made by a unit this turn and substract them from the unit's total movement points. If the unit has not made any moves, return the unit's total movement points.
    }

    pub fn has_moves_left(&self, unit_id: &str) -> bool {
        unimplemented!()
        // get a movement report for the unit and check if it has any moves left
    }

    pub fn get_turns_left_for_planned_move(&self, unit_id: &str) -> Option<i32> {
        unimplemented!()
        // TODO: look up a planned move for the unit and caluclate the number of turns remaining to reach the planned destination tile. If the unit doesnt have a planned move, return None.
    }

    pub fn can_reach(&self, unit_id: &str, order: &MovementOrder) -> bool {
        // check if the unit can reach the destination tile.
        unimplemented!()
    }

    /// Executes all moves in the manager
    pub fn move_unit(&mut self, unit_id: &str, order: &MovementOrder) -> Result<(), String> {
        // Find the unit move in current moves
        if let Some(unit_move) = self.current_moves.iter_mut().find(|m| m.unit_id == unit_id && m.to_tile == order.to_tile) {
            // Check if the move is valid
            if unit_move.is_valid() {
                // Execute the move logic here (e.g., update unit position, reduce movement points)
                // For now, just return Ok
                Ok(())
            } else {
                Err("Invalid move".to_string())
            }
        } else {
            Err("Unit move not found".to_string())
        }
    }
}

// TODO: account for a movement that takes multiple turns, like a ship crossing the ocean or a unit moving through a mountain range

// TODO: on end of turn, move all current moves to past moves and clear current moves
// TODO: figure out mechanic where player has planned a move but not asked the unit to move yet, and conduct all or part of that move at the end of the turn.