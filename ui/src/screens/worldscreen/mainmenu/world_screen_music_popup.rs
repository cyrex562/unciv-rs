// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/mainmenu/WorldScreenMusicPopup.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{self, Ui, Response, Rect, Vec2, Color32, Grid, Button, Label, ScrollArea};
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::components::widgets::ExpanderTab;
use crate::ui::images::ImageGetter;
use crate::ui::audio::MusicController;
use crate::models::metadata::GameSettings;
use crate::constants::DEFAULT_FONT_SIZE;
use crate::utils::translation::tr;

/// Screen size dependent popup size calculation
fn calc_size(screen_size: GameSettings::ScreenSize) -> f32 {
    match screen_size {
        GameSettings::ScreenSize::Tiny => 0.95,
        GameSettings::ScreenSize::Small => 0.85,
        _ => 0.75,
    }
}

/// A popup for controlling music playback
pub struct WorldScreenMusicPopup {
    /// The world screen
    world_screen: Rc<WorldScreen>,
    /// The music controller
    music_controller: Rc<RefCell<MusicController>>,
    /// The history expander
    history_expander: Option<ExpanderTab>,
    /// The visual mods
    visual_mods: Vec<String>,
    /// The mods
    mods: Vec<String>,
    /// The track style
    track_style: TrackStyle,
}

/// Style for track buttons
struct TrackStyle {
    /// The font size
    font_size: f32,
    /// The padding
    padding: f32,
    /// The colors for different states
    colors: TrackColors,
}

/// Colors for different track button states
struct TrackColors {
    /// The normal color
    normal: Color32,
    /// The hover color
    hover: Color32,
    /// The pressed color
    pressed: Color32,
    /// The disabled color
    disabled: Color32,
}

impl WorldScreenMusicPopup {
    /// Creates a new WorldScreenMusicPopup
    pub fn new(world_screen: Rc<WorldScreen>) -> Self {
        let music_controller = world_screen.game.music_controller.clone();
        let visual_mods = world_screen.game.settings.visual_mods.clone();
        let mods = world_screen.game_info.game_parameters.mods.clone();

        let track_style = TrackStyle {
            font_size: 14.0,
            padding: 5.0,
            colors: TrackColors {
                normal: Color32::from_rgb(60, 60, 60),
                hover: Color32::from_rgb(80, 80, 80),
                pressed: Color32::from_rgb(40, 40, 40),
                disabled: Color32::from_rgb(40, 40, 40),
            },
        };

        let mut instance = Self {
            world_screen,
            music_controller,
            history_expander: None,
            visual_mods,
            mods,
            track_style,
        };

        instance.init();
        instance
    }

    /// Initializes the WorldScreenMusicPopup
    fn init(&mut self) {
        // Create history expander
        self.history_expander = Some(ExpanderTab::new(
            "—History—",
            DEFAULT_FONT_SIZE,
            None,
            false,
            0.0,
            5.0,
            "MusicPopup.History",
        ));

        // Set up music controller callback
        let history_expander = self.history_expander.as_mut().unwrap();
        self.music_controller.borrow_mut().set_on_change(Box::new(move || {
            history_expander.clear();
            history_expander.update_track_list(&self.music_controller.borrow().get_history());
        }));
    }

    /// Creates a small untranslated button
    fn create_track_button(&self, text: &str, right_side: bool) -> Button {
        let mut button = Button::new(text);
        button = button.small();

        if right_side {
            button = button.fill(self.track_style.colors.disabled);
        } else {
            button = button.fill(self.track_style.colors.normal);
        }

        button = button.min_size(Vec2::new(0.0, self.track_style.font_size * 1.5));
        button = button.padding(Vec2::new(self.track_style.padding, self.track_style.padding));

        button
    }

    /// Draws the WorldScreenMusicPopup
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                // Add music mods
                let mods_to_tracks = self.music_controller.borrow().get_all_music_file_info()
                    .into_iter()
                    .fold(std::collections::HashMap::new(), |mut acc, info| {
                        acc.entry(info.mod_name.clone())
                            .or_insert_with(Vec::new)
                            .push(info);
                        acc
                    });

                // Sort mods alphabetically
                let mut mods: Vec<_> = mods_to_tracks.keys().collect();
                mods.sort();

                for mod_name in mods {
                    let tracks = &mods_to_tracks[mod_name];
                    self.add_track_list(ui, mod_name, tracks);
                }

                // Add history
                if let Some(history_expander) = &mut self.history_expander {
                    history_expander.draw(ui);
                }

                // Add music controls
                self.add_music_controls(ui);
            })
            .response
    }

    /// Adds a track list for a mod
    fn add_track_list(&self, ui: &mut Ui, mod_name: &str, tracks: &[MusicController::MusicTrackInfo]) {
        let title = if mod_name.is_empty() { "—Default—" } else { mod_name };

        // Get icon based on mod type
        let icon = if self.mods.contains(&mod_name.to_string()) {
            Some(ImageGetter::get_image("OtherIcons/Mods"))
        } else if self.visual_mods.contains(&mod_name.to_string()) {
            Some(ImageGetter::get_image("UnitPromotionIcons/Scouting"))
        } else {
            None
        };

        // Create expander
        let mut expander = ExpanderTab::new(
            title,
            DEFAULT_FONT_SIZE,
            icon,
            false,
            0.0,
            5.0,
            &format!("MusicPopup.{}", title),
        );

        // Add tracks
        Grid::new(format!("tracks_{}", mod_name))
            .spacing([5.0, 5.0])
            .show(ui, |ui| {
                for track in tracks {
                    // Track name button
                    let track_button = self.create_track_button(&track.track, false);
                    if ui.add(track_button).clicked() {
                        self.music_controller.borrow_mut().start_track(track.clone());
                    }

                    // Track type label
                    ui.add(self.create_track_button(&track.type_name, true));
                    ui.end_row();
                }
            });
    }

    /// Adds music controls
    fn add_music_controls(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Volume slider
            ui.label("Volume:");
            let mut volume = self.music_controller.borrow().get_volume();
            if ui.add(egui::Slider::new(&mut volume, 0.0..=1.0)).changed() {
                self.music_controller.borrow_mut().set_volume(volume);
            }

            // Play/Pause button
            if ui.button(if self.music_controller.borrow().is_playing() {
                "⏸"
            } else {
                "▶"
            }).clicked() {
                self.music_controller.borrow_mut().toggle_play_pause();
            }

            // Next track button
            if ui.button("⏭").clicked() {
                self.music_controller.borrow_mut().next_track();
            }
        });
    }
}

// TODO: Implement:
// - Proper track list sorting
// - Track search functionality
// - Track favorites
// - Track queue management
// - Track progress bar
// - Track metadata display
// - Track artwork display
// - Playlist management
// - Custom theme support
// - Volume persistence
// - Keyboard shortcuts