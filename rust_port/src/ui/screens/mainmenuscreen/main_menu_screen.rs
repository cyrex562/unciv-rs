use std::time::Duration;
use bevy::prelude::*;
use bevy::math::Vec2;
use bevy::time::Timer;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rand::Rng;

use crate::constants::VERSION;
use crate::game::UncivGame;
use crate::gui::GUI;
use crate::logic::game_info::GameInfo;
use crate::logic::game_starter::GameStarter;
use crate::logic::holiday_dates::HolidayDates;
use crate::logic::map::{MapParameters, MapShape, MapSize, MapType};
use crate::logic::map::map_generator::MapGenerator;
use crate::models::metadata::{BaseRuleset, GameSetupInfo};
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::tilesets::TileSetCache;
use crate::ui::audio::SoundPlayer;
use crate::ui::components::widgets::{Button, Image, Label, ScrollArea, Stack};
use crate::ui::components::extensions::{center, surround_with_circle, surround_with_thin_circle};
use crate::ui::components::input::{KeyShortcutDispatcherVeto, KeyboardBinding};
use crate::ui::components::tilegroups::TileGroupMap;
use crate::ui::images::ImageGetter;
use crate::ui::popups::{Popup, ToastPopup};
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::mainmenuscreen::easter_egg_rulesets::EasterEggRulesets;
use crate::ui::screens::mainmenuscreen::easter_egg_floating_art::EasterEggFloatingArt;
use crate::ui::screens::mapeditorscreen::{EditorMapHolder, MapEditorScreen};
use crate::ui::screens::modmanager::ModManagementScreen;
use crate::ui::screens::multiplayerscreens::MultiplayerScreen;
use crate::ui::screens::newgamescreen::NewGameScreen;
use crate::ui::screens::savescreens::{LoadGameScreen, QuickSave};
use crate::ui::screens::worldscreen::{BackgroundActor, WorldScreen};
use crate::ui::screens::worldscreen::mainmenu::WorldScreenMenuPopup;
use crate::utils::concurrency::Concurrency;

/// Main menu screen for the game
pub struct MainMenuScreen {
    background_stack: Stack,
    single_column: bool,
    background_map_ruleset: Ruleset,
    easter_egg_ruleset: Option<Ruleset>,
    background_map_generation_job: Option<Entity>,
    background_map_exists: bool,
    map_fade_timer: Timer,
    map_replace_timer: Timer,
}

impl MainMenuScreen {
    /// Constants for map animation
    const MAP_FADE_TIME: f32 = 1.3;
    const MAP_FIRST_FADE_TIME: f32 = 0.3;
    const MAP_REPLACE_DELAY: f32 = 20.0;

    /// Creates a new MainMenuScreen
    pub fn new(game: &mut UncivGame) -> Self {
        // Initialize sound for main menu
        SoundPlayer::initialize_for_main_menu();

        // Check if we're in a cramped portrait mode
        let single_column = Self::is_cramped_portrait();

        // Get base ruleset and set up images
        let base_ruleset = RulesetCache::get_vanilla_ruleset();
        ImageGetter::set_new_ruleset(&base_ruleset);

        // Handle easter eggs if enabled
        let mut easter_egg_ruleset = None;
        if game.settings.enable_easter_eggs {
            let holiday = HolidayDates::get_holiday_by_date();
            if let Some(holiday) = holiday {
                // Easter egg floating art will be added to the stage later
                let easter_egg_mod = EasterEggRulesets::get_today_easter_egg_ruleset();
                if let Some(easter_egg_mod) = easter_egg_mod {
                    easter_egg_ruleset = Some(RulesetCache::get_complex_ruleset(&base_ruleset, vec![easter_egg_mod]));
                }
            }
        }

        // Use easter egg ruleset if available, otherwise use base ruleset
        let background_map_ruleset = easter_egg_ruleset.clone().unwrap_or_else(|| base_ruleset);

        // Create timers for map animations
        let map_fade_timer = Timer::from_seconds(Self::MAP_FADE_TIME, false);
        let map_replace_timer = Timer::from_seconds(Self::MAP_REPLACE_DELAY, false);

        Self {
            background_stack: Stack::new(),
            single_column,
            background_map_ruleset,
            easter_egg_ruleset,
            background_map_generation_job: None,
            background_map_exists: false,
            map_fade_timer,
            map_replace_timer,
        }
    }

