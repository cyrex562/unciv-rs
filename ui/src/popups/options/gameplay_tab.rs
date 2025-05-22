use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::GameSettings;
use crate::ui::components::widgets::{Checkbox, Slider};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;

pub struct GameplayTab {
    options_popup: OptionsPopup,
}

impl GameplayTab {
    pub fn new(options_popup: OptionsPopup) -> Self {
        Self { options_popup }
    }

    pub fn render(&self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        let settings = &mut self.options_popup.settings;

        // Add unit management settings
        self.add_unit_management_settings(&mut table, settings);

        // Add movement settings
        self.add_movement_settings(&mut table, settings);

        // Add trade settings
        self.add_trade_settings(&mut table, settings);

        // Add turn settings
        self.add_turn_settings(&mut table, settings);

        // Add notification settings
        self.add_notification_settings(&mut table, settings);

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn add_unit_management_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add heading
        let heading = Text::new("Unit Management")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Check for idle units
        let check_idle_units = Checkbox::new("Check for idle units", settings.check_for_due_units);
        check_idle_units.on_change(Box::new(move |value| {
            settings.check_for_due_units = value;
        }));
        table.add(check_idle_units);

        // Next unit button cycles idle units
        let cycle_idle_units = Checkbox::new("'Next unit' button cycles idle units", settings.check_for_due_units_cycles);
        cycle_idle_units.on_change(Box::new(move |value| {
            settings.check_for_due_units_cycles = value;
        }));
        table.add(cycle_idle_units);

        // Show small skip/cycle unit button
        let small_unit_button = Checkbox::new("Show Small Skip/Cycle Unit Button", settings.small_unit_button);
        small_unit_button.on_change(Box::new(move |value| {
            settings.small_unit_button = value;
        }));
        table.add(small_unit_button);

        // Auto unit cycle
        let auto_unit_cycle = Checkbox::new("Auto Unit Cycle", settings.auto_unit_cycle);
        auto_unit_cycle.on_change(Box::new(move |value| {
            settings.auto_unit_cycle = value;
        }));
        table.add(auto_unit_cycle);
    }

    fn add_movement_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add heading
        let heading = Text::new("Movement Controls")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Move units with a single tap
        let single_tap_move = Checkbox::new("Move units with a single tap", settings.single_tap_move);
        single_tap_move.on_change(Box::new(move |value| {
            settings.single_tap_move = value;
        }));
        table.add(single_tap_move);

        // Move units with a long tap
        let long_tap_move = Checkbox::new("Move units with a long tap", settings.long_tap_move);
        long_tap_move.on_change(Box::new(move |value| {
            settings.long_tap_move = value;
        }));
        table.add(long_tap_move);
    }

    fn add_trade_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add heading
        let heading = Text::new("Trade Settings")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Order trade offers by amount
        let order_trade_offers = Checkbox::new("Order trade offers by amount", settings.order_trade_offers_by_amount);
        order_trade_offers.on_change(Box::new(move |value| {
            settings.order_trade_offers_by_amount = value;
        }));
        table.add(order_trade_offers);
    }

    fn add_turn_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add heading
        let heading = Text::new("Turn Settings")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Ask for confirmation when pressing next turn
        let confirm_next_turn = Checkbox::new("Ask for confirmation when pressing next turn", settings.confirm_next_turn);
        confirm_next_turn.on_change(Box::new(move |value| {
            settings.confirm_next_turn = value;
        }));
        table.add(confirm_next_turn);
    }

    fn add_notification_settings(&self, table: &mut BaseScreen, settings: &mut GameSettings) {
        // Add heading
        let heading = Text::new("Notification Settings")
            .set_font_size(Constants::HEADING_FONT_SIZE)
            .set_color(Color::WHITE);
        table.add_label(heading).colspan(2).row();

        // Add notification log max turns slider
        table.add_label("Notifications log max turns").left().fill_x();

        let notification_slider = Slider::new(
            3.0, 15.0, 1.0,
            settings.notifications_log_max_turns as f32
        );

        notification_slider.on_change(Box::new(move |value| {
            settings.notifications_log_max_turns = value as i32;
        }));

        table.add(notification_slider)
            .min_width(self.options_popup.select_box_min_width)
            .pad(10.0)
            .row();
    }
}