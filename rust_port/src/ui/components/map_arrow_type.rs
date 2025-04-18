use ggez::graphics::Color;
use serde::{Serialize, Deserialize};

/// Base trait for classes the instances of which signify a distinctive type of look and feel
/// with which to draw arrows on the map.
pub trait MapArrowType {}

/// Enum constants describing how/why a unit changed position. Each is also associated with an arrow type to draw on the map overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitMovementMemoryType {
    /// Unit moved normally
    UnitMoved,
    /// For when attacked, killed, and moved into tile.
    UnitAttacked,
    /// Caravel, destroyer, etc.
    UnitWithdrew,
    /// Paradrop, open borders end, air rebase, etc.
    UnitTeleported,
}

impl MapArrowType for UnitMovementMemoryType {}

/// Enum constants describing assorted commonly used arrow types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MiscArrowTypes {
    /// Unit is currently moving
    UnitMoving,
    /// For attacks that didn't result in moving into the target tile.
    /// E.G. Ranged, air strike, melee but the target survived, melee but not allowed in target terrain.
    UnitHasAttacked,
}

impl MapArrowType for MiscArrowTypes {}

/// Struct for arrow types signifying that a generic arrow style should be used and tinted.
/// Not currently used in core code, but allows one-off colour-coded arrows to be drawn without having to add a whole new texture and enum constant.
/// Could be useful for debug— Visualize what your AI is doing, or which tiles are affecting resource placement or whatever.
/// Also thinking of mod scripting.
#[derive(Debug, Clone)]
pub struct TintedMapArrow {
    /// The colour that the arrow should be tinted.
    pub color: Color,
}

impl MapArrowType for TintedMapArrow {}