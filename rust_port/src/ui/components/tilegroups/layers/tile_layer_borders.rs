use std::collections::HashMap;
use std::f32::consts::PI;
use ggez::graphics::{DrawParam, Drawable, Mesh, Rect};
use ggez::mint::Point2;
use ggez::Context;
use crate::logic::civilization::Civilization;
use crate::logic::map::tile::Tile;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::components::tilegroups::layers::tile_layer::{BaseTileLayer, TileLayer};
use crate::ui::images::ImageGetter;

/// A border segment for a tile
pub struct BorderSegment {
    /// The images that make up the border segment
    pub images: Vec<Mesh>,
    /// Whether the left side of the border is concave
    pub is_left_concave: bool,
    /// Whether the right side of the border is concave
    pub is_right_concave: bool,
}

impl BorderSegment {
    /// Create a new border segment
    pub fn new(images: Vec<Mesh>, is_left_concave: bool, is_right_concave: bool) -> Self {
        Self {
            images,
            is_left_concave,
            is_right_concave,
        }
    }
}

/// A layer that draws borders between tiles
pub struct TileLayerBorders {
    /// The base tile layer
    base: BaseTileLayer,
    /// The previous tile owner
    previous_tile_owner: Option<Civilization>,
    /// The border segments for each tile
    border_segments: HashMap<Tile, BorderSegment>,
}

impl TileLayerBorders {
    /// Create a new tile layer borders
    pub fn new(tile_group: TileGroup, size: f32) -> Self {
        Self {
            base: BaseTileLayer::new(tile_group, size),
            previous_tile_owner: None,
            border_segments: HashMap::new(),
        }
    }

    /// Reset the border segments
    pub fn reset(&mut self) {
        if !self.border_segments.is_empty() {
            for border_segment in self.border_segments.values() {
                for image in &border_segment.images {
                    // In Rust, we don't need to explicitly remove images
                    // as they will be dropped when the HashMap is cleared
                }
            }
            self.border_segments.clear();
        }
    }

    /// Get the left shared neighbor of this tile and the given neighbor
    fn get_left_shared_neighbor(&self, tile: &Tile, neighbor: &Tile) -> Option<&Tile> {
        let clock_position = tile.tile_map().get_neighbor_tile_clock_position(tile, neighbor);
        let left_clock_position = (clock_position - 2) % 12;
        tile.tile_map().get_clock_position_neighbor_tile(tile, left_clock_position)
    }

    /// Get the right shared neighbor of this tile and the given neighbor
    fn get_right_shared_neighbor(&self, tile: &Tile, neighbor: &Tile) -> Option<&Tile> {
        let clock_position = tile.tile_map().get_neighbor_tile_clock_position(tile, neighbor);
        let right_clock_position = (clock_position + 2) % 12;
        tile.tile_map().get_clock_position_neighbor_tile(tile, right_clock_position)
    }

