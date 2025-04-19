use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, Slider, Ui, Vec2};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::mapeditorscreen::MapEditorScreen;
use crate::ui::components::widgets::tabbed_pager::TabbedPager;
use crate::ui::components::widgets::tabbed_pager::PageExtensions;
use crate::ui::components::widgets::expander_tab::ExpanderTab;
use crate::ui::components::widgets::wrappable_label::WrappableLabel;
use crate::ui::components::widgets::unciv_slider::UncivSlider;
use crate::ui::components::extensions::*;
use crate::ui::popups::toast_popup::ToastPopup;
use crate::ui::screens::civilopediascreen::{FormattedLine, MarkupRenderer, IconDisplay};
use crate::logic::game_info::GameInfo;
use crate::logic::civilization::Civilization;
use crate::logic::map::tile_map::TileMap;
use crate::logic::map::tile::Tile;
use crate::logic::map::tile_description::TileDescription;
use crate::models::counter::Counter;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::nation::Nation;
use crate::models::ruleset::resource_type::ResourceType;
use crate::models::stats::Stats;
use crate::models::translations::tr;
use crate::utils::log::Log;
use crate::UncivGame;

/// Tab for viewing map information
pub struct MapEditorViewTab {
    editor_screen: Rc<RefCell<MapEditorScreen>>,
    tile_data_cell: Option<egui::Frame>,
    mock_civ: Civilization,
    natural_wonders: Counter<String>,
    round_robin_index: usize,
    label_width: f32,
}

impl MapEditorViewTab {
    pub fn new(editor_screen: Rc<RefCell<MapEditorScreen>>) -> Self {
        let label_width = editor_screen.borrow().get_tools_width() - 40.0;

        Self {
            editor_screen,
            tile_data_cell: None,
            mock_civ: Self::create_mock_civ(&editor_screen.borrow().ruleset),
            natural_wonders: Counter::new(),
            round_robin_index: 0,
            label_width,
        }
    }

    fn create_mock_civ(ruleset: &Ruleset) -> Civilization {
        let mut civ = Civilization::new();

        // This construct exists only to allow us to call Tile.TileStatFunctions.getTileStats
        let mut nation = Nation::new();
        nation.name = "Test".to_string();
        civ.nation = nation;

        let mut game_info = GameInfo::new();
        game_info.ruleset = ruleset.clone();
        civ.game_info = game_info;
        civ.cache.update_state();

        // Show yields of strategic resources too
        for tech in ruleset.technologies.keys() {
            civ.tech.techs_researched.insert(tech.clone());
        }

        civ
    }

    fn update_mock_civ(&mut self, ruleset: &Ruleset) {
        if self.mock_civ.game_info.ruleset.id == ruleset.id {
            return;
        }

        self.mock_civ.game_info.ruleset = ruleset.clone();

        // Show yields of strategic resources too
        for tech in ruleset.technologies.keys() {
            self.mock_civ.tech.techs_researched.insert(tech.clone());
        }
    }

    pub fn update(&mut self) {
        let editor = self.editor_screen.borrow();
        self.update_mock_civ(&editor.ruleset);

        let tile_map = &editor.tile_map;

        // Try to assign continents
        match tile_map.assign_continents(TileMap::AssignContinentsMode::Ensure) {
            Ok(_) => {},
            Err(e) => {
                ToastPopup::new(&format!("Error assigning continents: {}", e), &editor).show();
            }
        }

        // Update natural wonders if needed
        if editor.natural_wonders_need_refresh {
            self.natural_wonders.clear();

            for tile in tile_map.values() {
                if let Some(wonder) = &tile.natural_wonder {
                    self.natural_wonders.add(wonder.clone(), 1);
                }
            }

            // Sort natural wonders
            let mut wonders: Vec<_> = self.natural_wonders.iter().collect();
            wonders.sort_by(|a, b| a.0.cmp(b.0));

            // Clear and re-add sorted wonders
            self.natural_wonders.clear();
            for (wonder, count) in wonders {
                self.natural_wonders.add(wonder.clone(), count);
            }

            editor.natural_wonders_need_refresh = false;
        }
    }

