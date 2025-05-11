use ggez::graphics::{Color, DrawParam, Image, Mesh, Rect, Text};
use ggez::Context;
use std::sync::Arc;
use std::f32;

use crate::constants::Constants;
use crate::models::civilization::Civilization;
use crate::models::map::tile::Tile;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::tilegroups::layers::*;
use crate::ui::utils::debug_utils::DebugUtils;

// TileGroup struct
pub struct TileGroup {
    pub tile: Arc<Tile>,
    pub tile_set_strings: TileSetStrings,
    pub group_size: f32,
    pub hexagon_image_width: f32,
    pub hexagon_image_origin: (f32, f32),
    pub hexagon_image_position: (f32, f32),
    pub is_force_visible: bool,
    pub is_for_map_editor_icon: bool,
    pub layer_terrain: TileLayerTerrain,
    pub layer_features: TileLayerFeatures,
    pub layer_borders: TileLayerBorders,
    pub layer_misc: TileLayerMisc,
    pub layer_resource: TileLayerResource,
    pub layer_improvement: TileLayerImprovement,
    pub layer_yield: TileLayerYield,
    pub layer_overlay: TileLayerOverlay,
    pub layer_unit_art: TileLayerUnitSprite,
    pub layer_unit_flag: TileLayerUnitFlag,
    pub layer_city_button: TileLayerCityButton,
    pub width: f32,
    pub height: f32,
    pub is_transform: bool,
}

impl TileGroup {
    pub fn new(tile: Arc<Tile>, tile_set_strings: TileSetStrings, group_size: Option<f32>) -> Self {
        let group_size = group_size.unwrap_or(TileGroupMap::group_size() + 4.0);

        // Calculate hexagon dimensions
        let hexagon_image_width = group_size * 1.5;
        let hexagon_image_origin = (
            hexagon_image_width / 2.0,
            ((hexagon_image_width / 2.0).powf(2.0) - (hexagon_image_width / 4.0).powf(2.0)).sqrt()
        );
        let hexagon_image_position = (
            -hexagon_image_origin.0 / 3.0,
            -hexagon_image_origin.1 / 4.0
        );

        let mut group = Self {
            tile,
            tile_set_strings,
            group_size,
            hexagon_image_width,
            hexagon_image_origin,
            hexagon_image_position,
            is_force_visible: DebugUtils::VISIBLE_MAP,
            is_for_map_editor_icon: false,
            layer_terrain: TileLayerTerrain::new(Arc::new(group.clone()), group_size),
            layer_features: TileLayerFeatures::new(Arc::new(group.clone()), group_size),
            layer_borders: TileLayerBorders::new(Arc::new(group.clone()), group_size),
            layer_misc: TileLayerMisc::new(Arc::new(group.clone()), group_size),
            layer_resource: TileLayerResource::new(Arc::new(group.clone()), group_size),
            layer_improvement: TileLayerImprovement::new(Arc::new(group.clone()), group_size),
            layer_yield: TileLayerYield::new(Arc::new(group.clone()), group_size),
            layer_overlay: TileLayerOverlay::new(Arc::new(group.clone()), group_size),
            layer_unit_art: TileLayerUnitSprite::new(Arc::new(group.clone()), group_size),
            layer_unit_flag: TileLayerUnitFlag::new(Arc::new(group.clone()), group_size),
            layer_city_button: TileLayerCityButton::new(Arc::new(group.clone()), group_size),
            width: group_size,
            height: group_size,
            is_transform: false,
        };

        // Initialize layers
        group.init();

        group
    }

    fn init(&mut self) {
        // Set size
        self.width = self.group_size;
        self.height = self.group_size;

        // Cannot be a NonTransformGroup as this causes font-rendered terrain to be upside-down
        self.is_transform = false;

        // Add all layers
        self.add_all_layers();

        // Update terrain layer
        self.layer_terrain.update(None, &LocalUniqueCache::new(false));
    }

    fn add_all_layers(&mut self) {
        // In Rust, we don't need to explicitly add actors to a group
        // The layers are already part of the TileGroup struct
    }

