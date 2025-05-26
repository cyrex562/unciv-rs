use serde::{Deserialize, Serialize};
use crate::modifier::UnitModifier;
use crate::tile::tile::Tile;


pub enum MovementType {
    Land,
    WaterSurface,
    WaterSubsurface,
    Air,
}



#[derive(Default,Debug,Clone,Serialize,Deserialize)]/// Handles unit movement on the map.
pub struct MovementProfile {
    land_movement: u32,
    water_surface_movement: u32,
    water_subsurface_movement: u32,
    air_movement: u32,
    movement_modifiers: Vec<UnitModifier>
}


impl MovementProfile {
    /// Creates a new Movement instance with default values.
    pub fn new() -> Self {
        Self {
            land_movement: 0,
            water_surface_movement: 0,
            water_subsurface_movement: 0,
            air_movement: 0,
            movement_modifiers: Vec::new(),
        }
    }
}
