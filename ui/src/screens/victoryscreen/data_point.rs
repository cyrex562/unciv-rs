// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/LineChart.kt

use std::rc::Rc;
use crate::models::civilization::Civilization;

/// A data point for a line chart
#[derive(Debug, Clone)]
pub struct DataPoint<T> {
    /// X coordinate (typically turn number)
    pub x: T,
    /// Y coordinate (typically stat value)
    pub y: T,
    /// Civilization this data point belongs to
    pub civ: Rc<Civilization>,
}

impl<T> DataPoint<T> {
    /// Creates a new data point
    pub fn new(x: T, y: T, civ: Rc<Civilization>) -> Self {
        Self { x, y, civ }
    }
}