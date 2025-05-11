// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/status/NextTurnProgress.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Response, Ui, Color32, Vec2, Rect};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::worldscreen::status::next_turn_button::NextTurnButton;
use crate::ui::screens::worldscreen::status::next_turn_action::NextTurnAction;
use crate::ui::images::ImageGetter;
use crate::utils::concurrency::Concurrency;
use crate::utils::launch_on_gl_thread;
use std::time::{Duration, Instant};

/// Progress bar for next turn actions
pub struct NextTurnProgress {
    /// The next turn button reference (nullable so we can free the reference once the ProgressBar is shown)
    next_turn_button: Option<Rc<RefCell<NextTurnButton>>>,
    /// The progress
    progress: i32,
    /// The maximum progress
    progress_max: i32,
    /// Whether the progress is dirty
    is_dirty: bool,
    /// The bar width
    bar_width: f32,
    /// The world screen hash
    world_screen_hash: i32,
    /// The background color
    background_color: Color32,
    /// The progress color
    progress_color: Color32,
    /// The bar height
    bar_height: f32,
    /// The bar Y position
    bar_y_pos: f32,
    /// The horizontal padding to remove
    remove_horizontal_pad: f32,
    /// The fade in duration
    fade_in_duration: f32,
    /// The alpha value
    alpha: f32,
    /// The fade in start time
    fade_in_start_time: Option<Instant>,
}

impl NextTurnProgress {
    /// The default right color
    const DEFAULT_RIGHT_COLOR: Color32 = Color32::from_rgba_premultiplied(0x60, 0x00, 0x00, 0xff);
    /// The default bar height
    const DEFAULT_BAR_HEIGHT: f32 = 4.0;
    /// The bar Y position
    const BAR_Y_POS: f32 = 1.0;
    /// The horizontal padding to remove
    const REMOVE_HORIZONTAL_PAD: f32 = 25.0;
    /// The fade in duration
    const FADE_IN_DURATION: f32 = 1.0;

    /// Creates a new NextTurnProgress
    pub fn new(next_turn_button: Option<Rc<RefCell<NextTurnButton>>>) -> Self {
        Self {
            next_turn_button,
            progress: -1,
            progress_max: 0,
            is_dirty: false,
            bar_width: 0.0,
            world_screen_hash: 0,
            background_color: Self::DEFAULT_RIGHT_COLOR,
            progress_color: Color32::from_rgb(0, 128, 0), // FOREST color
            bar_height: Self::DEFAULT_BAR_HEIGHT,
            bar_y_pos: Self::BAR_Y_POS,
            remove_horizontal_pad: Self::REMOVE_HORIZONTAL_PAD,
            fade_in_duration: Self::FADE_IN_DURATION,
            alpha: 0.0,
            fade_in_start_time: None,
        }
    }

    /// Starts the progress
    pub fn start(&mut self, world_screen: &WorldScreen) {
        self.progress = 0;
        let game = &world_screen.game_info;
        self.world_screen_hash = world_screen.hash_code();

        // Calculate progress max
        self.progress_max = 3 + // one extra step after clone and just before new worldscreen, 1 extra so it's never 100%
            if game.turns > 0 {
                // Later turns = two steps per city (startTurn and endTurn)
                // Note we ignore cities being founded or destroyed - after turn 0 that proportion
                // should be small, so the bar may clamp at max for a short while;
                // or the new WordScreen starts before it's full. Far simpler code this way.
                game.get_cities().len() * 2
            } else if game.game_parameters.is_random_number_of_civs() {
                // If we shouldn't disclose how many civs there are to Mr. Eagle Eye counting steps:
                game.game_parameters.min_number_of_civs()
            } else {
                // One step per expected city to be founded (they get an endTurn, no startTurn)
                game.civilizations.iter()
                    .filter(|civ| (civ.is_major_civ() && civ.is_ai()) || civ.is_city_state())
                    .count()
            };

        self.start_update_progress();
    }

    /// Increments the progress
    pub fn increment(&mut self) {
        self.progress += 1;
        self.start_update_progress();
    }

    /// Starts updating the progress
    fn start_update_progress(&mut self) {
        self.is_dirty = true;
        launch_on_gl_thread(move || {
            // This will be handled by the update_progress method
        });
    }

    /// Updates the progress
    fn update_progress(&mut self) {
        if !self.is_dirty {
            return;
        }
        self.is_dirty = false;

        let current_world_screen_hash = crate::game::UncivGame::current().get_world_screen_if_active()
            .map(|screen| screen.hash_code())
            .unwrap_or(-1);

        if self.progress_max == 0 || current_world_screen_hash != self.world_screen_hash {
            // Remove the progress bar
            return;
        }

        // On first update the button text is not yet updated. To stabilize geometry, do it now
        if self.progress == 0 {
            if let Some(button) = &self.next_turn_button {
                let mut button = button.borrow_mut();
                button.disable();

                let world_screen = crate::game::UncivGame::current().get_world_screen_if_active();
                if let Some(world_screen) = world_screen {
                    if world_screen.auto_play.is_auto_playing() {
                        button.update_button(NextTurnAction::AutoPlay);
                    } else {
                        button.update_button(NextTurnAction::Working);
                    }
                }

                // Calculate bar width
                let button_width = button.get_button().min_size().x;
                self.bar_width = button_width - self.remove_horizontal_pad - 20.0; // Approximate for the rounded parts

                // Set position
                let x_pos = (button_width - self.bar_width) / 2.0;
                // In Rust, we'll handle positioning in the draw method
            }
        }

        // Calculate cell width
        let cell_width = self.bar_width * (self.progress.min(self.progress_max) as f32) / (self.progress_max as f32);

        // Set size
        let cell_height = self.bar_height.max(Self::DEFAULT_BAR_HEIGHT);

        // In Rust, we'll handle drawing in the draw method

        // Start fade in if not already started
        if self.fade_in_start_time.is_none() {
            self.alpha = 0.0;
            self.fade_in_start_time = Some(Instant::now());

            // Release reference as early as possible
            self.next_turn_button = None;
        }
    }

    /// Draws the progress bar
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        // Update fade in
        if let Some(start_time) = self.fade_in_start_time {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed < self.fade_in_duration {
                self.alpha = elapsed / self.fade_in_duration;
            } else {
                self.alpha = 1.0;
                self.fade_in_start_time = None;
            }
        }

        // Calculate cell width
        let cell_width = self.bar_width * (self.progress.min(self.progress_max) as f32) / (self.progress_max as f32);

        // Create progress bar rect
        let progress_rect = Rect::from_min_size(
            Vec2::new(0.0, self.bar_y_pos),
            Vec2::new(self.bar_width, self.bar_height),
        );

        // Draw background (right part)
        let mut background_color = self.background_color;
        background_color.a = (background_color.a as f32 * self.alpha) as u8;
        ui.painter().rect_filled(
            progress_rect,
            0.0,
            background_color,
        );

        // Draw progress (left part)
        let mut progress_color = self.progress_color;
        progress_color.a = (progress_color.a as f32 * self.alpha) as u8;
        ui.painter().rect_filled(
            Rect::from_min_size(
                progress_rect.min,
                Vec2::new(cell_width, progress_rect.height()),
            ),
            0.0,
            progress_color,
        );

        // Return response
        ui.allocate_rect(progress_rect, egui::Sense::hover())
    }

    /// Gets the progress
    pub fn get_progress(&self) -> i32 {
        self.progress
    }

    /// Gets the progress max
    pub fn get_progress_max(&self) -> i32 {
        self.progress_max
    }
}