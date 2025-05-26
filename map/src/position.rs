use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub z: i32,
}