    /// Checks if the screen is in a cramped portrait mode
    fn is_cramped_portrait() -> bool {
        // This would be based on screen dimensions in the actual implementation
        // For now, we'll use a placeholder
        false
    }

    /// Creates a menu button with the given properties
    fn get_menu_button(
        text: &str,
        icon: &str,
        binding: KeyboardBinding,
        function: Box<dyn FnMut() + Send>,
    ) -> Button {
        let mut button = Button::new();

        // Set up the button layout
        button.set_padding(15.0, 30.0, 15.0, 30.0);

        // Add background
        button.set_background("MainMenuScreen/MenuButton");

        // Add icon
        let icon_image = Image::new(icon);
        icon_image.set_size(50.0);
        icon_image.set_padding_right(20.0);
        button.add_child(icon_image);

        // Add text
        let text_label = Label::new(text);
        text_label.set_font_size(30);
        text_label.set_alignment(egui::Align::LEFT);
        text_label.set_min_width(200.0);
        button.add_child(text_label);

        // Set up click handler
        button.on_click(move || {
            // Stop background map generation
            // function();
        });

        // Add keyboard shortcut
        button.add_keyboard_shortcut(binding);

        button
    }

    /// Starts generating the background map
    fn start_background_map_generation(&mut self, game: &mut UncivGame) {
        self.stop_background_map_generation(game);

        // Calculate map dimensions and scale
        let mut scale = 1.0;
        let mut map_width = game.screen_width / TileGroupMap::GROUP_HORIZONTAL_ADVANCE;
        let mut map_height = game.screen_height / TileGroupMap::GROUP_SIZE;

        if map_width * map_height > 3000.0 {
            scale = map_width * map_height / 3000.0;
            map_width /= scale;
            map_height /= scale;
            scale = scale.min(20.0);
        }

        // Create map parameters
        let mut map_params = MapParameters::new();
        map_params.shape = MapShape::Rectangular;
        map_params.map_size = MapSize::Small;
        map_params.map_type = MapType::Pangaea;
        map_params.temperature_intensity = 0.7;
        map_params.water_threshold = -0.1; // mainly land, gets about 30% water

        // Apply easter egg modifications if applicable
        EasterEggRulesets::modify_for_easter_egg(&mut map_params);

        // Generate the map in a background thread
        let map_generator = MapGenerator::new(&self.background_map_ruleset);
        let new_map = map_generator.generate_map(&map_params);

        // Update the UI on the main thread
        game.run_on_main_thread(move |game| {
            // Set the ruleset for the map
            ImageGetter::set_new_ruleset(&self.background_map_ruleset);

            // Create the map holder
            let mut map_holder = EditorMapHolder::new(game, new_map);
            map_holder.set_scale(scale);

            // Set initial alpha to 0 for fade-in
            map_holder.set_alpha(0.0);

            // Add to background stack
            self.background_stack.add_child(map_holder);

            // Handle fade-in animation
            if self.background_map_exists {
                // Fade in the new map and remove the old one
                map_holder.fade_in(Self::MAP_FADE_TIME);

                // Remove the old map after fade-in
                if let Some(old_map) = self.background_stack.get_child_at(1) {
                    old_map.remove_from_parent();
                }
            } else {
                // First map, just fade in
                self.background_map_exists = true;
                map_holder.fade_in(Self::MAP_FIRST_FADE_TIME);
            }

            // Schedule next map generation
            self.map_replace_timer.reset();
        });
    }

