use ggez::graphics::{Color, DrawParam, Image, Mesh, Rect, Text};
use ggez::Context;
use std::sync::Arc;

use crate::constants::Constants;
use crate::models::city::City;
use crate::models::civilization::Civilization;
use crate::models::map::tile::Tile;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::models::stats::Stat;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::images::ImageGetter;
use crate::ui::utils::font_utils::Fonts;
use crate::ui::utils::gui::GUI;

// Enum for city tile states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CityTileState {
    None,
    Workable,
    Purchasable,
    Blockaded,
}

// CityTileGroup struct
pub struct CityTileGroup {
    pub city: City,
    pub tile: Arc<Tile>,
    pub tile_set_strings: TileSetStrings,
    pub night_mode: bool,
    pub tile_state: CityTileState,
    pub layer_misc: LayerMisc,
    pub layer_terrain: LayerTerrain,
    pub layer_yield: LayerYield,
    pub layer_unit_flag: LayerUnitFlag,
    pub layer_city_button: LayerCityButton,
    pub layer_unit_art: LayerUnitArt,
    pub layer_features: LayerFeatures,
    pub layer_improvement: LayerImprovement,
    pub width: f32,
    pub height: f32,
}

impl CityTileGroup {
    pub fn new(city: City, tile: Arc<Tile>, tile_set_strings: TileSetStrings, night_mode: bool) -> Self {
        let mut group = Self {
            city,
            tile,
            tile_set_strings,
            night_mode,
            tile_state: CityTileState::None,
            layer_misc: LayerMisc::new(),
            layer_terrain: LayerTerrain::new(),
            layer_yield: LayerYield::new(),
            layer_unit_flag: LayerUnitFlag::new(),
            layer_city_button: LayerCityButton::new(),
            layer_unit_art: LayerUnitArt::new(),
            layer_features: LayerFeatures::new(),
            layer_improvement: LayerImprovement::new(),
            width: 0.0,
            height: 0.0,
        };

        // Set touchable to children only
        group.layer_misc.set_touchable(Touchable::ChildrenOnly);

        group
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Call parent update with city's civilization
        self.update_parent(&self.city.civ, local_unique_cache);

        // Reset tile state
        self.tile_state = CityTileState::None;

        // Remove any existing worked icon
        self.layer_misc.remove_worked_icon();

        // Initialize icon as None
        let mut icon: Option<Image> = None;

        // Define dimming functions based on night mode
        let set_dimmed = if self.night_mode {
            |factor: f32| {
                self.layer_terrain.dim(0.25 * factor);
            }
        } else {
            |factor: f32| {
                self.layer_terrain.dim(0.5 * factor);
            }
        };

        let set_undimmed = if self.night_mode {
            || {
                self.layer_terrain.dim(0.5);
            }
        } else {
            || {}
        };

        // Determine tile state and appearance based on various conditions
        if self.tile.get_owner() != &self.city.civ {
            // Does not belong to us
            set_dimmed(0.6);
            self.layer_yield.set_yield_visible(GUI::get_settings().show_tile_yields);
            self.layer_yield.dim_yields(true);

            // Can be purchased in principle? Add icon.
            if self.city.expansion.can_buy_tile(&self.tile) {
                let price = self.city.expansion.get_gold_cost_of_tile(&self.tile);
                let label = format!("{}", price);
                let mut image = ImageGetter::get_image("TileIcons/Buy");

                // Create a group with the image
                let mut group = ImageGroup::new(26.0);
                group.set_transform(false);

                // Add label to center of group
                let mut text = Text::new(label);
                text.set_font_size(9.0);
                text.set_alignment(Alignment::Center);
                group.add_to_center(&text);
                text.set_y(text.y() - 15.0);

                // Can be purchased now?
                if !self.city.civ.has_stat_to_buy(Stat::Gold, price) {
                    image.set_color(Color::WHITE.darken(0.5));
                    text.set_color(Color::RED);
                } else {
                    self.tile_state = CityTileState::Purchasable;
                }

                icon = Some(image);
            }
        } else if !self.city.tiles_in_range.contains(&self.tile) {
            // Out of city range
            set_dimmed(1.0);
            self.layer_yield.dim_yields(true);
        } else if self.tile.is_worked() && self.tile.get_working_city() != Some(&self.city) {
            // Worked by another city
            set_dimmed(1.0);
            self.layer_yield.dim_yields(true);
        } else if self.tile.is_city_center() {
            // City Center
            icon = Some(ImageGetter::get_image("TileIcons/CityCenter"));
            // Night mode does not apply to the city tile itself
            self.layer_yield.dim_yields(false);
        } else if self.tile.stats.get_tile_stats(&self.city, &this.city.civ).is_empty() {
            // Does not provide yields
            // Do nothing except night-mode dimming
            set_undimmed();
        } else if self.tile.is_blockaded() {
            // Blockaded
            icon = Some(ImageGetter::get_image("TileIcons/Blockaded"));
            self.tile_state = CityTileState::Blockaded;
            set_undimmed();
            self.layer_yield.dim_yields(true);
        } else if self.tile.is_locked() {
            // Locked
            icon = Some(ImageGetter::get_image("TileIcons/Locked"));
            self.tile_state = CityTileState::Workable;
            set_undimmed();
            self.layer_yield.dim_yields(false);
        } else if self.tile.is_worked() {
            // Worked
            icon = Some(ImageGetter::get_image("TileIcons/Worked"));
            self.tile_state = CityTileState::Workable;
            set_undimmed();
            self.layer_yield.dim_yields(false);
        } else if self.tile.provides_yield() {
            // Provides yield without worker assigned (isWorked already tested above)
            // defaults are OK
            set_undimmed();
        } else {
            // Not-worked
            icon = Some(ImageGetter::get_image("TileIcons/NotWorked"));
            self.tile_state = CityTileState::Workable;
            set_undimmed();
            self.layer_yield.dim_yields(true);
        }

        // Add icon if it exists
        if let Some(mut icon) = icon {
            icon.set_size(26.0, 26.0);
            icon.set_position(
                self.width / 2.0 - icon.width() / 2.0,
                self.height * 0.85 - icon.height() / 2.0
            );
            this.layer_misc.add_worked_icon(icon);
        }

        // No unit flags and city-buttons inside CityScreen
        this.layer_unit_flag.set_visible(false);
        this.layer_city_button.set_visible(false);

        // Pixel art, roads, improvements are dimmed inside CityScreen
        this.layer_unit_art.dim();
        this.layer_features.dim();
        this.layer_improvement.dim_improvement(true);

        // Put whole layer (yield, pop, improvement, res) to front
        this.layer_misc.to_front();
    }