    pub fn clone(&self) -> Self {
        Self::new(self.tile.clone(), self.tile_set_strings.clone(), Some(self.group_size))
    }

    pub fn is_viewable(&self, viewing_civ: &Civilization) -> bool {
        self.is_force_visible
            || viewing_civ.viewable_tiles.contains(&self.tile)
            || viewing_civ.is_spectator()
    }

    fn reset(&mut self, local_unique_cache: &LocalUniqueCache) {
        self.layer_terrain.reset();
        self.layer_borders.reset();
        self.layer_misc.reset();
        self.layer_resource.reset();
        self.layer_improvement.reset();
        self.layer_yield.reset(local_unique_cache);
        self.layer_overlay.reset();
        self.layer_unit_art.reset();
        self.layer_unit_flag.reset();
    }

    fn set_all_layers_visible(&mut self, is_visible: bool) {
        self.layer_terrain.set_visible(is_visible);
        self.layer_features.set_visible(is_visible);
        self.layer_borders.set_visible(is_visible);
        self.layer_misc.set_visible(is_visible);
        self.layer_resource.set_visible(is_visible);
        self.layer_improvement.set_visible(is_visible);
        self.layer_yield.set_visible(is_visible);
        self.layer_overlay.set_visible(is_visible);
        self.layer_unit_art.set_visible(is_visible);
        self.layer_unit_flag.set_visible(is_visible);
        self.layer_city_button.set_visible(is_visible);
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: Option<&LocalUniqueCache>) {
        let local_unique_cache = local_unique_cache.unwrap_or(&LocalUniqueCache::new(false));

        // Reset overlays
        self.layer_misc.remove_hex_outline();
        self.layer_misc.hide_terrain_overlay();
        self.layer_overlay.hide_highlight();
        self.layer_overlay.hide_crosshair();
        self.layer_overlay.hide_good_city_location_indicator();

        let was_previously_visible = self.layer_terrain.is_visible();

        // Show all layers by default
        self.set_all_layers_visible(true);

        // Do not update layers if tile is not explored by viewing player
        if let Some(viewing_civ) = viewing_civ {
            if !(self.is_force_visible || viewing_civ.has_explored(&self.tile)) {
                self.reset(local_unique_cache);

                // If tile has explored neighbors - reveal layers partially
                if self.tile.neighbors.iter().all(|neighbor| !viewing_civ.has_explored(neighbor)) {
                    // Else - hide all layers
                    self.set_all_layers_visible(false);
                } else {
                    self.layer_overlay.set_unexplored(viewing_civ);
                }
                return;
            }
        }

        self.remove_missing_mod_references();

        // Update all layers
        self.layer_terrain.update(viewing_civ, local_unique_cache);
        self.layer_features.update(viewing_civ, local_unique_cache);
        self.layer_borders.update(viewing_civ, local_unique_cache);
        self.layer_misc.update(viewing_civ, local_unique_cache);
        self.layer_resource.update(viewing_civ, local_unique_cache);
        self.layer_improvement.update(viewing_civ, local_unique_cache);
        self.layer_yield.update(viewing_civ, local_unique_cache);
        self.layer_overlay.update(viewing_civ, local_unique_cache);
        self.layer_unit_art.update(viewing_civ, local_unique_cache);
        self.layer_unit_flag.update(viewing_civ, local_unique_cache);
        self.layer_city_button.update(viewing_civ, local_unique_cache);

        // If tile was previously invisible, add fade-in animation
        if !was_previously_visible {
            // In Rust, we would need to implement an animation system
            // This is a placeholder for the Kotlin code:
            // layerTerrain.parent.addAction(
            //     Actions.sequence(
            //         Actions.targeting(layerTerrain, Actions.alpha(0f)),
            //         Actions.targeting(layerTerrain, Actions.fadeIn(0.5f)),
            //     ))
            // );
        }
    }