    fn tile_click_handler(&mut self, tile: &Tile) {
        let mut lines = Vec::new();

        // Position
        lines.push(FormattedLine::new(&format!("Position: [{}]",
            tile.position.to_string().replace(".0", ""))));
        lines.push(FormattedLine::empty());

        // Tile description
        lines.extend(TileDescription::to_markup(tile, None));

        // Stats
        let stats = match tile.stats.get_tile_stats(None, &self.mock_civ) {
            Ok(stats) => stats,
            Err(e) => {
                // Maps aren't always fixed to remove dead references... like resource "Gold"
                if let Some(msg) = e.to_string().as_ref() {
                    ToastPopup::new(msg, &self.editor_screen.borrow()).show();
                }
                Stats::new()
            }
        };

        if !stats.is_empty() {
            lines.push(FormattedLine::empty());
            lines.push(FormattedLine::new(&stats.to_string()));
        }

        // Starting locations
        let nations = tile_map_get_tile_starting_location_summary(tile);
        if !nations.is_empty() {
            lines.push(FormattedLine::empty());
            lines.push(FormattedLine::new(&format!("Starting location(s): [{}]", nations)));
        }

        // Continent
        let continent = tile.get_continent();
        if continent >= 0 {
            lines.push(FormattedLine::empty());
            lines.push(FormattedLine::with_link(
                &format!("Continent: [{}] ([{}] tiles)",
                    continent,
                    tile.tile_map.continent_sizes[continent as usize]),
                "continent"
            ));
        }

        // Render the info
        let mut rendered_info = MarkupRenderer::render(&lines, self.label_width, |link| {
            if link == "continent" {
                // Visualize the continent this tile is on
                let mut editor = self.editor_screen.borrow_mut();
                editor.hide_selection();

                let color = Color32::from_rgb(139, 69, 19).darken(0.5);
                for mark_tile in tile.tile_map.values() {
                    if mark_tile.get_continent() == continent {
                        editor.highlight_tile(mark_tile, color);
                    }
                }
            } else {
                // This needs CivilopediaScreen to be able to work without a GameInfo!
                let mut editor = self.editor_screen.borrow_mut();
                editor.open_civilopedia(link);
            }
        });

        // Resource abundance slider
        if tile.resource.is_some() && (tile.resource_amount > 0 || tile.tile_resource.resource_type == ResourceType::Strategic) {
            rendered_info.add_separator(Color32::GRAY);

            let mut frame = egui::Frame::none();
            frame.fill(egui::Color32::TRANSPARENT);

            frame.show(rendered_info.ui(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Resource abundance");

                    let mut amount = tile.resource_amount as f32;
                    let slider = UncivSlider::new(0.0, 42.0, 1.0, amount)
                        .on_change(|value| {
                            let mut editor = self.editor_screen.borrow_mut();
                            tile.resource_amount = value as i32;
                            editor.update_tile(tile);
                            editor.is_dirty = true;
                        });

                    slider.set_snap_to_values(5.0, &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0, 20.0, 30.0, 40.0]);

                    ui.add_space(80.0);
                    ui.add(slider);
                });
            });
        }

        self.tile_data_cell = Some(rendered_info);

        let mut editor = self.editor_screen.borrow_mut();
        editor.hide_selection();
        editor.highlight_tile(tile, Color32::from_rgb(255, 127, 80)); // CORAL
    }

    fn scroll_to_wonder(&mut self, name: &str) {
        let editor = self.editor_screen.borrow();
        let tiles: Vec<_> = editor.tile_map.values()
            .filter(|t| t.natural_wonder.as_ref().map_or(false, |w| w == name))
            .collect();

        self.scroll_to_next_tile_of(&tiles);
    }

    fn scroll_to_start_of_nation(&mut self, name: &str) {
        let editor = self.editor_screen.borrow();
        let tiles = editor.tile_map.starting_locations_by_nation.get(name)
            .map(|tiles| tiles.iter().collect::<Vec<_>>());

        if let Some(tiles) = tiles {
            self.scroll_to_next_tile_of(&tiles);
        }
    }

    fn scroll_to_next_tile_of(&mut self, tiles: &[&Tile]) {
        if tiles.is_empty() {
            return;
        }

        if self.round_robin_index >= tiles.len() {
            self.round_robin_index = 0;
        }

        let tile = tiles[self.round_robin_index];
        self.round_robin_index += 1;

        let mut editor = self.editor_screen.borrow_mut();
        editor.map_holder.set_center_position(tile.position, true);
        self.tile_click_handler(tile);
    }
}

impl PageExtensions for MapEditorViewTab {
    fn activated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        let mut editor = self.editor_screen.borrow_mut();
        editor.tile_click_handler = Some(Box::new(|tile| {
            self.tile_click_handler(tile);
        }));
        self.update();
    }

    fn deactivated(&mut self, _index: usize, _caption: &str, _pager: &mut TabbedPager) {
        let mut editor = self.editor_screen.borrow_mut();
        editor.hide_selection();
        self.tile_data_cell = None;
        editor.tile_click_handler = None;
    }
}

