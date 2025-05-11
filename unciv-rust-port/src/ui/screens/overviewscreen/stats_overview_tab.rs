// Source: orig_src/core/src/com/unciv/ui/screens/overviewscreen/StatsOverviewTab.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align, ScrollArea, Button, Image, Response, Slider};
use crate::models::civilization::Civilization;
use crate::models::stats::{Stat, StatMap};
use crate::models::ruleset::unique::{Unique, UniqueType};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::{TabbedPager, ExpanderTab};
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;

pub struct StatsOverviewTab {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<StatsOverviewTabPersistableData>>,
    happiness_table: Rc<RefCell<Ui>>,
    unhappiness_table: Rc<RefCell<UnhappinessTable>>,
    gold_and_slider_table: Rc<RefCell<Ui>>,
    gold_table: Rc<RefCell<Ui>>,
    science_table: Rc<RefCell<Ui>>,
    culture_table: Rc<RefCell<Ui>>,
    faith_table: Rc<RefCell<Ui>>,
    great_people_table: Rc<RefCell<Ui>>,
    score_table: Rc<RefCell<Ui>>,
    is_religion_enabled: bool,
}

impl StatsOverviewTab {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<StatsOverviewTabPersistableData>,
    ) -> Self {
        let mut tab = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            happiness_table: Rc::new(RefCell::new(Ui::default())),
            unhappiness_table: Rc::new(RefCell::new(UnhappinessTable::new(
                Rc::clone(&viewing_player),
                Rc::clone(&overview_screen),
            ))),
            gold_and_slider_table: Rc::new(RefCell::new(Ui::default())),
            gold_table: Rc::new(RefCell::new(Ui::default())),
            science_table: Rc::new(RefCell::new(Ui::default())),
            culture_table: Rc::new(RefCell::new(Ui::default())),
            faith_table: Rc::new(RefCell::new(Ui::default())),
            great_people_table: Rc::new(RefCell::new(Ui::default())),
            score_table: Rc::new(RefCell::new(Ui::default())),
            is_religion_enabled: false, // Will be set in init
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        // Set religion enabled flag
        self.is_religion_enabled = self.overview_screen.borrow().game_info.borrow().is_religion_enabled();

        // Set up tables with padding
        let table_padding = 30.0;
        self.happiness_table.borrow_mut().defaults().pad(table_padding).top();
        self.gold_table.borrow_mut().defaults().pad(5.0);
        self.science_table.borrow_mut().defaults().pad(5.0);
        self.culture_table.borrow_mut().defaults().pad(5.0);
        self.faith_table.borrow_mut().defaults().pad(5.0);
        self.great_people_table.borrow_mut().defaults().pad(5.0);
        self.score_table.borrow_mut().defaults().pad(5.0);

        // Update unhappiness table
        self.unhappiness_table.borrow_mut().update();

        // Add gold table to gold and slider table
        self.gold_and_slider_table.borrow_mut().add(self.gold_table.clone()).row();

        // Add gold slider if mod option is enabled
        if self.overview_screen.borrow().game_info.borrow().ruleset.borrow().mod_options.has_unique(UniqueType::ConvertGoldToScience) {
            self.add_gold_slider();
        }

        // Update all tables
        self.update();

        // Calculate optimum columns based on screen width
        let all_stat_tables = vec![
            self.happiness_table.clone(),
            self.unhappiness_table.clone(),
            self.gold_and_slider_table.clone(),
            self.science_table.clone(),
            self.culture_table.clone(),
            self.faith_table.clone(),
            self.great_people_table.clone(),
            self.score_table.clone(),
        ];

        let mut optimum_columns = 1;
        for num_columns in (1..=all_stat_tables.len()).rev() {
            let total_width = all_stat_tables.iter()
                .enumerate()
                .filter(|(i, _)| i % num_columns == 0)
                .map(|(_, table)| {
                    table.borrow().pref_width + table_padding * 2.0
                })
                .sum::<f32>();

            if total_width < self.overview_screen.borrow().stage.width {
                optimum_columns = num_columns;
                break;
            }
        }

        // Add tables to the layout
        for (i, table) in all_stat_tables.iter().enumerate() {
            if i % optimum_columns == 0 {
                self.row();
            }
            self.add(table.clone());
        }
    }

    pub fn update(&mut self) {
        let stat_map = self.viewing_player.borrow().stats.get_stat_map_for_next_turn();
        self.update_happiness_table();
        self.update_stat_table(self.gold_table.clone(), Stat::Gold, &stat_map);
        self.update_stat_table(self.science_table.clone(), Stat::Science, &stat_map);
        self.update_stat_table(self.culture_table.clone(), Stat::Culture, &stat_map);
        if self.is_religion_enabled {
            self.update_stat_table(self.faith_table.clone(), Stat::Faith, &stat_map);
        }
        self.update_great_people_table();
        self.update_score_table();
    }

    fn add_heading(&self, table: &mut Ui, label: &str) {
        table.clear();
        table.add_label(label, Constants::heading_font_size()).colspan(2).row();
        table.add_separator();
    }

    fn add_labeled_value(&self, table: &mut Ui, label: &str, value: f32) {
        let rounded_value = value.round() as i32;
        if rounded_value == 0 {
            return;
        }
        table.add_label(label, 0, true).left();
        table.add_label(&rounded_value.to_string(), 0, false).right().row();
    }

    fn add_total(&self, table: &mut Ui, value: f32) {
        table.add_label("Total", 0, false).left();
        table.add_label(&value.round().to_string(), 0, false).right();
        table.pack();
    }

    fn update_happiness_table(&mut self) {
        let mut table = self.happiness_table.borrow_mut();
        self.add_heading(&mut table, "Happiness");

        let happiness_breakdown = self.viewing_player.borrow().stats.get_happiness_breakdown();
        for (key, value) in happiness_breakdown {
            self.add_labeled_value(&mut table, &key, value);
        }

        let total = happiness_breakdown.values().sum::<f32>();
        self.add_total(&mut table, total);
    }

    fn update_stat_table(&self, table: Rc<RefCell<Ui>>, stat: Stat, stat_map: &StatMap) {
        let mut table = table.borrow_mut();
        self.add_heading(&mut table, &stat.name);

        let mut total = 0.0;
        for (source, stats) in stat_map {
            self.add_labeled_value(&mut table, source, stats.get(stat));
            total += stats.get(stat);
        }

        self.add_total(&mut table, total);
    }

    fn add_gold_slider(&mut self) {
        let mut table = self.gold_and_slider_table.borrow_mut();
        table.add_separator();

        let mut slider_table = Ui::default();
        slider_table.add_label("Convert gold to science", 0, false).row();

        let initial_value = self.viewing_player.borrow().tech.gold_percent_converted_to_science;
        let mut slider = Slider::new(initial_value, 0.0..=1.0)
            .step_by(0.1)
            .text("Convert gold to science");

        slider.on_changed(|value| {
            self.viewing_player.borrow_mut().tech.gold_percent_converted_to_science = value;
            for city in self.viewing_player.borrow().cities.iter() {
                city.borrow_mut().city_stats.update();
            }
            self.update();
        });

        if !self.overview_screen.borrow().can_change_state() {
            slider.disabled();
        }

        slider_table.add(slider).pad_top(15.0);
        table.add(slider_table).colspan(2);
    }

    fn update_great_people_table(&mut self) {
        let mut table = self.great_people_table.borrow_mut();
        table.clear();

        let mut great_people_header = Ui::default();
        let mut great_people_icon = ImageGetter::get_stat_icon("Specialist");
        great_people_icon.color = Color32::ROYAL;
        great_people_header.add(great_people_icon).pad_right(1.0).size(Constants::heading_font_size() as f32);
        great_people_header.add_label("Great person points", Constants::heading_font_size(), false);
        table.add(great_people_header).colspan(3).row();
        table.add_separator();
        table.add_empty();
        table.add_label("Current points", 0, false);
        table.add_label("Points per turn", 0, false).row();

        let great_person_points = self.viewing_player.borrow().great_people.great_person_points_counter.clone();
        let great_person_points_per_turn = self.viewing_player.borrow().great_people.get_great_person_points_for_next_turn();

        for (great_person, points) in great_person_points {
            let points_to_great_person = self.viewing_player.borrow().great_people.get_points_required_for_great_person(&great_person);
            table.add_label(&great_person, 0, false).left();
            table.add_label(&format!("{}/{}", points, points_to_great_person), 0, false);
            table.add_label(&great_person_points_per_turn.get(&great_person).unwrap_or(&0).to_string(), 0, false).right().row();
        }

        let great_general_points = self.viewing_player.borrow().great_people.great_general_points_counter.clone();
        let points_for_next_great_general = self.viewing_player.borrow().great_people.points_for_next_great_general_counter.clone();

        for (unit, points) in great_general_points {
            let points_to_great_general = points_for_next_great_general.get(&unit).unwrap_or(&0);
            table.add_label(&unit, 0, false).left();
            table.add_label(&format!("{}/{}", points, points_to_great_general), 0, false);
        }

        table.pack();
    }

    fn update_score_table(&mut self) {
        let mut table = self.score_table.borrow_mut();
        table.clear();

        let mut score_header = Ui::default();
        let mut score_icon = ImageGetter::get_image("OtherIcons/Score");
        score_icon.color = Color32::FIREBRICK;
        score_header.add(score_icon).pad_right(1.0).size(Constants::heading_font_size() as f32);
        score_header.add_label("Score", Constants::heading_font_size(), false);
        table.add(score_header).colspan(2).row();
        table.add_separator();

        let score_breakdown = self.viewing_player.borrow().calculate_score_breakdown();
        for (label, value) in score_breakdown {
            self.add_labeled_value(&mut table, &label, value as f32);
        }

        let total = score_breakdown.values().sum::<i32>() as f32;
        self.add_total(&mut table, total);
    }
}

