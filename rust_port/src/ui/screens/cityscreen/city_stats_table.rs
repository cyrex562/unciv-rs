use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui, Align};
use std::collections::HashMap;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::gui::GUI;
use crate::logic::city::{City, CityFocus, CityFlags, CityResources};
use crate::logic::city::population::Population;
use crate::logic::city::religion::CityReligion;
use crate::logic::city::constructions::CityConstructions;
use crate::logic::city::stats::CityStats;
use crate::logic::city::expansion::CityExpansion;
use crate::logic::city::great_people::GreatPersonPointsBreakdown;
use crate::models::building::Building;
use crate::models::counter::Counter;
use crate::models::ruleset::unique::UniqueType;
use crate::models::ruleset::tile::TileResource;
use crate::models::stats::{Stat, Stats};
use crate::ui::components::extensions::{darken, disable, is_enabled, to_label, to_text_button};
use crate::ui::components::input::{key_shortcuts, on_activation, onClick, KeyboardBinding};
use crate::ui::components::widgets::{ExpanderTab, ScrollPane};
use crate::ui::fonts::Fonts;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::ui::screens::cityscreen::citizen_management_table::CitizenManagementTable;
use crate::ui::screens::cityscreen::specialist_allocation_table::SpecialistAllocationTable;
use crate::ui::screens::cityscreen::city_religion_info_table::CityReligionInfoTable;
use crate::ui::screens::cityscreen::detailed_stats_popup::DetailedStatsPopup;
use crate::ui::screens::cityscreen::great_person_points_breakdown_popup::GreatPersonPointsBreakdownPopup;
use crate::utils::translations::tr;

/// Table that displays city statistics and information
pub struct CityStatsTable {
    /// Reference to the parent city screen
    city_screen: Box<dyn CityScreen>,

    /// The city this table belongs to
    city: City,

    /// The inner table containing the actual content
    inner_table: egui::Frame,

    /// The upper table for fixed position content
    upper_table: egui::Frame,

    /// The lower table that will be in the ScrollPane
    lower_table: egui::Frame,

    /// The scroll pane for the lower table
    lower_pane: ScrollPane,

    /// The header icon for expanding/collapsing
    header_icon: egui::Image,

    /// The click area for the header icon
    header_icon_click_area: egui::Frame,

    /// Whether the table is currently expanded
    is_open: bool,

    /// The detailed stats button
    detailed_stats_button: egui::Button,
}

impl CityStatsTable {
    /// Create a new CityStatsTable
    pub fn new(city_screen: Box<dyn CityScreen>) -> Self {
        let city = city_screen.get_city().clone();

        let mut inner_table = egui::Frame::none();
        inner_table.set_style(egui::Style {
            background: Some(egui::Color32::from_rgba_premultiplied(194, 180, 131, 255)),
            ..Default::default()
        });

        let mut upper_table = egui::Frame::none();
        upper_table.set_style(egui::Style {
            padding: egui::style::Margin::same(2.0),
            ..Default::default()
        });

        let mut lower_table = egui::Frame::none();
        lower_table.set_style(egui::Style {
            padding: egui::style::Margin::same(2.0),
            ..Default::default()
        });

        let lower_pane = ScrollPane::new(lower_table);

        let header_icon = ImageGetter::get_image("OtherIcons/BackArrow");
        header_icon.set_size(18.0, 18.0);
        header_icon.set_origin(Align::Center);
        header_icon.set_rotation(90.0);

        let mut header_icon_click_area = egui::Frame::none();
        header_icon_click_area.set_style(egui::Style {
            padding: egui::style::Margin::new(6.0, 12.0, 6.0, 2.0),
            ..Default::default()
        });

        let mut detailed_stats_button = to_text_button("Stats");
        detailed_stats_button.set_padding(10.0);
        detailed_stats_button.on_activation(KeyboardBinding::ShowStats, move || {
            DetailedStatsPopup::new(city_screen.clone()).open();
        });

        let is_open = !city_screen.is_cramped_portrait();

        Self {
            city_screen,
            city,
            inner_table,
            upper_table,
            lower_table,
            lower_pane,
            header_icon,
            header_icon_click_area,
            is_open,
            detailed_stats_button,
        }
    }

