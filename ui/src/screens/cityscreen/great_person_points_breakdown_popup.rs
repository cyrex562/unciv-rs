use bevy::prelude::*;
use bevy_egui::egui::{self, Ui};
use std::collections::HashMap;

use crate::logic::city::GreatPersonPointsBreakdown;
use crate::ui::components::extensions::to_string_signed;
use crate::ui::popups::popup::Popup;
use crate::ui::screens::cityscreen::CityScreen;
use crate::ui::screens::civilopediascreen::{FormattedLine, MarkupRenderer};

/// Popup that displays a breakdown of great person points for a city
pub struct GreatPersonPointsBreakdownPopup {
    /// Reference to the parent city screen
    city_screen: Box<dyn CityScreen>,

    /// The great person points breakdown
    gpp_breakdown: GreatPersonPointsBreakdown,

    /// The specific great person to show, if any
    great_person: Option<String>,

    /// The formatted lines to display
    lines: Vec<FormattedLine>,
}

impl GreatPersonPointsBreakdownPopup {
    /// Create a new GreatPersonPointsBreakdownPopup
    pub fn new(
        city_screen: Box<dyn CityScreen>,
        gpp_breakdown: GreatPersonPointsBreakdown,
        great_person: Option<String>
    ) -> Self {
        let mut lines = Vec::new();

        // Create header text
        let header_text = format!(
            "«GOLD»{{{}}}«» ({})",
            great_person.as_deref().unwrap_or("Great person points"),
            city_screen.get_city().name
        );

        // Add header line
        lines.push(FormattedLine::new(header_text, 2, true, None));

        // Add separator
        lines.push(FormattedLine::separator());

        // Create the popup
        let mut popup = Self {
            city_screen,
            gpp_breakdown,
            great_person,
            lines,
        };

        // Add formatted entries
        popup.add_formatted_entries();

        popup
    }

    /// Add formatted entries to the lines
    fn add_formatted_entries(&mut self) {
        // Add base points
        for entry in &self.gpp_breakdown.base_points {
            self.add_formatted_entry(entry, false);
        }

        // Add percentage bonuses
        for entry in &self.gpp_breakdown.percent_bonuses {
            self.add_formatted_entry(entry, true);
        }
    }

    /// Add a formatted entry to the lines
    fn add_formatted_entry(&mut self, entry: &GreatPersonPointsBreakdown::Entry, is_percentage: bool) {
        let text = if self.great_person.is_none() {
            // Popup shows all GP for a city - this will resolve the counters if necessary and show GP names from the keys
            self.entry_to_string(entry, is_percentage)
        } else {
            // Popup shows only a specific GP - check counters directly
            let great_person = self.great_person.as_ref().unwrap();
            let amount = entry.counter.get(great_person).copied().unwrap_or(0);

            if amount == 0 {
                return;
            }

            // Formatter does not need the GP name as in all cases the one in the header is clear enough
            self.entry_to_string_with_amount(entry, is_percentage, amount)
        };

        // Add the line with the pedia link if available
        let pedia_link = entry.pedia_link.clone();
        self.lines.push(FormattedLine::new(text, 0, false, pedia_link));
    }

    /// Convert an entry to a string
    fn entry_to_string(&self, entry: &GreatPersonPointsBreakdown::Entry, is_percentage: bool) -> String {
        let mut result = format!("{{{}}}: ", entry.source);

        if entry.is_all_gp {
            // For all GP entries, just show the first value
            let value = entry.counter.values().next().copied().unwrap_or(0);
            result.push_str(&to_string_signed(value));

            if is_percentage {
                result.push_str("%");
            }
        } else if is_percentage {
            // For percentage entries, show each GP with its percentage
            let entries: Vec<String> = entry.counter.iter()
                .map(|(key, value)| format!("{}{}% {{{}}}", to_string_signed(*value), "", key))
                .collect();

            result.push_str(&entries.join(", "));
        } else {
            // For regular entries, show each GP with its value
            let entries: Vec<String> = entry.counter.iter()
                .map(|(key, value)| format!("{}{} {{{}}}", to_string_signed(*value), "", key))
                .collect();

            result.push_str(&entries.join(", "));
        }

        result
    }

    /// Convert an entry to a string with a specific amount
    fn entry_to_string_with_amount(&self, entry: &GreatPersonPointsBreakdown::Entry, is_percentage: bool, amount: i32) -> String {
        let mut result = format!("{{{}}}: ", entry.source);
        result.push_str(&to_string_signed(amount));

        if is_percentage {
            result.push_str("%");
        }

        result
    }

    /// Render the popup
    pub fn render(&mut self, ui: &mut Ui) {
        let mut popup = Popup::new(self.city_screen.clone());

        // Render the formatted lines
        let city_screen_clone = self.city_screen.clone();
        popup.add(MarkupRenderer::render(&self.lines, move |link| {
            city_screen_clone.open_civilopedia(link);
        }));

        // Add close button
        popup.add_close_button(None);

        // Open the popup
        popup.open(true);

        // Render the popup
        popup.render(ui);
    }
}