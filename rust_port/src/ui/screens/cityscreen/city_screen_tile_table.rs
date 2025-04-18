use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui};
use std::collections::HashMap;

use crate::gui::GUI;
use crate::logic::city::City;
use crate::logic::map::tile::{Tile, TileDescription};
use crate::models::stats::{Stat, Stats};
use crate::ui::components::extensions::{darken, disable, is_enabled, to_label, to_text_button};
use crate::ui::components::input::{key_shortcuts, on_activation, onClick, KeyboardBinding};
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::ui::screens::civilopediascreen::{FormattedLine, IconDisplay, MarkupRenderer};
use crate::utils::translations::tr;

/// Table that displays information about a selected tile in the city screen
pub struct CityScreenTileTable {
    /// Reference to the parent city screen
    city_screen: Box<dyn CityScreen>,

    /// The city this table belongs to
    city: City,

    /// The inner table containing the actual content
    inner_table: egui::Frame,

    /// Whether the table is currently visible
    is_visible: bool,

    /// The current width of the table
    width: f32,

    /// The current height of the table
    height: f32,

    /// The current x position of the table
    x: f32,

    /// The current y position of the table
    y: f32,

    /// The current top position of the table
    top: f32,
}

impl CityScreenTileTable {
    /// Create a new CityScreenTileTable
    pub fn new(city_screen: Box<dyn CityScreen>) -> Self {
        let city = city_screen.get_city().clone();

        let mut inner_table = egui::Frame::none();
        inner_table.set_style(egui::Style {
            background: Some(egui::Color32::from_rgba_premultiplied(40, 40, 40, 200)),
            ..Default::default()
        });

        Self {
            city_screen,
            city,
            inner_table,
            is_visible: false,
            width: 0.0,
            height: 0.0,
            x: 0.0,
            y: 0.0,
            top: 0.0,
        }
    }

    /// Update the table with a selected tile
    pub fn update(&mut self, selected_tile: Option<&Tile>) {
        self.inner_table.clear();

        if selected_tile.is_none() {
            self.is_visible = false;
            return;
        }

        self.is_visible = true;
        let selected_tile = selected_tile.unwrap();

        // Get tile stats
        let stats = selected_tile.stats.get_tile_stats(&self.city, &self.city.civ);

        // Add tile description
        let markup = TileDescription::to_markup(selected_tile, &self.city.civ);
        let description = MarkupRenderer::render(&markup, IconDisplay::None, |text| {
            self.city_screen.open_civilopedia(text);
        });

        self.inner_table.add(description);
        self.inner_table.add_space(5.0);

        // Add tile stats
        self.inner_table.add(self.get_tile_stats_table(&stats));
        self.inner_table.add_space(5.0);

        // Add buy tile button if applicable
        if self.city.expansion.can_buy_tile(selected_tile) {
            let gold_cost_of_tile = self.city.expansion.get_gold_cost_of_tile(selected_tile);
            let mut buy_tile_button = to_text_button(&format!("Buy for [{}] gold", gold_cost_of_tile));

            buy_tile_button.on_activation(KeyboardBinding::BuyTile, move || {
                buy_tile_button.disable();
                self.city_screen.ask_to_buy_tile(selected_tile);
            });

            buy_tile_button.set_enabled(
                self.city_screen.can_city_be_changed() &&
                self.city.civ.has_stat_to_buy(Stat::Gold, gold_cost_of_tile)
            );

            self.inner_table.add(buy_tile_button);
            self.inner_table.add_space(5.0);
        }

        // Add ownership information
        if let Some(owning_city) = &selected_tile.owning_city {
            self.inner_table.add(to_label(&format!("Owned by [{}]", owning_city.name)));
            self.inner_table.add_space(5.0);
        }

        if let Some(working_city) = selected_tile.get_working_city() {
            self.inner_table.add(to_label(&format!("Worked by [{}]", working_city.name)));
            self.inner_table.add_space(5.0);
        }

        // Add lock/unlock button if tile is worked
        if self.city.is_worked(selected_tile) {
            if selected_tile.is_locked() {
                let mut unlock_button = to_text_button("Unlock");
                unlock_button.on_click(move || {
                    self.city.locked_tiles.remove(&selected_tile.position);
                    self.update(Some(selected_tile));
                    self.city_screen.update();
                });

                if !self.city_screen.can_city_be_changed() {
                    unlock_button.disable();
                }

                self.inner_table.add(unlock_button);
                self.inner_table.add_space(5.0);
            } else {
                let mut lock_button = to_text_button("Lock");
                lock_button.on_click(move || {
                    self.city.locked_tiles.insert(selected_tile.position);
                    self.update(Some(selected_tile));
                    self.city_screen.update();
                });

                if !self.city_screen.can_city_be_changed() {
                    lock_button.disable();
                }

                self.inner_table.add(lock_button);
                self.inner_table.add_space(5.0);
            }
        }

        // Add move to city button if applicable
        if selected_tile.is_city_center() &&
           selected_tile.get_city().is_some() &&
           selected_tile.get_city().unwrap() != self.city &&
           selected_tile.get_city().unwrap().civ == self.city.civ {

            let mut move_button = to_text_button("Move to city");
            move_button.on_click(move || {
                let city = selected_tile.get_city().unwrap();
                let new_screen = CityScreen::new(city.clone(), None, None, None);
                GUI::replace_current_screen(Box::new(new_screen));
            });

            self.inner_table.add(move_button);
        }

        // Pack the table
        self.inner_table.pack();
    }

