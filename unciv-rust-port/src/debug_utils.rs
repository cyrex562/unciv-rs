use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Debug utilities for the game
///
/// This module contains various debug flags that can be used during development
/// and testing. These flags should be set to false before committing code.
pub struct DebugUtils;

impl DebugUtils {
    /// This exists so that when debugging we can see the entire map.
    /// Remember to turn this to false before commit and upload!
    /// Or use the "secret" debug page of the options popup instead.
    pub static VISIBLE_MAP: AtomicBool = AtomicBool::new(false);

    /// This flag paints the tile coordinates directly onto the map tiles.
    pub static SHOW_TILE_COORDS: AtomicBool = AtomicBool::new(false);

    /// Flag to show tile image locations
    pub static SHOW_TILE_IMAGE_LOCATIONS: AtomicBool = AtomicBool::new(false);

    /// For when you need to test something in an advanced game and don't have time to faff around
    pub static SUPERCHARGED: AtomicBool = AtomicBool::new(false);

    /// Simulate until this turn on the first "Next turn" button press.
    /// Does not update World View changes until finished.
    /// Set to 0 to disable.
    pub static SIMULATE_UNTIL_TURN: AtomicI32 = AtomicI32::new(0);

    /// Get the value of VISIBLE_MAP
    pub fn is_visible_map() -> bool {
        VISIBLE_MAP.load(Ordering::Relaxed)
    }

    /// Set the value of VISIBLE_MAP
    pub fn set_visible_map(value: bool) {
        VISIBLE_MAP.store(value, Ordering::Relaxed);
    }

    /// Get the value of SHOW_TILE_COORDS
    pub fn is_show_tile_coords() -> bool {
        SHOW_TILE_COORDS.load(Ordering::Relaxed)
    }

    /// Set the value of SHOW_TILE_COORDS
    pub fn set_show_tile_coords(value: bool) {
        SHOW_TILE_COORDS.store(value, Ordering::Relaxed);
    }

    /// Get the value of SHOW_TILE_IMAGE_LOCATIONS
    pub fn is_show_tile_image_locations() -> bool {
        SHOW_TILE_IMAGE_LOCATIONS.load(Ordering::Relaxed)
    }

    /// Set the value of SHOW_TILE_IMAGE_LOCATIONS
    pub fn set_show_tile_image_locations(value: bool) {
        SHOW_TILE_IMAGE_LOCATIONS.store(value, Ordering::Relaxed);
    }

    /// Get the value of SUPERCHARGED
    pub fn is_supercharged() -> bool {
        SUPERCHARGED.load(Ordering::Relaxed)
    }

    /// Set the value of SUPERCHARGED
    pub fn set_supercharged(value: bool) {
        SUPERCHARGED.store(value, Ordering::Relaxed);
    }

    /// Get the value of SIMULATE_UNTIL_TURN
    pub fn get_simulate_until_turn() -> i32 {
        SIMULATE_UNTIL_TURN.load(Ordering::Relaxed)
    }

    /// Set the value of SIMULATE_UNTIL_TURN
    pub fn set_simulate_until_turn(value: i32) {
        SIMULATE_UNTIL_TURN.store(value, Ordering::Relaxed);
    }
}