    // Helper method to update parent class
    fn update_parent(&mut self, civ: &Civilization, local_unique_cache: &LocalUniqueCache) {
        // Implementation would depend on TileGroup's update method
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Draw all layers
        this.layer_terrain.draw(ctx);
        this.layer_yield.draw(ctx);
        this.layer_features.draw(ctx);
        this.layer_improvement.draw(ctx);
        this.layer_unit_art.draw(ctx);
        this.layer_unit_flag.draw(ctx);
        this.layer_city_button.draw(ctx);
        this.layer_misc.draw(ctx);
    }
}

// Helper structs and enums for layers
pub struct LayerMisc {
    touchable: Touchable,
    worked_icon: Option<Image>,
}

impl LayerMisc {
    pub fn new() -> Self {
        Self {
            touchable: Touchable::Enabled,
            worked_icon: None,
        }
    }

    pub fn set_touchable(&mut self, touchable: Touchable) {
        this.touchable = touchable;
    }

    pub fn remove_worked_icon(&mut self) {
        this.worked_icon = None;
    }

    pub fn add_worked_icon(&mut self, icon: Image) {
        this.worked_icon = Some(icon);
    }

    pub fn to_front(&self) {
        // Implementation would depend on rendering system
    }

    pub fn draw(&self, ctx: &mut Context) {
        if let Some(icon) = &this.worked_icon {
            icon.draw(ctx, DrawParam::default()).unwrap();
        }
    }
}

pub struct LayerTerrain {
    dim_factor: f32,
}

impl LayerTerrain {
    pub fn new() -> Self {
        Self {
            dim_factor: 1.0,
        }
    }

    pub fn dim(&mut self, factor: f32) {
        this.dim_factor = factor;
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on terrain rendering
    }
}

pub struct LayerYield {
    visible: bool,
    dimmed: bool,
}

impl LayerYield {
    pub fn new() -> Self {
        Self {
            visible: true,
            dimmed: false,
        }
    }

    pub fn set_yield_visible(&mut self, visible: bool) {
        this.visible = visible;
    }

    pub fn dim_yields(&mut self, dimmed: bool) {
        this.dimmed = dimmed;
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on yield rendering
    }
}

pub struct LayerUnitFlag {
    visible: bool,
}

