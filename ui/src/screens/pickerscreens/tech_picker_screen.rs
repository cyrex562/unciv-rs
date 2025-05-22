// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/TechPickerScreen.kt

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea, RichText, Vec2, Grid};
use crate::models::civilization::Civilization;
use crate::models::civilization::managers::TechManager;
use crate::models::ruleset::tech::Technology;
use crate::models::ruleset::unique::UniqueType;
use crate::models::unciv_sound::UncivSound;
use crate::ui::images::ImageGetter;
use crate::ui::popups::toast_popup::ToastPopup;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::utils::translation::tr;
use crate::utils::concurrency::Concurrency;
use super::tech_button::TechButton;

/// Screen for picking technologies to research
pub struct TechPickerScreen {
    civ_info: Rc<RefCell<Civilization>>,
    center_on_tech: Option<Rc<Technology>>,
    free_tech_pick: bool,
    ruleset: Rc<crate::models::ruleset::Ruleset>,
    tech_name_to_button: HashMap<String, TechButton>,
    selected_tech: Option<Rc<Technology>>,
    civ_tech: Rc<TechManager>,
    temp_techs_to_research: Vec<String>,
    lines: Vec<Image>,
    order_indicators: Vec<Image>,
    era_labels: Vec<String>,
    tech_table: Grid,
    researchable_techs: HashSet<String>,
    current_tech_color: Color32,
    researched_tech_color: Color32,
    researchable_tech_color: Color32,
    queued_tech_color: Color32,
    researched_future_tech_color: Color32,
    turns_to_tech: HashMap<String, i32>,
    picker_screen: PickerScreen,
}

impl TechPickerScreen {
    /// Creates a new tech picker screen
    pub fn new(civ_info: Rc<RefCell<Civilization>>, center_on_tech: Option<Rc<Technology>>) -> Self {
        let civ = civ_info.borrow();
        let free_tech_pick = civ.tech.free_techs != 0;
        let ruleset = Rc::new(civ.game_info.ruleset.clone());
        let civ_tech = Rc::new(civ.tech.clone());

        // Get researchable techs
        let researchable_techs = ruleset.technologies.keys()
            .filter(|&tech_name| civ_tech.can_be_researched(tech_name))
            .cloned()
            .collect::<HashSet<String>>();

        // Get turns to tech
        let turns_to_tech = ruleset.technologies.iter()
            .map(|(name, _)| (name.clone(), civ_tech.turns_to_tech(name)))
            .collect::<HashMap<String, i32>>();

        // Create screen
        let mut screen = Self {
            civ_info: Rc::clone(&civ_info),
            center_on_tech,
            free_tech_pick,
            ruleset: Rc::clone(&ruleset),
            tech_name_to_button: HashMap::new(),
            selected_tech: None,
            civ_tech: Rc::clone(&civ_tech),
            temp_techs_to_research: Vec::new(),
            lines: Vec::new(),
            order_indicators: Vec::new(),
            era_labels: Vec::new(),
            tech_table: Grid::new("tech_table"),
            researchable_techs,
            current_tech_color: Color32::from_rgba_premultiplied(72, 147, 175, 255),
            researched_tech_color: Color32::from_rgba_premultiplied(255, 215, 0, 255),
            researchable_tech_color: Color32::from_rgba_premultiplied(28, 170, 0, 255),
            queued_tech_color: Color32::from_rgba_premultiplied(14, 92, 86, 255),
            researched_future_tech_color: Color32::from_rgba_premultiplied(127, 50, 0, 255),
            turns_to_tech,
            picker_screen: PickerScreen::new(false),
        };

        screen.init();
        screen
    }

    /// Initializes the screen
    fn init(&mut self) {
        // Set default close action
        self.picker_screen.set_default_close_action();

        // Set up description label click handler
        self.picker_screen.set_description_label_on_click(|| {
            if let Some(tech) = &self.selected_tech {
                // TODO: Implement open_civilopedia
                // self.open_civilopedia(tech.make_link());
            }
        });

        // Initialize temp techs to research
        self.temp_techs_to_research = self.civ_tech.techs_to_research.clone();

        // Create tech table
        self.create_tech_table();

        // Set buttons info
        self.set_buttons_info();

        // Add lines and order indicators to tech table
        // TODO: Implement adding lines and order indicators

        // Add tech table to top table
        self.picker_screen.add_to_top_table(self.tech_table.clone());

        // Set background colors
        // TODO: Implement setting background colors

        // Set right side button text
        let button_text = if self.free_tech_pick {
            tr("Pick a free tech")
        } else {
            tr("Pick a tech")
        };
        self.picker_screen.set_right_side_button_text(&button_text);

        // Set right side button click handler
        self.picker_screen.set_right_side_button_on_click(UncivSound::Paper, || {
            self.try_exit();
        });

        // Center on technology
        if let Some(tech) = &self.center_on_tech {
            // Select only if it doesn't mess up temp_techs_to_research
            if self.civ_tech.is_researched(&tech.name) || self.civ_tech.techs_to_research.len() <= 1 {
                self.select_technology(Some(Rc::clone(tech)), false, true);
            } else {
                self.center_on_technology(tech);
            }
        } else {
            // Center on any possible technology which is ready for research right now
            if let Some(first_available) = self.researchable_techs.iter().next() {
                if let Some(first_available_tech) = self.ruleset.technologies.get(first_available) {
                    self.center_on_technology(first_available_tech);
                }
            }
        }
    }