    /// Create a table displaying tile stats
    fn get_tile_stats_table(&self, stats: &Stats) -> egui::Frame {
        let mut stats_table = egui::Frame::none();

        for (key, value) in stats {
            let icon = ImageGetter::get_stat_icon(&key.name);
            stats_table.add(icon).size(20.0);

            let value_label = to_label(&format!("{}", value.round() as i32));
            stats_table.add(value_label).pad_right(5.0);
        }

        stats_table
    }

    /// Set the position of the table
    pub fn set_position(&mut self, x: f32, y: f32, align: egui::Align) {
        self.x = x;
        self.y = y;

        match align {
            egui::Align::TOP_LEFT => {
                self.inner_table.set_position(x, y);
            },
            egui::Align::TOP_RIGHT => {
                self.inner_table.set_position(x - self.width, y);
            },
            egui::Align::BOTTOM_LEFT => {
                self.inner_table.set_position(x, y - self.height);
            },
            egui::Align::BOTTOM_RIGHT => {
                self.inner_table.set_position(x - self.width, y - self.height);
            },
            egui::Align::TOP => {
                self.inner_table.set_position(x - self.width / 2.0, y);
            },
            egui::Align::BOTTOM => {
                self.inner_table.set_position(x - self.width / 2.0, y - self.height);
            },
            egui::Align::LEFT => {
                self.inner_table.set_position(x, y - self.height / 2.0);
            },
            egui::Align::RIGHT => {
                self.inner_table.set_position(x - self.width, y - self.height / 2.0);
            },
            egui::Align::CENTER => {
                self.inner_table.set_position(x - self.width / 2.0, y - self.height / 2.0);
            },
        }
    }

    /// Pack the table to calculate its size
    pub fn pack_if_needed(&mut self) -> &Self {
        if self.width == 0.0 || self.height == 0.0 {
            let (width, height) = self.inner_table.pack();
            self.width = width;
            self.height = height;
            self.top = self.y - self.height;
        }

        self
    }

    /// Get the width of the table
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get the height of the table
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Get the x position of the table
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Get the y position of the table
    pub fn y(&self) -> f32 {
        self.y
    }

    /// Get the top position of the table
    pub fn top(&self) -> f32 {
        self.top
    }

    /// Check if the table is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Render the table
    pub fn render(&mut self, ui: &mut Ui) {
        if self.is_visible {
            self.inner_table.render(ui);
        }
    }
}