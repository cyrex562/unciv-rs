use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::gui::GUI;
use crate::models::civilization::PlayerType;
use crate::models::metadata::GameSettings;
use crate::ui::components::widgets::{Checkbox, Slider};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;

pub struct AutomationTab {
    options_popup: OptionsPopup,
}

impl AutomationTab {
    pub fn new(options_popup: OptionsPopup) -> Self {
        Self { options_popup }
    }

    pub fn render(&self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        // Add heading
        let heading = Text::new("Automation")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Add automation settings
        self.add_auto_assign_city_production(&mut table);
        self.add_auto_build_roads(&mut table);
        self.add_automated_workers_replace_improvements(&mut table);
        self.add_automated_units_move_on_turn_start(&mut table);
        self.add_automated_units_can_upgrade(&mut table);
        self.add_automated_units_choose_promotions(&mut table);
        self.add_cities_auto_bombard_at_end_of_turn(&mut table);

        // Add separator
        table.add_separator();

        // Add AutoPlay heading
        let autoplay_heading = Text::new("AutoPlay")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(autoplay_heading).colspan(2).row();

        // Add AutoPlay settings
        self.add_show_autoplay_button(&mut table);
        self.add_autoplay_until_victory(&mut table);

        // Add max turns slider if not playing until victory
        if !self.options_popup.settings.auto_play.auto_play_until_end {
            self.add_autoplay_max_turns_slider(&mut table);
        }

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn add_auto_assign_city_production(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Auto-assign city production", settings.auto_assign_city_production);

        checkbox.on_change(Box::new(move |value| {
            settings.auto_assign_city_production = value;

            // If enabled and we're in a world screen with the current player being human
            if value {
                if let Some(world_screen) = GUI::get_world_screen_if_active() {
                    if world_screen.viewing_civ.is_current_player() &&
                       world_screen.viewing_civ.player_type == PlayerType::Human {
                        // Auto-assign production for all cities
                        for city in world_screen.game_info.get_current_player_civilization().cities.iter() {
                            city.city_constructions.choose_next_construction();
                        }
                    }
                }
            }
        }));

        table.add(checkbox);
    }

    fn add_auto_build_roads(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new("Auto-build roads", settings.auto_building_roads);

        checkbox.on_change(Box::new(move |value| {
            settings.auto_building_roads = value;
        }));

        table.add(checkbox);
    }

    fn add_automated_workers_replace_improvements(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Automated workers replace improvements",
            settings.automated_workers_replace_improvements
        );

        checkbox.on_change(Box::new(move |value| {
            settings.automated_workers_replace_improvements = value;
        }));

        table.add(checkbox);
    }

    fn add_automated_units_move_on_turn_start(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Automated units move on turn start",
            settings.automated_units_move_on_turn_start
        );

        checkbox.on_change(Box::new(move |value| {
            settings.automated_units_move_on_turn_start = value;
        }));

        table.add(checkbox);
    }

    fn add_automated_units_can_upgrade(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Automated units can upgrade",
            settings.automated_units_can_upgrade
        );

        checkbox.on_change(Box::new(move |value| {
            settings.automated_units_can_upgrade = value;
        }));

        table.add(checkbox);
    }

    fn add_automated_units_choose_promotions(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Automated units choose promotions",
            settings.automated_units_choose_promotions
        );

        checkbox.on_change(Box::new(move |value| {
            settings.automated_units_choose_promotions = value;
        }));

        table.add(checkbox);
    }

    fn add_cities_auto_bombard_at_end_of_turn(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Cities auto-bombard at end of turn",
            settings.cities_auto_bombard_at_end_of_turn
        );

        checkbox.on_change(Box::new(move |value| {
            settings.cities_auto_bombard_at_end_of_turn = value;
        }));

        table.add(checkbox);
    }

    fn add_show_autoplay_button(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "Show AutoPlay button",
            settings.auto_play.show_autoplay_button
        );

        checkbox.on_change(Box::new(move |value| {
            settings.auto_play.show_autoplay_button = value;

            // Stop autoplay if button is hidden
            if let Some(world_screen) = GUI::get_world_screen_if_active() {
                if let Some(autoplay) = &world_screen.autoplay {
                    autoplay.stop_autoplay();
                }
            }
        }));

        table.add(checkbox);
    }

    fn add_autoplay_until_victory(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;
        let checkbox = Checkbox::new(
            "AutoPlay until victory",
            settings.auto_play.auto_play_until_end
        );

        checkbox.on_change(Box::new(move |value| {
            settings.auto_play.auto_play_until_end = value;

            // If not playing until victory, add the max turns slider
            if !value {
                self.add_autoplay_max_turns_slider(table);
            } else {
                // Otherwise, refresh the tab
                self.options_popup.tabs.replace_page(
                    self.options_popup.tabs.active_page,
                    AutomationTab::new(self.options_popup.clone())
                );
            }
        }));

        table.add(checkbox);
    }

    fn add_autoplay_max_turns_slider(&self, table: &mut BaseScreen) {
        let settings = &mut self.options_popup.settings;

        table.add_label("Multi-turn AutoPlay amount").left().fill_x();

        let mut slider = Slider::new(
            1.0, 200.0, 1.0,
            settings.auto_play.auto_play_max_turns as f32
        );

        slider.on_change(Box::new(move |value| {
            let turns = value as i32;
            settings.auto_play.auto_play_max_turns = turns;
        }));

        table.add(slider)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }
}