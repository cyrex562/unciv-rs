use ggez::graphics::{Color, DrawParam, Image, Mesh, Rect, Text};
use ggez::Context;
use std::sync::Arc;
use std::collections::HashMap;

use crate::constants::Constants;
use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::diplomacy::RelationshipLevel;
use crate::models::game_info::GameInfo;
use crate::models::population::Population;
use crate::models::religion::Religion;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::models::unit::Unit;
use crate::ui::components::tilegroups::TileGroup;
use crate::ui::images::ImageGetter;
use crate::ui::screens::city_screen::CityScreen;
use crate::ui::screens::diplomacy_screen::DiplomacyScreen;
use crate::ui::screens::world_screen::WorldScreen;
use crate::ui::utils::debug_utils::DebugUtils;
use crate::ui::utils::font_utils::Fonts;
use crate::ui::utils::gui::GUI;
use crate::ui::utils::popup::Popup;
use crate::ui::utils::sound::UncivSound;

// Constants for colors
pub const COLOR_CONSTRUCTION: Color = Color::new(196.0/255.0, 140.0/255.0, 62.0/255.0, 1.0);
pub const COLOR_GROWTH: Color = Color::new(130.0/255.0, 225.0/255.0, 78.0/255.0, 1.0);

// Enum for hidden unit marker positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenUnitMarkerPosition {
    Left,
    Center,
    Right,
}

// InfluenceTable component
pub struct InfluenceTable {
    width: f32,
    height: f32,
    influence: f32,
    relationship_level: RelationshipLevel,
}

impl InfluenceTable {
    pub fn new(influence: f32, relationship_level: RelationshipLevel, width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            influence,
            relationship_level,
        }
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        // Draw background
        let bg_color = ImageGetter::CHARCOAL;
        let bg_rect = Rect::new(x, y, self.width, self.height);
        let bg_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bg_rect, bg_color).unwrap();
        bg_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Calculate normalized influence
        let normalized_influence = (self.influence.max(-60.0).min(60.0)) / 30.0;

        // Determine color based on relationship level
        let color = match self.relationship_level {
            RelationshipLevel::Unforgivable => Color::RED,
            RelationshipLevel::Enemy => Color::ORANGE,
            RelationshipLevel::Afraid => Color::YELLOW,
            RelationshipLevel::Neutral | RelationshipLevel::Friend => Color::LIME,
            RelationshipLevel::Ally => Color::SKY,
            _ => Color::DARK_GRAY,
        };

        // Calculate percentages for each bar segment
        let mut percentages = [0.0; 4];
        if normalized_influence < -1.0 {
            percentages[0] = -normalized_influence - 1.0;
            percentages[1] = 1.0;
        } else if normalized_influence < 0.0 {
            percentages[1] = -normalized_influence;
        } else if normalized_influence < 1.0 {
            percentages[2] = normalized_influence;
        } else {
            percentages[2] = 1.0;
            percentages[3] = normalized_influence - 1.0;
        }

        // Draw each bar segment
        let bar_piece_size = self.width / 4.0;
        for i in 0..4 {
            self.draw_bar_piece(ctx, x + i as f32 * bar_piece_size, y, percentages[i], color, i < 2);
        }
    }

    fn draw_bar_piece(&self, ctx: &mut Context, x: f32, y: f32, percentage: f32, color: Color, negative: bool) {
        let bar_piece_size = self.width / 4.0;

        if negative {
            // Draw empty part
            let empty_rect = Rect::new(x, y, (1.0 - percentage) * bar_piece_size, self.height);
            let empty_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), empty_rect, Color::DARK_GRAY).unwrap();
            empty_mesh.draw(ctx, DrawParam::default()).unwrap();

            // Draw filled part
            let full_rect = Rect::new(x + (1.0 - percentage) * bar_piece_size, y, percentage * bar_piece_size, self.height);
            let full_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), full_rect, color).unwrap();
            full_mesh.draw(ctx, DrawParam::default()).unwrap();
        } else {
            // Draw filled part
            let full_rect = Rect::new(x, y, percentage * bar_piece_size, self.height);
            let full_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), full_rect, color).unwrap();
            full_mesh.draw(ctx, DrawParam::default()).unwrap();

            // Draw empty part
            let empty_rect = Rect::new(x + percentage * bar_piece_size, y, (1.0 - percentage) * bar_piece_size, self.height);
            let empty_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), empty_rect, Color::DARK_GRAY).unwrap();
            empty_mesh.draw(ctx, DrawParam::default()).unwrap();
        }
    }
}