    /// Update the table with the current city information
    pub fn update(&mut self, height: f32) {
        self.upper_table.clear();
        self.lower_table.clear();

        // Create mini stats table
        let mut mini_stats_table = egui::Frame::none();
        let selected = egui::Color32::from_rgba_premultiplied(255, 255, 0, 128); // Selection color

        for (stat, amount) in &self.city.city_stats.current_city_stats {
            if *stat == Stat::Faith && !self.city.civ.game_info.is_religion_enabled() {
                continue;
            }

            let mut icon = egui::Frame::none();
            let focus = CityFocus::safe_value_of(*stat);
            let toggled_focus = if focus == self.city.get_city_focus() {
                icon.add(ImageGetter::get_stat_icon(&stat.name).surround_with_circle(27.0, false, selected));
                CityFocus::NoFocus
            } else {
                icon.add(ImageGetter::get_stat_icon(&stat.name).surround_with_circle(27.0, false, Color32::TRANSPARENT));
                focus
            };

            if self.city_screen.can_city_be_changed() {
                icon.on_activation(toggled_focus.binding, move || {
                    self.city.set_city_focus(toggled_focus);
                    self.city.reassign_population();
                    self.city_screen.update();
                });
            }

            mini_stats_table.add(icon).size(27.0).pad_right(3.0);

            let value_to_display = if *stat == Stat::Happiness {
                self.city.city_stats.happiness_list.values.iter().sum::<f32>()
            } else {
                *amount
            };

            mini_stats_table.add(to_label(&format!("{}", value_to_display.round() as i32))).pad_right(5.0);

            if self.city_screen.is_cramped_portrait() && !self.is_open && *stat == Stat::Gold {
                mini_stats_table.row();
            }
        }

        self.upper_table.add(mini_stats_table).expand_x();

        self.lower_table.add(self.detailed_stats_button.clone()).row();
        self.add_text();

        // Begin lower table
        self.add_citizen_management();
        self.add_great_person_point_info(&self.city);

        if !self.city.population.get_max_specialists().is_empty() {
            self.add_specialist_info();
        }

        if !self.city.religion.get_number_of_followers().is_empty() && self.city.civ.game_info.is_religion_enabled() {
            self.add_religion_info();
        }

        self.add_buildings_info();

        self.header_icon.set_rotation(if self.is_open { 90.0 } else { 0.0 });

        self.inner_table.clear();
        self.inner_table.add(self.upper_table.clone()).expand_x();
        self.inner_table.add(self.header_icon_click_area.clone()).row();

        if self.is_open {
            self.inner_table.add(self.lower_pane.clone()).colspan(2);
        }

        self.upper_table.pack();
        self.lower_table.pack();
        self.lower_pane.layout();
        self.lower_pane.update_visual_scroll();

        if self.is_open {
            let lower_cell = self.inner_table.get_cell_mut(2, 0);
            lower_cell.max_height(height - self.upper_table.height() - 8.0); // 2 on each side of each cell in inner_table
        }

        self.inner_table.pack(); // Update inner_table
        self.pack(); // Update self last
    }

    /// Handle content resize
    fn on_content_resize(&mut self) {
        self.pack();
        self.set_position(
            self.city_screen.stage().width() - CityScreen::pos_from_edge(),
            self.city_screen.stage().height() - CityScreen::pos_from_edge(),
            Align::TopRight
        );
    }

