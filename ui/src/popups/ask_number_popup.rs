use std::sync::Arc;
use ggez::graphics::{Color, DrawParam, Mesh, MeshBatch, Rect};
use ggez::input::keyboard::KeyCode;
use ggez::{Context, GameResult};
use ggez::mint::Vector2;

use crate::ui::components::input::{KeyCharAndCode, KeyboardBinding};
use crate::ui::components::widgets::{Button, Container, Table, TextField};
use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::images::icon_circle_group::IconCircleGroup;
use crate::ui::images::image_getter::ImageGetter;
use crate::utils::concurrency::Concurrency;
use crate::utils::translations::tr;

/// A popup that prompts the user to enter a number
///
/// # Arguments
///
/// * `screen` - The screen to show the popup on
/// * `label` - The text to display above the input field
/// * `icon` - The icon to display at the top of the popup
/// * `default_value` - The default value to display in the input field
/// * `amount_buttons` - A list of values to use for quick adjustment buttons
/// * `bounds` - The valid range for the input value
/// * `error_text` - The text to display when validation fails
/// * `validate` - A function that validates the input value
/// * `action_on_ok` - A function to call when the user clicks OK with a valid input
pub struct AskNumberPopup {
    popup: Popup,
    text_field: TextField,
    error_label: Option<Button>,
    bounds: (i32, i32),
    validate: Box<dyn Fn(i32) -> bool>,
    action_on_ok: Box<dyn Fn(i32)>,
}

impl AskNumberPopup {
    /// Creates a new AskNumberPopup
    pub fn new(
        screen: &BaseScreen,
        label: &str,
        icon: Option<IconCircleGroup>,
        default_value: &str,
        amount_buttons: &[i32],
        bounds: (i32, i32),
        error_text: &str,
        validate: Box<dyn Fn(i32) -> bool>,
        action_on_ok: Box<dyn Fn(i32)>,
    ) -> Self {
        let mut popup = Popup::new(screen, false);

        // Create the main table
        let mut table = Table::new();
        table.defaults().pad(5.0, 15.0, 5.0, 15.0).grow_x();

        // Add the icon and label
        let mut wrapper = Table::new();
        wrapper.defaults().pad_right(10.0);

        if let Some(icon) = icon {
            wrapper.add(icon);
        } else {
            // Default icon if none provided
            let default_icon = ImageGetter::get_image("OtherIcons/Pencil")
                .with_color(Color::new(0.2, 0.2, 0.2, 1.0))
                .surround_with_circle(80.0);
            wrapper.add(default_icon);
        }

        let label_button = Button::new_with_text(label);
        wrapper.add(label_button);

        table.add(wrapper).colspan(2).row();

        // Create the text field
        let mut text_field = TextField::new(label, default_value);
        text_field.set_filter(Box::new(|_, c| c.is_ascii_digit() || c == '-'));

        // Create the center table for the text field and adjustment buttons
        let mut center_table = Table::new();
        center_table.defaults().pad(5.0);

        // Add negative adjustment buttons
        for &value in amount_buttons.iter().rev() {
            let button = Self::create_value_button(-value, &text_field, bounds);
            center_table.add(button);
        }

        // Add the text field
        center_table.add(text_field.clone()).grow_x().pad(10.0);

        // Add positive adjustment buttons
        for &value in amount_buttons {
            let button = Self::create_value_button(value, &text_field, bounds);
            center_table.add(button);
        }

        table.add(center_table).colspan(2).row();

        // Create the error label (hidden initially)
        let error_label = Button::new_with_text(error_text);
        error_label.set_color(Color::RED);

        // Add the buttons
        popup.add_close_button();
        popup.add_ok_button(
            Box::new(move || {
                let text = text_field.text();
                let is_valid = Self::is_valid_int(&text) && validate(text.parse::<i32>().unwrap_or(0));

                if !is_valid {
                    table.add(error_label.clone()).colspan(2).center();
                }

                is_valid
            }),
            Box::new(move || {
                let text = text_field.text();
                if Self::is_valid_int(&text) {
                    let value = text.parse::<i32>().unwrap_or(0);
                    action_on_ok(value);
                }
            }),
        );

        popup.equalize_last_two_button_widths();

        // Set keyboard focus to the text field
        popup.set_keyboard_focus(&text_field);

        Self {
            popup,
            text_field,
            error_label: Some(error_label),
            bounds,
            validate,
            action_on_ok,
        }
    }

    /// Creates a button that adjusts the value in the text field
    fn create_value_button(value: i32, text_field: &TextField, bounds: (i32, i32)) -> Button {
        let text = if value >= 0 {
            format!("+{}", value)
        } else {
            value.to_string()
        };

        let mut button = Button::new_with_text(&text);
        let text_field_clone = text_field.clone();
        let bounds_clone = bounds;

        button.set_on_activation(KeyboardBinding::LEFT_CLICK, Box::new(move || {
            let current_text = text_field_clone.text();
            if Self::is_valid_int(&current_text) {
                let current_value = current_text.parse::<i32>().unwrap_or(0);
                let new_value = current_value + value;
                let clamped_value = Self::clamp_in_bounds(new_value, bounds_clone);
                text_field_clone.set_text(&clamped_value.to_string());
            }
        }));

        button
    }

    /// Checks if a string can be parsed as an integer
    fn is_valid_int(input: &str) -> bool {
        input.parse::<i32>().is_ok()
    }

    /// Clamps a value to the given bounds
    fn clamp_in_bounds(value: i32, bounds: (i32, i32)) -> i32 {
        value.clamp(bounds.0, bounds.1)
    }

    /// Shows the popup
    pub fn show(&mut self) {
        self.popup.open(true);
    }

    /// Closes the popup
    pub fn close(&mut self) {
        self.popup.close();
    }

    /// Returns a reference to the underlying popup
    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    /// Returns a mutable reference to the underlying popup
    pub fn popup_mut(&mut self) -> &mut Popup {
        &mut self.popup
    }
}