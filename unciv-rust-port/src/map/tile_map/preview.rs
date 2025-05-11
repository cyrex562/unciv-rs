use serde::{Deserialize, Serialize};
use crate::map_parameters::MapParameters;

/// A preview of a tile map with minimal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub map_parameters: MapParameters,
    // Add other preview fields as needed
}