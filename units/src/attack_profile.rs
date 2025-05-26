use serde::{Deserialize, Serialize};
use crate::effect::AreaOfEffect;
use crate::modifier::UnitModifier;
use crate::unit::AttackType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackProfile {
    id: String,
    name: String,
    description: String,
    range: u32, // in tiles, 0 is close combat
    line_of_sight: bool,
    // strength of the weapon, e.g. how much damage it can do.
    strength: u32,
    // when attacking, this is the chance to hit the target
    accuracy: u32,
    // the shape of the effect; in most cases this is a point, but certain types of weapons may have a different shape or even scatter
    area_of_effect: AreaOfEffect,
    sound: Option<String>,
    attack_types: Vec<AttackType>,
    modifiers: Vec<UnitModifier>
}

impl AttackProfile {
    pub fn new(id: &str, name: &str, description: &str, range: u32, line_of_sight: bool, strength: u32, accuracy: u32, area_of_effect: AreaOfEffect, sound: Option<String>, attack_types: &[AttackType]) -> Self {
        
        let sound_val = match sound {
            Some(s) => Some(s.clone()),
            None => None,
        };
        
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            range,
            line_of_sight,
            strength,
            accuracy,
            area_of_effect,
            sound: sound_val,
            attack_types: attack_types.clone().to_vec(),
        }
    }
}