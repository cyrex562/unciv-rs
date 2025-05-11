use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui, Align, ScrollArea};
use std::collections::{HashMap, BTreeMap};
use std::f32;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::gui::GUI;
use crate::logic::city::{City, CityStats};
use crate::logic::city::stats::StatTreeNode;
use crate::models::stats::{Stat, Stats};
use crate::ui::components::extensions::{add_separator, brighten, darken, pack_if_needed, pad, surround_with_circle, to_label};
use crate::ui::components::input::{key_shortcuts, on_activation, onClick, KeyboardBinding, KeyCharAndCode};
use crate::ui::components::widgets::{AutoScrollPane, IconCircleGroup};
use crate::ui::popups::popup::Popup;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::utils::translations::tr;

/// Popup that displays detailed statistics for a city
pub struct DetailedStatsPopup {
    /// Reference to the parent city screen
    city_screen: Box<dyn CityScreen>,

    /// The header table containing column headers
    header_table: egui::Frame,

    /// The main table containing the statistics
    total_table: egui::Frame,

    /// The currently highlighted source
    source_highlighted: Option<String>,

    /// The stat to filter by, if any
    only_with_stat: Option<Stat>,

    /// Whether to show detailed information
    is_detailed: bool,

    /// Color for total rows
    color_total: Color32,

    /// Color for selected rows
    color_selector: Color32,

    /// Formatter for percentage values
    percent_formatter: String,

    /// Formatter for decimal values
    decimal_formatter: String,
}

impl DetailedStatsPopup {
    /// Create a new DetailedStatsPopup
    pub fn new(city_screen: Box<dyn CityScreen>) -> Self {
        let mut header_table = egui::Frame::none();
        header_table.set_style(egui::Style {
            padding: egui::style::Margin::same(3.0),
            ..Default::default()
        });

        let mut total_table = egui::Frame::none();
        total_table.set_style(egui::Style {
            padding: egui::style::Margin::same(3.0),
            ..Default::default()
        });

        Self {
            city_screen,
            header_table,
            total_table,
            source_highlighted: None,
            only_with_stat: None,
            is_detailed: false,
            color_total: brighten(Color32::BLUE, 0.5),
            color_selector: darken(Color32::GREEN, 0.5),
            percent_formatter: String::from("0.#%"),
            decimal_formatter: String::from("0.#"),
        }
    }

