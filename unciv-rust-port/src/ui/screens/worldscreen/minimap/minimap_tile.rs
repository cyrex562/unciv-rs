// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/minimap/MinimapTile.kt

use egui::{Color32, Rect, Response, Ui, Vec2};
use crate::game::map::Tile;
use crate::ui::screens::worldscreen::WorldScreen;

/// Represents a tile in the minimap
pub struct MinimapTile {
    /// The position of the tile in the minimap
    pub position: Vec2,
    /// The color of the tile
    pub color: Color32,
    /// The size of the tile
    pub size: f32,
    /// The world screen reference
    pub world_screen: std::rc::Rc<std::cell::RefCell<WorldScreen>>,
    /// The tile reference
    pub tile: std::rc::Rc<std::cell::RefCell<Tile>>,
}

impl MinimapTile {
    /// Creates a new minimap tile
    pub fn new(
        position: Vec2,
        color: Color32,
        size: f32,
        world_screen: std::rc::Rc<std::cell::RefCell<WorldScreen>>,
        tile: std::rc::Rc<std::cell::RefCell<Tile>>,
    ) -> Self {
        Self {
            position,
            color,
            size,
            world_screen,
            tile,
        }
    }

    /// Draws the minimap tile and returns the response
    pub fn draw(&self) -> Response {
        let rect = Rect::from_min_size(
            self.position,
            Vec2::new(self.size, self.size)
        );

        let mut ui = self.world_screen.borrow_mut().ui();
        let response = ui.allocate_rect(rect, egui::Sense::click());

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Pointer);
        }

        if response.clicked() {
            let mut world_screen = self.world_screen.borrow_mut();
            let tile = self.tile.borrow();
            world_screen.map_screen.center_on_tile(&tile);
        }

        ui.painter().rect_filled(rect, 0.0, self.color);

        response
    }
}