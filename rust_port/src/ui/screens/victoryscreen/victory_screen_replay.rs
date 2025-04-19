// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/VictoryScreenReplay.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use egui::{Color32, Ui, Align, Response, Rect, Vec2, RichText, Slider, Button, Image};
use crate::models::civilization::Civilization;
use crate::models::UncivSound;
use crate::utils::translation::tr;
use crate::ui::components::YearTextUtil;
use crate::ui::images::ImageGetter;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::components::widgets::TabbedPager;
use crate::ui::components::widgets::TabbedPagerPageExtensions;
use crate::constants::DEFAULT_FONT_SIZE;

/// The replay screen for the victory screen
pub struct VictoryScreenReplay {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// The game info
    game_info: Rc<RefCell<crate::models::GameInfo>>,
    /// The final turn
    final_turn: i32,
    /// The replay timer
    replay_timer: Option<Instant>,
    /// The header table
    header: Vec<VictoryScreenCivGroup>,
    /// The year label
    year_label: String,
    /// The slider value
    slider_value: f32,
    /// The replay map
    replay_map: Rc<RefCell<ReplayMap>>,
    /// The play image
    play_image: Image,
    /// The pause image
    pause_image: Image,
    /// The play/pause button
    play_pause_button: Button,
    /// The is playing flag
    is_playing: bool,
}

impl VictoryScreenReplay {
    /// Creates a new VictoryScreenReplay
    pub fn new(world_screen: Rc<WorldScreen>) -> Self {
        let game_info = world_screen.game_info.clone();
        let final_turn = game_info.borrow().turns;
        let first_turn = game_info.borrow().history_start_turn;
        let stage_width = world_screen.stage.width;
        let is_portrait = world_screen.is_portrait();

        // Calculate slider width
        let max_slider_percent = if is_portrait { 0.75 } else { 0.5 };
        let slider_width = ((final_turn - first_turn) as f32 * 15.0 + 60.0)
            .min(stage_width * max_slider_percent)
            .min(stage_width - 190.0)
            .max(120.0);

        // Create replay map
        let replay_map = Rc::new(RefCell::new(ReplayMap::new(
            game_info.borrow().tile_map.clone(),
            world_screen.viewing_civ.clone(),
            stage_width - 50.0,
            stage_width - 250.0, // Empiric: `stage.height - pager.contentScroll_field.height` after init is 244.
        )));

        // Create images
        let play_image = ImageGetter::get_image("OtherIcons/ForwardArrow");
        let pause_image = ImageGetter::get_image("OtherIcons/Pause");

        // Create play/pause button
        let play_pause_button = Button::new("")
            .min_size(Vec2::new(26.0, 26.0))
            .image(pause_image.clone());

        let mut instance = Self {
            world_screen,
            game_info,
            final_turn,
            replay_timer: None,
            header: Vec::new(),
            year_label: "".to_string(),
            slider_value: first_turn as f32,
            replay_map,
            play_image,
            pause_image,
            play_pause_button,
            is_playing: false,
        };

        instance.init();
        instance
    }

    /// Initializes the VictoryScreenReplay
    fn init(&mut self) {
        let first_turn = self.game_info.borrow().history_start_turn;

        // Set up year label
        self.year_label = format!(
            "{} / {} Turn",
            YearTextUtil::to_year_text(
                self.game_info.borrow().get_year(first_turn - self.final_turn),
                self.game_info.borrow().current_player_civ.is_long_count_display()
            ),
            tr(&first_turn.to_string())
        );

        // Set up header
        let mut header_group = VictoryScreenCivGroup::new(
            Rc::new(Civilization::new()), // Dummy civ for header
            "",
            self.year_label.clone(),
            self.world_screen.viewing_civ.clone(),
            DefeatedPlayerStyle::Regular,
        );

        self.header.push(header_group);

        // Set up play/pause button
        self.play_pause_button = self.play_pause_button.clone()
            .on_click(move |_| self.toggle_pause());
    }

    /// Toggles the pause state
    fn toggle_pause(&mut self) {
        if self.replay_timer.is_none() {
            self.restart_timer();
        } else {
            self.reset_timer();
        }
    }

    /// Restarts the timer
    fn restart_timer(&mut self) {
        self.reset_timer();

        let first_turn = self.slider_value as i32;
        let next_turn = first_turn + 1;

        // Schedule timer to update every 0.1 seconds
        self.replay_timer = Some(Instant::now());
        self.is_playing = true;

        // Update UI
        self.play_pause_button = self.play_pause_button.clone()
            .image(self.pause_image.clone());
    }

    /// Resets the timer
    fn reset_timer(&mut self) {
        self.replay_timer = None;
        self.is_playing = false;

        // Update UI
        self.play_pause_button = self.play_pause_button.clone()
            .image(self.play_image.clone());
    }

    /// Handles slider change
    fn slider_changed(&mut self, value: f32) {
        self.reset_timer();
        self.update_replay_table(value as i32);
    }

