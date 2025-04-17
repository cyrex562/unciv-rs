use ggez::graphics::{DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::components::input::KeyboardBinding;
use crate::constants::CANCEL;

/// A popup dialog for confirming actions with the user
pub struct ConfirmPopup {
    base: Popup,
    question: String,
    confirm_text: String,
    is_confirm_positive: bool,
    restore_default: Box<dyn Fn()>,
    action: Box<dyn Fn()>,
    prompt_label: Text,
}

impl ConfirmPopup {
    /// Creates a new ConfirmPopup
    pub fn new(
        screen: &BaseScreen,
        question: impl Into<String>,
        confirm_text: impl Into<String>,
        is_confirm_positive: bool,
        restore_default: impl Fn() + 'static,
        action: impl Fn() + 'static,
    ) -> Self {
        let question = question.into();
        let confirm_text = confirm_text.into();

        let mut popup = Self {
            base: Popup::new(screen),
            question: question.clone(),
            confirm_text,
            is_confirm_positive,
            restore_default: Box::new(restore_default),
            action: Box::new(action),
            prompt_label: Text::new(&question),
        };

        popup.setup_ui();
        popup
    }

    fn setup_ui(&mut self) {
        // Set up prompt label
        self.prompt_label.set_alignment(ggez::graphics::Align::Center);
        self.base.add(&self.prompt_label);

        // Add close button
        self.base.add_close_button(
            CANCEL,
            KeyboardBinding::Cancel,
            Box::new(move |_| (self.restore_default)(),
        );

        // Add confirm button
        let confirm_style = if self.is_confirm_positive {
            self.base.get_positive_button_style()
        } else {
            self.base.get_negative_button_style()
        };

        self.base.add_ok_button_with_style(
            &self.confirm_text,
            KeyboardBinding::Confirm,
            confirm_style,
            Box::new(move |_| (self.action)(),
        );

        self.base.equalize_last_two_button_widths();
    }
}

impl super::Popup for ConfirmPopup {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.base.draw(ctx)
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.base.update(ctx)
    }
}