impl MapEditorViewTab {
    pub fn render(&mut self, ui: &mut Ui) {
        let editor = self.editor_screen.borrow();
        let tile_map = &editor.tile_map;

        ui.vertical(|ui| {
            // Map parameters
            let header_text = if tile_map.map_parameters.name.is_empty() {
                "New map".to_string()
            } else {
                tile_map.map_parameters.name.clone()
            };

            ExpanderTab::new(&header_text, false, 0.0, |ui| {
                let map_parameter_text = tile_map.map_parameters.to_string()
                    .replace(&format!("\"{}\" ", tile_map.map_parameters.name), "");

                let mut label = WrappableLabel::new(&map_parameter_text, self.label_width);
                label.wrap = true;
                ui.add(label);
            });

            // Map statistics
            let area = tile_map.values.len();
            let water_count = tile_map.values.iter().filter(|t| t.is_water).count();
            let water_percent = (water_count as f32 * 100.0 / area as f32) as i32;

            let impassable_count = tile_map.values.iter().filter(|t| t.is_impassible()).count();
            let impassable_percent = (impassable_count as f32 * 100.0 / area as f32) as i32;

            let continents = tile_map.continent_sizes.len();

            let stats_text = format!(
                "Area: [{}] tiles, [{}]% water, [{}]% impassable, [{}] continents/islands",
                area, water_percent, impassable_percent, continents
            );

            let mut label = WrappableLabel::new(&stats_text, self.label_width);
            label.wrap = true;
            ui.add(label);

            // Description
            ui.add(editor.description_text_field.clone());

            // Natural wonders
            if !this.natural_wonders.is_empty() {
                let mut lines = Vec::new();

                for (wonder, count) in this.natural_wonders.iter() {
                    let text = if *count == 1 {
                        wonder.clone()
                    } else {
                        format!("{{{}}} ({})", wonder, count)
                    };

                    lines.push(FormattedLine::with_link(&text, &format!("Terrain/{}", wonder)));
                }

                ExpanderTab::new(&format!("{{Natural Wonders}} ({})", this.natural_wonders.len()), 21, false, 5.0, |ui| {
                    MarkupRenderer::render_with_icon_display(&lines, IconDisplay::NoLink, |name| {
                        self.scroll_to_wonder(name);
                    });
                });
            }

            // Starting locations
            if !tile_map.starting_locations_by_nation.is_empty() {
                let mut lines = Vec::new();

                for (nation_name, count) in tile_map_get_starting_location_summary(tile_map) {
                    let text = if count == 1 {
                        nation_name.clone()
                    } else {
                        format!("{{{}}} ({})", nation_name, count)
                    };

                    lines.push(FormattedLine::with_link(&text, &format!("Nation/{}", nation_name)));
                }

                ExpanderTab::new(&format!("{{Starting locations}} ({})", tile_map.starting_locations_by_nation.len()), 21, false, 5.0, |ui| {
                    MarkupRenderer::render_with_icon_display(&lines, IconDisplay::NoLink, |name| {
                        self.scroll_to_start_of_nation(name);
                    });
                });
            }

            ui.add_space(10.0);
            ui.separator();

            // Tile data
            if let Some(frame) = &self.tile_data_cell {
                frame.show(ui, |ui| {});
            }

            ui.add_space(10.0);
            ui.separator();

            // Exit button
            if ui.button("Exit map editor").clicked() {
                let mut editor = self.editor_screen.borrow_mut();
                editor.close_editor();
            }
        });
    }
}

// Helper functions

fn tile_map_get_tile_starting_location_summary(tile: &Tile) -> String {
    let mut result = Vec::new();

    for location in &tile.tile_map.starting_locations {
        if location.position == tile.position {
            if let Some(nation) = tile.tile_map.ruleset.as_ref().and_then(|r| r.nations.get(&location.nation)) {
                let usage = &location.usage;
                result.push(format!("{{{}}} ({})", nation.name, usage.label));
            }
        }
    }

    // Sort by city state status and then by name
    result.sort_by(|a, b| {
        // This is a simplified version of the Kotlin sorting
        a.cmp(b)
    });

    result.join(", ")
}

fn tile_map_get_starting_location_summary(tile_map: &TileMap) -> Vec<(String, i32)> {
    let mut result = Vec::new();

    for (nation_name, locations) in &tile_map.starting_locations_by_nation {
        if let Some(nation) = tile_map.ruleset.as_ref().and_then(|r| r.nations.get(nation_name)) {
            result.push((nation.name.clone(), locations.len() as i32));
        }
    }

    // Sort by city state status and then by name
    result.sort_by(|a, b| {
        // This is a simplified version of the Kotlin sorting
        a.0.cmp(&b.0)
    });

    result
}