    /// Update the popup with the current city statistics
    pub fn update(&mut self) {
        self.header_table.clear();
        self.total_table.clear();

        let city_stats = &self.city_screen.get_city().city_stats;
        let show_faith = self.city_screen.get_city().civ.game_info.is_religion_enabled();

        // Determine which stats to display
        let stats = match self.only_with_stat {
            Some(stat) => vec![stat],
            None if !show_faith => Stat::values().iter()
                .filter(|&s| *s != Stat::Faith)
                .cloned()
                .collect(),
            None => Stat::values().to_vec(),
        };

        let column_count = stats.len() + 1;
        let stat_col_min_width = if self.only_with_stat.is_some() { 150.0 } else { 110.0 };

        // Add toggle button
        self.header_table.add(self.get_toggle_button(self.is_detailed)).min_width(150.0).grow();

        // Add stat headers
        for stat in &stats {
            let mut label = to_label(&stat.name);

            let stat_clone = *stat;
            label.on_click(move || {
                self.only_with_stat = if self.only_with_stat == Some(stat_clone) { None } else { Some(stat_clone) };
                self.update();
            });

            let color = if self.only_with_stat == Some(*stat) {
                Some(self.color_selector)
            } else {
                None
            };

            self.header_table.add(self.wrap_in_table(label, color)).min_width(stat_col_min_width).grow();
        }

        self.header_table.row();
        self.header_table.add(add_separator()).pad_bottom(2.0);

        // Add base values section
        let mut base_values_label = to_label("Base values");
        base_values_label.set_alignment(Align::Center);
        self.total_table.add(base_values_label).colspan(column_count).grow_x().row();

        self.total_table.add(add_separator().colspan(column_count)).pad_top(2.0);

        self.traverse_tree(&mut self.total_table, &stats, &city_stats.base_stat_tree, true, false);

        // Add bonuses section
        self.total_table.add(add_separator()).pad_bottom(2.0);

        let mut bonuses_label = to_label("Bonuses");
        bonuses_label.set_alignment(Align::Center);
        self.total_table.add(bonuses_label).colspan(column_count).grow_x().row();

        self.total_table.add(add_separator()).pad_top(2.0);

        self.traverse_tree(&mut self.total_table, &stats, &city_stats.stat_percent_bonus_tree, false, true);

        // Add final values section
        self.total_table.add(add_separator()).pad_bottom(2.0);

        let mut final_label = to_label("Final");
        final_label.set_alignment(Align::Center);
        self.total_table.add(final_label).colspan(column_count).grow_x().row();

        self.total_table.add(add_separator()).pad_top(2.0);

        let mut final_values = HashMap::new();
        let mut map = city_stats.final_stat_list.to_sorted_map();

        // Add happiness values
        for (key, value) in &self.city_screen.get_city().city_stats.happiness_list {
            if !map.contains_key(key) {
                let mut stats = Stats::new();
                stats.set(Stat::Happiness, *value);
                map.insert(key.clone(), stats);
            } else if map[key].get(Stat::Happiness) == 0.0 {
                map.get_mut(key).unwrap().set(Stat::Happiness, *value);
            }
        }

        // Add final values
        for (source, final_stats) in &map {
            if final_stats.is_empty() {
                continue;
            }

            if let Some(stat) = self.only_with_stat {
                if final_stats.get(stat) == 0.0 {
                    continue;
                }
            }

            let mut label = to_label(source, true);
            label.set_alignment(Align::Left);

            let source_clone = source.clone();
            label.on_click(move || {
                self.source_highlighted = if self.source_highlighted.as_ref() == Some(&source_clone) {
                    None
                } else {
                    Some(source_clone.clone())
                };
                self.update();
            });

            let color = if self.source_highlighted.as_ref() == Some(source) {
                Some(self.color_selector)
            } else {
                None
            };

            self.total_table.add(self.wrap_in_table(label, color, Align::Left)).grow();

            for stat in &stats {
                let value = final_stats.get(*stat);
                let cell = if value == 0.0 {
                    to_label("-")
                } else {
                    self.to_one_decimal_label(value)
                };

                self.total_table.add(self.wrap_in_table(cell, color)).grow();

                let entry = final_values.entry(*stat).or_insert(0.0);
                *entry += value;
            }

            self.total_table.row();
        }

        // Add total row
        let mut total_label = to_label("Total");
        self.total_table.add(self.wrap_in_table(total_label, Some(self.color_total))).grow();

        for stat in &stats {
            let value = final_values.get(stat).unwrap_or(&0.0);
            let cell = self.to_one_decimal_label(*value);

            self.total_table.add(self.wrap_in_table(cell, Some(self.color_total)))
                .min_width(stat_col_min_width)
                .grow();
        }

        self.total_table.row();

        // Equalize column widths
        pack_if_needed(&mut self.header_table);
        pack_if_needed(&mut self.total_table);

        let first_column_width = f32::max(
            self.total_table.get_column_width(0),
            self.header_table.get_column_width(0)
        );

        if let Some(cell) = self.header_table.get_cell_mut(0, 0) {
            cell.min_width(first_column_width);
        }

        if let Some(cell) = self.total_table.get_cell_mut(0, 0) {
            cell.min_width(first_column_width);
        }

        self.header_table.invalidate();
        self.total_table.invalidate();
    }

    /// Get the toggle button for showing/hiding detailed information
    fn get_toggle_button(&self, show_details: bool) -> IconCircleGroup {
        let label_text = if show_details { "-" } else { "+" };
        let mut label = to_label(label_text);
        label.set_alignment(Align::Center);

        let mut button = label
            .surround_with_circle(25.0, Some(BaseScreen::get_skin_color()))
            .surround_with_circle(27.0, false);

        let mut is_detailed = self.is_detailed;
        button.on_activation(KeyboardBinding::ShowStatDetails, move || {
            is_detailed = !is_detailed;
            self.update();
        });

        button.key_shortcuts_add(KeyCharAndCode::Plus);

        button
    }

