// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/VictoryScreenGraphs.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Ui, Align, Response, Rect, Vec2, RichText};
use crate::models::civilization::Civilization;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::constants::DEFAULT_FONT_SIZE;

/// The graphs screen
pub struct VictoryScreenGraphs {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// The header text
    header_text: String,
    /// The graphs table
    graphs_table: Rc<RefCell<egui::Grid>>,
}

impl VictoryScreenGraphs {
    /// Creates a new VictoryScreenGraphs
    pub fn new(world_screen: Rc<WorldScreen>) -> Self {
        Self {
            world_screen,
            header_text: "Graphs".to_string(),
            graphs_table: Rc::new(RefCell::new(egui::Grid::new("graphs_grid"))),
        }
    }

    /// Draws the VictoryScreenGraphs
    pub fn draw(&self, ui: &mut Ui) -> Response {
        let mut response = Response::default();

        // Draw header
        let header_height = 40.0;
        let header_rect = ui.allocate_response(
            Vec2::new(ui.available_width(), header_height),
            egui::Sense::hover(),
        ).rect;

        // Draw header background
        ui.painter().rect_filled(
            header_rect,
            0.0,
            Color32::from_rgba_premultiplied(40, 40, 40, 255),
        );

        // Draw header text
        ui.painter().text(
            header_rect.center(),
            Align::Center,
            self.header_text.clone(),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );

        // Draw separator
        ui.painter().line_segment(
            [
                Vec2::new(header_rect.min.x, header_rect.max.y),
                Vec2::new(header_rect.max.x, header_rect.max.y),
            ],
            egui::Stroke::new(1.0, Color32::GRAY),
        );

        // Draw content
        let content_rect = Rect::from_min_size(
            Vec2::new(header_rect.min.x, header_rect.max.y + 1.0),
            Vec2::new(header_rect.width(), ui.available_height() - header_height - 1.0),
        );

        // Draw graphs table
        self.graphs_table.borrow_mut().draw(ui, content_rect);

        response.rect = Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), content_rect.max.y - header_rect.min.y));
        response
    }

    /// Shows the graphs screen
    pub fn show(&self, ui: &mut Ui) -> Response {
        self.draw(ui)
    }
}