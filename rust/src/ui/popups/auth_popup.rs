use ggez::graphics::{Color, DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::ui::components::UncivTextField;
use crate::ui::popups::Popup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::game::UncivGame;

/// A popup dialog for server authentication
pub struct AuthPopup {
    base: Popup,
    password_field: UncivTextField,
    auth_successful: Option<Box<dyn Fn(bool)>>,
    error_label: Option<Text>,
}

impl AuthPopup {
    /// Creates a new AuthPopup
    pub fn new(
        screen: &BaseScreen,
        auth_successful: Option<Box<dyn Fn(bool)>>,
    ) -> Self {
        let mut popup = Self {
            base: Popup::new(screen),
            password_field: UncivTextField::new("Password", ""),
            auth_successful,
            error_label: None,
        };

        popup.setup_ui();
        popup
    }

    fn setup_ui(&mut self) {
        // Add password field
        self.base.add_good_sized_label("Please enter your server password");
        self.base.add(self.password_field.clone());

        // Add buttons
        let negative_style = self.base.get_negative_button_style();

        self.base.add_close_button_with_style(negative_style.clone(), move |_| {
            if let Some(callback) = &self.auth_successful {
                callback(false);
            }
        });

        let auth_button = self.base.add_button("Authenticate", move |ctx| {
            match UncivGame::current().online_multiplayer().multiplayer_server().authenticate(
                self.password_field.get_text()
            ) {
                Ok(_) => {
                    if let Some(callback) = &self.auth_successful {
                        callback(true);
                    }
                    self.base.close();
                }
                Err(_) => {
                    self.base.clear();
                    self.base.add_good_sized_label("Authentication failed");
                    self.base.add(self.password_field.clone());

                    self.base.add_close_button_with_style(
                        negative_style.clone(),
                        move |_| {
                            if let Some(callback) = &self.auth_successful {
                                callback(false);
                            }
                        }
                    );

                    self.base.add_button("Authenticate", move |ctx| {
                        // Retry authentication
                        match UncivGame::current().online_multiplayer().multiplayer_server().authenticate(
                            self.password_field.get_text()
                        ) {
                            Ok(_) => {
                                if let Some(callback) = &self.auth_successful {
                                    callback(true);
                                }
                                self.base.close();
                            }
                            Err(_) => {
                                // Keep the error state
                            }
                        }
                    });
                }
            }
        });

        self.base.equalize_last_two_button_widths();
        self.base.set_keyboard_focus(&self.password_field);
    }
}

impl super::Popup for AuthPopup {
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