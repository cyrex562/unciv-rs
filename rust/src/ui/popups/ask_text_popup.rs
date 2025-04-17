use ggez::graphics::{Color, DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::ui::components::UncivTextField;
use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::images::ImageGetter;

/// A popup dialog that prompts the user to enter text
pub struct AskTextPopup {
    base: Popup,
    label: String,
    icon: ImageGetter,
    default_text: String,
    error_text: String,
    max_length: usize,
    validate: Box<dyn Fn(&str) -> bool>,
    action_on_ok: Box<dyn Fn(String)>,
    text_field: UncivTextField,
    error_label: Option<Text>,
    illegal_chars: &'static str,
}

impl AskTextPopup {
    /// Creates a new AskTextPopup
    pub fn new(
        screen: &BaseScreen,
        label: impl Into<String>,
        icon: ImageGetter,
        default_text: impl Into<String>,
        error_text: impl Into<String>,
        max_length: usize,
        validate: impl Fn(&str) -> bool + 'static,
        action_on_ok: impl Fn(String) + 'static,
    ) -> Self {
        let label = label.into();
        let default_text = default_text.into();
        let error_text = error_text.into();

        let mut popup = Self {
            base: Popup::new(screen),
            label,
            icon,
            default_text: default_text.clone(),
            error_text,
            max_length,
            validate: Box::new(validate),
            action_on_ok: Box::new(action_on_ok),
            text_field: UncivTextField::new(&label, &default_text),
            error_label: None,
            illegal_chars: "[]{}\"\\<>",
        };

        popup.setup_ui();
        popup
    }

    fn setup_ui(&mut self) {
        // Add icon and label
        let mut wrapper = self.base.add_table();
        wrapper.add(self.icon.clone());
        wrapper.add_text(&self.label);
        self.base.add(wrapper);

        // Setup text field
        self.text_field.set_max_length(self.max_length);
        self.text_field.set_filter(|c| !self.illegal_chars.contains(c));
        self.base.add(self.text_field.clone());

        // Add buttons
        self.base.add_close_button();
        self.base.add_ok_button(
            move |ctx| {
                let text = self.text_field.get_text();
                if !(self.validate)(&text) {
                    if self.error_label.is_none() {
                        let mut error_text = Text::new(&self.error_text);
                        error_text.set_color(Color::RED);
                        self.error_label = Some(error_text);
                    }
                    false
                } else {
                    true
                }
            },
            move |ctx| {
                (self.action_on_ok)(self.text_field.get_text());
            },
        );

        self.base.equalize_last_two_button_widths();
        self.base.set_keyboard_focus(&self.text_field);
    }
}

impl super::Popup for AskTextPopup {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.base.draw(ctx)?;

        if let Some(error_label) = &self.error_label {
            let pos = Point2 {
                x: self.base.get_x() + self.base.get_width() / 2.0,
                y: self.base.get_y() + self.base.get_height() - 100.0,
            };
            error_label.draw(ctx, DrawParam::new().dest(pos))?;
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.base.update(ctx)
    }
}