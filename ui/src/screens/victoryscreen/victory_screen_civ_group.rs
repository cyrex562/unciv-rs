// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/VictoryScreenCivGroup.kt

use std::rc::Rc;
use egui::{Color32, Ui, Align, RichText, Response, Rect, Vec2};
use crate::models::civilization::Civilization;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;
use crate::constants::UNKNOWN_NATION_NAME;

/// Element displaying one Civilization as seen by another Civilization on a rounded-edge background.
pub struct VictoryScreenCivGroup {
    /// The civilization to display
    civ: Rc<Civilization>,
    /// The separator between civ name and additional info
    separator: String,
    /// Additional information to display
    additional_info: String,
    /// The viewing civilization
    current_player: Rc<Civilization>,
    /// The style for defeated players
    defeated_player_style: DefeatedPlayerStyle,
    /// The background color
    background_color: Color32,
    /// The label color
    label_color: Color32,
    /// The label text
    label_text: String,
    /// The icon texture ID
    icon_texture_id: Option<egui::TextureId>,
    /// The icon size
    icon_size: f32,
}

/// Style for displaying defeated players
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatedPlayerStyle {
    /// Regular style
    Regular,
    /// Greyed out style
    GreyedOut,
}

impl VictoryScreenCivGroup {
    /// Creates a new VictoryScreenCivGroup with a CivWithStat
    pub fn from_civ_with_stat(
        civ_entry: &CivWithStat,
        current_player: Rc<Civilization>,
        defeated_player_style: DefeatedPlayerStyle,
    ) -> Self {
        Self::new(
            civ_entry.civ.clone(),
            ": ",
            if civ_entry.civ.is_defeated() {
                "".to_string()
            } else {
                civ_entry.value.to_string()
            },
            current_player,
            defeated_player_style,
        )
    }

    /// Creates a new VictoryScreenCivGroup with a civilization and additional info
    pub fn with_additional_info(
        civ: Rc<Civilization>,
        additional_info: String,
        current_player: Rc<Civilization>,
        defeated_player_style: DefeatedPlayerStyle,
    ) -> Self {
        Self::new(
            civ,
            "\n",
            additional_info,
            current_player,
            defeated_player_style,
        )
    }

    /// Creates a new VictoryScreenCivGroup
    pub fn new(
        civ: Rc<Civilization>,
        separator: String,
        additional_info: String,
        current_player: Rc<Civilization>,
        defeated_player_style: DefeatedPlayerStyle,
    ) -> Self {
        let label_text = if current_player.knows(&civ) || current_player == civ ||
            civ.is_defeated() || current_player.is_defeated() {
            if additional_info.is_empty() {
                civ.civ_name.clone()
            } else {
                format!("{}{}{}", civ.civ_name, separator, additional_info)
            }
        } else {
            UNKNOWN_NATION_NAME.to_string()
        };

        let (icon_texture_id, background_color, label_color) = Self::get_civ_image_and_colors(
            &civ,
            &current_player,
            defeated_player_style,
        );

        Self {
            civ,
            separator,
            additional_info,
            current_player,
            defeated_player_style,
            background_color,
            label_color,
            label_text,
            icon_texture_id,
            icon_size: 30.0,
        }
    }

    /// Gets the civilization image and colors
    pub fn get_civ_image_and_colors(
        civ: &Rc<Civilization>,
        current_player: &Rc<Civilization>,
        defeated_player_style: DefeatedPlayerStyle,
    ) -> (Option<egui::TextureId>, Color32, Color32) {
        if civ.is_defeated() && defeated_player_style == DefeatedPlayerStyle::GreyedOut {
            let icon = ImageGetter::get_image("OtherIcons/DisbandUnit");
            return (
                Some(icon.texture_id()),
                Color32::LIGHT_GRAY,
                Color32::from_rgb(21, 21, 21), // CHARCOAL
            );
        } else if current_player.is_spectator()
            || (civ.is_defeated() && defeated_player_style == DefeatedPlayerStyle::Regular)
            || current_player == civ
            || current_player.knows(civ)
            || current_player.is_defeated()
            || current_player.victory_manager.has_won() {
            let icon = ImageGetter::get_nation_portrait(&civ.nation, 30.0);
            return (
                Some(icon.texture_id()),
                civ.nation.get_outer_color(),
                civ.nation.get_inner_color(),
            );
        } else {
            let icon = ImageGetter::get_random_nation_portrait(30.0);
            return (
                Some(icon.texture_id()),
                Color32::LIGHT_GRAY,
                Color32::from_rgb(21, 21, 21), // CHARCOAL
            );
        }
    }

    /// Draws the VictoryScreenCivGroup
    pub fn draw(&self, ui: &mut Ui) -> Response {
        let mut response = Response::default();

        // Create a rounded rectangle background
        let rect = ui.allocate_response(
            Vec2::new(ui.available_width(), 40.0),
            egui::Sense::hover(),
        ).rect;

        // Draw the background
        ui.painter().rect_filled(
            rect,
            5.0, // corner radius
            self.background_color,
        );

        // Draw the icon
        if let Some(texture_id) = self.icon_texture_id {
            let icon_rect = Rect::from_min_size(
                rect.min + Vec2::new(5.0, 5.0),
                Vec2::new(self.icon_size, self.icon_size),
            );
            ui.painter().image(
                texture_id,
                icon_rect,
                Rect::from_min_size(Vec2::ZERO, Vec2::new(1.0, 1.0)),
                self.label_color,
            );
        }

        // Draw the label
        let label_rect = Rect::from_min_size(
            rect.min + Vec2::new(self.icon_size + 10.0, 5.0),
            Vec2::new(rect.width() - self.icon_size - 15.0, 30.0),
        );

        ui.painter().text(
            label_rect.center(),
            Align::Center,
            self.label_text.clone(),
            egui::FontId::proportional(14.0),
            self.label_color,
        );

        response.rect = rect;
        response
    }
}

/// A civilization with a stat value
pub struct CivWithStat {
    /// The civilization
    pub civ: Rc<Civilization>,
    /// The stat value
    pub value: i32,
    /// The ranking type
    pub ranking_type: RankingType,
}

impl CivWithStat {
    /// Creates a new CivWithStat
    pub fn new(civ: Rc<Civilization>, ranking_type: RankingType) -> Self {
        let value = match ranking_type {
            RankingType::Score => civ.victory_manager.score,
            RankingType::Population => civ.population,
            RankingType::Growth => civ.growth,
            RankingType::Production => civ.production,
            RankingType::Gold => civ.gold,
            RankingType::Territory => civ.territory,
            RankingType::Force => civ.force,
            RankingType::Happiness => civ.happiness,
            RankingType::Technologies => civ.technologies.len() as i32,
            RankingType::Culture => civ.culture,
        };

        Self {
            civ,
            value,
            ranking_type,
        }
    }
}