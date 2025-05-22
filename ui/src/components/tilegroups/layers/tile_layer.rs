use std::option::Option;
use ggez::graphics::{DrawParam, Drawable, Mesh, Rect, Text};
use ggez::mint::Point2;
use ggez::Context;
use crate::logic::civilization::Civilization;
use crate::logic::map::tile::Tile;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::models::tilesets::TileSetCache;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::components::tilegroups::TileSetStrings;
use crate::ui::images::ImageGetter;

/// A trait for tile layers that can be drawn on the map
pub trait TileLayer: Drawable {
    /// Get the tile group associated with this layer
    fn tile_group(&self) -> &TileGroup;

    /// Get the size of the layer
    fn size(&self) -> f32;

    /// Get the tile associated with this layer
    fn tile(&self) -> &Tile {
        self.tile_group().tile()
    }

    /// Get the tile set strings associated with this layer
    fn strings(&self) -> &TileSetStrings {
        self.tile_group().tile_set_strings()
    }

    /// Set the hexagon size for an image
    fn set_hexagon_size(&self, image: &mut Mesh, scale: Option<f32>) {
        let tile_group = self.tile_group();
        let width = tile_group.hexagon_image_width();
        let height = image.height() * tile_group.hexagon_image_width() / image.width();

        image.set_bounds(Rect::new(
            tile_group.hexagon_image_position().x,
            tile_group.hexagon_image_position().y,
            width,
            height
        ));

        image.set_origin(Point2 {
            x: tile_group.hexagon_image_origin().x,
            y: tile_group.hexagon_image_origin().y,
        });

        let scale = scale.unwrap_or_else(|| TileSetCache::get_current().config().tile_scale());
        image.set_scale(scale, scale);
    }

    /// Check if the tile is viewable by the given civilization
    fn is_viewable(&self, viewing_civ: &Civilization) -> bool {
        self.tile_group().is_viewable(viewing_civ)
    }

    /// Update the layer with the given viewing civilization and local unique cache
    fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: Option<&LocalUniqueCache>) {
        let local_unique_cache = local_unique_cache.unwrap_or_else(|| &LocalUniqueCache::new(false));
        self.do_update(viewing_civ, local_unique_cache);
        self.determine_visibility();
    }

    /// Determine the visibility of the layer
    fn determine_visibility(&mut self) {
        // Default implementation - subclasses should override if needed
    }

    /// Update the layer with the given viewing civilization and local unique cache
    fn do_update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache);

    /// Check if the layer has children
    fn has_children(&self) -> bool;

    /// Set the visibility of the layer
    fn set_visible(&mut self, visible: bool);

    /// Get the visibility of the layer
    fn is_visible(&self) -> bool;
}

/// A base implementation of the TileLayer trait
pub struct BaseTileLayer {
    /// The tile group associated with this layer
    tile_group: TileGroup,
    /// The size of the layer
    size: f32,
    /// Whether the layer is visible
    visible: bool,
}

impl BaseTileLayer {
    /// Create a new base tile layer
    pub fn new(tile_group: TileGroup, size: f32) -> Self {
        Self {
            tile_group,
            size,
            visible: false,
        }
    }
}

impl TileLayer for BaseTileLayer {
    fn tile_group(&self) -> &TileGroup {
        &self.tile_group
    }

    fn size(&self) -> f32 {
        self.size
    }

    fn do_update(&mut self, _viewing_civ: Option<&Civilization>, _local_unique_cache: &LocalUniqueCache) {
        // Default implementation - subclasses should override
    }

    fn has_children(&self) -> bool {
        false // Default implementation - subclasses should override
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Drawable for BaseTileLayer {
    fn draw(&self, _ctx: &mut Context, _param: DrawParam) -> ggez::GameResult {
        // Default implementation - subclasses should override
        Ok(())
    }

    fn dimensions(&self, _ctx: &mut Context) -> Option<Rect> {
        Some(Rect::new(0.0, 0.0, self.size, self.size))
    }
}