// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/UndoHandler.kt

use std::rc::Rc;
use std::cell::RefCell;
use crate::logic::game::GameInfo;

/// Encapsulates the undo functionality.
///
/// The undo functionality works by saving clones of GameInfo.
/// These clones are saved in checkpoints, and can be restored by calling restoreCheckpoint.
pub struct UndoHandler {
    game_info: Rc<RefCell<GameInfo>>,
    checkpoints: Vec<GameInfo>,
}

impl UndoHandler {
    pub fn new(game_info: Rc<RefCell<GameInfo>>) -> Self {
        Self {
            game_info,
            checkpoints: Vec::new(),
        }
    }

    /// Returns true if an undo is possible.
    pub fn can_undo(&self) -> bool {
        !self.checkpoints.is_empty()
    }

    /// Records a checkpoint of the current game state.
    pub fn record_checkpoint(&mut self) {
        self.checkpoints.push(self.game_info.borrow().clone());
    }

    /// Restores the last checkpoint.
    pub fn restore_checkpoint(&mut self) {
        if let Some(checkpoint) = self.checkpoints.pop() {
            *self.game_info.borrow_mut() = checkpoint;
        }
    }

    /// Clears all checkpoints.
    pub fn clear_checkpoints(&mut self) {
        self.checkpoints.clear();
    }
}