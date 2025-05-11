use std::option::Option;
use ggez::graphics::{DrawParam, Drawable, Rect};
use ggez::Context;
use ggez::mint::Point2;
use crate::logic::civilization::Civilization;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::tilegroups::CityButton;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::components::tilegroups::WorldTileGroup;
use crate::ui::components::tilegroups::layers::tile_layer::{BaseTileLayer, TileLayer};
use crate::utils::debug_utils::DebugUtils;

/// A layer that draws city buttons on tiles
pub struct TileLayerCityButton {
    /// The base tile layer
    base: BaseTileLayer,
    /// The city button
    city_button: Option<CityButton>,
}

impl TileLayerCityButton {
    /// Create a new tile layer city button
    pub fn new(tile_group: TileGroup, size: f32) -> Self {
        let mut base = BaseTileLayer::new(tile_group, size);
        // Set origin to center
        base.set_origin(Point2 { x: size / 2.0, y: size / 2.0 });

        Self {
            base,
            city_button: None,
        }
    }

    /// Move the city button up
    pub fn move_up(&mut self) {
        if let Some(city_button) = &mut self.city_button {
            city_button.move_button_up();
        }
    }

    /// Move the city button down
    pub fn move_down(&mut self) {
        if let Some(city_button) = &mut self.city_button {
            city_button.move_button_down();
        }
    }
}

impl TileLayer for TileLayerCityButton {
    fn tile_group(&self) -> &TileGroup {
        self.base.tile_group()
    }

    fn size(&self) -> f32 {
        self.base.size()
    }

    fn do_update(&mut self, viewing_civ: Option<&Civilization>, _local_unique_cache: &LocalUniqueCache) {
        // Only update if this is a world tile group
        if !self.tile_group().is::<WorldTileGroup>() {
            return;
        }

        let city = self.tile().get_city();

        // There used to be a city here but it was razed
        if city.is_none() && self.city_button.is_some() {
            self.city_button = None;
        }

        if viewing_civ.is_none() {
            return;
        }

        if city.is_none() || !self.tile().is_city_center() {
            return;
        }

        // Create (if not yet) and update city button
        if self.city_button.is_none() {
            self.city_button = Some(CityButton::new(city.unwrap(), self.tile_group().clone()));
        }

        if let Some(city_button) = &mut self.city_button {
            city_button.update(DebugUtils::VISIBLE_MAP || self.is_viewable(viewing_civ.unwrap()));
        }
    }

    fn has_children(&self) -> bool {
        self.city_button.is_some()
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.base.is_visible()
    }
}

impl Drawable for TileLayerCityButton {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> ggez::GameResult {
        if !self.is_visible() || !self.tile().is_city_center() {
            return Ok(());
        }

        if let Some(city_button) = &self.city_button {
            city_button.draw(ctx, param)?;
        }

        Ok(())
    }

    fn dimensions(&self, _ctx: &mut Context) -> Option<Rect> {
        Some(Rect::new(0.0, 0.0, self.size(), self.size()))
    }
}