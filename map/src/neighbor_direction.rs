use std::collections::HashMap;

/// Represents the six directions in a hexagonal grid using clock positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighborDirection {
    TopRight = 2,
    BottomRight = 4,
    Bottom = 6,
    BottomLeft = 8,
    TopLeft = 10,
    Top = 12,
}

impl NeighborDirection {
    /// Returns a map of clock positions to NeighborDirection values
    pub fn by_clock_position() -> HashMap<i32, NeighborDirection> {
        let mut map = HashMap::new();
        map.insert(2, NeighborDirection::TopRight);
        map.insert(4, NeighborDirection::BottomRight);
        map.insert(6, NeighborDirection::Bottom);
        map.insert(8, NeighborDirection::BottomLeft);
        map.insert(10, NeighborDirection::TopLeft);
        map.insert(12, NeighborDirection::Top);
        map
    }
}