    fn remove_missing_mod_references(&mut self) {
        // Remove units with missing mod references
        let units = self.tile.get_units();
        for unit in units {
            if !self.tile.ruleset.nations.contains_key(&unit.owner) {
                unit.remove_from_tile();
            }
        }
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Draw all layers
        self.layer_terrain.draw(ctx);
        self.layer_features.draw(ctx);
        self.layer_borders.draw(ctx);
        self.layer_misc.draw(ctx);
        self.layer_resource.draw(ctx);
        self.layer_improvement.draw(ctx);
        self.layer_yield.draw(ctx);
        self.layer_overlay.draw(ctx);
        self.layer_unit_art.draw(ctx);
        self.layer_unit_flag.draw(ctx);
        self.layer_city_button.draw(ctx);
    }

    pub fn act(&mut self, delta: f32) {
        // Update animations or other time-based effects
        // This is called every frame with the time since the last frame
    }
}

// Helper struct for tile group map
pub struct TileGroupMap {
    // Implementation would depend on tile group map structure
}

impl TileGroupMap {
    pub fn group_size() -> f32 {
        // Implementation would depend on tile group map structure
        64.0 // Default value
    }
}

// Helper struct for tile set strings
pub struct TileSetStrings {
    // Implementation would depend on tile set string structure
}

impl Clone for TileSetStrings {
    fn clone(&self) -> Self {
        // Implementation would depend on tile set string structure
        Self {}
    }
}

// Helper trait for layers
pub trait TileLayer {
    fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache);
    fn draw(&self, ctx: &mut Context);
    fn reset(&mut self);
    fn set_visible(&mut self, visible: bool);
    fn is_visible(&self) -> bool;
}

// Layer implementations
pub struct TileLayerTerrain {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerTerrain {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on terrain layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on terrain layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on terrain layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerFeatures {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerFeatures {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on features layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on features layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on features layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerBorders {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerBorders {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on borders layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on borders layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on borders layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerMisc {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerMisc {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on misc layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on misc layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on misc layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn remove_hex_outline(&mut self) {
        // Implementation would depend on misc layer
    }

    pub fn hide_terrain_overlay(&mut self) {
        // Implementation would depend on misc layer
    }
}

pub struct TileLayerResource {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerResource {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on resource layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on resource layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on resource layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerImprovement {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerImprovement {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on improvement layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on improvement layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on improvement layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerYield {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerYield {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on yield layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on yield layer
    }

    pub fn reset(&mut self, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on yield layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerOverlay {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerOverlay {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on overlay layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on overlay layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on overlay layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide_highlight(&mut self) {
        // Implementation would depend on overlay layer
    }

    pub fn hide_crosshair(&mut self) {
        // Implementation would depend on overlay layer
    }

    pub fn hide_good_city_location_indicator(&mut self) {
        // Implementation would depend on overlay layer
    }

    pub fn set_unexplored(&mut self, viewing_civ: &Civilization) {
        // Implementation would depend on overlay layer
    }
}

pub struct TileLayerUnitSprite {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerUnitSprite {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on unit sprite layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on unit sprite layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on unit sprite layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerUnitFlag {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerUnitFlag {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on unit flag layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on unit flag layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on unit flag layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

pub struct TileLayerCityButton {
    tile_group: Arc<TileGroup>,
    group_size: f32,
    visible: bool,
}

impl TileLayerCityButton {
    pub fn new(tile_group: Arc<TileGroup>, group_size: f32) -> Self {
        Self {
            tile_group,
            group_size,
            visible: true,
        }
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on city button layer
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on city button layer
    }

    pub fn reset(&mut self) {
        // Implementation would depend on city button layer
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

// Helper trait for Clone
pub trait Clone {
    fn clone(&self) -> Self;
}

// Helper trait for Drawable
pub trait Drawable {
    fn draw(&self, ctx: &mut Context);
}

// Helper trait for Updatable
pub trait Updatable {
    fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache);
}

// Helper trait for Resettable
pub trait Resettable {
    fn reset(&mut self);
}

// Helper trait for Visibility
pub trait Visibility {
    fn set_visible(&mut self, visible: bool);
    fn is_visible(&self) -> bool;
}

// Helper trait for Animation
pub trait Animation {
    fn act(&mut self, delta: f32);
}