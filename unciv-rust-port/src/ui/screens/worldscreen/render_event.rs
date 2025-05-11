// Port of orig_src/core/src/com/unciv/ui/screens/worldscreen/RenderEvent.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Vec2, Color32};

use crate::game::civilization::Civilization;
use crate::game::map::map_unit::MapUnit;
use crate::models::ruleset::{Event, EventChoice};
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::ui::screens::world_screen::WorldScreen;
use crate::ui::components::keyboard::KeyCharAndCode;
use crate::ui::screens::civilopedia_screen::FormattedLine;

pub struct RenderEvent {
    event: Event,
    world_screen: Rc<RefCell<WorldScreen>>,
    unit: Option<Rc<RefCell<MapUnit>>>,
    on_choice: Box<dyn Fn(&EventChoice)>,
    is_valid: bool,
}

impl RenderEvent {
    pub fn new(
        event: Event,
        world_screen: Rc<RefCell<WorldScreen>>,
        unit: Option<Rc<RefCell<MapUnit>>>,
        on_choice: Box<dyn Fn(&EventChoice)>,
    ) -> Self {
        let game_info = world_screen.borrow().game_info.clone();
        let current_player_civ = game_info.borrow().current_player_civ.clone();
        let state = StateForConditionals::new(current_player_civ, unit.clone());

        let choices = event.get_matching_choices(&state);
        let is_valid = choices.is_some();

        Self {
            event,
            world_screen,
            unit,
            on_choice,
            is_valid,
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        if !self.is_valid {
            return;
        }

        let stage_width = ui.available_width();

        // Show event text if present
        if !self.event.text.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label(&self.event.text);
            });
            ui.add_space(5.0);
        }

        // Show civilopedia text if present
        if !self.event.civilopedia_text.is_empty() {
            self.render_civilopedia_text(ui, stage_width * 0.5);
            ui.add_space(5.0);
        }

        // Show choices
        if let Some(choices) = self.event.get_matching_choices(&self.get_state()) {
            for choice in choices {
                self.add_choice(ui, &choice);
                ui.add_space(5.0);
            }
        }
    }

    fn add_choice(&self, ui: &mut Ui, choice: &EventChoice) {
        ui.separator();

        // Choice button
        if ui.button(&choice.text).clicked() {
            (self.on_choice)(choice);
            choice.trigger_choice(
                &self.world_screen.borrow().game_info.borrow().current_player_civ,
                self.unit.as_ref()
            );
        }

        // Keyboard shortcut
        if let Some(key) = KeyCharAndCode::parse(&choice.key_shortcut) {
            if ui.input().key_pressed(key.into()) {
                (self.on_choice)(choice);
                choice.trigger_choice(
                    &self.world_screen.borrow().game_info.borrow().current_player_civ,
                    self.unit.as_ref()
                );
            }
        }

        // Show civilopedia text and unique objects
        let mut lines = Vec::new();
        if !choice.civilopedia_text.is_empty() {
            lines.extend(choice.civilopedia_text.iter().cloned());
        }

        lines.extend(
            choice.unique_objects.iter()
                .filter(|u| u.is_triggerable() || u.unique_type == UniqueType::Comment)
                .filter(|u| !u.is_hidden_to_users())
                .map(|u| FormattedLine::new(u))
        );

        if !lines.is_empty() {
            self.render_formatted_lines(ui, &lines);
        }
    }

    fn render_civilopedia_text(&self, ui: &mut Ui, width: f32) {
        // TODO: Implement civilopedia text rendering with markup
    }

    fn render_formatted_lines(&self, ui: &mut Ui, lines: &[FormattedLine]) {
        // TODO: Implement formatted lines rendering
    }

    fn get_state(&self) -> StateForConditionals {
        let current_player_civ = self.world_screen.borrow().game_info.borrow().current_player_civ.clone();
        StateForConditionals::new(current_player_civ, self.unit.clone())
    }

    fn open_civilopedia(&self, link: &str) {
        // TODO: Implement civilopedia opening
        self.world_screen.borrow_mut().open_civilopedia(link);
    }
}