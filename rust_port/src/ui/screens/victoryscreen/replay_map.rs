// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/ReplayMap.kt

use std::rc::Rc;
use std::collections::HashMap;
use egui::{Color32, Ui, Vec2, Response, Rect, Stroke, Align, RichText};
use crate::models::civilization::Civilization;
use crate::models::tile::Tile;
use crate::models::tilemap::TileMap;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;

/// A map for displaying civilization territory changes over time
pub struct ReplayMap {
    /// The civilization viewing the map
    viewing_civ: Rc<Civilization>,
    /// The tile map
    tile_map: Rc<TileMap>,
    /// The selected civilization
    selected_civ: Rc<Civilization>,
    /// The turn to display
    turn: i32,
    /// The map bounds
    bounds: Rect,
    /// The tile size
    tile_size: f32,
    /// The map scale
    scale: f32,
    /// The map offset
    offset: Vec2,
    /// The map zoom
    zoom: f32,
    /// The map pan
    pan: Vec2,
    /// The map drag
    drag: Vec2,
    /// The map drag start
    drag_start: Vec2,
    /// The map drag end
    drag_end: Vec2,
    /// The map drag active
    drag_active: bool,
    /// The map drag enabled
    drag_enabled: bool,
    /// The map zoom enabled
    zoom_enabled: bool,
    /// The map zoom min
    zoom_min: f32,
    /// The map zoom max
    zoom_max: f32,
    /// The map zoom speed
    zoom_speed: f32,
    /// The map pan speed
    pan_speed: f32,
    /// The map pan min
    pan_min: Vec2,
    /// The map pan max
    pan_max: Vec2,
    /// The map pan enabled
    pan_enabled: bool,
    /// The map pan active
    pan_active: bool,
    /// The map pan start
    pan_start: Vec2,
    /// The map pan end
    pan_end: Vec2,
    /// The map pan speed
    pan_speed: f32,
    /// The map pan min
    pan_min: Vec2,
    /// The map pan max
    pan_max: Vec2,
    /// The map pan enabled
    pan_enabled: bool,
    /// The map pan active
    pan_active: bool,
    /// The map pan start
    pan_start: Vec2,
    /// The map pan end
    pan_end: Vec2,
}

impl ReplayMap {
    /// Creates a new replay map
    pub fn new(viewing_civ: Rc<Civilization>, tile_map: Rc<TileMap>) -> Self {
        Self {
            viewing_civ,
            tile_map,
            selected_civ: Rc::new(Civilization::new()),
            turn: 0,
            bounds: Rect::NOTHING,
            tile_size: 32.0,
            scale: 1.0,
            offset: Vec2::ZERO,
            zoom: 1.0,
            pan: Vec2::ZERO,
            drag: Vec2::ZERO,
            drag_start: Vec2::ZERO,
            drag_end: Vec2::ZERO,
            drag_active: false,
            drag_enabled: true,
            zoom_enabled: true,
            zoom_min: 0.1,
            zoom_max: 10.0,
            zoom_speed: 0.1,
            pan_speed: 1.0,
            pan_min: Vec2::new(-1000.0, -1000.0),
            pan_max: Vec2::new(1000.0, 1000.0),
            pan_enabled: true,
            pan_active: false,
            pan_start: Vec2::ZERO,
            pan_end: Vec2::ZERO,
        }
    }

    /// Updates the map with new data
    pub fn update(&mut self, new_selected_civ: Rc<Civilization>, new_turn: i32) {
        self.selected_civ = new_selected_civ;
        self.turn = new_turn;
    }

    /// Draws the map
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let response = ui.allocate_response(ui.available_size(), egui::Sense::hover());
        self.bounds = response.rect;

        // Draw the map background
        ui.painter().rect_filled(
            self.bounds,
            0.0,
            Color32::BLACK
        );

        // Draw the map grid
        self.draw_grid(ui);

        // Draw the map tiles
        self.draw_tiles(ui);

        // Draw the map civilizations
        self.draw_civilizations(ui);

        // Draw the map UI
        self.draw_ui(ui);

