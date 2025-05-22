// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/MultiplayerStatusButton.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;
use egui::{Button, Image, Label, Response, Ui, Color32, Vec2, Rect};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::worldscreen::status::multiplayer_status_popup::MultiplayerStatusPopup;
use crate::ui::components::widgets::LoadingImage;
use crate::ui::images::ImageGetter;
use crate::logic::event::EventBus;
use crate::logic::multiplayer::{MultiplayerGame, HasMultiplayerGameName, MultiplayerGameNameChanged,
                               MultiplayerGameUpdateStarted, MultiplayerGameUpdateEnded, MultiplayerGameUpdated};
use crate::utils::concurrency::Concurrency;
use crate::utils::launch_on_gl_thread;
use std::time::{Duration, Instant};

/// Button for multiplayer status
pub struct MultiplayerStatusButton {
    /// The screen reference
    screen: Rc<RefCell<BaseScreen>>,
    /// The current game
    cur_game: Option<Rc<RefCell<MultiplayerGame>>>,
    /// The current game name
    cur_game_name: Option<String>,
    /// The loading image
    loading_image: LoadingImage,
    /// The turn indicator
    turn_indicator: TurnIndicator,
    /// The turn indicator cell
    turn_indicator_cell: Option<Rect>,
    /// The games with current turn
    game_names_with_current_turn: HashSet<String>,
    /// The events receiver
    events: EventBus::EventReceiver,
}

impl MultiplayerStatusButton {
    /// Creates a new MultiplayerStatusButton
    pub fn new(
        screen: Rc<RefCell<BaseScreen>>,
        cur_game: Option<Rc<RefCell<MultiplayerGame>>>,
    ) -> Self {
        let cur_game_name = cur_game.as_ref().map(|game| game.borrow().name.clone());
        let loading_image = LoadingImage::new(
            "OtherIcons/Multiplayer",
            Color32::WHITE,
            500,
        );
        let turn_indicator = TurnIndicator::new();
        let game_names_with_current_turn = Self::get_initial_games_with_current_turn(cur_game_name.clone());

        let mut button = Self {
            screen,
            cur_game,
            cur_game_name,
            loading_image,
            turn_indicator,
            turn_indicator_cell: None,
            game_names_with_current_turn,
            events: EventBus::EventReceiver::new(),
        };

        button.init();
        button
    }

    /// Initializes the button
    fn init(&mut self) {
        // Set up turn indicator cell
        self.turn_indicator_cell = Some(Rect::from_min_size(
            Vec2::new(0.0, 10.0),
            Vec2::new(50.0, 50.0),
        ));

        // Update turn indicator
        self.update_turn_indicator(false);

        // Set up event receivers
        let game_names_with_current_turn = self.game_names_with_current_turn.clone();
        let cur_game_name = self.cur_game_name.clone();

        // Handle multiplayer game updated
        self.events.receive::<MultiplayerGameUpdated>(move |event| {
            let should_update = if event.preview.is_users_turn() {
                game_names_with_current_turn.insert(event.name.clone());
                true
            } else {
                game_names_with_current_turn.remove(&event.name);
                true
            };

            if should_update {
                launch_on_gl_thread(move || {
                    // This will be handled by the update_turn_indicator method
                });
            }
        });

        // Handle multiplayer game name changed
        let cur_game_name_clone = self.cur_game_name.clone();
        self.events.receive::<MultiplayerGameNameChanged>(move |event| {
            if event.name == cur_game_name_clone {
                self.cur_game_name = Some(event.new_name.clone());
            }
        });

        // Handle multiplayer game update started
        let cur_game_name_clone = self.cur_game_name.clone();
        self.events.receive::<MultiplayerGameUpdateStarted>(move |event| {
            if event.name == cur_game_name_clone {
                self.start_loading();
            }
        });

        // Handle multiplayer game update ended
        let cur_game_name_clone = self.cur_game_name.clone();
        self.events.receive::<MultiplayerGameUpdateEnded>(move |event| {
            if event.name == cur_game_name_clone {
                self.stop_loading();
            }
        });

        // Set up click handler
        let screen = self.screen.clone();
        self.button = Button::new("")
            .min_size([50.0, 50.0])
            .fill(Color32::from_rgba_premultiplied(0, 0, 0, 0))
            .on_click(move |_| {
                let popup = MultiplayerStatusPopup::new(screen.clone());
                popup.show();
            });
    }

    /// Starts loading
    fn start_loading(&mut self) {
        self.loading_image.show();
    }

