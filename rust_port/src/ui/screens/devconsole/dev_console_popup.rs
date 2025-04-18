use std::collections::VecDeque;
use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui};

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::devconsole::cli_input::CliInput;
use crate::ui::screens::devconsole::console_command::ConsoleCommandRoot;
use crate::ui::screens::devconsole::dev_console_response::DevConsoleResponse;
use crate::ui::components::extensions::*;
use crate::ui::components::input::KeyCharAndCode;
use crate::ui::components::widgets::UncivTextField;
use crate::ui::images::ImageGetter;
use crate::ui::popups::Popup;
use crate::logic::civilization::Civilization;
use crate::logic::map::mapunit::MapUnit;
use crate::utils::concurrency::Concurrency;

const MAX_HISTORY_SIZE: usize = 42;

/// A popup that provides a developer console interface for executing commands
pub struct DevConsolePopup {
    screen: Arc<WorldScreen>,
    history: VecDeque<String>,
    keep_open: bool,
    current_history_entry: usize,
    text_field: UncivTextField,
    response_label: String,
    response_color: Color32,
    command_root: ConsoleCommandRoot,
    game_info: Arc<crate::logic::game::GameInfo>,
}

impl DevConsolePopup {
    /// Creates a new DevConsolePopup
    pub fn new(screen: Arc<WorldScreen>) -> Self {
        let history = screen.game.settings.console_command_history.clone();
        let keep_open = screen.game.settings.keep_console_open;
        let current_history_entry = history.len();
        let text_field = UncivTextField::new("");
        let response_label = String::new();
        let response_color = Color32::RED;
        let command_root = ConsoleCommandRoot;
        let game_info = screen.game_info.clone();

        Self {
            screen,
            history,
            keep_open,
            current_history_entry,
            text_field,
            response_label,
            response_color,
            command_root,
            game_info,
        }
    }

    /// Shows the popup
    pub fn show(&mut self, ctx: &mut EguiContexts) {
        egui::Window::new("Developer Console")
            .resizable(true)
            .default_width(self.screen.stage.width() * 0.5)
            .show(ctx.ctx_mut(), |ui| {
                self.render(ui);
            });
    }