    /// Gets the civilopedia ruleset
    pub fn get_civilopedia_ruleset(&self) -> Rc<crate::models::ruleset::Ruleset> {
        Rc::clone(&self.ruleset)
    }

    /// Tries to exit the screen
    fn try_exit(&mut self) {
        if self.free_tech_pick {
            if let Some(tech) = &self.selected_tech {
                let free_tech = tech.name.clone();
                // More evil people fast-clicking to cheat - #4977
                if !self.researchable_techs.contains(&free_tech) {
                    return;
                }
                self.civ_tech.get_free_technology(&free_tech);
            }
        } else {
            self.civ_tech.techs_to_research = self.temp_techs_to_research.clone();
        }

        self.civ_tech.update_research_progress();

        // TODO: Implement add_completed_tutorial_task
        // self.game.settings.add_completed_tutorial_task("Pick technology");

        // TODO: Implement pop_screen
        // self.game.pop_screen();
    }

    /// Creates the tech table
    fn create_tech_table(&mut self) {
        // Clear existing era labels
        self.era_labels.clear();

        // Get all techs
        let all_techs = self.ruleset.technologies.values().cloned().collect::<Vec<Rc<Technology>>>();
        if all_techs.is_empty() {
            return;
        }

        // Get max columns and rows
        let columns = all_techs.iter().map(|tech| tech.column.as_ref().map(|c| c.column_number).unwrap_or(0)).max().unwrap_or(0) + 1;
        let rows = all_techs.iter().map(|tech| tech.row).max().unwrap_or(0) + 1;

        // Create tech matrix
        let mut tech_matrix = vec![vec![None; rows]; columns];

        // Fill tech matrix
        for technology in &all_techs {
            if let Some(column) = technology.column.as_ref() {
                let column_index = column.column_number;
                let row_index = technology.row - 1;
                tech_matrix[column_index][row_index] = Some(Rc::clone(technology));
            }
        }

        // Create era labels
        let mut eras_names_to_columns = HashMap::new();
        for tech in &all_techs {
            let era = tech.era();
            let column_number = tech.column.as_ref().map(|c| c.column_number).unwrap_or(0);

            eras_names_to_columns.entry(era).or_insert_with(Vec::new).push(column_number);
        }

        // Add era labels
        for (era, era_columns) in &eras_names_to_columns {
            let column_span = era_columns.len();
            let color = if self.civ_tech.era.name == *era {
                self.queued_tech_color
            } else if self.ruleset.eras.get(era).map(|e| e.era_number).unwrap_or(0) < self.civ_tech.era.era_number {
                Color32::from_rgba_premultiplied(255, 175, 0, 255)
            } else {
                ImageGetter::CHARCOAL
            };

            // Add era label
            self.era_labels.push(era.clone());

            // TODO: Implement adding era label to tech table
        }

        // Add tech buttons
        for row_index in 0..rows {
            self.tech_table.row();

            for column_index in 0..columns {
                let tech = tech_matrix[column_index][row_index].clone();

                // Create table for tech button
                let mut table = Grid::new("tech_button_table");
                table.pad(2.0);
                table.pad_right(60.0);
                table.pad_left(20.0);

                if row_index == 0 {
                    table.pad_top(7.0);
                }

                // Set background color for current era
                if let Some(era_columns) = eras_names_to_columns.get(&self.civ_tech.era.name) {
                    if era_columns.contains(&column_index) {
                        // TODO: Implement setting background color
                    }
                }

                if tech.is_none() {
                    // Add empty table
                    self.tech_table.add(table);
                } else {
                    let tech = tech.unwrap();
                    let tech_button = TechButton::new(tech.name.clone(), Rc::clone(&self.civ_tech), false);

                    // Add tech button to table
                    table.add(tech_button);

                    // Add tech button to map
                    self.tech_name_to_button.insert(tech.name.clone(), tech_button);

                    // Set up click handlers
                    // TODO: Implement click handlers

                    // Add table to tech table
                    self.tech_table.add(table);
                }
            }
        }
    }

    /// Sets the buttons info
    fn set_buttons_info(&mut self) {
        for (tech_name, tech_button) in &mut self.tech_name_to_button {
            let is_researched = self.civ_tech.is_researched(tech_name);

            // Set button color
            let color = if is_researched && tech_name != "Future Tech" {
                self.researched_tech_color
            } else if is_researched {
                self.researched_future_tech_color
            } else if self.temp_techs_to_research.first().map_or(false, |t| t == tech_name) && !self.free_tech_pick {
                self.current_tech_color
            } else if self.researchable_techs.contains(tech_name) {
                self.researchable_tech_color
            } else if self.temp_techs_to_research.contains(tech_name) {
                self.queued_tech_color
            } else {
                ImageGetter::CHARCOAL
            };

            tech_button.set_button_color(color);

            // Set text color for researched techs
            if is_researched && tech_name != "Future Tech" {
                // TODO: Implement setting text color
            }

            // Set turns text for unresearched techs
            if !is_researched || tech_name == "Future Tech" {
                // TODO: Implement setting turns text
            }

            // Set text
            // TODO: Implement setting text
        }

        // Add connecting lines
        self.add_connecting_lines();

        // Add order indicators
        self.add_order_indicators();
    }