    /// Stops the background map generation
    fn stop_background_map_generation(&mut self, game: &mut UncivGame) {
        // Clear any pending actions
        self.background_stack.clear_actions();

        // Cancel the current job if it exists
        if let Some(job_entity) = self.background_map_generation_job {
            game.world.entity_mut(job_entity).despawn();
            self.background_map_generation_job = None;
        }
    }

    /// Resumes the last saved game
    fn resume_game(&self, game: &mut UncivGame) {
        if GUI::is_world_loaded() {
            let current_tile_set = GUI::get_map().current_tile_set_strings.clone();
            let current_game_setting = GUI::get_settings();

            if current_tile_set.tile_set_name != current_game_setting.tile_set ||
               current_tile_set.unit_set_name != current_game_setting.unit_set {
                // Remove world screens and reload
                game.remove_screens_of_type::<WorldScreen>();
                QuickSave::auto_load_game(game);
            } else {
                // Just reset to world screen
                GUI::reset_to_world_screen();

                // Close any open popups
                if let Some(world_screen) = GUI::get_world_screen() {
                    for popup in world_screen.get_popups() {
                        if popup.is::<WorldScreenMenuPopup>() {
                            popup.close();
                        }
                    }
                }
            }
        } else {
            // No world loaded, just load the autosave
            QuickSave::auto_load_game(game);
        }
    }

    /// Starts a new game with quick settings
    fn quickstart_new_game(&self, game: &mut UncivGame) {
        // Show working toast
        ToastPopup::new("Working...", game);

        let error_text = "Cannot start game with the default new game parameters!";

        // Run in background thread
        game.run_in_background("QuickStart", move |game| {
            let new_game: GameInfo;

            // Try to create a new game
            match GameSetupInfo::from_settings("Chieftain") {
                Ok(mut game_info) => {
                    // Ensure victory types are set
                    if game_info.game_parameters.victory_types.is_empty() {
                        let rule_set = RulesetCache::get_complex_ruleset(&game_info.game_parameters);
                        game_info.game_parameters.victory_types = rule_set.victories.keys().cloned().collect();
                    }

                    // Start the new game
                    match GameStarter::start_new_game(&game_info) {
                        Ok(game) => new_game = game,
                        Err(e) => {
                            // Handle specific exceptions
                            if let Some(message) = LoadGameScreen::get_load_exception_message(&e) {
                                game.run_on_main_thread(move |game| {
                                    ToastPopup::new(&message, game);
                                });
                            } else {
                                game.run_on_main_thread(move |game| {
                                    ToastPopup::new(error_text, game);
                                });
                            }
                            return;
                        }
                    }
                },
                Err(e) => {
                    // Handle specific exceptions
                    if let Some(message) = LoadGameScreen::get_load_exception_message(&e) {
                        game.run_on_main_thread(move |game| {
                            ToastPopup::new(&message, game);
                        });
                    } else {
                        game.run_on_main_thread(move |game| {
                            ToastPopup::new(error_text, game);
                        });
                    }
                    return;
                }
            }

            // Load the new game
            match game.load_game(&new_game) {
                Ok(_) => {},
                Err(e) => {
                    // Handle specific exceptions
                    if let Some(message) = LoadGameScreen::get_load_exception_message(&e) {
                        game.run_on_main_thread(move |game| {
                            ToastPopup::new(&message, game);
                        });
                    } else {
                        game.run_on_main_thread(move |game| {
                            ToastPopup::new(error_text, game);
                        });
                    }
                }
            }
        });
    }

    /// Gets the ruleset to use for the civilopedia
    fn get_civilopedia_ruleset(&self, game: &UncivGame) -> Ruleset {
        // Use easter egg ruleset if available
        if let Some(ref ruleset) = self.easter_egg_ruleset {
            return ruleset.clone();
        }

        // Use last game setup if available
        if let Some(ref last_game_setup) = game.settings.last_game_setup {
            return RulesetCache::get_complex_ruleset(&last_game_setup.game_parameters);
        }

        // Fall back to vanilla ruleset
        RulesetCache::get(BaseRuleset::CivVGnK.full_name.clone())
            .expect("No ruleset found")
    }