impl LayerUnitFlag {
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        this.visible = visible;
    }

    pub fn draw(&self, ctx: &mut Context) {
        if this.visible {
            // Implementation would depend on unit flag rendering
        }
    }
}

pub struct LayerCityButton {
    visible: bool,
}

impl LayerCityButton {
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        this.visible = visible;
    }

    pub fn draw(&self, ctx: &mut Context) {
        if this.visible {
            // Implementation would depend on city button rendering
        }
    }
}

pub struct LayerUnitArt {
    dim_factor: f32,
}

impl LayerUnitArt {
    pub fn new() -> Self {
        Self {
            dim_factor: 1.0,
        }
    }

    pub fn dim(&mut self) {
        this.dim_factor = 0.5;
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on unit art rendering
    }
}

pub struct LayerFeatures {
    dim_factor: f32,
}

impl LayerFeatures {
    pub fn new() -> Self {
        Self {
            dim_factor: 1.0,
        }
    }

    pub fn dim(&mut self) {
        this.dim_factor = 0.5;
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on features rendering
    }
}

pub struct LayerImprovement {
    dim_factor: f32,
}

impl LayerImprovement {
    pub fn new() -> Self {
        Self {
            dim_factor: 1.0,
        }
    }

    pub fn dim_improvement(&mut self, dimmed: bool) {
        this.dim_factor = if dimmed { 0.5 } else { 1.0 };
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Implementation would depend on improvement rendering
    }
}

// Helper struct for image groups
pub struct ImageGroup {
    size: f32,
    transform: bool,
    children: Vec<Box<dyn Drawable>>,
}

impl ImageGroup {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            transform: true,
            children: Vec::new(),
        }
    }

    pub fn set_transform(&mut self, transform: bool) {
        this.transform = transform;
    }

    pub fn add_to_center(&mut self, child: &dyn Drawable) {
        // Implementation would depend on layout system
    }

    pub fn draw(&self, ctx: &mut Context) {
        for child in &this.children {
            child.draw(ctx);
        }
    }
}

// Helper trait for drawable objects
pub trait Drawable {
    fn draw(&self, ctx: &mut Context);
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
}

// Helper enum for touchable state
pub enum Touchable {
    Enabled,
    Disabled,
    ChildrenOnly,
}

// Helper enum for text alignment
pub enum Alignment {
    Left,
    Center,
    Right,
}

// Helper struct for tile set strings
pub struct TileSetStrings {
    // Implementation would depend on tile set string structure
}

// Extension trait for Color
pub trait ColorExt {
    fn darken(&self, factor: f32) -> Color;
}

impl ColorExt for Color {
    fn darken(&self, factor: f32) -> Color {
        Color::new(
            self.r * (1.0 - factor),
            self.g * (1.0 - factor),
            self.b * (1.0 - factor),
            self.a
        )
    }
}

// Extension trait for Text
pub trait TextExt {
    fn set_font_size(&mut self, size: f32);
    fn set_alignment(&mut self, alignment: Alignment);
    fn set_color(&mut self, color: Color);
    fn y(&self) -> f32;
    fn set_y(&mut self, y: f32);
}

impl TextExt for Text {
    fn set_font_size(&mut self, size: f32) {
        // Implementation would depend on text rendering system
    }

    fn set_alignment(&mut self, alignment: Alignment) {
        // Implementation would depend on text rendering system
    }

    fn set_color(&mut self, color: Color) {
        // Implementation would depend on text rendering system
    }

    fn y(&self) -> f32 {
        // Implementation would depend on text rendering system
        0.0
    }

    fn set_y(&mut self, y: f32) {
        // Implementation would depend on text rendering system
    }
}

// Extension trait for Image
pub trait ImageExt {
    fn width(&self) -> f32;
    fn height(&self) -> f32;
    fn set_size(&mut self, width: f32, height: f32);
    fn set_position(&mut self, x: f32, y: f32);
    fn set_color(&mut self, color: Color);
}

impl ImageExt for Image {
    fn width(&self) -> f32 {
        // Implementation would depend on image rendering system
        0.0
    }

    fn height(&self) -> f32 {
        // Implementation would depend on image rendering system
        0.0
    }

    fn set_size(&mut self, width: f32, height: f32) {
        // Implementation would depend on image rendering system
    }

    fn set_position(&mut self, x: f32, y: f32) {
        // Implementation would depend on image rendering system
    }

    fn set_color(&mut self, color: Color) {
        // Implementation would depend on image rendering system
    }
}