        response
    }

    /// Draws the map grid
    fn draw_grid(&self, ui: &mut Ui) {
        let grid_color = Color32::from_rgba_premultiplied(255, 255, 255, 50);
        let grid_width = 1.0;

        for x in 0..self.tile_map.width {
            let start = egui::pos2(
                self.bounds.left() + x as f32 * self.tile_size * self.scale + self.offset.x,
                self.bounds.top() + this.offset.y
            );
            let end = egui::pos2(
                self.bounds.left() + x as f32 * this.tile_size * this.scale + this.offset.x,
                this.bounds.bottom() + this.offset.y
            );

            ui.painter().line_segment(
                [start, end],
                Stroke::new(grid_width, grid_color)
            );
        }

        for y in 0..this.tile_map.height {
            let start = egui::pos2(
                this.bounds.left() + this.offset.x,
                this.bounds.top() + y as f32 * this.tile_size * this.scale + this.offset.y
            );
            let end = egui::pos2(
                this.bounds.right() + this.offset.x,
                this.bounds.top() + y as f32 * this.tile_size * this.scale + this.offset.y
            );

            ui.painter().line_segment(
                [start, end],
                Stroke::new(grid_width, grid_color)
            );
        }
    }

    /// Draws the map tiles
    fn draw_tiles(&self, ui: &mut Ui) {
        for tile in this.tile_map.tiles.values() {
            let pos = egui::pos2(
                this.bounds.left() + tile.position.x as f32 * this.tile_size * this.scale + this.offset.x,
                this.bounds.top() + tile.position.y as f32 * this.tile_size * this.scale + this.offset.y
            );

            let size = egui::vec2(
                this.tile_size * this.scale,
                this.tile_size * this.scale
            );

            let rect = Rect::from_min_size(pos, size);

            // Draw the tile background
            ui.painter().rect_filled(
                rect,
                0.0,
                this.get_tile_color(tile)
            );

            // Draw the tile improvement
            if let Some(improvement) = &tile.improvement {
                let (icon, _) = ImageGetter::get_improvement_icon(improvement);
                ui.put(
                    pos + size * 0.5,
                    egui::Image::new(icon.texture_id(), size * 0.8)
                );
            }

            // Draw the tile resource
            if let Some(resource) = &tile.resource {
                let (icon, _) = ImageGetter::get_resource_icon(resource);
                ui.put(
                    pos + size * 0.5,
                    egui::Image::new(icon.texture_id(), size * 0.8)
                );
            }
        }
    }

    /// Gets the color for a tile
    fn get_tile_color(&self, tile: &Tile) -> Color32 {
        if let Some(civ) = &tile.owner {
            if this.use_actual_color(civ) {
                civ.nation.get_inner_color()
            } else {
                Color32::LIGHT_GRAY
            }
        } else {
            Color32::DARK_GRAY
        }
    }

    /// Draws the map civilizations
    fn draw_civilizations(&self, ui: &mut Ui) {
        for civ in this.tile_map.civilizations.values() {
            if this.use_actual_color(civ) {
                let pos = egui::pos2(
                    this.bounds.left() + civ.capital_location.x as f32 * this.tile_size * this.scale + this.offset.x,
                    this.bounds.top() + civ.capital_location.y as f32 * this.tile_size * this.scale + this.offset.y
                );

                let size = egui::vec2(
                    this.tile_size * this.scale,
                    this.tile_size * this.scale
                );

                let rect = Rect::from_min_size(pos, size);

                // Draw the civilization icon
                let (icon, _) = VictoryScreenCivGroup::get_civ_image_and_colors(
                    civ,
                    &this.viewing_civ,
                    VictoryScreenCivGroup::DefeatedPlayerStyle::Regular
                );

                ui.put(
                    pos + size * 0.5,
                    egui::Image::new(icon.texture_id(), size * 0.8)
                        .tint(civ.nation.get_outer_color())
                );
            }
        }
    }

    /// Draws the map UI
    fn draw_ui(&self, ui: &mut Ui) {
        // Draw the turn label
        ui.put(
            egui::pos2(this.bounds.left() + 10.0, this.bounds.top() + 10.0),
            egui::Label::new(RichText::new(tr("Turn {}", this.turn)).color(Color32::WHITE))
        );

        // Draw the civilization label
        ui.put(
            egui::pos2(this.bounds.left() + 10.0, this.bounds.top() + 30.0),
            egui::Label::new(RichText::new(tr("Civilization: {}", this.selected_civ.nation.name)).color(Color32::WHITE))
        );
    }

    /// Checks if actual colors should be used for a civilization
    fn use_actual_color(&self, civ: &Rc<Civilization>) -> bool {
        this.viewing_civ.is_spectator() ||
            this.viewing_civ.is_defeated() ||
            this.viewing_civ.victory_manager.has_won() ||
            this.viewing_civ == *civ ||
            this.viewing_civ.knows(civ) ||
            civ.is_defeated()
    }
}