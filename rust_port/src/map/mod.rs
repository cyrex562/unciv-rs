pub mod astar;
pub mod map_visualization;
pub mod neighbor_direction;
pub mod unit_movement;
pub mod map_unit_cache; // Keep for backward compatibility
pub mod unit_cache; // New location
pub mod unit;

// Re-export for backward compatibility
pub use unit::MapUnit;
pub use unit_cache::MapUnitCache;
pub mod mapgenerator;
pub mod bfs;