    /// Renders the popup UI
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Developer Console");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.keep_open, "Keep open");
            });
        });

        ui.horizontal(|ui| {
            if !self.screen.gui.keyboard_available {
                ui.add(self.get_autocomplete_button());
            }
            ui.add(TextEdit::singleline(&mut self.text_field.text)
                .desired_width(f32::INFINITY)
                .hint_text("Enter command..."));
            if !self.screen.gui.keyboard_available {
                ui.add(self.get_history_buttons());
            }
        });

        ui.add_space(5.0);

        ui.add(TextEdit::multiline(&mut self.response_label)
            .desired_width(f32::INFINITY)
            .text_color(self.response_color)
            .interactive(false));

        // Handle keyboard shortcuts
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.on_enter();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            self.on_autocomplete();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Backspace) && i.modifiers.alt) {
            self.on_alt_delete();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Right) && i.modifiers.alt) {
            self.on_alt_right();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Left) && i.modifiers.alt) {
            self.on_alt_left();
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.navigate_history(-1);
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.navigate_history(1);
        }
    }

    /// Gets the autocomplete button
    fn get_autocomplete_button(&self) -> egui::Button {
        egui::Button::new("▼")
            .fill(Color32::DARK_GRAY)
            .on_click(|| self.on_autocomplete())
    }

    /// Gets the history buttons
    fn get_history_buttons(&self) -> egui::Frame {
        egui::Frame::none()
            .fill(Color32::DARK_GRAY)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if ui.button("▲").clicked() {
                        self.navigate_history(-1);
                    }
                    if ui.button("▼").clicked() {
                        self.navigate_history(1);
                    }
                });
            });
    }

    /// Handles autocomplete
    fn on_autocomplete(&mut self) {
        if let Some((to_remove, to_add)) = self.get_autocomplete() {
            let text = self.text_field.text.clone();
            let new_text = format!("{}{}",
                text[..text.len().saturating_sub(to_remove)].to_string(),
                to_add
            );
            self.text_field.text = new_text;
            self.text_field.cursor_position = usize::MAX;
        }
    }

    /// Handles Alt+Delete
    fn on_alt_delete(&mut self) {
        if !ui.input(|i| i.modifiers.alt) {
            return;
        }

        Concurrency::run_on_gl_thread(|| {
            let text = self.text_field.text.clone();
            let cursor_pos = self.text_field.cursor_position;
            let last_space = text[..cursor_pos.saturating_sub(1)].rfind(' ').unwrap_or(0);

            if last_space == 0 {
                self.text_field.text = text[cursor_pos..].to_string();
                return;
            }

            self.text_field.text = format!("{}{}",
                text[..last_space + 1].to_string(),
                text[cursor_pos..].to_string()
            );
            self.text_field.cursor_position = last_space + 1;
        });
    }

    /// Handles Alt+Right
    fn on_alt_right(&mut self) {
        if !ui.input(|i| i.modifiers.alt) {
            return;
        }

        Concurrency::run_on_gl_thread(|| {
            let text = self.text_field.text.clone();
            let cursor_pos = self.text_field.cursor_position;
            let next_space = text[cursor_pos..].find(' ').map(|i| i + cursor_pos).unwrap_or(text.len());

            self.text_field.cursor_position = next_space;
        });
    }

    /// Handles Alt+Left
    fn on_alt_left(&mut self) {
        if !ui.input(|i| i.modifiers.alt) {
            return;
        }

        Concurrency::run_on_gl_thread(|| {
            let text = self.text_field.text.clone();
            let cursor_pos = self.text_field.cursor_position;
            let previous_space = text[..cursor_pos.saturating_sub(1)].rfind(' ').unwrap_or(0);

            self.text_field.cursor_position = previous_space;
        });
    }

    /// Navigates through command history
    fn navigate_history(&mut self, delta: i32) {
        if self.history.is_empty() {
            return;
        }

        self.current_history_entry = (self.current_history_entry as i32 + delta)
            .clamp(0, self.history.len() as i32) as usize;

        self.text_field.text = self.history[self.current_history_entry].clone();
        self.text_field.cursor_position = self.text_field.text.len();
    }

    /// Shows command history
    pub fn show_history(&self) {
        if self.history.is_empty() {
            return;
        }

        egui::Window::new("Command History")
            .resizable(true)
            .show(ctx.ctx_mut(), |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for (index, entry) in self.history.iter().enumerate() {
                        if ui.button(entry).clicked() {
                            self.current_history_entry = index;
                            self.navigate_history(0);
                        }
                    }
                });
            });
    }

    /// Handles Enter key press
    fn on_enter(&mut self) {
        let response = self.handle_command();
        if response.is_ok {
            self.screen.should_update = true;
            self.add_history();
            if !self.keep_open {
                self.close();
            } else {
                self.text_field.text.clear();
            }
            return;
        }
        self.show_response(&response.message, response.color);
    }

    /// Adds current command to history
    fn add_history(&mut self) {
        let text = self.text_field.text.clone();
        if text.trim().is_empty() {
            return;
        }
        if !self.history.is_empty() && self.history.back().unwrap() == &text {
            return;
        }
        if self.history.len() >= MAX_HISTORY_SIZE {
            self.history.retain(|x| x != &text);
            if self.history.len() >= MAX_HISTORY_SIZE {
                self.history.pop_front();
            }
        }
        self.history.push_back(text);
        self.current_history_entry = self.history.len();
    }

    /// Shows a response message
    pub fn show_response(&mut self, message: &str, color: Color32) {
        self.response_label = message.to_string();
        self.response_color = color;
    }

    /// Handles command execution
    fn handle_command(&mut self) -> DevConsoleResponse {
        let params = CliInput::split_to_cli_input(&self.text_field.text.trim());
        self.command_root.handle(self, &params)
    }

    /// Gets autocomplete suggestions
    fn get_autocomplete(&mut self) -> Option<(usize, String)> {
        let params = CliInput::split_to_cli_input(&self.text_field.text);
        let auto_complete_string = self.command_root.autocomplete(self, &params)?;
        let replace_length = params.last().map_or(0, |p| p.original_length());
        Some((replace_length, auto_complete_string))
    }

    /// Gets a civilization by name
    pub fn get_civ_by_name(&self, name: &CliInput) -> Result<&Civilization, Box<dyn std::error::Error>> {
        self.get_civ_by_name_or_null(name)
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Unknown civ: {}", name)
            )))
    }

    /// Gets a civilization by name or the selected one
    pub fn get_civ_by_name_or_selected(&self, name: Option<&CliInput>) -> &Civilization {
        name.map_or_else(
            || self.screen.selected_civ.as_ref().unwrap(),
            |n| self.get_civ_by_name(n).unwrap()
        )
    }

    /// Gets a civilization by name or returns None
    pub fn get_civ_by_name_or_null(&self, name: &CliInput) -> Option<&Civilization> {
        self.game_info.civilizations.iter()
            .find(|civ| name.equals(&civ.civ_name))
    }

    /// Gets the selected tile
    pub fn get_selected_tile(&self) -> Result<&Tile, Box<dyn std::error::Error>> {
        self.screen.map_holder.selected_tile
            .as_ref()
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Select tile"
            )))
    }

    /// Gets the city on the selected tile
    pub fn get_selected_city(&self) -> Result<&City, Box<dyn std::error::Error>> {
        self.get_selected_tile()?
            .get_city()
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Select tile belonging to city"
            )))
    }

    /// Gets a city by name
    pub fn get_city(&self, city_name: &CliInput) -> Result<&City, Box<dyn std::error::Error>> {
        self.game_info.get_cities().iter()
            .find(|city| city_name.equals(&city.name))
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Unknown city: {}", city_name)
            )))
    }

    /// Gets the selected unit
    pub fn get_selected_unit(&self) -> Result<&MapUnit, Box<dyn std::error::Error>> {
        let selected_tile = self.get_selected_tile()?;
        if selected_tile.get_first_unit().is_none() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Select tile with units"
            )));
        }

        let units: Vec<&MapUnit> = selected_tile.get_units().collect();
        let selected_unit = self.screen.bottom_unit_table.selected_unit.as_ref();

        if let Some(unit) = selected_unit {
            if unit.get_tile() == selected_tile {
                return Ok(unit);
            }
        }

        Ok(units.first().unwrap())
    }
}