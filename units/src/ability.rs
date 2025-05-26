// Modifications to abilities

use crate::effect::{AreaOfEffect, EffectRange};
use serde::{Deserialize, Serialize};

// Different abilities for units, like attack, heal, move, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub cooldown: u32,
    pub range: EffectRange,
    /// Area of effect for abilities that affect multiple tiles.
    pub area_of_effect: AreaOfEffect,
    pub additional_action_point_cost: u32, // number of action points beyond the use ability action cose (typically 1
    /// Duration in turns for abilities that last multiple turns.
    pub duration: u32,
    // TODO: in the future add a cost for abilities that need things like money, raw materials, etc.
}

impl Ability {
    /// Creates a new Ability instance with default values.
    pub fn new(id: &str, name: &str, description: &str, icon: &str, cooldown: u32, range: EffectRange, area_of_effect: AreaOfEffect, duration: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            icon: icon.to_string(),
            cooldown,
            range,
            area_of_effect,
            additional_action_point_cost: 0,
            duration,
        }
    }
}
