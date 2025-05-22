// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/topbar/WorldScreenTopBar.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Response, Ui, Vec2, Color32};

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::topbar::resources::WorldScreenTopBarResources;
use crate::ui::screens::worldscreen::topbar::stats::WorldScreenTopBarStats;

/// Top bar for the world screen containing resources and stats
pub struct WorldScreenTopBar {
    world_screen: Rc<RefCell<WorldScreen>>,
    resources: WorldScreenTopBarResources,
    stats: WorldScreenTopBarStats,
    height: f32,
    background_color: Color32,
}

impl WorldScreenTopBar {
    /// Creates a new world screen top bar
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        let resources = WorldScreenTopBarResources::new(world_screen.clone());
        let stats = WorldScreenTopBarStats::new(world_screen.clone());

        Self {
            world_screen,
            resources,
            stats,
            height: 45.0,
            background_color: Color32::from_rgba_premultiplied(0, 0, 0, 180),
        }
    }

    /// Updates the top bar
    pub fn update(&mut self) {
        self.resources.update();
        self.stats.update();
    }

    /// Draws the top bar
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), self.height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Draw background
            painter.rect_filled(
                rect,
                0.0,
                self.background_color,
            );

            // Create child UI for resources and stats
            let mut child_ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center));

            // Draw resources on the left
            self.resources.draw(&mut child_ui);

            // Add spacing between resources and stats
            child_ui.add_space(20.0);

            // Draw stats on the right
            self.stats.draw(&mut child_ui);
        }

        response
    }

    /// Gets the height of the top bar
    pub fn get_height(&self) -> f32 {
        self.height
    }
}