    /// Opens the civilopedia with the given link
    fn open_civilopedia(&self, game: &mut UncivGame, link: &str) {
        // Stop background map generation
        self.stop_background_map_generation(game);

        // Get the ruleset
        let ruleset = self.get_civilopedia_ruleset(game);

        // Set up translations and images
        game.translations.translation_active_mods = ruleset.mods.clone();
        ImageGetter::set_new_ruleset(&ruleset);

        // Open the civilopedia
        game.open_civilopedia(&ruleset, link);
    }
}

impl BaseScreen for MainMenuScreen {
    fn build(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        // Create the background
        let background = ImageGetter::get_background("MainMenuScreen/Background");
        let background_actor = BackgroundActor::new(background);
        self.background_stack.add_child(background_actor);

        // Add the background stack to the stage
        game.add_to_stage(&self.background_stack);
        self.background_stack.set_fill_parent(true);

        // Start background map generation if tileset is valid
        if TileSetCache::contains(&game.settings.tile_set) {
            self.start_background_map_generation(game);
        }

        // Create menu columns
        let mut column1 = egui::Frame::new(egui::style::Frame::none())
            .inner_margin(10.0)
            .fill_x(true);

        let column2 = if !self.single_column {
            egui::Frame::new(egui::style::Frame::none())
                .inner_margin(10.0)
                .fill_x(true)
        } else {
            column1.clone()
        };

        // Add menu buttons to columns
        if game.files.autosaves.autosave_exists() {
            let resume_button = Self::get_menu_button(
                "Resume",
                "OtherIcons/Resume",
                KeyboardBinding::Resume,
                Box::new(move || {
                    // Resume game logic
                })
            );
            column1.add(resume_button);
        }

        let quickstart_button = Self::get_menu_button(
            "Quickstart",
            "OtherIcons/Quickstart",
            KeyboardBinding::Quickstart,
            Box::new(move || {
                // Quickstart logic
            })
        );
        column1.add(quickstart_button);

        let new_game_button = Self::get_menu_button(
            "Start new game",
            "OtherIcons/New",
            KeyboardBinding::StartNewGame,
            Box::new(move || {
                game.push_screen(Box::new(NewGameScreen::new(game)));
            })
        );
        column1.add(new_game_button);

        let load_game_button = Self::get_menu_button(
            "Load game",
            "OtherIcons/Load",
            KeyboardBinding::MainMenuLoad,
            Box::new(move || {
                game.push_screen(Box::new(LoadGameScreen::new(game)));
            })
        );
        column1.add(load_game_button);

        let multiplayer_button = Self::get_menu_button(
            "Multiplayer",
            "OtherIcons/Multiplayer",
            KeyboardBinding::Multiplayer,
            Box::new(move || {
                game.push_screen(Box::new(MultiplayerScreen::new(game)));
            })
        );
        column2.add(multiplayer_button);

        let map_editor_button = Self::get_menu_button(
            "Map editor",
            "OtherIcons/MapEditor",
            KeyboardBinding::MapEditor,
            Box::new(move || {
                game.push_screen(Box::new(MapEditorScreen::new(game)));
            })
        );
        column2.add(map_editor_button);

        let mods_button = Self::get_menu_button(
            "Mods",
            "OtherIcons/Mods",
            KeyboardBinding::ModManager,
            Box::new(move || {
                game.push_screen(Box::new(ModManagementScreen::new(game)));
            })
        );
        column2.add(mods_button);

        let options_button = Self::get_menu_button(
            "Options",
            "OtherIcons/Options",
            KeyboardBinding::MainMenuOptions,
            Box::new(move || {
                game.open_options_popup(false);
            })
        );
        options_button.on_long_press(move || {
            game.open_options_popup(true);
        });
        column2.add(options_button);

        // Create main table
        let mut table = egui::Frame::new(egui::style::Frame::none())
            .inner_margin(10.0);

        table.add(column1);
        if !self.single_column {
            table.add(column2);
        }

        // Create scroll area
        let scroll_area = ScrollArea::new(table);
        scroll_area.set_fill_parent(true);
        game.add_to_stage(&scroll_area);

        // Center the table in the scroll area
        center(&table, &scroll_area);

        // Add global shortcuts
        game.add_global_shortcut(KeyboardBinding::QuitMainMenu, move |game| {
            if game.has_open_popups() {
                game.close_all_popups();
                return;
            }
            game.pop_screen();
        });

        // Create civilopedia button
        let mut civilopedia_button = Label::new("?");
        civilopedia_button.set_font_size(48);
        civilopedia_button.set_alignment(egui::Align::CENTER);
        civilopedia_button = surround_with_circle(civilopedia_button, 60.0, game.skin_config.base_color);
        civilopedia_button.set_position(30.0, 30.0);
        civilopedia_button.on_click(move || {
            game.open_civilopedia("");
        });
        civilopedia_button.add_keyboard_shortcut(KeyboardBinding::Civilopedia);
        civilopedia_button.add_tooltip(KeyboardBinding::Civilopedia, 30.0);
        game.add_to_stage(&civilopedia_button);

        // Create right side buttons
        let mut right_side_buttons = egui::Frame::new(egui::style::Frame::none())
            .inner_margin(10.0);

        let discord_button = Image::new("OtherIcons/Discord");
        let discord_button = surround_with_circle(discord_button, 60.0, game.skin_config.base_color);
        let discord_button = surround_with_thin_circle(discord_button, egui::Color32::WHITE);
        discord_button.on_click(move || {
            game.open_uri("https://discord.gg/bjrB4Xw");
        });
        right_side_buttons.add(discord_button);

        let github_button = Image::new("OtherIcons/Github");
        let github_button = surround_with_circle(github_button, 60.0, game.skin_config.base_color);
        let github_button = surround_with_thin_circle(github_button, egui::Color32::WHITE);
        github_button.on_click(move || {
            game.open_uri("https://github.com/yairm210/Unciv");
        });
        right_side_buttons.add(github_button);

        right_side_buttons.set_position(game.screen_width - 30.0, 30.0, egui::Align::RIGHT_BOTTOM);
        game.add_to_stage(&right_side_buttons);

        // Create version label
        let version_text = format!("Version {}", VERSION);
        let version_label = Label::new(&version_text);
        version_label.set_alignment(egui::Align::CENTER);

        let mut version_table = egui::Frame::new(egui::style::Frame::none())
            .fill(egui::Color32::from_rgba_premultiplied(50, 50, 50, 180));
        version_table.add(version_label);
        version_table.set_position(game.screen_width / 2.0, 10.0, egui::Align::BOTTOM);
        game.add_to_stage(&version_table);

        // Add easter egg floating art if applicable
        if game.settings.enable_easter_eggs {
            if let Some(holiday) = HolidayDates::get_holiday_by_date() {
                let floating_art = EasterEggFloatingArt::new(game.screen_width, game.screen_height, &holiday.name);
                game.add_to_stage(&floating_art);
            }
        }
    }

    fn update(&mut self, ctx: &mut EguiContexts, game: &mut UncivGame) {
        // Update map fade timer
        if self.map_fade_timer.tick(game.time.delta()).finished() {
            // Handle map fade completion
        }

        // Update map replace timer
        if self.map_replace_timer.tick(game.time.delta()).finished() {
            self.start_background_map_generation(game);
        }
    }

    fn get_shortcut_dispatcher_vetoer(&self) -> Option<KeyShortcutDispatcherVeto> {
        Some(KeyShortcutDispatcherVeto::create_tile_group_map_dispatcher_vetoer())
    }
}

impl RecreateOnResize for MainMenuScreen {
    fn recreate(&self, game: &mut UncivGame) -> Box<dyn BaseScreen> {
        self.stop_background_map_generation(game);
        Box::new(Self::new(game))
    }

    fn resume(&mut self, game: &mut UncivGame) {
        self.start_background_map_generation(game);
    }
}