    /// Traverse the stat tree and add rows to the table
    fn traverse_tree(
        &mut self,
        table: &mut egui::Frame,
        stats: &[Stat],
        stat_tree_node: &StatTreeNode,
        merge_happiness: bool,
        percentage: bool,
        indentation: i32
    ) {
        let mut total = HashMap::new();
        let mut map = stat_tree_node.children.to_sorted_map();

        // Merge happiness values if needed
        if merge_happiness {
            for (key, value) in &self.city_screen.get_city().city_stats.happiness_list {
                if !map.contains_key(key) {
                    let mut node = StatTreeNode::new();
                    node.set_inner_stat(Stat::Happiness, *value);
                    map.insert(key.clone(), node);
                } else if map[key].total_stats.get(Stat::Happiness) == 0.0 {
                    map.get_mut(key).unwrap().set_inner_stat(Stat::Happiness, *value);
                }
            }
        }

        // Process each child node
        for (name, child) in &map {
            let text = format!("{}{}", "- ".repeat(indentation as usize), tr(name));

            // Skip if all stats are zero
            if child.total_stats.is_all_zero() {
                table.row();
                continue;
            }

            // Skip if filtered by stat and this node has zero for that stat
            if let Some(stat) = self.only_with_stat {
                if child.total_stats.get(stat) == 0.0 {
                    table.row();
                    continue;
                }
            }

            let mut label = to_label(&text, true);
            label.set_alignment(Align::Left);

            let text_clone = text.clone();
            label.on_click(move || {
                self.source_highlighted = if self.source_highlighted.as_ref() == Some(&text_clone) {
                    None
                } else {
                    Some(text_clone.clone())
                };
                self.update();
            });

            let color = if self.source_highlighted.as_ref() == Some(&text) {
                Some(self.color_selector)
            } else {
                None
            };

            table.add(self.wrap_in_table(label, color, Align::Left)).fill().left();

            // Add stat values
            for stat in stats {
                let value = child.total_stats.get(*stat);
                let cell = if value == 0.0 {
                    to_label("-")
                } else if percentage {
                    self.to_percent_label(value)
                } else {
                    self.to_one_decimal_label(value)
                };

                table.add(self.wrap_in_table(cell, color)).grow();

                // Update total for top-level nodes
                if indentation == 0 {
                    let entry = total.entry(*stat).or_insert(0.0);
                    *entry += value;
                }
            }

            table.row();

            // Recursively process child nodes if detailed view is enabled
            if self.is_detailed {
                self.traverse_tree(table, stats, child, false, percentage, indentation + 1);
            }
        }

        // Add total row for top-level nodes
        if indentation == 0 {
            let mut total_label = to_label("Total");
            table.add(self.wrap_in_table(total_label, Some(self.color_total))).grow();

            for stat in stats {
                let value = total.get(stat).unwrap_or(&0.0);
                let cell = if percentage {
                    self.to_percent_label(*value)
                } else {
                    self.to_one_decimal_label(*value)
                };

                table.add(self.wrap_in_table(cell, Some(self.color_total))).grow();
            }

            table.row();
        }
    }

    /// Wrap a label in a table with optional background color
    fn wrap_in_table(&self, label: egui::Label, color: Option<Color32>, align: Align) -> egui::Frame {
        let mut table = egui::Frame::none();

        if let Some(color) = color {
            table.set_style(egui::Style {
                background: Some(BaseScreen::get_ui_background("General/Border", color)),
                ..Default::default()
            });
        }

        table.add(label).grow_x();

        table
    }

    /// Convert a float to a percent label
    fn to_percent_label(&self, value: f32) -> egui::Label {
        let formatted = format!("{:+}%", value * 100.0);
        to_label(&formatted)
    }

    /// Convert a float to a one decimal label
    fn to_one_decimal_label(&self, value: f32) -> egui::Label {
        let formatted = format!("{:.1}", value);
        to_label(&formatted)
    }

    /// Render the popup
    pub fn render(&mut self, ui: &mut Ui) {
        let mut popup = Popup::new(self.city_screen.clone(), false);

        popup.add(&mut self.header_table).pad_bottom(0.0).row();

        let mut scroll_pane = AutoScrollPane::new(&mut self.total_table);
        scroll_pane.set_overscroll(false, false);

        let scroll_pane_cell = popup.add(scroll_pane).pad_top(0.0);
        scroll_pane_cell.max_height(self.city_screen.stage().height() * 3.0 / 4.0);

        popup.row();
        popup.add_close_button(Some(KeyCharAndCode::Space));

        popup.render(ui);
    }
}