    /// Update the borders
    fn update_borders(&mut self) {
        // This is longer than it could be, because of performance -
        // before fixing, about half (!) the time of update() was wasted on
        // removing all the border images and putting them back again!

        let tile = self.tile();
        let tile_owner = tile.get_owner();

        // If owner changed - clear previous borders
        if self.previous_tile_owner.as_ref().map(|c| c.id()) != tile_owner.as_ref().map(|c| c.id()) {
            self.reset();
        }

        self.previous_tile_owner = tile_owner.clone();

        // No owner - no borders
        if tile_owner.is_none() {
            return;
        }

        // Setup new borders
        let civ_outer_color = tile_owner.as_ref().unwrap().nation().outer_color();
        let civ_inner_color = tile_owner.as_ref().unwrap().nation().inner_color();

        for neighbor in tile.neighbors() {
            let mut should_remove_border_segment = false;
            let mut should_add_border_segment = false;

            let mut border_segment_should_be_left_concave = false;
            let mut border_segment_should_be_right_concave = false;

            let neighbor_owner = neighbor.get_owner();
            if neighbor_owner.as_ref().map(|c| c.id()) == tile_owner.as_ref().map(|c| c.id()) && self.border_segments.contains_key(neighbor) {
                // the neighbor used to not belong to us, but now it's ours
                should_remove_border_segment = true;
            } else if neighbor_owner.as_ref().map(|c| c.id()) != tile_owner.as_ref().map(|c| c.id()) {
                let left_shared_neighbor = self.get_left_shared_neighbor(tile, neighbor);
                let right_shared_neighbor = self.get_right_shared_neighbor(tile, neighbor);

                // If a shared neighbor doesn't exist (because it's past a map edge), we act as if it's our tile for border concave/convex-ity purposes.
                // This is because we do not draw borders against non-existing tiles either.
                border_segment_should_be_left_concave = left_shared_neighbor.is_none() ||
                    left_shared_neighbor.unwrap().get_owner().as_ref().map(|c| c.id()) == tile_owner.as_ref().map(|c| c.id());
                border_segment_should_be_right_concave = right_shared_neighbor.is_none() ||
                    right_shared_neighbor.unwrap().get_owner().as_ref().map(|c| c.id()) == tile_owner.as_ref().map(|c| c.id());

                if !self.border_segments.contains_key(neighbor) {
                    // there should be a border here but there isn't
                    should_add_border_segment = true;
                } else if let Some(border_segment) = self.border_segments.get(neighbor) {
                    if border_segment_should_be_left_concave != border_segment.is_left_concave ||
                        border_segment_should_be_right_concave != border_segment.is_right_concave {
                        // the concave/convex-ity of the border here is wrong
                        should_remove_border_segment = true;
                        should_add_border_segment = true;
                    }
                }
            }

            if should_remove_border_segment {
                self.border_segments.remove(neighbor);
            }

            if should_add_border_segment {
                let mut images = Vec::new();
                let border_segment = BorderSegment::new(
                    images.clone(),
                    border_segment_should_be_left_concave,
                    border_segment_should_be_right_concave
                );
                self.border_segments.insert(neighbor.clone(), border_segment);

                let border_shape_string = if border_segment_should_be_left_concave && border_segment_should_be_right_concave {
                    "Concave"
                } else if !border_segment_should_be_left_concave && !border_segment_should_be_right_concave {
                    "Convex"
                } else if !border_segment_should_be_left_concave && border_segment_should_be_right_concave {
                    "ConvexConcave"
                } else if border_segment_should_be_left_concave && !border_segment_should_be_right_concave {
                    "ConcaveConvex"
                } else {
                    panic!("This shouldn't happen?");
                };

                let relative_world_position = tile.tile_map().get_neighbor_tile_position_as_world_coords(tile, neighbor);

                let sign = if relative_world_position.x < 0.0 { -1.0 } else { 1.0 };
                let angle = sign * (relative_world_position.y.atan2(sign * relative_world_position.x) * 180.0 / PI - 90.0);

                let inner_border_image = ImageGetter::get_image(
                    self.strings().or_fallback(|| self.get_border(border_shape_string, "Inner"))
                );
                let mut inner_border_mesh = Mesh::new_rectangle(
                    &Context::new().unwrap(),
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                    ggez::graphics::Color::WHITE,
                ).unwrap();
                self.set_hexagon_size(&mut inner_border_mesh, None);
                inner_border_mesh.set_rotation(angle);
                inner_border_mesh.set_color(civ_outer_color);
                images.push(inner_border_mesh);

                let outer_border_image = ImageGetter::get_image(
                    self.strings().or_fallback(|| self.get_border(border_shape_string, "Outer"))
                );
                let mut outer_border_mesh = Mesh::new_rectangle(
                    &Context::new().unwrap(),
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(0.0, 0.0, 1.0, 1.0),
                    ggez::graphics::Color::WHITE,
                ).unwrap();
                self.set_hexagon_size(&mut outer_border_mesh, None);
                outer_border_mesh.set_rotation(angle);
                outer_border_mesh.set_color(civ_inner_color);
                images.push(outer_border_mesh);

                // Update the border segment with the new images
                if let Some(border_segment) = self.border_segments.get_mut(neighbor) {
                    border_segment.images = images;
                }
            }
        }
    }

    /// Get a border image
    fn get_border(&self, shape: &str, part: &str) -> String {
        format!("TileSet/Borders/{}/{}", shape, part)
    }
}

impl TileLayer for TileLayerBorders {
    fn tile_group(&self) -> &TileGroup {
        self.base.tile_group()
    }

    fn size(&self) -> f32 {
        self.base.size()
    }

    fn do_update(&mut self, viewing_civ: Option<&Civilization>, _local_unique_cache: &LocalUniqueCache) {
        self.update_borders();
    }

    fn has_children(&self) -> bool {
        !self.border_segments.is_empty()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.base.is_visible()
    }
}

impl Drawable for TileLayerBorders {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> ggez::GameResult {
        if !self.is_visible() {
            return Ok(());
        }

        for border_segment in self.border_segments.values() {
            for image in &border_segment.images {
                image.draw(ctx, param)?;
            }
        }

        Ok(())
    }

    fn dimensions(&self, _ctx: &mut Context) -> Option<Rect> {
        Some(Rect::new(0.0, 0.0, self.size(), self.size()))
    }
}