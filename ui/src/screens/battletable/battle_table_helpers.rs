// Source: orig_src/core/src/com/unciv/ui/screens/battletable/BattleTableHelpers.kt

use std::rc::Rc;
use egui::{Color32, Ui, Response, Rect, Vec2, RichText};
use crate::models::gamedata::{Battle, BattleData, BattleModifier};
use crate::models::civilization::Civilization;
use crate::models::units::Unit;
use crate::utils::translation::tr;

/// Helper functions for the BattleTable
pub struct BattleTableHelpers;

impl BattleTableHelpers {
    /// Formats a combat modifier for display
    pub fn format_modifier(modifier: &BattleModifier) -> String {
        let mut text = String::new();

        if modifier.value > 0.0 {
            text.push('+');
        }

        text.push_str(&format!("{:.0}% ", modifier.value * 100.0));
        text.push_str(&tr(&modifier.description));

        text
    }

    /// Gets the color for a combat stat
    pub fn get_stat_color(value: f32) -> Color32 {
        if value > 0.0 {
            Color32::from_rgb(0, 255, 0) // Green for positive
        } else if value < 0.0 {
            Color32::from_rgb(255, 0, 0) // Red for negative
        } else {
            Color32::WHITE // White for neutral
        }
    }

    /// Formats a combat stat for display
    pub fn format_stat(name: &str, value: f32) -> String {
        format!("{}: {}{:.1}", tr(name), if value > 0.0 { "+" } else { "" }, value)
    }

    /// Draws a combat modifier
    pub fn draw_modifier(ui: &mut Ui, modifier: &BattleModifier, rect: Rect) -> Response {
        let text = Self::format_modifier(modifier);
        let color = Self::get_stat_color(modifier.value);

        ui.put(
            rect,
            egui::Label::new(RichText::new(text).color(color))
        )
    }

    /// Draws a combat stat
    pub fn draw_stat(ui: &mut Ui, name: &str, value: f32, rect: Rect) -> Response {
        let text = Self::format_stat(name, value);
        let color = Self::get_stat_color(value);

        ui.put(
            rect,
            egui::Label::new(RichText::new(text).color(color))
        )
    }

    /// Gets the predicted outcome text
    pub fn get_outcome_text(battle_data: &BattleData) -> String {
        let mut text = String::new();

        // Add attacker damage
        text.push_str(&format!(
            "{} {} {} {}",
            tr("Attacker"),
            tr("deals"),
            battle_data.attacker_damage,
            tr("damage")
        ));

        // Add defender damage if applicable
        if battle_data.defender_can_retaliate {
            text.push_str(&format!(
                "\n{} {} {} {}",
                tr("Defender"),
                tr("deals"),
                battle_data.defender_damage,
                tr("damage")
            ));
        }

        text
    }

    /// Draws the predicted outcome
    pub fn draw_outcome(ui: &mut Ui, battle_data: &BattleData, rect: Rect) -> Response {
        let text = Self::get_outcome_text(battle_data);

        ui.put(
            rect,
            egui::Label::new(RichText::new(text).color(Color32::WHITE))
        )
    }

    /// Gets the unit name with strength
    pub fn get_unit_name_with_strength(unit: &Unit) -> String {
        format!("{} ({})", tr(&unit.name), unit.get_strength())
    }
}

// TODO: Implement additional helper functions for:
// - Drawing unit health bars
// - Drawing unit experience bars
// - Drawing unit promotions
// - Drawing terrain bonuses
// - Drawing combat animations