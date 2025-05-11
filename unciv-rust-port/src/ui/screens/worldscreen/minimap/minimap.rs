// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/minimap/Minimap.kt

use egui::{Color32, Rect, Response, Ui, Vec2};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::game::map::Tile;
use crate::ui::screens::worldscreen::WorldScreen;
use super::minimap_tile::{MinimapTile, MinimapTileUtil};

/// Represents the minimap in the world screen
pub struct Minimap {
    /// The world screen reference
    pub world_screen: Rc<RefCell<WorldScreen>>,
    /// The tiles in the minimap
    pub tiles: Vec<MinimapTile>,
    /// The size of each tile in the minimap
    pub tile_size: f32,
    /// The position of the minimap
    pub position: Vec2,
    /// The size of the minimap
    pub size: Vec2,
    /// The color of the minimap background
    pub background_color: Color32,
    /// The color of the minimap border
    pub border_color: Color32,
    /// The width of the minimap border
    pub border_width: f32,
}

impl Minimap {
    /// Creates a new minimap
    pub fn new(
        world_screen: Rc<RefCell<WorldScreen>>,
        tile_size: f32,
        position: Vec2,
        size: Vec2,
    ) -> Self {
        Self {
            world_screen,
            tiles: Vec::new(),
            tile_size,
            position,
            size,
            background_color: Color32::from_rgba_premultiplied(0, 0, 0, 128),
            border_color: Color32::WHITE,
            border_width: 1.0,
        }
    }

    /// Updates the minimap tiles
    pub fn update_tiles(&mut self) {
        self.tiles.clear();
        let world_screen = self.world_screen.borrow();
        let map = world_screen.game.borrow().map();

        for tile in map.tiles.values() {
            let position = self.tile_to_minimap_position(&tile);
            let color = self.tile_to_color(&tile);
            let minimap_tile = MinimapTile::new(
                position,
                color,
                self.tile_size,
                self.world_screen.clone(),
                tile.clone(),
            );
            self.tiles.push(minimap_tile);
        }
    }

    /// Converts a tile position to a minimap position
    fn tile_to_minimap_position(&self, tile: &Tile) -> Vec2 {
        let map = self.world_screen.borrow().game.borrow().map();
        let map_size = map.size;
        let x = (tile.position.x as f32 / map_size.x as f32) * self.size.x;
        let y = (tile.position.y as f32 / map_size.y as f32) * self.size.y;
        Vec2::new(x, y) + self.position
    }

    /// Converts a tile to a color
    fn tile_to_color(&self, tile: &Tile) -> Color32 {
        if tile.is_water() {
            Color32::from_rgba_premultiplied(0, 0, 255, 255)
        } else if tile.is_land() {
            Color32::from_rgba_premultiplied(0, 255, 0, 255)
        } else {
            Color32::from_rgba_premultiplied(128, 128, 128, 255)
        }
    }

    /// Draws the minimap
    pub fn draw(&self) -> Response {
        let rect = Rect::from_min_size(self.position, self.size);
        let mut ui = self.world_screen.borrow_mut().ui();
        let response = ui.allocate_rect(rect, egui::Sense::click());

        // Draw background
        ui.painter().rect_filled(rect, 0.0, self.background_color);

        // Draw border
        ui.painter().rect_stroke(rect, 0.0, self.border_width, self.border_color);

        // Draw tiles
        for tile in &self.tiles {
            tile.draw();
        }

        response
    }
}