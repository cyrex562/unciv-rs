// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/VictoryScreenCivRankings.kt

use std::rc::Rc;
use std::collections::HashMap;
use egui::{Color32, Ui, Align, Response, Rect, Vec2, RichText};
use crate::models::civilization::Civilization;
use crate::ui::images::ImageGetter;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::components::widgets::TabbedPager;
use crate::ui::components::widgets::TabbedPagerPageExtensions;
use crate::constants::DEFAULT_FONT_SIZE;

/// Displays civilization rankings in the victory screen
pub struct VictoryScreenCivRankings {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// The header table
    header: Vec<VictoryScreenCivGroup>,
    /// The columns for each ranking type
    columns: HashMap<RankingType, Vec<VictoryScreenCivGroup>>,
}

impl VictoryScreenCivRankings {
    /// Creates a new VictoryScreenCivRankings
    pub fn new(world_screen: Rc<WorldScreen>) -> Self {
        let mut instance = Self {
            world_screen,
            header: Vec::new(),
            columns: HashMap::new(),
        };

        instance.init();
        instance
    }

    /// Initializes the VictoryScreenCivRankings
    fn init(&mut self) {
        let major_civs: Vec<Rc<Civilization>> = self.world_screen.game_info.civilizations
            .iter()
            .filter(|civ| civ.is_major_civ())
            .cloned()
            .collect();

        // Create columns for each ranking type
        for ranking_type in RankingType::all() {
            // Create header for this ranking type
            let mut header_group = VictoryScreenCivGroup::new(
                Rc::new(Civilization::new()), // Dummy civ for header
                "",
                ranking_type.label().to_string(),
                self.world_screen.viewing_civ.clone(),
                DefeatedPlayerStyle::Regular,
            );

            // Add icon if available
            if let Some(image) = ranking_type.get_image() {
                header_group.icon_texture_id = Some(image.texture_id());
            }

            self.header.push(header_group);

            // Create column for this ranking type
            let mut civ_data: Vec<CivWithStat> = major_civs
                .iter()
                .map(|civ| CivWithStat::new(civ.clone(), ranking_type))
                .collect();

            // Sort by civ name first, then by value (defeated civs at the end)
            civ_data.sort_by(|a, b| {
                if a.civ.civ_name == b.civ.civ_name {
                    if a.civ.is_defeated() {
                        std::cmp::Ordering::Greater
                    } else if b.civ.is_defeated() {
                        std::cmp::Ordering::Less
                    } else {
                        b.value.cmp(&a.value)
                    }
                } else {
                    a.civ.civ_name.cmp(&b.civ.civ_name)
                }
            });

            // Create civ groups for each civ
            let column: Vec<VictoryScreenCivGroup> = civ_data
                .iter()
                .map(|civ_entry| VictoryScreenCivGroup::from_civ_with_stat(
                    civ_entry,
                    this.world_screen.viewing_civ.clone(),
                    DefeatedPlayerStyle::GreyedOut,
                ))
                .collect();

            this.columns.insert(ranking_type, column);
        }
    }

    /// Draws the VictoryScreenCivRankings
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
        let column_width = header_rect.width() / this.columns.len() as f32;
        for (i, header_group) in this.header.iter().enumerate() {
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

        for (i, (ranking_type, column)) in this.columns.iter().enumerate() {
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

impl TabbedPagerPageExtensions for VictoryScreenCivRankings {
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
        this.header.clone()
    }
}