    /// Add text information to the lower table
    fn add_text(&mut self) {
        let unassigned_pop_string = format!(
            "{}: {}/{}",
            tr("Unassigned population"),
            tr(&self.city.population.get_free_population().to_string()),
            tr(&self.city.population.population.to_string())
        );

        let mut unassigned_pop_label = to_label(&unassigned_pop_string);
        if self.city_screen.can_change_state() {
            unassigned_pop_label.on_click(move || {
                self.city.reassign_population();
                self.city_screen.update();
            });
        }

        let mut turns_to_expansion_string = if self.city.city_stats.current_city_stats.culture > 0.0 &&
                                              self.city.expansion.get_choosable_tiles().any() {
            let remaining_culture = self.city.expansion.get_culture_to_next_tile() - self.city.expansion.culture_stored;
            let mut turns_to_expansion = (remaining_culture / self.city.city_stats.current_city_stats.culture).ceil() as i32;
            if turns_to_expansion < 1 {
                turns_to_expansion = 1;
            }
            tr(&format!("{} turns to expansion", turns_to_expansion))
        } else {
            tr("Stopped expansion")
        };

        if self.city.expansion.get_choosable_tiles().any() {
            turns_to_expansion_string += &format!(
                " ({} culture/{})",
                self.city.expansion.culture_stored,
                self.city.expansion.get_culture_to_next_tile()
            );
        }

        let mut turns_to_pop_string = match () {
            _ if self.city.is_starving() => {
                format!("{} turns to lose population", self.city.population.get_num_turns_to_starvation())
            },
            _ if self.city.get_ruleset().units.get(&self.city.city_constructions.current_construction_from_queue)
                .map_or(false, |unit| unit.has_unique(UniqueType::ConvertFoodToProductionWhenConstructed)) => {
                "Food converts to production".to_string()
            },
            _ if self.city.is_growing() => {
                format!("{} turns to new population", self.city.population.get_num_turns_to_new_population())
            },
            _ => "Stopped population growth".to_string()
        };
        turns_to_pop_string = tr(&turns_to_pop_string);
        turns_to_pop_string += &format!(
            " ({} food/{})",
            self.city.population.food_stored,
            self.city.population.get_food_to_next_population()
        );

        self.lower_table.add(unassigned_pop_label).row();
        self.lower_table.add(to_label(&turns_to_expansion_string)).row();
        self.lower_table.add(to_label(&turns_to_pop_string)).row();

        let mut table_with_icons = egui::Frame::none();
        table_with_icons.set_style(egui::Style {
            padding: egui::style::Margin::same(2.0),
            ..Default::default()
        });

        if self.city.is_in_resistance() {
            let mut resistance_table = egui::Frame::none();
            resistance_table.add(ImageGetter::get_image("StatIcons/Resistance")).size(20.0).pad_right(2.0);
            resistance_table.add(to_label(&format!(
                "In resistance for another {} turns",
                self.city.get_flag(CityFlags::Resistance)
            )));
            table_with_icons.add(resistance_table);
        }

        let mut resource_table = egui::Frame::none();
        let mut resource_counter = Counter::<TileResource>::new();

        for resource_supply in CityResources::get_city_resources_available_to_city(&self.city) {
            resource_counter.add(resource_supply.resource, resource_supply.amount);
        }

        for (resource, amount) in resource_counter {
            if resource.is_city_wide {
                resource_table.add(to_label(&amount.to_string()));
                resource_table.add(ImageGetter::get_resource_portrait(&resource.name, 20.0)).pad_right(5.0);
            }
        }

        if !resource_table.is_empty() {
            table_with_icons.add(resource_table);
        }

        let (wltk_icon, wltk_label) = if self.city.is_we_love_the_king_day_active() {
            (
                Some(ImageGetter::get_stat_icon("Food")),
                Some(to_label(&format!(
                    "We Love The King Day for another {} turns",
                    self.city.get_flag(CityFlags::WeLoveTheKing)
                )))
            )
        } else if !self.city.demanded_resource.is_empty() {
            (
                Some(ImageGetter::get_resource_portrait(&self.city.demanded_resource, 20.0)),
                Some(to_label(&format!("Demanding {}", self.city.demanded_resource)))
            )
        } else {
            (None, None)
        };

        if let (Some(icon), Some(label)) = (wltk_icon, wltk_label) {
            let mut wltk_table = egui::Frame::none();
            wltk_table.add(icon).size(20.0).pad_right(5.0);
            wltk_table.add(label).row();

            label.on_click(move || {
                self.city_screen.open_civilopedia("Tutorial/We Love The King Day");
            });

            table_with_icons.add(wltk_table);
        }

        self.lower_table.add(table_with_icons).row();
    }

    /// Add citizen management section
    fn add_citizen_management(&mut self) {
        let expander_tab = CitizenManagementTable::new(self.city_screen.clone()).as_expander(|| {
            self.on_content_resize();
        });
        self.lower_table.add(expander_tab).grow_x().row();
    }

