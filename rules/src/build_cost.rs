use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BuildCost {
    pub money: i32,
    pub production: i32,
    pub resources: HashMap<String, i32>,
    pub faith: i32,
    pub culture: i32,
}