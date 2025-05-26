use rules::build_cost::BuildCost;
use serde::{Deserialize, Serialize};
use crate::ability::Ability;
use crate::action::UnitAction;
use crate::movement_profile::MovementProfile;
use crate::attack_profile::AttackProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitClass {
    pub id: String,
    pub name: String,
    pub description: String,
    pub encyclopedia_entry_id: String,
    pub cost: BuildCost,
    // pub hurry_cost_modifier: i32, this value is now calculated dynamically
    pub move_profile: MovementProfile,
    // different types of attack that unit can do.
    pub attack_profiles: Vec<AttackProfile>,
    pub toughness: u32,
    pub required_tech: Option<String>,
    pub upgrades_to: Option<String>,
    pub replaces: Option<String>, // id of unit
    pub move_sound: Option<String>,
    pub action_points: u32,
    pub actions: Vec<UnitAction>,
    pub abilities: Vec<Ability>
}

impl UnitClass {
    pub fn new(cost: BuildCost, movement: MovementProfile, attack_profiles: &[AttackProfile], toughness: u32, required_tech: Option<String>, upgrades_to: Option<String>, replaces: Option<String>, id: &str, name: &str, description: &str, encyclopedia_entry_id: &str ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            encyclopedia_entry_id: encyclopedia_entry_id.to_string(),
            cost: BuildCost::default(),
            move_profile: MovementProfile::default(),
            attack_profiles: attack_profiles.clone().to_vec(),
            required_tech: None,
            upgrades_to: None,
            replaces: None,
            move_sound: None,
            action_points: 0,
            actions: vec![],
            toughness: 0,
            abilities: vec![],
        }
    }
}