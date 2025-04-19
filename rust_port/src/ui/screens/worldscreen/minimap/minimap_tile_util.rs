// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/minimap/MinimapTileUtil.kt

use egui::{Rect, Vec2};
use crate::ui::screens::worldscreen::minimap::MinimapTile;

/// Utility functions for minimap tiles
pub struct MinimapTileUtil;

impl MinimapTileUtil {
    /// Spreads out minimap tiles in a layer and returns the bounding rectangle
    pub fn spread_out_minimap_tiles(tile_layer: &mut egui::Grid, tiles: &[MinimapTile], tile_size: f32) -> Rect {
        let mut top_x = f32::NEG_INFINITY;
        let mut top_y = f32::NEG_INFINITY;
        let mut bottom_x = f32::INFINITY;
        let mut bottom_y = f32::INFINITY;

        for tile in tiles {
            // Add tile to the layer
            tile_layer.add(tile.draw());

            // Keep track of the current top/bottom/left/rightmost tiles to size and position the minimap correctly
            top_x = top_x.max(tile.position.x + tile_size);
            top_y = top_y.max(tile.position.y + tile_size);
            bottom_x = bottom_x.min(tile.position.x);
            bottom_y = bottom_y.min(tile.position.y);
        }

        Rect::from_min_size(
            Vec2::new(bottom_x, bottom_y),
            Vec2::new(top_x - bottom_x, top_y - bottom_y)
        )
    }
}