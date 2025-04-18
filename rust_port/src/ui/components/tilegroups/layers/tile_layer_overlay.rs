use ggez::graphics::{Color, DrawParam, Image, Mesh, Rect};
use ggez::Context;
use std::sync::Arc;

use crate::constants::Constants;
use crate::models::civilization::Civilization;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::images::ImageGetter;

pub struct TileLayerOverlay {
    tile_group: Arc<TileGroup>,
    size: f32,
    highlight: Option<Image>,
    crosshair: Option<Image>,
    good_city_location_indicator: Option<Image>,
    fog: Option<Image>,
    unexplored: Option<Image>,
}

impl TileLayerOverlay {
    pub fn new(tile_group: Arc<TileGroup>, size: f32) -> Self {
        Self {
            tile_group,
            size,
            highlight: None,
            crosshair: None,
            good_city_location_indicator: None,
            fog: None,
            unexplored: None,
        }
    }

    fn get_highlight(&self) -> Image {
        ImageGetter::get_image("highlight").set_hexagon_size()
    }

    fn get_crosshair(&self) -> Image {
        ImageGetter::get_image("crosshair").set_hexagon_size()
    }

    fn get_good_city_location_indicator(&self) -> Image {
        ImageGetter::get_image("OtherIcons/Cities").set_hexagon_size(0.25)
    }

    fn get_fog(&self) -> Image {
        let mut fog = ImageGetter::get_image("crosshatchHexagon").set_hexagon_size();
        fog.set_color(Color::new(1.0, 1.0, 1.0, 0.2));
        fog
    }

    fn get_unexplored(&self) -> Image {
        ImageGetter::get_image("unexploredTile").set_hexagon_size()
    }

    pub fn order_to_front(&mut self) {
        if let Some(unexplored) = &mut self.unexplored {
            unexplored.to_front();
        }
        if let Some(highlight) = &mut self.highlight {
            highlight.to_front();
        }
        if let Some(fog) = &mut self.fog {
            fog.to_front();
        }
        if let Some(crosshair) = &mut self.crosshair {
            crosshair.to_front();
        }
        if let Some(indicator) = &mut self.good_city_location_indicator {
            indicator.to_front();
        }
    }

    pub fn show_crosshair(&mut self, alpha: f32) {
        if self.crosshair.is_none() {
            self.crosshair = Some(self.get_crosshair());
            self.determine_visibility();
        }
        if let Some(crosshair) = &mut self.crosshair {
            crosshair.set_color(Color::new(1.0, 1.0, 1.0, alpha));
        }
    }

    pub fn hide_crosshair(&mut self) {
        self.crosshair = None;
        self.determine_visibility();
    }

    pub fn show_highlight(&mut self, color: Color, alpha: f32) {
        if self.highlight.is_none() {
            self.highlight = Some(self.get_highlight());
            self.determine_visibility();
        }
        if let Some(highlight) = &mut self.highlight {
            let mut new_color = color;
            new_color.a = alpha;
            highlight.set_color(new_color);
        }
    }

    pub fn hide_highlight(&mut self) {
        self.highlight = None;
        self.determine_visibility();
    }

    pub fn show_good_city_location_indicator(&mut self) {
        if self.good_city_location_indicator.is_none() {
            self.good_city_location_indicator = Some(self.get_good_city_location_indicator());
            self.determine_visibility();
        }
    }

    pub fn hide_good_city_location_indicator(&mut self) {
        self.good_city_location_indicator = None;
        self.determine_visibility();
    }

    pub fn reset(&mut self) {
        self.hide_highlight();
        self.hide_crosshair();
        self.hide_good_city_location_indicator();
        self.determine_visibility();
    }

    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        let is_viewable = viewing_civ.map_or(true, |civ| self.is_viewable(civ));
        self.set_fog(is_viewable);

        if viewing_civ.is_none() {
            return;
        }

        // Additional update logic would go here
    }

    fn determine_visibility(&mut self) {
        // Implementation for determining visibility of overlays
    }

    fn is_viewable(&self, civ: &Civilization) -> bool {
        // Implementation for checking if a tile is viewable by a civilization
        true // Placeholder
    }

    fn set_fog(&mut self, is_viewable: bool) {
        if is_viewable {
            if self.fog.is_none() {
                self.fog = Some(self.get_fog());
            }
        } else {
            self.fog = None;
        }
        self.determine_visibility();
    }

    pub fn draw(&self, ctx: &mut Context) {
        // Draw all active overlays
        if let Some(highlight) = &self.highlight {
            highlight.draw(ctx, DrawParam::default());
        }
        if let Some(crosshair) = &self.crosshair {
            crosshair.draw(ctx, DrawParam::default());
        }
        if let Some(indicator) = &self.good_city_location_indicator {
            indicator.draw(ctx, DrawParam::default());
        }
        if let Some(fog) = &self.fog {
            fog.draw(ctx, DrawParam::default());
        }
        if let Some(unexplored) = &self.unexplored {
            unexplored.draw(ctx, DrawParam::default());
        }
    }
}