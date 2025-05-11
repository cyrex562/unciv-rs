// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/mainmenu/WorldScreenMenuPopup.kt

use std::rc::Rc;
use egui::{self, Ui, Response, Rect, Vec2, Button, Grid};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::savescreens::LoadGameScreen;
use crate::ui::screens::victoryscreen::VictoryScreen;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::popups::Popup;
use crate::ui::components::widgets::ScrollPane;

/// The in-game menu called from the "Hamburger" button top-left
pub struct WorldScreenMenuPopup {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// Whether to show expert mode options
    expert_mode: bool,
    /// Whether to use single column layout
    single_column: bool,
    /// The current column in the grid
    current_column: i32,
}

impl WorldScreenMenuPopup {
    /// Creates a new WorldScreenMenuPopup
    pub fn new(world_screen: Rc<WorldScreen>, expert_mode: bool) -> Self {
        let mut instance = Self {
            world_screen,
            expert_mode,
            single_column: false,
            current_column: 0,
        };

        instance.init();
        instance
    }

    /// Initializes the WorldScreenMenuPopup
    fn init(&mut self) {
        // Stop auto play if active
        self.world_screen.auto_play.stop_auto_play();
    }

    /// Moves to the next column or row
    fn next_column(&mut self) {
        if !self.single_column && self.current_column == 0 {
            self.current_column = 1;
        } else {
            self.current_column = 0;
        }
    }

    /// Draws the WorldScreenMenuPopup
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let show_save = !self.world_screen.game_info.game_parameters.is_online_multiplayer;
        let show_music = self.world_screen.game.music_controller.is_music_available();
        let show_console = show_save && self.expert_mode;
        let button_count = 8 + (if show_save { 1 } else { 0 }) +
                              (if show_music { 1 } else { 0 }) +
                              (if show_console { 1 } else { 0 });

        // Calculate layout
        let available_width = ui.available_width();
        let button_height = 30.0;
        let total_height = button_height * button_count as f32;

        self.single_column = self.world_screen.is_cramped_portrait() ||
            2.0 * available_width > ui.available_width() ||
            total_height < ui.available_height();

        // Create grid layout
        Grid::new("menu_grid")
            .spacing([10.0, 5.0])
            .striped(false)
            .show(ui, |ui| {
                // Main menu button
                if ui.button("Main menu").clicked() {
                    self.world_screen.game.go_to_main_menu();
                }
                self.next_column();

                // Civilopedia button
                if ui.button("Civilopedia").clicked() {
                    self.world_screen.open_civilopedia();
                }
                self.next_column();

                // Save game button
                if show_save {
                    if ui.button("Save game").clicked() {
                        self.world_screen.open_save_game_screen();
                    }
                    self.next_column();
                }

                // Load game button
                if ui.button("Load game").clicked() {
                    self.world_screen.game.push_screen(LoadGameScreen::new());
                }
                self.next_column();

                // New game button
                if ui.button("Start new game").clicked() {
                    self.world_screen.open_new_game_screen();
                }
                self.next_column();

                // Victory status button
                if ui.button("Victory status").clicked() {
                    self.world_screen.game.push_screen(VictoryScreen::new(self.world_screen.clone()));
                }
                self.next_column();

                // Options button
                let options_response = ui.button("Options");
                if options_response.clicked() {
                    self.world_screen.open_options_popup(false);
                }
                if options_response.long_clicked() {
                    self.world_screen.open_options_popup(true);
                }
                self.next_column();

                // Music button
                if show_music {
                    if ui.button("Music").clicked() {
                        crate::ui::screens::worldscreen::mainmenu::WorldScreenMusicPopup::new(
                            self.world_screen.clone()
                        ).open(true);
                    }
                    self.next_column();
                }

                // Developer console button
                if show_console {
                    if ui.button("Developer Console").clicked() {
                        self.world_screen.open_developer_console();
                    }
                    self.next_column();
                }

                // Exit button
                if ui.add(Button::new("Exit").fill(egui::Color32::from_rgb(200, 50, 50))).clicked() {
                    std::process::exit(0);
                }
                self.next_column();
            });

        ui.separator();

        // Close button
        if ui.button("Close").clicked() {
            // TODO: Implement close functionality
        }

        ui.available_response()
    }
}

// TODO: Implement:
// - Keyboard shortcuts for all actions
// - Long press detection for options button
// - Proper popup closing behavior
// - Screen transitions and animations
// - Proper layout calculations based on screen size
// - Integration with game state management
// - Proper styling and theming