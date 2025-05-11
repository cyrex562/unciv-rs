// Source: orig_src/core/src/com/unciv/ui/screens/tileinfo/TileInfoTable.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Ui, Response, Rect, Vec2, RichText};
use crate::tile::tile::Tile;
use crate::models::civilization::Civilization;
use crate::models::stats::Stats;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;

/// A table showing information about a tile
pub struct TileInfoTable {
    /// The tile
    tile: Rc<RefCell<Tile>>,
    /// The viewing civilization
    viewing_civ: Rc<Civilization>,
    /// The tile stats
    stats: Stats,
    /// The tile yields
    yields: Vec<(String, f32)>,
    /// The tile resources
    resources: Vec<String>,
    /// The tile improvements
    improvements: Vec<String>,
}

impl TileInfoTable {
    /// Creates a new TileInfoTable
    pub fn new(tile: Rc<RefCell<Tile>>, viewing_civ: Rc<Civilization>) -> Self {
        let mut instance = Self {
            tile,
            viewing_civ,
            stats: Stats::default(),
            yields: Vec::new(),
            resources: Vec::new(),
            improvements: Vec::new(),
        };

        instance.init();
        instance
    }

    /// Initializes the TileInfoTable
    fn init(&mut self) {
        let tile = self.tile.borrow();

        // Get tile stats
        self.stats = tile.get_stats_for_civilization(&self.viewing_civ);

        // Get tile yields
        self.yields = self.get_tile_yields(&tile);

        // Get tile resources
        if let Some(resource) = &tile.resource {
            self.resources.push(resource.name.clone());
        }

        // Get tile improvements
        if let Some(improvement) = &tile.improvement {
            self.improvements.push(improvement.name.clone());
        }
    }

    /// Gets the tile yields
    fn get_tile_yields(&self, tile: &Tile) -> Vec<(String, f32)> {
        let mut yields = Vec::new();

        // Add food yield
        if self.stats.food > 0.0 {
            yields.push(("Food".to_string(), self.stats.food));
        }

        // Add production yield
        if self.stats.production > 0.0 {
            yields.push(("Production".to_string(), self.stats.production));
        }

        // Add gold yield
        if self.stats.gold > 0.0 {
            yields.push(("Gold".to_string(), self.stats.gold));
        }

        // Add science yield
        if self.stats.science > 0.0 {
            yields.push(("Science".to_string(), self.stats.science));
        }

        // Add culture yield
        if self.stats.culture > 0.0 {
            yields.push(("Culture".to_string(), self.stats.culture));
        }

        yields
    }

    /// Draws the TileInfoTable
    pub fn draw(&self, ui: &mut Ui) -> Response {
        let mut response = Response::default();

        // Draw tile type
        let tile = self.tile.borrow();
        ui.label(RichText::new(tr(&tile.type_name)).color(Color32::WHITE));

        // Draw tile yields
        for (yield_type, value) in &self.yields {
            let text = format!("{}: {}", tr(yield_type), value);
            ui.label(RichText::new(text).color(Color32::WHITE));
        }

        // Draw resources
        if !self.resources.is_empty() {
            ui.label(RichText::new(tr("Resources")).color(Color32::WHITE));
            for resource in &self.resources {
                ui.label(RichText::new(tr(resource)).color(Color32::WHITE));
            }
        }

        // Draw improvements
        if !self.improvements.is_empty() {
            ui.label(RichText::new(tr("Improvements")).color(Color32::WHITE));
            for improvement in &self.improvements {
                ui.label(RichText::new(tr(improvement)).color(Color32::WHITE));
            }
        }

        response
    }

    /// Updates the TileInfoTable
    pub fn update(&mut self) {
        self.init(); // Refresh all data
    }
}

// TODO: Implement additional features:
// - Display terrain features
// - Show civilization borders
// - Display unit information
// - Show city workable tiles
// - Display tile defense bonuses
// - Show natural wonders
// - Display tile improvements under construction
// - Show resource quantities
// - Display tile culture and influence