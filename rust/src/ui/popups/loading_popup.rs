use ggez::graphics::{DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::ui::popups::{Popup, Scrollability};
use crate::ui::screens::base_screen::BaseScreen;
use crate::constants::LOADING;

/// A simple popup that displays a "Loading..." message and automatically opens itself
pub struct LoadingPopup {
    base: Popup,
}

impl LoadingPopup {
    /// Creates a new LoadingPopup
    pub fn new(screen: &BaseScreen) -> Self {
        let mut popup = Self {
            base: Popup::new_with_scrollability(screen, Scrollability::None),
        };

        popup.setup_ui();
        popup
    }

    fn setup_ui(&mut self) {
        self.base.add_good_sized_label(LOADING);
        self.base.open(true);
    }
}

impl super::Popup for LoadingPopup {
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.base.draw(ctx)
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.base.update(ctx)
    }
}