    /// Updates the replay table
    fn update_replay_table(&mut self, turn: i32) {
        let year = self.game_info.borrow().get_year(turn - self.final_turn);
        let is_long_count = self.game_info.borrow().current_player_civ.is_long_count_display();

        self.year_label = format!(
            "{} / {} Turn",
            YearTextUtil::to_year_text(year, is_long_count),
            tr(&turn.to_string())
        );

        self.slider_value = turn as f32;
        self.replay_map.borrow_mut().update(turn);

        if turn == self.final_turn {
            self.reset_timer();
        }
    }

    /// Draws the VictoryScreenReplay
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let mut response = Response::default();

        // Check if timer needs to update
        if self.is_playing {
            if let Some(timer) = self.replay_timer {
                if timer.elapsed() >= Duration::from_millis(100) {
                    let next_turn = self.slider_value as i32 + 1;
                    if next_turn <= self.final_turn {
                        self.update_replay_table(next_turn);
                        self.replay_timer = Some(Instant::now());
                    } else {
                        self.reset_timer();
                    }
                }
            }
        }

        // Draw header
        let header_height = 40.0;
        let header_rect = ui.allocate_response(
            Vec2::new(ui.available_width(), header_height),
            egui::Sense::hover(),
        ).rect;

        // Draw header background
        ui.painter().rect_filled(
            header_rect,
            0.0,
            Color32::from_rgba_premultiplied(40, 40, 40, 255),
        );

        // Draw year label
        ui.painter().text(
            Vec2::new(header_rect.min.x + 80.0, header_rect.center().y),
            Align::Right,
            self.year_label.clone(),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );

        // Draw slider
        let slider_rect = Rect::from_min_size(
            Vec2::new(header_rect.min.x + 95.0, header_rect.min.y + 15.0),
            Vec2::new(header_rect.width() - 190.0, 10.0),
        );

        let slider_response = ui.add(
            Slider::new(&mut self.slider_value, self.game_info.borrow().history_start_turn as f32..=self.final_turn as f32)
                .text("")
                .on_changed(|value| self.slider_changed(value))
        );

        // Draw play/pause button
        let button_rect = Rect::from_min_size(
            Vec2::new(header_rect.max.x - 80.0, header_rect.min.y + 7.0),
            Vec2::new(26.0, 26.0),
        );

        if ui.add(Button::new("").min_size(Vec2::new(26.0, 26.0)).image(
            if self.is_playing { self.pause_image.clone() } else { self.play_image.clone() }
        )).clicked() {
            self.toggle_pause();
        }

        // Draw separator
        ui.painter().line_segment(
            [
                Vec2::new(header_rect.min.x, header_rect.max.y),
                Vec2::new(header_rect.max.x, header_rect.max.y),
            ],
            egui::Stroke::new(1.0, Color32::GRAY),
        );

        // Draw replay map
        let content_rect = Rect::from_min_size(
            Vec2::new(header_rect.min.x, header_rect.max.y + 1.0),
            Vec2::new(header_rect.width(), ui.available_height() - header_height - 1.0),
        );

        self.replay_map.borrow_mut().draw(ui, content_rect);

        response.rect = Rect::from_min_size(header_rect.min, Vec2::new(header_rect.width(), content_rect.max.y - header_rect.min.y));
        response
    }
}

impl TabbedPagerPageExtensions for VictoryScreenReplay {
    /// Called when the page is activated
    fn activated(&mut self, index: i32, caption: String, pager: &mut TabbedPager) {
        self.restart_timer();
    }

    /// Called when the page is deactivated
    fn deactivated(&mut self, index: i32, caption: String, pager: &mut TabbedPager) {
        self.reset_timer();
    }

    /// Gets the fixed content
    fn get_fixed_content(&self) -> Vec<VictoryScreenCivGroup> {
        self.header.clone()
    }
}

/// A map for replaying the game
pub struct ReplayMap {
    /// The tile map
    tile_map: Rc<RefCell<crate::models::TileMap>>,
    /// The viewing civilization
    viewing_civ: Rc<Civilization>,
    /// The width
    width: f32,
    /// The height
    height: f32,
    /// The current turn
    current_turn: i32,
}

impl ReplayMap {
    /// Creates a new ReplayMap
    pub fn new(
        tile_map: Rc<RefCell<crate::models::TileMap>>,
        viewing_civ: Rc<Civilization>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            tile_map,
            viewing_civ,
            width,
            height,
            current_turn: 0,
        }
    }

    /// Updates the replay map
    pub fn update(&mut self, turn: i32) {
        self.current_turn = turn;
        // TODO: Update the map based on the turn
    }

    /// Draws the replay map
    pub fn draw(&self, ui: &mut Ui, rect: Rect) {
        // TODO: Draw the map
        ui.painter().rect_filled(
            rect,
            0.0,
            Color32::from_rgba_premultiplied(30, 30, 30, 255),
        );

        ui.painter().text(
            rect.center(),
            Align::Center,
            format!("Replay Map (Turn {})", self.current_turn),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
}