// DefenceTable component
pub struct DefenceTable {
    city: City,
    width: f32,
    height: f32,
}

impl DefenceTable {
    pub fn new(city: City) -> Self {
        Self {
            city,
            width: 100.0,
            height: 30.0,
        }
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        let selected_civ = GUI::get_selected_player();
        let border_size = 4.0;
        let bg_color = ImageGetter::CHARCOAL;

        // Determine border color based on relationship
        let bg_border_color = if self.city.civ == selected_civ {
            Color::new(255.0/255.0, 237.0/255.0, 200.0/255.0, 1.0)
        } else if self.city.civ.is_at_war_with(selected_civ) {
            Color::RED
        } else {
            ImageGetter::CHARCOAL
        };

        // Draw background with rounded top edge
        let bg_rect = Rect::new(x, y, self.width, self.height);
        let bg_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bg_rect, bg_color).unwrap();
        bg_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw border
        let border_rect = Rect::new(x, y, self.width, border_size);
        let border_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), border_rect, bg_border_color).unwrap();
        border_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw city strength
        let city_strength = self.city.get_defending_strength();
        let strength_text = format!("{} {}", Fonts::STRENGTH, city_strength);
        let text = Text::new(strength_text);
        text.draw(ctx, DrawParam::default().dest([x + self.width/2.0, y + self.height/2.0])).unwrap();
    }
}

// AirUnitTable component
pub struct AirUnitTable {
    city: City,
    number_of_units: i32,
    size: f32,
}

impl AirUnitTable {
    pub fn new(city: City, number_of_units: i32, size: f32) -> Self {
        Self {
            city,
            number_of_units,
            size,
        }
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        let text_color = self.city.civ.nation.get_inner_color();
        let bg_color = self.city.civ.nation.get_outer_color();

        // Draw background with rounded edges
        let bg_rect = Rect::new(x, y, self.size * 2.0, self.size);
        let bg_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bg_rect, bg_color).unwrap();
        bg_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw aircraft icon
        let aircraft_image = ImageGetter::get_image("OtherIcons/Aircraft");
        aircraft_image.set_color(text_color);
        aircraft_image.set_size(self.size, self.size);
        aircraft_image.draw(ctx, DrawParam::default().dest([x + 2.0, y + 2.0])).unwrap();

        // Draw number of units
        let units_text = format!("{}", self.number_of_units);
        let text = Text::new(units_text);
        text.draw(ctx, DrawParam::default().dest([x + self.size + 5.0, y + self.size/2.0])).unwrap();
    }
}

// StatusTable component
pub struct StatusTable {
    city: City,
    icon_size: f32,
}

impl StatusTable {
    pub fn new(city: City, icon_size: f32) -> Self {
        Self {
            city,
            icon_size,
        }
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        let selected_civ = GUI::get_selected_player();
        let pad_between = 2.0;
        let mut current_x = x;

        // Draw status icons based on city state
        if self.city.civ == selected_civ {
            if self.city.is_blockaded() {
                let blockade_image = ImageGetter::get_image("OtherIcons/Blockade");
                blockade_image.set_size(self.icon_size, self.icon_size);
                blockade_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
                current_x += self.icon_size + pad_between;

                // Display tutorial for city blockade
                GUI::get_world_screen().display_tutorial("CityBlockade");
            } else if !self.city.is_capital() && self.city.is_connected_to_capital() {
                let connection_image = ImageGetter::get_stat_icon("CityConnection");
                connection_image.set_size(self.icon_size, self.icon_size);
                connection_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
                current_x += self.icon_size + pad_between;
            }
        }

        if self.city.is_in_resistance() {
            let resistance_image = ImageGetter::get_image("StatIcons/Resistance");
            resistance_image.set_size(self.icon_size, self.icon_size);
            resistance_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
            current_x += self.icon_size + pad_between;
        }

        if self.city.is_puppet() {
            let puppet_image = ImageGetter::get_image("OtherIcons/Puppet");
            puppet_image.set_size(self.icon_size, self.icon_size);
            puppet_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
            current_x += self.icon_size + pad_between;
        }

        if self.city.is_being_razed() {
            let fire_image = ImageGetter::get_image("OtherIcons/Fire");
            fire_image.set_size(self.icon_size, self.icon_size);
            fire_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
            current_x += self.icon_size + pad_between;
        }

        if self.city.civ == selected_civ && self.city.is_we_love_the_king_day_active() {
            let wltkd_image = ImageGetter::get_image("OtherIcons/WLTKD");
            wltkd_image.set_size(self.icon_size, self.icon_size);
            wltkd_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
        }
    }
}