impl EmpireOverviewTab for StatsOverviewTab {
    fn viewing_player(&self) -> &Rc<RefCell<Civilization>> {
        &self.viewing_player
    }

    fn overview_screen(&self) -> &Rc<RefCell<dyn BaseScreen>> {
        &self.overview_screen
    }

    fn persist_data(&self) -> &Rc<RefCell<dyn EmpireOverviewTabPersistableData>> {
        &self.persist_data
    }

    fn activated(&mut self, index: i32, caption: &str, pager: &mut TabbedPager) {
        self.overview_screen.borrow().game.borrow_mut().settings.add_completed_tutorial_task("See your stats breakdown");
        super::activated(index, caption, pager);
    }
}

#[derive(Default)]
pub struct StatsOverviewTabPersistableData {
    // Add any persistent data fields here
}

impl EmpireOverviewTabPersistableData for StatsOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        true // Implement based on actual fields
    }
}

pub struct UnhappinessTable {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    show: bool,
    uniques: Vec<Unique>,
}

impl UnhappinessTable {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
    ) -> Self {
        let mut table = Self {
            viewing_player,
            overview_screen,
            show: false,
            uniques: Vec::new(),
        };

        table.init();
        table
    }

    fn init(&mut self) {
        let mut uniques = Vec::new();

        // Get conditional uniques related to happiness
        let conditional_types = vec![
            UniqueType::ConditionalBetweenHappiness,
            UniqueType::ConditionalBelowHappiness,
        ];

        for conditional_type in conditional_types {
            let triggered_uniques = self.viewing_player.borrow().get_triggered_uniques(conditional_type);

            // Sort by type to maintain consistent order
            let mut sorted_uniques: Vec<_> = triggered_uniques.into_iter().collect();
            sorted_uniques.sort_by(|a, b| a.type_.cmp(&b.type_));

            // Filter out hidden uniques
            for unique in sorted_uniques {
                if !unique.is_hidden_to_users() {
                    uniques.push(unique);
                }
            }
        }

        self.uniques = uniques;
        self.show = !self.uniques.is_empty();
    }

    pub fn update(&mut self) {
        let mut table = Ui::default();
        table.defaults().pad(5.0);

        let mut malcontent_icon = ImageGetter::get_stat_icon("Malcontent");
        malcontent_icon.size(Constants::heading_font_size() as f32);
        malcontent_icon.right().pad_right(1.0);
        table.add(malcontent_icon);

        table.add_label("Unhappiness", Constants::heading_font_size(), false).left();
        table.add_separator();

        // Render uniques with markup
        let label_width = (self.overview_screen.borrow().stage.width * 0.25).max(self.great_people_table.borrow().width * 0.8);

        let formatted_lines: Vec<_> = self.uniques.iter()
            .map(|unique| FormattedLine::from_unique(unique))
            .collect();

        table.add(MarkupRenderer::render(
            &formatted_lines,
            label_width,
            FormattedLine::IconDisplay::NoLink,
        )).colspan(2);

        self.table = table;
    }

    pub fn get_table(&self) -> &Ui {
        &self.table
    }
}