    /// Add specialist information section
    fn add_specialist_info(&mut self) {
        let expander_tab = SpecialistAllocationTable::new(self.city_screen.clone()).as_expander(|| {
            self.on_content_resize();
        });
        self.lower_table.add(expander_tab).grow_x().row();
    }

    /// Add religion information section
    fn add_religion_info(&mut self) {
        let expander_tab = CityReligionInfoTable::new(self.city.religion.clone()).as_expander(|| {
            self.on_content_resize();
        });
        self.lower_table.add(expander_tab).grow_x().row();
    }

    /// Add buildings information section
    fn add_buildings_info(&mut self) {
        let mut wonders = Vec::new();
        let mut specialist_buildings = Vec::new();
        let mut other_buildings = Vec::new();

        for building in self.city.city_constructions.get_built_buildings() {
            if building.is_any_wonder() {
                wonders.push(building);
            } else if !building.new_specialists().is_empty() {
                specialist_buildings.push(building);
            } else {
                other_buildings.push(building);
            }
        }

        // Buildings sorted alphabetically
        wonders.sort_by(|a, b| a.name.cmp(&b.name));
        specialist_buildings.sort_by(|a, b| a.name.cmp(&b.name));
        other_buildings.sort_by(|a, b| a.name.cmp(&b.name));

        let mut total_table = egui::Frame::none();
        self.lower_table.add_category("Buildings", total_table.clone(), KeyboardBinding::BuildingsDetail, false);

        if !specialist_buildings.is_empty() {
            let mut specialist_buildings_table = egui::Frame::none();
            total_table.add().row();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));
            total_table.add(to_label("Specialist Buildings").set_alignment(Align::Center)).grow_x();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));

            for building in &specialist_buildings {
                self.add_building_button(building, &mut specialist_buildings_table);
            }

            total_table.add(specialist_buildings_table).grow_x().right().row();
        }

        if !wonders.is_empty() {
            let mut wonders_table = egui::Frame::none();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));
            total_table.add(to_label("Wonders").set_alignment(Align::Center)).grow_x();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));

            for building in &wonders {
                self.add_building_button(building, &mut wonders_table);
            }

            total_table.add(wonders_table).grow_x().right().row();
        }

        if !other_buildings.is_empty() {
            let mut regular_buildings_table = egui::Frame::none();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));
            total_table.add(to_label("Other").set_alignment(Align::Center)).grow_x();
            total_table.add_separator(Color32::from_rgba_premultiplied(200, 200, 200, 255));

            for building in &other_buildings {
                self.add_building_button(building, &mut regular_buildings_table);
            }

            total_table.add(regular_buildings_table).grow_x().right().row();
        }
    }

    /// Add a building button to the destination table
    fn add_building_button(&self, building: &Building, destination_table: &mut egui::Frame) {
        let mut button = egui::Frame::none();

        let mut info = egui::Frame::none();
        let mut stats_and_specialists = egui::Frame::none();

        let icon = ImageGetter::get_construction_portrait(&building.name, 50.0);
        let is_free = self.city_screen.has_free_building(building);
        let display_name = if is_free {
            format!("{} (Free)", building.name)
        } else {
            building.name.clone()
        };

        info.add(to_label(&display_name).set_font_size(DEFAULT_FONT_SIZE)).pad_bottom(5.0).right().row();

        let stats = building.get_stats(&self.city).iter()
            .map(|(key, value)| format!("{}{}", value.round() as i32, key.character))
            .collect::<Vec<String>>()
            .join(" ");

        stats_and_specialists.add(to_label(&stats).set_font_size(DEFAULT_FONT_SIZE)).right();

        let mut assigned_spec = self.city.population.get_new_specialists().clone();

        let mut specialist_icons = egui::Frame::none();
        for (specialist_name, amount) in building.new_specialists() {
            if let Some(specialist) = self.city.get_ruleset().specialists.get(specialist_name) {
                for _ in 0..amount {
                    if assigned_spec.get(specialist_name).unwrap_or(&0) > &0 {
                        specialist_icons.add(ImageGetter::get_specialist_icon(specialist.color_object))
                            .size(20.0);
                        assigned_spec.add(specialist_name, -1);
                    } else {
                        specialist_icons.add(ImageGetter::get_specialist_icon(Color32::GRAY)).size(20.0);
                    }
                }
            }
        }

        stats_and_specialists.add(specialist_icons).right();

        info.add(stats_and_specialists).right();

        button.add(info).right().top().pad_right(10.0).pad_top(5.0);
        button.add(icon).right();

        button.on_click(move || {
            self.city_screen.select_construction(building);
            self.city_screen.update();
        });

        destination_table.add(button).pad(1.0).pad_bottom(2.0).pad_top(2.0).expand_x().right().row();
    }

    /// Add a category to the table
    fn add_category(
        &mut self,
        category: &str,
        show_hide_table: egui::Frame,
        toggle_key: KeyboardBinding,
        starts_opened: bool
    ) -> ExpanderTab {
        let expander_tab = ExpanderTab::new(
            category,
            DEFAULT_FONT_SIZE,
            &format!("CityInfo.{}", category),
            starts_opened,
            toggle_key,
            || {
                self.on_content_resize();
            }
        );

        expander_tab.add(show_hide_table).fill_x().right();
        self.lower_table.add(expander_tab.clone()).grow_x().row();

        expander_tab
    }

    /// Add great person point information
    fn add_great_person_point_info(&mut self, city: &City) {
        let mut great_people_table = egui::Frame::none();

        let gpp_breakdown = GreatPersonPointsBreakdown::new(city);
        if gpp_breakdown.all_names.is_empty() {
            return;
        }

        let great_person_points = gpp_breakdown.sum();

        // Iterating over allNames instead of greatPersonPoints will include those where the aggregation had points but ended up zero
        for great_person_name in &gpp_breakdown.all_names {
            let gpp_per_turn = great_person_points.get(great_person_name).unwrap_or(&0);

            let mut info = egui::Frame::none();

            if let Some(great_person) = city.get_ruleset().units.get(great_person_name) {
                info.add(ImageGetter::get_unit_icon(great_person, Color32::GOLD).to_group(20.0))
                    .left().pad_bottom(4.0).pad_right(5.0);
                info.add(to_label(&format!("{} (+{})", great_person_name, gpp_per_turn)).hide_icons())
                    .left().pad_bottom(4.0).expand_x().row();

                let gpp_current = city.civ.great_people.great_person_points_counter.get(great_person_name).unwrap_or(&0);
                let gpp_needed = city.civ.great_people.get_points_required_for_great_person(great_person_name);

                let percent = *gpp_current as f32 / gpp_needed as f32;

                let mut progress_bar = ImageGetter::progress_bar(300.0, 25.0, false);
                progress_bar.set_background(egui::Color32::from_rgba_premultiplied(40, 40, 40, 200));
                progress_bar.set_progress(Color32::ORANGE, percent);

                let bar = ImageGetter::get_white_dot();
                bar.set_color(Color32::GRAY);
                bar.set_size(progress_bar.width() + 5.0, progress_bar.height() + 5.0);
                bar.center(&progress_bar);
                progress_bar.add_actor(bar);
                bar.to_back();

                progress_bar.set_label(Color32::WHITE, &format!("{}/{}", gpp_current, gpp_needed), 14);

                info.add(progress_bar).colspan(2).left().expand_x().row();

                info.on_click(move || {
                    GreatPersonPointsBreakdownPopup::new(self.city_screen.clone(), gpp_breakdown.clone(), Some(great_person_name.clone()));
                });

                great_people_table.add(info).grow_x().top().pad_bottom(10.0);

                let icon = ImageGetter::get_construction_portrait(great_person_name, 50.0);
                icon.on_click(move || {
                    GreatPersonPointsBreakdownPopup::new(self.city_screen.clone(), gpp_breakdown.clone(), None);
                });

                great_people_table.add(icon).row();
            }
        }

        self.lower_table.add_category("Great People", great_people_table, KeyboardBinding::GreatPeopleDetail);
    }

    /// Pack the table to calculate its size
    pub fn pack(&mut self) {
        self.inner_table.pack();
    }

    /// Set the position of the table
    pub fn set_position(&mut self, x: f32, y: f32, align: Align) {
        self.inner_table.set_position(x, y, align);
    }

    /// Render the table
    pub fn render(&mut self, ui: &mut Ui) {
        self.inner_table.render(ui);
    }
}