// CityTable component
pub struct CityTable {
    city: City,
    for_popup: bool,
    width: f32,
    height: f32,
}

impl CityTable {
    pub fn new(city: City, for_popup: bool) -> Self {
        Self {
            city,
            for_popup,
            width: 200.0,
            height: 50.0,
        }
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        let selected_civ = GUI::get_selected_player();
        let viewing_civ = GUI::get_viewing_player();
        let text_color = self.city.civ.nation.get_inner_color();

        // Determine border color based on relationship
        let bg_border_color = if self.city.civ == selected_civ {
            Color::new(233.0/255.0, 233.0/255.0, 172.0/255.0, 1.0)
        } else if self.city.civ.is_at_war_with(selected_civ) {
            Color::new(230.0/255.0, 51.0/255.0, 0.0/255.0, 1.0)
        } else {
            ImageGetter::CHARCOAL
        };

        // Determine border size
        let border_size = if self.city.civ == selected_civ || self.city.civ.is_at_war_with(selected_civ) {
            4.0
        } else {
            2.0
        };

        // Set background color
        let mut bg_color = self.city.civ.nation.get_outer_color();
        bg_color.a = 0.9;

        // Draw background with rounded edges
        let bg_rect = Rect::new(x, y, self.width, self.height);
        let bg_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bg_rect, bg_color).unwrap();
        bg_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw border
        let border_rect = Rect::new(x, y, self.width, border_size);
        let border_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), border_rect, bg_border_color).unwrap();
        border_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Determine if detailed info should be shown
        let is_show_detailed_info = DebugUtils::VISIBLE_MAP ||
                                   self.city.civ == selected_civ ||
                                   viewing_civ.is_spectator();

        // Draw city population
        self.draw_city_pop_number(ctx, x + 4.0, y + 5.0, text_color);

        // Draw city growth bar if detailed info is shown
        if is_show_detailed_info {
            self.draw_city_growth_bar(ctx, x + 30.0, y + 5.0, text_color);
        }

        // Draw city text (name, capital icon, religion)
        self.draw_city_text(ctx, x + 10.0, y + 15.0, text_color);

        // Draw city construction if detailed info is shown
        if is_show_detailed_info {
            self.draw_city_construction(ctx, x + self.width - 60.0, y + 5.0, text_color);
        }

        // Draw civilization icon if not viewing own civilization
        if self.city.civ != viewing_civ {
            self.draw_civ_icon(ctx, x + self.width - 30.0, y + 15.0);
        }
    }

    fn draw_city_pop_number(&self, ctx: &mut Context, x: f32, y: f32, text_color: Color) {
        let pop_text = format!("{}", self.city.population.population);
        let text = Text::new(pop_text);
        text.draw(ctx, DrawParam::default().dest([x, y]).color(text_color)).unwrap();
    }

    fn draw_city_growth_bar(&self, ctx: &mut Context, x: f32, y: f32, text_color: Color) {
        let mut growth_percentage = self.city.population.food_stored as f32 /
                                   self.city.population.get_food_to_next_population() as f32;
        growth_percentage = growth_percentage.max(0.0).min(1.0);

        // Determine turn label text
        let turn_label_text = if self.city.is_growing() {
            let turns_to_growth = self.city.population.get_num_turns_to_new_population();
            if turns_to_growth.is_some() && turns_to_growth.unwrap() < 100 {
                format!("{}", turns_to_growth.unwrap())
            } else {
                Fonts::INFINITY.to_string()
            }
        } else if self.city.is_starving() {
            let turns_to_starvation = self.city.population.get_num_turns_to_starvation();
            if turns_to_starvation.is_some() && turns_to_starvation.unwrap() < 100 {
                format!("{}", turns_to_starvation.unwrap())
            } else {
                Fonts::INFINITY.to_string()
            }
        } else {
            "-".to_string()
        };

        // Draw growth bar
        let bar_color = if self.city.is_starving() { Color::RED } else { COLOR_GROWTH };
        let bar_rect = Rect::new(x, y, 4.0, 30.0 * growth_percentage);
        let bar_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bar_rect, bar_color).unwrap();
        bar_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw turn label
        let text = Text::new(turn_label_text);
        text.draw(ctx, DrawParam::default().dest([x + 6.0, y + 30.0]).color(text_color)).unwrap();
    }

    fn draw_city_text(&self, ctx: &mut Context, x: f32, y: f32, text_color: Color) {
        let mut current_x = x;

        // Draw capital icon if applicable
        if self.city.is_capital() {
            let capital_icon = if self.city.civ.is_city_state() {
                let mut icon = ImageGetter::get_nation_icon("CityState");
                icon.set_color(text_color);
                icon
            } else {
                ImageGetter::get_image("OtherIcons/Capital")
            };

            capital_icon.set_size(20.0, 20.0);
            capital_icon.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
            current_x += 25.0;
        }

        // Draw city name
        let city_name = self.city.name.clone();
        let text = Text::new(city_name);
        text.draw(ctx, DrawParam::default().dest([current_x, y]).color(text_color)).unwrap();
        current_x += 80.0;

        // Draw religion icon if not in popup
        if !self.for_popup {
            if let Some(city_religion) = self.city.religion.get_majority_religion() {
                let mut religion_image = ImageGetter::get_religion_icon(city_religion.get_icon_name());
                religion_image.set_color(text_color);
                religion_image.set_size(20.0, 20.0);
                religion_image.draw(ctx, DrawParam::default().dest([current_x, y])).unwrap();
            }
        }
    }

    fn draw_city_construction(&self, ctx: &mut Context, x: f32, y: f32, text_color: Color) {
        let city_constructions = &self.city.city_constructions;
        let city_current_construction = city_constructions.get_current_construction();

        let mut next_turn_percentage = 0.0;
        let mut percentage = 0.0;
        let mut turns = "-".to_string();
        let mut icon = None;

        if !city_constructions.current_construction_from_queue.is_empty() {
            if !city_current_construction.is_perpetual() {
                let turns_to_construction = city_constructions.turns_to_construction(city_current_construction.name.clone());
                if turns_to_construction < 100 {
                    turns = format!("{}", turns_to_construction);
                }

                percentage = city_constructions.get_work_done(city_current_construction.name.clone()) as f32 /
                           city_current_construction.get_production_cost(&self.city.civ, &self.city) as f32;

                next_turn_percentage = (city_constructions.get_work_done(city_current_construction.name.clone()) +
                                      self.city.city_stats.current_city_stats.production) as f32 /
                                     city_current_construction.get_production_cost(&self.city.civ, &self.city) as f32;

                next_turn_percentage = next_turn_percentage.max(0.0).min(1.0);
            } else {
                turns = Fonts::INFINITY.to_string();
            }

            icon = Some(ImageGetter::get_construction_portrait(city_current_construction.name.clone(), 24.0));
        }

        // Draw production bar
        let bar_rect = Rect::new(x, y, 4.0, 30.0 * percentage);
        let bar_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), bar_rect, COLOR_CONSTRUCTION).unwrap();
        bar_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw next turn progress
        let next_bar_rect = Rect::new(x, y + 30.0 * percentage, 4.0, 30.0 * (next_turn_percentage - percentage));
        let next_bar_mesh = Mesh::new_rectangle(ctx, DrawParam::default(), next_bar_rect,
                                               Color::new(COLOR_CONSTRUCTION.r * 0.6,
                                                        COLOR_CONSTRUCTION.g * 0.6,
                                                        COLOR_CONSTRUCTION.b * 0.6, 1.0)).unwrap();
        next_bar_mesh.draw(ctx, DrawParam::default()).unwrap();

        // Draw turns label
        let text = Text::new(turns);
        text.draw(ctx, DrawParam::default().dest([x + 6.0, y + 30.0]).color(text_color)).unwrap();

        // Draw construction icon
        if let Some(icon) = icon {
            icon.draw(ctx, DrawParam::default().dest([x + 10.0, y])).unwrap();
        }
    }

    fn draw_civ_icon(&self, ctx: &mut Context, x: f32, y: f32) {
        let icon = if self.city.civ.is_major_civ() {
            ImageGetter::get_nation_icon(self.city.civ.nation.name.clone())
        } else {
            ImageGetter::get_image(format!("CityStateIcons/{}", self.city.civ.city_state_type.name()))
        };

        icon.set_color(self.city.civ.nation.get_inner_color());
        icon.set_size(20.0, 20.0);
        icon.draw(ctx, DrawParam::default().dest([x, y])).unwrap();
    }
}