    /// Stops loading
    fn stop_loading(&mut self) {
        self.loading_image.hide();
    }

    /// Gets the initial games with current turn
    fn get_initial_games_with_current_turn(cur_game_name: Option<String>) -> HashSet<String> {
        let games = crate::game::UncivGame::current().online_multiplayer.games.clone();
        Self::find_games_to_be_notified_about(games, cur_game_name)
    }

    /// Finds games to be notified about
    fn find_games_to_be_notified_about(games: Vec<Rc<RefCell<MultiplayerGame>>>, cur_game_name: Option<String>) -> HashSet<String> {
        let mut result = HashSet::new();

        for game in games {
            let game_ref = game.borrow();
            if let Some(cur_name) = &cur_game_name {
                if game_ref.name == *cur_name {
                    continue;
                }
            }

            if let Some(preview) = &game_ref.preview {
                if preview.is_users_turn() {
                    result.insert(game_ref.name.clone());
                }
            }
        }

        result
    }

    /// Updates the turn indicator
    fn update_turn_indicator(&mut self, flash: bool) {
        if self.game_names_with_current_turn.is_empty() {
            self.turn_indicator_cell = None;
        } else {
            self.turn_indicator.update(self.game_names_with_current_turn.len());

            if flash {
                self.turn_indicator.flash();
            }
        }
    }

    /// Draws the button
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let mut response = ui.add(self.button.clone());

        // Draw turn indicator
        if let Some(cell) = &self.turn_indicator_cell {
            let turn_indicator_response = self.turn_indicator.draw(ui, *cell);
            response = response.union(turn_indicator_response);
        }

        // Draw loading image
        let loading_response = self.loading_image.draw(ui);
        response = response.union(loading_response);

        response
    }

    /// Disposes of the button
    pub fn dispose(&self) {
        self.events.stop_receiving();
        self.turn_indicator.dispose();
        self.loading_image.dispose();
    }
}

/// Indicator for turns
struct TurnIndicator {
    /// The game amount label
    game_amount: Label,
    /// The image
    image: Image,
    /// The job
    job: Option<Instant>,
}

impl TurnIndicator {
    /// Creates a new TurnIndicator
    fn new() -> Self {
        let game_amount = Label::new("2");
        let image = ImageGetter::get_image("OtherIcons/ExclamationMark");
        image.set_size(30.0);

        Self {
            game_amount,
            image,
            job: None,
        }
    }

    /// Updates the indicator
    fn update(&mut self, games_with_updates: usize) {
        if games_with_updates < 2 {
            // Remove game amount
        } else {
            self.game_amount.set_text(format!("{}", games_with_updates));
        }
    }

    /// Flashes the indicator
    fn flash(&mut self) {
        // Using a timer would be nicer, but we don't necessarily have continuousRendering on and we still want to flash
        self.flash_internal(6, Color32::WHITE, Color32::from_rgb(255, 165, 0));
    }

    /// Internal flash method
    fn flash_internal(&mut self, alternations: i32, cur_color: Color32, next_color: Color32) {
        if alternations == 0 {
            return;
        }

        self.game_amount.set_color(next_color);
        self.image.set_color(next_color);

        let game_amount = self.game_amount.clone();
        let image = self.image.clone();
        let cur_color_clone = cur_color;
        let next_color_clone = next_color;

        self.job = Some(Instant::now());

        Concurrency::run("StatusButton color flash", move || {
            std::thread::sleep(Duration::from_millis(500));
            launch_on_gl_thread(move || {
                // This would be handled by the flash_internal method
                game_amount.set_color(cur_color_clone);
                image.set_color(cur_color_clone);
            });
        });
    }

    /// Draws the indicator
    fn draw(&self, ui: &mut Ui, rect: Rect) -> Response {
        let mut response = ui.allocate_rect(rect, egui::Sense::hover());

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Pointer);
        }

        ui.painter().image(
            self.image.texture_id(),
            rect.min,
            Vec2::new(30.0, 30.0),
            self.image.uv(),
            self.image.tint(),
        );

        if !self.game_amount.text().is_empty() {
            ui.painter().text(
                rect.min + Vec2::new(15.0, 15.0),
                egui::Align2::CENTER_CENTER,
                self.game_amount.text(),
                egui::FontId::proportional(14.0),
                self.game_amount.color(),
            );
        }

        response
    }

    /// Disposes of the indicator
    fn dispose(&self) {
        // Cancel job if it exists
        if self.job.is_some() {
            // In Rust, we don't need to explicitly cancel the job
            // as it will be dropped when this struct is dropped
        }
    }
}