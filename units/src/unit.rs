use serde::{Deserialize, Serialize};

use crate::state::UnitState;
use util::error::Error;
use crate::class::UnitClass;
use uuid::Uuid;

pub type UnitId = Uuid;

/// Represents the basic information of a unit as specified in Units.json,
/// in contrast to MapUnit which represents a specific unit on the map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub class: UnitClass,
    pub state: UnitState,
}

impl Unit {
    /// Creates a new BaseUnit with default values
    pub fn new(id: &str, name: &str, class: UnitClass, state: UnitState) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.to_string(),
            class,
            state,
        }
    }

    pub fn can_upgrade(&self) -> bool {
        // Check if the unit can be upgraded
        unimplemented!()
    }

    pub fn get_encyclopedia_entry_id(&self) -> Result<String, Error> {
        unimplemented!()
        // TODO: this method needs to look up a file or database by the encryclopedia _entry_id. If nothing is found, then it needs to return an Error Result
    }

    /// Gets the required techs for this unit
    pub fn required_techs(&self) -> Vec<String> {
        unimplemented!()
        // get the graph of techs required for this unit to be built.
    }
    
    pub fn make_obsole_techs(&self) -> Vec<String> {
        unimplemented!()
        // get the graph of techs that make this unit obsolete.
    }
    
    

}