// Main CityButton struct
pub struct CityButton {
    city: City,
    tile_group: Arc<TileGroup>,
    city_table: Option<CityTable>,
    hidden_unit_markers: Vec<Image>,
    is_button_moved: bool,
    is_viewable: bool,
    viewing_player: Civilization,
}

impl CityButton {
    pub fn new(city: City, tile_group: Arc<TileGroup>) -> Self {
        Self {
            city,
            tile_group,
            city_table: None,
            hidden_unit_markers: Vec::new(),
            is_button_moved: false,
            is_viewable: true,
            viewing_player: GUI::get_viewing_player(),
        }
    }

    pub fn update(&mut self, is_city_viewable: bool) {
        self.is_viewable = is_city_viewable;

        // Clear existing components
        self.hidden_unit_markers.clear();

        // Create new city table
        self.city_table = Some(CityTable::new(self.city.clone(), false));

        // Update hidden unit markers
        self.update_hidden_unit_markers(is_city_viewable);
    }

    fn update_hidden_unit_markers(&mut self, is_city_viewable: bool) {
        if !is_city_viewable {
            return;
        }

        // Detect civilian in the city center
        if !self.is_button_moved && self.tile_group.tile.civilian_unit.is_some() {
            self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Center);
        }

        // Check neighboring tiles
        let tiles_around_city = self.tile_group.tile.neighbors.clone();
        for tile in tiles_around_city {
            let direction = self.tile_group.tile.position.sub(tile.position);

            if self.is_button_moved {
                // Detect civilian left-below the city
                if tile.civilian_unit.is_some() && direction.epsilon_equals(0.0, 1.0) {
                    self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Left);
                }
                // Detect military under the city
                else if tile.military_unit.is_some() && !tile.has_enemy_invisible_unit(&self.viewing_player) &&
                        direction.epsilon_equals(1.0, 1.0) {
                    self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Center);
                }
                // Detect civilian right-below the city
                else if tile.civilian_unit.is_some() && direction.epsilon_equals(1.0, 0.0) {
                    self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Right);
                }
            } else if tile.military_unit.is_some() && !tile.has_enemy_invisible_unit(&self.viewing_player) {
                // Detect military left from the city
                if direction.epsilon_equals(0.0, 1.0) {
                    self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Left);
                }
                // Detect military right from the city
                else if direction.epsilon_equals(1.0, 0.0) {
                    self.insert_hidden_unit_marker(HiddenUnitMarkerPosition::Right);
                }
            }
        }
    }

    fn insert_hidden_unit_marker(&mut self, pos: HiddenUnitMarkerPosition) {
        if let Some(city_table) = &self.city_table {
            // Calculate position based on marker position
            let position_x = city_table.width / 2.0 + (pos as i32 - 1) as f32 * 60.0;

            // Create indicator
            let mut indicator = ImageGetter::get_triangle();
            indicator.set_color(self.city.civ.nation.get_inner_color());
            indicator.set_size(12.0, 8.0);

            if !self.is_button_moved {
                indicator.set_rotation(180.0);
                indicator.set_position(position_x - 6.0, -8.0);
            } else {
                indicator.set_position(position_x - 6.0, -8.0);
            }

            self.hidden_unit_markers.push(indicator);
        }
    }

    fn belongs_to_viewing_civ(&self) -> bool {
        self.city.civ == self.viewing_player
    }

    pub fn set_button_actions(&mut self) {
        let unit_table = GUI::get_unit_table();

        // Handle click events
        self.on_click(UncivSound::Click, move |_| {
            if self.is_button_moved {
                self.enter_city_or_info_popup();
            } else {
                self.move_button_down();
                if (unit_table.selected_unit.is_none() || !unit_table.selected_unit.as_ref().unwrap().has_movement()) &&
                   self.belongs_to_viewing_civ() {
                    unit_table.city_selected(self.city.clone());
                }
            }
        });

        // Handle right click events
        self.on_right_click(UncivSound::Click, move |_| {
            self.enter_city_or_info_popup();
        });

        // Reset button position when deselected
        if unit_table.selected_city.as_ref() != Some(&self.city) &&
           (unit_table.selected_unit.is_none() ||
            unit_table.selected_unit.as_ref().unwrap().current_tile != self.city.get_center_tile()) &&
           unit_table.selected_spy.is_none() {
            self.move_button_up();
        }
    }

    pub fn move_button_down(&mut self) {
        if self.is_button_moved {
            return;
        }

        // Animate button movement
        // In a real implementation, this would use ggez's animation system
        self.is_button_moved = true;
        self.update_hidden_unit_markers(self.is_viewable);
    }

    pub fn move_button_up(&mut self) {
        if !self.is_button_moved {
            return;
        }

        // Animate button movement
        // In a real implementation, this would use ggez's animation system
        self.is_button_moved = false;
        self.update_hidden_unit_markers(self.is_viewable);
    }

    fn enter_city_or_info_popup(&self) {
        let unit_table = GUI::get_unit_table();

        // Determine if we should show city screen or info popup
        if DebugUtils::VISIBLE_MAP || self.viewing_player.is_spectator() ||
           (self.belongs_to_viewing_civ() &&
            (unit_table.selected_unit.is_none() ||
             !unit_table.selected_unit.as_ref().unwrap().current_tile.air_units.contains(unit_table.selected_unit.as_ref().unwrap()))) {
            GUI::push_screen(Box::new(CityScreen::new(self.city.clone())));
        } else if self.viewing_player.knows(&self.city.civ) {
            self.foreign_city_info_popup();
        }
    }

    fn foreign_city_info_popup(&self) {
        let open_diplomacy = || {
            GUI::push_screen(Box::new(DiplomacyScreen::new(self.viewing_player.clone(), self.city.civ.clone())));
        };

        let espionage_visible = self.city.civ.game_info.is_espionage_enabled() &&
                               self.viewing_player.espionage_manager.get_spy_assigned_to_city(&self.city).map_or(false, |spy| spy.is_set_up());

        // Skip popup if no religion and no espionage
        if !self.city.civ.game_info.is_religion_enabled() && !espionage_visible {
            open_diplomacy();
            return;
        }

        // Create popup
        let mut popup = Popup::new(GUI::get_world_screen());
        popup.set_name("ForeignCityInfoPopup");

        // Add city table
        popup.add(CityTable::new(self.city.clone(), true));

        // Add religion info if enabled
        if self.city.civ.game_info.is_religion_enabled() {
            // Add religion info table (implementation would depend on CityReligionInfoTable)
        }

        // Add diplomacy button
        popup.add_button("Diplomacy", open_diplomacy);

        // Add view button if espionage is visible
        if espionage_visible {
            popup.add_button("View", || {
                GUI::push_screen(Box::new(CityScreen::new(self.city.clone())));
            });
        }

        // Add close button
        popup.add_close_button(|| {
            GUI::get_world_screen().next_turn_button.update();
        });

        popup.open();
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) {
        // Draw city button components
        if self.is_viewable && !self.tile_group.tile.air_units.is_empty() {
            let air_unit_table = AirUnitTable::new(self.city.clone(), self.tile_group.tile.air_units.len() as i32, 14.0);
            air_unit_table.draw(ctx, x, y);
        }

        // Draw defense table
        let defence_table = DefenceTable::new(self.city.clone());
        defence_table.draw(ctx, x, y + 20.0);

        // Draw city table
        if let Some(city_table) = &self.city_table {
            city_table.draw(ctx, x, y + 50.0);
        }

        // Draw influence table for city states
        let selected_player = GUI::get_selected_player();
        if self.city.civ.is_city_state && self.city.civ.knows(&selected_player) {
            if let Some(diplomacy_manager) = self.city.civ.get_diplomacy_manager(&selected_player) {
                let influence_table = InfluenceTable::new(
                    diplomacy_manager.get_influence(),
                    diplomacy_manager.relationship_level(),
                    100.0,
                    5.0
                );
                influence_table.draw(ctx, x, y + 100.0);
            }
        }

        // Draw status table
        let status_table = StatusTable::new(self.city.clone(), 18.0);
        status_table.draw(ctx, x, y + 105.0);

        // Draw health bar if city is damaged
        if self.is_viewable && self.city.health < self.city.get_max_health() as f32 {
            // Draw health bar (implementation would depend on health bar drawing)
        }

        // Draw hidden unit markers
        for marker in &self.hidden_unit_markers {
            marker.draw(ctx, DrawParam::default()).unwrap();
        }
    }

    // Helper methods for event handling
    fn on_click<F>(&mut self, sound: UncivSound, action: F) where F: FnMut(&mut Context) + 'static {
        // Implementation would depend on event handling system
    }

    fn on_right_click<F>(&mut self, sound: UncivSound, action: F) where F: FnMut(&mut Context) + 'static {
        // Implementation would depend on event handling system
    }
}