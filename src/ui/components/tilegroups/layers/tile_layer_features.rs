use std::collections::HashMap;
use std::f32::consts::PI;
use ggez::graphics::{DrawParam, Drawable, Mesh, Rect};
use ggez::mint::Point2;
use ggez::Context;
use crate::logic::civilization::Civilization;
use crate::logic::map::tile::{RoadStatus, Tile};
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::components::tilegroups::layers::tile_layer::{BaseTileLayer, TileLayer};
use crate::ui::images::ImageGetter;

/// A road image for a tile
struct RoadImage {
    /// The road status
    road_status: RoadStatus,
    /// The image mesh
    image: Option<Mesh>,
}

impl RoadImage {
    /// Create a new road image
    fn new() -> Self {
        Self {
            road_status: RoadStatus::None,
            image: None,
        }
    }
}

/// A map of road statuses to image paths
const ROADS_MAP: &[(&str, RoadStatus)] = &[
    ("TileSet/Roads/Road", RoadStatus::Road),
    ("TileSet/Roads/Railroad", RoadStatus::Railroad),
];

/// A layer that draws road features on tiles
pub struct TileLayerFeatures {
    /// The base tile layer
    base: BaseTileLayer,
    /// The road images for each tile
    road_images: HashMap<Tile, RoadImage>,
    /// The alpha value for the layer
    alpha: f32,
}

impl TileLayerFeatures {
    /// Create a new tile layer features
    pub fn new(tile_group: TileGroup, size: f32) -> Self {
        Self {
            base: BaseTileLayer::new(tile_group, size),
            road_images: HashMap::new(),
            alpha: 1.0,
        }
    }

    /// Update the road images
    fn update_road_images(&mut self) {
        if self.tile_group().is_for_map_editor_icon() {
            return;
        }

        let tile = self.tile();

        for neighbor in tile.neighbors() {
            let road_image = self.road_images.entry(neighbor.clone()).or_insert_with(RoadImage::new);

            let road_status = if tile.road_status() == RoadStatus::None || neighbor.road_status() == RoadStatus::None {
                RoadStatus::None
            } else if tile.road_status() == RoadStatus::Road || neighbor.road_status() == RoadStatus::Road {
                RoadStatus::Road
            } else {
                RoadStatus::Railroad
            };

            if road_image.road_status == road_status {
                continue; // the image is correct
            }

            road_image.road_status = road_status;

            // Clear the old image
            road_image.image = None;

            if road_status == RoadStatus::None {
                continue; // no road image
            }

            // Find the image path for the road status
            let image_path = ROADS_MAP.iter()
                .find(|(_, status)| *status == road_status)
                .map(|(path, _)| *path)
                .unwrap_or("TileSet/Roads/Road");

            let image = ImageGetter::get_image(self.strings().or_fallback(|| image_path.to_string()));

            // Create a mesh for the road
            let mut road_mesh = Mesh::new_rectangle(
                &Context::new().unwrap(),
                ggez::graphics::DrawMode::fill(),
                Rect::new(0.0, 0.0, 1.0, 1.0),
                ggez::graphics::Color::WHITE,
            ).unwrap();

            let relative_world_position = tile.tile_map().get_neighbor_tile_position_as_world_coords(tile, &neighbor);

            // This is some crazy voodoo magic so I'll explain.
            // Move road to center of tile
            road_mesh.set_position(Point2 { x: 25.0, y: 25.0 });

            // in addTiles, we set the position of groups by relative world position *0.8*groupSize, filter groupSize = 50
            // Here, we want to have the roads start HALFWAY THERE and extend towards the tiles, so we give them a position of 0.8*25.
            road_mesh.set_position(Point2 {
                x: 25.0 - relative_world_position.x * 0.8 * 25.0,
                y: 25.0 - relative_world_position.y * 0.8 * 25.0,
            });

            road_mesh.set_bounds(Rect::new(0.0, 0.0, 10.0, 6.0));
            road_mesh.set_origin(Point2 { x: 0.0, y: 3.0 }); // This is so that the rotation is calculated from the middle of the road and not the edge

            let angle = (180.0 / PI * relative_world_position.y.atan2(relative_world_position.x)) as f32;
            road_mesh.set_rotation(angle);

            road_image.image = Some(road_mesh);
        }
    }

    /// Dim the layer
    pub fn dim(&mut self) {
        self.alpha = 0.5;
    }
}

impl TileLayer for TileLayerFeatures {
    fn tile_group(&self) -> &TileGroup {
        self.base.tile_group()
    }

    fn size(&self) -> f32 {
        self.base.size()
    }

    fn do_update(&mut self, _viewing_civ: Option<&Civilization>, _local_unique_cache: &LocalUniqueCache) {
        self.update_road_images();
    }

    fn has_children(&self) -> bool {
        !self.road_images.is_empty()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.base.is_visible()
    }
}

impl Drawable for TileLayerFeatures {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> ggez::GameResult {
        if !self.is_visible() {
            return Ok(());
        }

        for road_image in self.road_images.values() {
            if let Some(image) = &road_image.image {
                let mut draw_param = param;
                draw_param.color.a = self.alpha;
                image.draw(ctx, draw_param)?;
            }
        }

        Ok(())
    }

    fn dimensions(&self, _ctx: &mut Context) -> Option<Rect> {
        Some(Rect::new(0.0, 0.0, self.size(), self.size()))
    }
}