    /// Adds connecting lines between techs
    fn add_connecting_lines(&mut self) {
        // TODO: Implement adding connecting lines
    }

    /// Adds order indicators to tech buttons
    fn add_order_indicators(&mut self) {
        // TODO: Implement adding order indicators
    }

    /// Selects a technology
    fn select_technology(&mut self, tech: Option<Rc<Technology>>, queue: bool, center: bool) {
        let previous_selected_tech = self.selected_tech.clone();
        self.selected_tech = tech.clone();

        // Set description label text
        if let Some(tech) = &tech {
            // TODO: Implement setting description label text
        }

        // Center on technology
        if center {
            if let Some(tech) = &tech {
                self.center_on_technology(tech);
            }
        }

        // Handle free tech pick
        if self.free_tech_pick {
            if let Some(tech) = &tech {
                self.select_technology_for_free_tech(tech);
                self.set_buttons_info();
            }
            return;
        }

        // Handle god mode
        if self.civ_info.borrow().game_info.game_parameters.god_mode
            && tech.as_ref().map_or(false, |t| !self.civ_tech.is_researched(&t.name))
            && self.selected_tech == previous_selected_tech {
            if let Some(tech) = &tech {
                self.civ_tech.add_technology(&tech.name);
            }
        }

        // Handle already researched tech
        if let Some(tech) = &tech {
            if self.civ_tech.is_researched(&tech.name) && !tech.is_continually_researchable() {
                self.picker_screen.set_right_side_button_text(&tr("Pick a tech"));
                self.picker_screen.set_right_side_button_enabled(false);
                self.set_buttons_info();
                return;
            }
        }

        // Handle state change
        // TODO: Implement checking if state change is allowed
        // if !GUI.is_allowed_change_state() {
        //     self.picker_screen.set_right_side_button_enabled(false);
        //     return;
        // }

        // Get path to tech
        if let Some(tech) = &tech {
            let path_to_tech = self.civ_tech.get_required_techs_to_destination(tech);

            // Check for unavailable techs
            for required_tech in &path_to_tech {
                for unique in required_tech.unique_objects.iter()
                    .filter(|u| u.unique_type == UniqueType::OnlyAvailable && !u.conditionals_apply(&self.civ_info.borrow().state)) {
                    self.picker_screen.set_right_side_button_text(&tr(&unique.get_display_text()));
                    self.picker_screen.set_right_side_button_enabled(false);
                    return;
                }
            }

            // Handle queue
            if queue {
                for path_tech in &path_to_tech {
                    if !self.temp_techs_to_research.contains(&path_tech.name) {
                        self.temp_techs_to_research.push(path_tech.name.clone());
                    }
                }
            } else {
                self.temp_techs_to_research.clear();
                self.temp_techs_to_research.extend(path_to_tech.iter().map(|t| t.name.clone()));
            }

            // Set right side button text
            if !self.temp_techs_to_research.is_empty() {
                let label = format!("Research [{}]", self.temp_techs_to_research[0]);
                let tech_progression = self.get_tech_progress_label(&self.temp_techs_to_research);
                self.picker_screen.pick(&format!("{}\n{}", tr(&label), tech_progression));
            } else {
                self.picker_screen.set_right_side_button_text(&tr("Unavailable"));
                self.picker_screen.set_right_side_button_enabled(false);
            }
        }

        self.set_buttons_info();
    }

    /// Gets the tech progress label
    fn get_tech_progress_label(&self, techs: &[String]) -> String {
        let progress = techs.iter().map(|tech| self.civ_tech.research_of_tech(tech)).sum::<i32>() + self.civ_tech.get_overflow_science();
        let tech_cost = techs.iter().map(|tech| self.civ_tech.cost_of_tech(tech)).sum::<i32>();
        format!("({}/{})", progress, tech_cost)
    }

    /// Centers on a technology
    fn center_on_technology(&self, tech: &Technology) {
        // TODO: Implement centering on technology
    }

    /// Selects a technology for free tech
    fn select_technology_for_free_tech(&mut self, tech: &Technology) {
        if self.researchable_techs.contains(&tech.name) {
            let label = format!("Pick [{}] as free tech", tech.name);
            let tech_progression = self.get_tech_progress_label(&[tech.name.clone()]);
            self.picker_screen.pick(&format!("{}\n{}", tr(&label), tech_progression));
        } else {
            self.picker_screen.set_right_side_button_text(&tr("Pick a free tech"));
            self.picker_screen.set_right_side_button_enabled(false);
        }
    }

    /// Shows the screen
    pub fn show(&mut self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}