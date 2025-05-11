// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/VictoryScreenOurVictory.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Ui, Align, Response, Rect, Vec2, RichText};
use crate::models::civilization::Civilization;
use crate::models::ruleset::Victory;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::components::widgets::TabbedPager;
use crate::ui::components::widgets::TabbedPagerPageExtensions;
use crate::constants::DEFAULT_FONT_SIZE;

/// The victory screen for our victory
pub struct VictoryScreenOurVictory {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// The header table
    header: Vec<VictoryScreenCivGroup>,
    /// The columns for each victory
    columns: Vec<(String, Vec<VictoryScreenCivGroup>)>,
}

impl VictoryScreenOurVictory {
    /// Creates a new VictoryScreenOurVictory
    pub fn new(world_screen: Rc<WorldScreen>) -> Self {
        let mut instance = Self {
            world_screen,
            header: Vec::new(),
            columns: Vec::new(),
        };

        instance.init();
        instance
    }

    /// Initializes the VictoryScreenOurVictory
    fn init(&mut self) {
        let game_info = &self.world_screen.game_info;
        let victories_to_show = game_info.get_enabled_victories();
        let viewing_civ = self.world_screen.viewing_civ.clone();

        // Create columns for each victory
        for (victory_name, victory) in victories_to_show {
            // Create header for this victory
            let mut header_group = VictoryScreenCivGroup::new(
                Rc::new(Civilization::new()), // Dummy civ for header
                "",
                format!("[{}] Victory", victory_name),
                viewing_civ.clone(),
                DefeatedPlayerStyle::Regular,
            );

            self.header.push(header_group);

            // Create column for this victory
            let column = self.get_column(victory, viewing_civ.clone());
            self.columns.push((victory_name.clone(), column));
        }

        // Add separator to header
        self.header.push(VictoryScreenCivGroup::new(
            Rc::new(Civilization::new()), // Dummy civ for separator
            "",
            "".to_string(),
            viewing_civ.clone(),
            DefeatedPlayerStyle::Regular,
        ));
    }

    /// Gets a column for a victory
    fn get_column(&self, victory: &Victory, player_civ: Rc<Civilization>) -> Vec<VictoryScreenCivGroup> {
        let mut column = Vec::new();
        let mut first_incomplete = true;

        for milestone in &victory.milestone_objects {
            let completion_status = if milestone.has_been_completed_by(&player_civ) {
                Victory::CompletionStatus::Completed
            } else if first_incomplete {
                first_incomplete = false;
                Victory::CompletionStatus::Partially
            } else {
                Victory::CompletionStatus::Incomplete
            };

            for button in milestone.get_victory_screen_buttons(completion_status, player_civ.clone()) {
                column.push(button);
            }
        }

        column
    }

    /// Draws the VictoryScreenOurVictory
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

        // Draw header groups
        let column_width = header_rect.width() / self.columns.len() as f32;
        for (i, header_group) in self.header.iter().enumerate() {
            if i < self.columns.len() {
                let x = header_rect.min.x + i as f32 * column_width;
                let header_group_rect = Rect::from_min_size(
                    Vec2::new(x, header_rect.min.y),
                    Vec2::new(column_width, header_height),
                );

                // Draw header group
                ui.painter().text(
                    header_group_rect.center(),
                    Align::Center,
                    header_group.label_text.clone(),
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }
        }

        // Draw separator
        ui.painter().line_segment(
            [
                Vec2::new(header_rect.min.x, header_rect.max.y),
                Vec2::new(header_rect.max.x, header_rect.max.y),
            ],
            egui::Stroke::new(1.0, Color32::GRAY),
        );

        // Draw columns
        let content_rect = Rect::from_min_size(
            Vec2::new(header_rect.min.x, header_rect.max.y + 1.0),
            Vec2::new(header_rect.width(), ui.available_height() - header_height - 1.0),
        );

        for (i, (_, column)) in self.columns.iter().enumerate() {
            let x = content_rect.min.x + i as f32 * column_width;
            let column_rect = Rect::from_min_size(
                Vec2::new(x, content_rect.min.y),
                Vec2::new(column_width, content_rect.height()),
            );

            // Draw column background
            ui.painter().rect_filled(
                column_rect,
                0.0,
                Color32::from_rgba_premultiplied(30, 30, 30, 255),
            );

            // Draw civ groups
            let mut y = column_rect.min.y;
            for civ_group in column {
                let civ_group_height = 40.0;
                let civ_group_rect = Rect::from_min_size(
                    Vec2::new(x, y),
                    Vec2::new(column_width, civ_group_height),
                );

                // Draw civ group
                civ_group.draw(ui);

                y += civ_group_height + 5.0;
            }
        }

        response.rect = Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), content_rect.max.y - header_rect.min.y));
        response
    }
}

impl TabbedPagerPageExtensions for VictoryScreenOurVictory {
    /// Called when the page is activated
    fn activated(&mut self, index: i32, caption: String, pager: &mut TabbedPager) {
        // Equalize columns
        pager.set_scroll_disabled(false);
    }

    /// Called when the page is deactivated
    fn deactivated(&mut self, index: i32, caption: String, pager: &mut TabbedPager) {
        // Nothing to do
    }

    /// Gets the fixed content
    fn get_fixed_content(&self) -> Vec<VictoryScreenCivGroup> {
        self.header.clone()
    }
}