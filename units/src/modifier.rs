use serde::{Deserialize, Serialize};
use crate::movement_profile::MovementType;

// #[derive(Default,Debug,Clone,Serialize,Deserialize)]
// pub struct MovementModifier {
//     pub name: String,
//     pub description: String,
//     pub movement_type: MovementType,
//     pub value: i32,
//     // TODO: account for features on a tile, like a city, biome, fortifications, minefield, mountation, etc.
//     // TODO: account for proximity to other units
// }

// pub struct AttackStatModifier {
//     pub name: String,
//     pub description: String,
//     pub value: i32,
//     // TODO: circumstances under which this modifier applies - terrain, biome, vs unit type, proximity to other units, etc.
// }

// pub struct DefenseStatModifier {
//     pub name: String,
//     pub description: String,
//     pub value: i32,
//     // TODO: circumstances under which this modifier applies - terrain, biome, vs unit type, proximity to other units, etc.
// }

// pub struct AbilityModifier {
//     pub name: String,
//     pub description: String,
//     pub value: i32,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnitModifierKind {
    Bonus,
    Penalty,
    Resistance,
    Weakness
}

#[derive(Debug, Clone)]
pub enum UnitModifierType {
    Attack,
    Defense,
    Movement,
    Action,
    Ability
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitModifier {
    pub id: String,
    pub name: String,
    pub description: String,
    pub value: i32,
    pub modifier_type: UnitModifierType,
    pub kind: UnitModifierKind,
    // TODO: add what this modifies. -- i.e. the stat or ability it modifies
    // TODO: change value to allow operators like +=, -=, *=, /=, etc.
    // TODO: change value to allow for percentage modifiers
}

impl UnitModifier {
    pub fn new(id: String, name: String, description: String, value: i32, modifier_type: UnitModifierType, kind: UnitModifierKind) -> Self {
        Self {
            id,
            name,
            description,
            value,
            modifier_type,
            kind
        }
    }
}
