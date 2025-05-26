use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::option::Option;
use std::thread;
use std::io::Write;
use std::fs::File;
use uuid::Uuid;
use lazy_static::lazy_static;
use crate::game_info::GameInfo;
use log::debug;
use crate::game_settings::GameSettings;

/// Represents the Unciv app itself:
/// - implements the Game interface Gdx requires.
/// - marshals platform-specific functionality.
/// - contains references to the game being played, and high-level UI elements.
pub struct UncivGame {
    is_console_mode: bool,
    deep_linked_multiplayer_game: Option<String>,
    custom_data_directory: Option<String>,

    /// The game currently in progress
    game_info: Option<Arc<GameInfo>>,

    settings: Arc<GameSettings>,
    music_controller: Arc<MusicController>,
    online_multiplayer: Arc<Multiplayer>,
    files: Arc<UncivFiles>,

    is_tutorial_task_collapsed: bool,

    world_screen: Option<Arc<WorldScreen>>,

    /// Flag used only during initialization until the end of create
    is_initialized: bool,

    translations: Arc<Translations>,

    screen_stack: Mutex<VecDeque<Arc<dyn BaseScreen>>>,
}

// GameSettings has been moved to src/game_settings.rs

impl UncivGame {
    /// Creates a new UncivGame instance
    pub fn new(is_console_mode: bool) -> Arc<Self> {
        let game = Arc::new(Self {
            is_console_mode,
            deep_linked_multiplayer_game: None,
            custom_data_directory: None,
            game_info: None,
            settings: Arc::new(GameSettings::default()),
            music_controller: Arc::new(MusicController::new()),
            online_multiplayer: Arc::new(Multiplayer::new()),
            files: Arc::new(UncivFiles::new()),
            is_tutorial_task_collapsed: false,
            world_screen: None,
            is_initialized: false,
            translations: Arc::new(Translations::new()),
            screen_stack: Mutex::new(VecDeque::new()),
        });

        // Set the current instance
        CURRENT_GAME.lock().unwrap().replace(Arc::clone(&game));

        game
    }

    /// Initializes the game
    pub fn create(&mut self) {
        self.is_initialized = false; // This could be on reload, therefore we need to keep setting this to false

        // Set up input handling
        if let Some(input) = Gdx::input() {
            input.set_catch_key(InputKey::Back, true);
        }

        // Set up display based on platform
        if Gdx::app_type() != ApplicationType::Desktop {
            DebugUtils::set_visible_map(false);
        }

        // Initialize files
        self.files = Arc::new(UncivFiles::new_with_directory(self.custom_data_directory.clone()));

        // Clean up temporary files
        Concurrency::run(|| {
            if let Some(mods_dir) = self.files.get_local_file("mods") {
                let temp_files: Vec<_> = mods_dir.list()
                    .filter(|f| !f.is_directory() && f.name().starts_with("temp-"))
                    .collect();

                for file in temp_files {
                    let _ = file.delete();
                }
            }
        });

        // Load settings
        self.settings = self.files.get_general_settings();
        Display::set_screen_mode(self.settings.screen_mode, &self.settings);

        // Set up initial screen
        self.set_as_root_screen(Arc::new(GameStartScreen::new()));

        // Initialize music controller
        self.music_controller = Arc::new(MusicController::new_with_settings(&self.settings));
        self.install_audio_hooks();

        // Initialize multiplayer
        self.online_multiplayer = Arc::new(Multiplayer::new());

        // Check server status
        Concurrency::run(|| {
            if let Err(e) = self.online_multiplayer.multiplayer_server.check_server_status() {
                debug!("Couldn't connect to server: {}", e);
            }
        });

        // Reset and reload images
        ImageGetter::reset_atlases();
        ImageGetter::reload_images();

        let image_getter_tilesets = ImageGetter::get_available_tilesets();
        let available_tile_sets = TileSetCache::get_available_tilesets(&image_getter_tilesets);

        if !available_tile_sets.contains(&self.settings.tile_set) {
            // If the configured tileset is no longer available, default back
            self.settings.tile_set = Constants::DEFAULT_TILESET.to_string();
        }

        // Set continuous rendering
        Gdx::graphics().set_continuous_rendering(self.settings.continuous_rendering);

        // Load JSON data
        Concurrency::run("LoadJSON", || {
            RulesetCache::load_rulesets();
            self.translations.try_read_translation_for_current_language();
            self.translations.load_percentage_complete_of_languages();
            TileSetCache::load_tile_set_configs();
            SkinCache::load_skin_configs();

            let vanilla_ruleset = RulesetCache::get_vanilla_ruleset();

            // Assign permanent user ID if needed
            if self.settings.multiplayer.user_id.is_empty() {
                self.settings.multiplayer.user_id = Uuid::new_v4().to_string();
                self.settings.save();
            }

            // Initialize fonts
            Fonts::font_implementation().set_font_family(
                &self.settings.font_family_data,
                self.settings.get_font_size()
            );

            // Run GL context operations
            launch_on_gl_thread(|| {
                BaseScreen::set_skin();

                self.music_controller.choose_track(
                    &[MusicMood::Menu, MusicMood::Ambient],
                    &[MusicTrackChooserFlags::SuffixMustMatch]
                );

                ImageGetter::set_ruleset(&vanilla_ruleset);

                // Set initial screen based on settings
                if self.settings.is_freshly_created {
                    self.set_as_root_screen(Arc::new(LanguagePickerScreen::new()));
                } else if self.deep_linked_multiplayer_game.is_none() {
                    self.set_as_root_screen(Arc::new(MainMenuScreen::new()));
                } else {
                    self.try_load_deep_linked_game();
                }

                self.is_initialized = true;
            });
        });
    }

    /// Loads a game, disposing all screens.
    /// Initializes the state of all important modules.
    /// Automatically runs on the appropriate thread.
    /// Sets the returned WorldScreen as the only active screen.
    pub fn load_game(&mut self, new_game_info: Arc<GameInfo>, auto_play: Option<AutoPlay>, call_from_load_screen: bool) -> Arc<WorldScreen> {
        let prev_game_info = self.game_info.clone();
        self.game_info = Some(Arc::clone(&new_game_info));

        // Check if player is allowed to spectate
        if new_game_info.game_parameters.is_online_multiplayer
            && !new_game_info.game_parameters.anyone_can_spectate
            && !new_game_info.civilizations.iter().any(|c| c.player_id == self.settings.multiplayer.user_id) {
            panic!("You are not allowed to spectate!");
        }

        self.initialize_resources(&new_game_info);

        let is_loading_same_game = self.world_screen.is_some()
            && prev_game_info.is_some()
            && prev_game_info.as_ref().unwrap().game_id == new_game_info.game_id;

        let world_screen_restore_state = if !call_from_load_screen && is_loading_same_game {
            self.world_screen.as_ref().map(|ws| ws.get_restore_state())
        } else {
            None
        };

        let auto_play = auto_play.unwrap_or_else(|| AutoPlay::new(self.settings.auto_play));

        // Create loading screen
        let loading_screen = with_gl_context(|| {
            let loading_screen = LoadingScreen::new(self.get_screen().cloned());
            self.set_screen(Arc::clone(&loading_screen));
            loading_screen
        });

        // Load the game
        with_gl_context(|| {
            // Clear screen stack
            let mut screen_stack = self.screen_stack.lock().unwrap();
            for screen in screen_stack.iter() {
                screen.dispose();
            }
            screen_stack.clear();

            // Create new world screen
            self.world_screen = None; // Allow GC to collect old world screen
            if let Some(input) = Gdx::input() {
                input.set_input_processor(None); // Avoid ANRs while loading
            }

            let new_world_screen = Arc::new(WorldScreen::new(
                Arc::clone(&new_game_info),
                auto_play,
                new_game_info.get_player_to_view_as(),
                world_screen_restore_state
            ));

            self.world_screen = Some(Arc::clone(&new_world_screen));

            // Determine which screen to show
            let more_than_one_player = new_game_info.civilizations.iter()
                .filter(|c| c.player_type == PlayerType::Human)
                .count() > 1;

            let is_singleplayer = !new_game_info.game_parameters.is_online_multiplayer;

            let screen_to_show = if more_than_one_player && is_singleplayer {
                Arc::new(PlayerReadyScreen::new(Arc::clone(&new_world_screen))) as Arc<dyn BaseScreen>
            } else {
                Arc::clone(&new_world_screen) as Arc<dyn BaseScreen>
            };

            // Add to screen stack and set as current
            screen_stack.push_back(Arc::clone(&screen_to_show));
            self.set_screen(Arc::clone(&screen_to_show));

            // Dispose loading screen
            loading_screen.dispose();

            new_world_screen
        })
    }

    /// Initialize resources for a new game
    fn initialize_resources(&self, new_game_info: &Arc<GameInfo>) {
        with_gl_context(|| {
            ImageGetter::set_new_ruleset(&new_game_info.ruleset, true);
        });

        let full_mod_list = new_game_info.game_parameters.get_mods_and_base_ruleset();
        self.music_controller.set_mod_list(&full_mod_list);
    }

    /// Re-creates the current world screen, if there is any
    pub fn reload_world_screen(&mut self) {
        let cur_world_screen = self.world_screen.clone();
        let cur_game_info = self.game_info.clone();

        if cur_world_screen.is_none() || cur_game_info.is_none() {
            return;
        }

        self.load_game(cur_game_info.unwrap(), None, false);
    }

    /// Sets the screen (internal implementation)
    fn set_screen(&self, new_screen: Arc<dyn BaseScreen>) {
        debug!("Setting new screen: {:?}, screenStack: {:?}", new_screen, self.screen_stack);

        if let Some(input) = Gdx::input() {
            input.set_input_processor(new_screen.stage());
        }

        // Set the screen
        if let Some(screen) = new_screen.as_any().downcast_ref::<WorldScreen>() {
            screen.set_should_update(true);
        }

        Gdx::graphics().request_rendering();
    }

    /// Removes & disposes all currently active screens in the screen stack and sets the given screen as the only screen
    fn set_as_root_screen(&self, root: Arc<dyn BaseScreen>) {
        let mut screen_stack = self.screen_stack.lock().unwrap();
        for screen in screen_stack.iter() {
            screen.dispose();
        }
        screen_stack.clear();
        screen_stack.push_back(Arc::clone(&root));
        self.set_screen(root);
    }

    /// Adds a screen to be displayed instead of the current screen, with an option to go back to the previous screen by calling pop_screen
    pub fn push_screen(&self, new_screen: Arc<dyn BaseScreen>) {
        let mut screen_stack = self.screen_stack.lock().unwrap();
        screen_stack.push_back(Arc::clone(&new_screen));
        self.set_screen(new_screen);
    }

    /// Pops the currently displayed screen off the screen stack and shows the previous screen.
    /// If there is no other screen than the current, will ask the user to quit the game and return None.
    /// Automatically disposes the old screen.
    pub fn pop_screen(&self) -> Option<Arc<dyn BaseScreen>> {
        let mut screen_stack = self.screen_stack.lock().unwrap();

        if screen_stack.len() == 1 {
            self.music_controller.pause();

            if let Some(world_screen) = &self.world_screen {
                if let Some(auto_play) = &world_screen.auto_play {
                    auto_play.stop_auto_play();
                }
            }

            let confirm_popup = ConfirmPopup::new(
                Arc::clone(&screen_stack.back().unwrap()),
                "Do you want to exit the game?".to_string(),
                "Exit".to_string(),
                Box::new(|| self.music_controller.resume_from_shutdown()),
                Box::new(|| Gdx::app().exit())
            );

            confirm_popup.open(true);
            return None;
        }

        let old_screen = screen_stack.pop_back().unwrap();
        let new_screen = Arc::clone(&screen_stack.back().unwrap());

        self.set_screen(Arc::clone(&new_screen));
        new_screen.resume();
        old_screen.dispose();

        Some(new_screen)
    }

    /// Replaces the current screen with a new one. Automatically disposes the old screen.
    pub fn replace_current_screen(&self, new_screen: Arc<dyn BaseScreen>) {
        let mut screen_stack = self.screen_stack.lock().unwrap();
        let old_screen = screen_stack.pop_back().unwrap();
        screen_stack.push_back(Arc::clone(&new_screen));
        self.set_screen(new_screen);
        old_screen.dispose();
    }

    /// Resets the game to the stored world screen and automatically disposes all other screens.
    pub fn reset_to_world_screen(&self) -> Arc<WorldScreen> {
        let mut screen_stack = self.screen_stack.lock().unwrap();

        // Dispose non-world screens
        let world_screens: Vec<_> = screen_stack.iter()
            .filter(|s| s.as_any().downcast_ref::<WorldScreen>().is_some())
            .cloned()
            .collect();

        screen_stack.clear();
        for screen in world_screens {
            screen_stack.push_back(screen);
        }

        let world_screen = screen_stack.back().unwrap()
            .as_any()
            .downcast_ref::<WorldScreen>()
            .unwrap()
            .clone();

        // Re-initialize translations, images etc.
        if let Some(game_info) = &self.game_info {
            let ruleset = &game_info.ruleset;
            self.translations.set_translation_active_mods(&ruleset.mods);
            ImageGetter::set_new_ruleset(ruleset, true);
        }

        self.set_screen(Arc::clone(&world_screen));
        world_screen
    }

    /// Get all currently existing screens of the specified type
    pub fn get_screens_of_type<T: BaseScreen + 'static>(&self) -> Vec<Arc<T>> {
        let screen_stack = self.screen_stack.lock().unwrap();
        screen_stack.iter()
            .filter_map(|s| s.as_any().downcast_ref::<T>().map(|t| Arc::new(t.clone())))
            .collect()
    }

    /// Dispose and remove all currently existing screens of the specified type
    pub fn remove_screens_of_type<T: BaseScreen + 'static>(&self) {
        let mut screen_stack = self.screen_stack.lock().unwrap();
        let to_remove: Vec<_> = screen_stack.iter()
            .filter(|s| s.as_any().downcast_ref::<T>().is_some())
            .cloned()
            .collect();

        for screen in &to_remove {
            screen.dispose();
        }

        screen_stack.retain(|s| !to_remove.contains(s));
    }

    /// Try to load a deep-linked game
    fn try_load_deep_linked_game(&self) {
        if self.deep_linked_multiplayer_game.is_none() {
            return;
        }

        let deep_link = self.deep_linked_multiplayer_game.clone();

        Concurrency::run("LoadDeepLinkedGame", move || {
            if deep_link.is_none() {
                return;
            }

            let deep_link = deep_link.unwrap();

            launch_on_gl_thread(|| {
                let mut screen_stack = self.screen_stack.lock().unwrap();
                if screen_stack.is_empty() || !screen_stack.front().unwrap().as_any().is::<GameStartScreen>() {
                    self.set_as_root_screen(Arc::new(LoadingScreen::new(self.get_screen().cloned())));
                }
            });

            match self.online_multiplayer.load_game(&deep_link) {
                Ok(_) => {},
                Err(e) => {
                    launch_on_gl_thread(|| {
                        let main_menu = Arc::new(MainMenuScreen::new());
                        self.replace_current_screen(Arc::clone(&main_menu));

                        let mut popup = Popup::new(Arc::clone(&main_menu));
                        let (message, _) = LoadGameScreen::get_load_exception_message(&e);
                        popup.add_good_sized_label(&message);
                        popup.row();
                        popup.add_close_button();
                        popup.open();
                    });
                }
            }

            // Clear deep link
            self.deep_linked_multiplayer_game = None;
        });
    }

    /// Resume the game
    pub fn resume(&self) {
        if !self.is_initialized {
            return; // The stuff from Create() is still happening, so the main screen will load eventually
        }

        self.music_controller.resume_from_shutdown();

        // This is also needed in resume to open links and notifications
        // correctly when the app was already running
        self.try_load_deep_linked_game();
    }

    /// Pause the game
    pub fn pause(&self) {
        // Needs to go ASAP - on Android, there's a tiny race condition
        if self.music_controller.is_initialized() {
            self.music_controller.pause();
        }

        // Since we're pausing the game, we don't need to clone it before autosave - no one else will touch it
        if let Some(game_info) = &self.game_info {
            self.files.autosaves.request_auto_save_un_cloned(game_info);
        }
    }

    /// Resize the game
    pub fn resize(&self, width: i32, height: i32) {
        if let Some(screen) = self.get_screen() {
            screen.resize(width, height);
        }
    }

    /// Render the game
    pub fn render(&self) {
        // This would be wrapped with crash handling in the actual implementation
        // For now, we'll just call the render method on the current screen
        if let Some(screen) = self.get_screen() {
            screen.render();
        }
    }

    /// Dispose the game
    pub fn dispose(&self) {
        if let Some(input) = Gdx::input() {
            input.set_input_processor(None); // Don't allow ANRs when shutting down
        }

        SoundPlayer::clear_cache();

        if self.music_controller.is_initialized() {
            self.music_controller.graceful_shutdown(); // Do allow fade-out
        }

        // Stop multiplayer updates
        if self.online_multiplayer.is_initialized() {
            self.online_multiplayer.multiplayer_game_updater.cancel();
        }

        // Auto-save if needed
        if let Some(game_info) = &self.game_info {
            let auto_save_job = self.files.autosaves.auto_save_job.clone();

            if let Some(job) = auto_save_job {
                if job.is_active() {
                    // Auto save is already in progress, let it finish
                    Concurrency::run_blocking(|| {
                        job.join();
                    });
                } else {
                    self.files.autosaves.auto_save(game_info);
                }
            } else {
                self.files.autosaves.auto_save(game_info);
            }
        }

        // Save settings
        self.settings.save();

        // Stop thread pools
        Concurrency::stop_thread_pools();

        // Log running threads
        self.log_running_threads();

        // DO NOT `exitProcess(0)` - bypasses all Gdx and GLFW cleanup
    }

    /// Log running threads
    fn log_running_threads(&self) {
        let num_threads = thread::active_count();
        let mut thread_list = vec![thread::current(); num_threads];
        thread::enumerate(&mut thread_list);

        for thread in thread_list.iter().filter(|t| t.id() != thread::current().id() && t.name().unwrap_or("") != "DestroyJavaVM") {
            debug!("Thread {} still running in UncivGame.dispose().", thread.name().unwrap_or("unknown"));
        }
    }

    /// Handles an uncaught exception or error
    pub fn handle_uncaught_throwable(&self, ex: &dyn std::error::Error) {
        // Check if it's a cancellation exception (used by coroutines for control flow)
        if ex.is::<CancellationException>() {
            return;
        }

        Log::error("Uncaught throwable", ex);

        // Write error to file
        if let Ok(mut file) = File::create(self.files.file_writer("lasterror.txt")) {
            let _ = writeln!(file, "{:?}", ex);
        }

        // Show crash screen
        Gdx::app().post_runnable(Box::new(move || {
            if let Some(input) = Gdx::input() {
                input.set_input_processor(None); // CrashScreen needs to toJson which can take a while
            }

            self.set_as_root_screen(Arc::new(CrashScreen::new(ex)));
        }));
    }

    /// Returns the world screen if it is the currently active screen of the game
    pub fn get_world_screen_if_active(&self) -> Option<Arc<WorldScreen>> {
        if let Some(screen) = self.get_screen() {
            if let Some(world_screen) = screen.as_any().downcast_ref::<WorldScreen>() {
                if Some(Arc::new(world_screen.clone())) == self.world_screen {
                    return self.world_screen.clone();
                }
            }
        }
        None
    }

    /// Go to the main menu
    pub fn go_to_main_menu(&self) -> Arc<MainMenuScreen> {
        // Auto-save current game if needed
        if let Some(game_info) = &self.game_info {
            self.files.autosaves.request_auto_save_un_cloned(game_info);
        }

        let main_menu_screen = Arc::new(MainMenuScreen::new());
        self.push_screen(Arc::clone(&main_menu_screen));
        main_menu_screen
    }

    /// Get the current screen
    pub fn get_screen(&self) -> Option<Arc<dyn BaseScreen>> {
        let screen_stack = self.screen_stack.lock().unwrap();
        screen_stack.back().cloned()
    }

    /// Set the deep linked multiplayer game
    pub fn set_deep_linked_multiplayer_game(&mut self, game_id: Option<String>) {
        self.deep_linked_multiplayer_game = game_id;
    }

    /// Get the deep linked multiplayer game
    pub fn deep_linked_multiplayer_game(&self) -> Option<String> {
        self.deep_linked_multiplayer_game.clone()
    }

    /// Set the custom data directory
    pub fn set_custom_data_directory(&mut self, directory: Option<String>) {
        self.custom_data_directory = directory;
    }

    /// Get the custom data directory
    pub fn custom_data_directory(&self) -> Option<String> {
        self.custom_data_directory.clone()
    }

    /// Get the game info
    pub fn game_info(&self) -> Option<Arc<GameInfo>> {
        self.game_info.clone()
    }

    /// Set the game info
    pub fn set_game_info(&mut self, game_info: Option<Arc<GameInfo>>) {
        self.game_info = game_info;
    }

    /// Get the settings
    pub fn settings(&self) -> Arc<GameSettings> {
        Arc::clone(&self.settings)
    }

    /// Get the music controller
    pub fn music_controller(&self) -> Arc<MusicController> {
        Arc::clone(&self.music_controller)
    }

    /// Get the online multiplayer
    pub fn online_multiplayer(&self) -> Arc<Multiplayer> {
        Arc::clone(&self.online_multiplayer)
    }

    /// Get the files
    pub fn files(&self) -> Arc<UncivFiles> {
        Arc::clone(&self.files)
    }

    /// Get the world screen
    pub fn world_screen(&self) -> Option<Arc<WorldScreen>> {
        self.world_screen.clone()
    }

    /// Get the translations
    pub fn translations(&self) -> Arc<Translations> {
        Arc::clone(&self.translations)
    }

    /// Check if the game is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Set the initialized flag
    pub fn set_initialized(&mut self, initialized: bool) {
        self.is_initialized = initialized;
    }

    /// Install audio hooks
    pub fn install_audio_hooks(&self) {
        // Implementation would depend on platform
    }
}

/// Global reference to the one Gdx.Game instance created by the platform launchers
lazy_static! {
    static ref CURRENT_GAME: Mutex<Option<Arc<UncivGame>>> = Mutex::new(None);
}

impl UncivGame {
    /// Check if the game is currently initialized
    pub fn is_current_initialized() -> bool {
        CURRENT_GAME.lock().unwrap().is_some()
    }

    /// Get the current game instance
    pub fn current() -> Arc<UncivGame> {
        CURRENT_GAME.lock().unwrap().as_ref().expect("Game not initialized").clone()
    }

    /// Get the game info or null if not initialized
    pub fn get_game_info_or_null(&self) -> Option<GameInfo> {
        // TODO: Implement this
        None
    }
}

/// Version information
#[derive(Clone, Debug, Default)]
pub struct Version {
    pub text: String,
    pub number: i32,
}

impl Version {
    /// Create a new version
    pub fn new(text: String, number: i32) -> Self {
        Self { text, number }
    }

    /// Create a default version
    pub fn default() -> Self {
        Self { text: String::new(), number: -1 }
    }

    /// Convert to a nice string
    pub fn to_nice_string(&self) -> String {
        format!("{} (Build {})", self.text, self.number)
    }
}

/// Game start screen
pub struct GameStartScreen {
    // Implementation details
}

impl GameStartScreen {
    /// Create a new game start screen
    pub fn new() -> Self {
        Self {}
    }
}

impl BaseScreen for GameStartScreen {
    // Implementation would be provided elsewhere
}


pub struct MusicController {
    // Implementation details
}

impl MusicController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn new_with_settings(settings: &GameSettings) -> Self {
        Self {}
    }

    pub fn is_initialized(&self) -> bool {
        true
    }

    pub fn pause(&self) {}

    pub fn resume_from_shutdown(&self) {}

    pub fn graceful_shutdown(&self) {}

    pub fn choose_track(&self, suffixes: &[MusicMood], flags: &[MusicTrackChooserFlags]) {}

    pub fn set_mod_list(&self, mod_list: &[String]) {}
}

pub struct Multiplayer {
    // Implementation details
}

impl Multiplayer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_initialized(&self) -> bool {
        true
    }

    pub fn multiplayer_server(&self) -> MultiplayerServer {
        MultiplayerServer {}
    }

    pub fn multiplayer_game_updater(&self) -> MultiplayerGameUpdater {
        MultiplayerGameUpdater {}
    }

    pub fn load_game(&self, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct MultiplayerServer {
    // Implementation details
}

impl MultiplayerServer {
    pub fn check_server_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub struct MultiplayerGameUpdater {
    // Implementation details
}

impl MultiplayerGameUpdater {
    pub fn cancel(&self) {}
}

pub struct UncivFiles {
    // Implementation details
}

impl UncivFiles {
    pub fn new() -> Self {
        Self {}
    }

    pub fn new_with_directory(directory: Option<String>) -> Self {
        Self {}
    }

    pub fn get_local_file(&self, path: &str) -> Option<FileHandle> {
        None
    }

    pub fn get_general_settings(&self) -> Arc<GameSettings> {
        Arc::new(GameSettings::default())
    }

    pub fn autosaves(&self) -> Autosaves {
        Autosaves {}
    }
}

pub struct Autosaves {
    // Implementation details
}

impl Autosaves {
    pub fn request_auto_save_un_cloned(&self, game_info: &Arc<GameInfo>) {}

    pub fn auto_save(&self, game_info: &Arc<GameInfo>) {}

    pub fn auto_save_job(&self) -> Option<JoinHandle> {
        None
    }
}

pub struct JoinHandle {
    // Implementation details
}

impl JoinHandle {
    pub fn is_active(&self) -> bool {
        true
    }

    pub fn join(&self) {}
}

pub struct Translations {
    // Implementation details
}

impl Translations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn try_read_translation_for_current_language(&self) {}

    pub fn load_percentage_complete_of_languages(&self) {}

    pub fn set_translation_active_mods(&self, mods: &[String]) {}
}

pub trait BaseScreen: Send + Sync {
    fn stage(&self) -> Option<Stage>;
    fn resize(&self, width: i32, height: i32);
    fn render(&self);
    fn resume(&self);
    fn dispose(&self);
    fn as_any(&self) -> &dyn std::any::Any;
}

impl dyn BaseScreen {
    pub fn set_skin() {}
}

pub struct Stage {
    // Implementation details
}

pub struct WorldScreen {
    // Implementation details
}

impl WorldScreen {
    pub fn new(game_info: Arc<GameInfo>, auto_play: AutoPlay, player_to_view: String, restore_state: Option<WorldScreenRestoreState>) -> Self {
        Self {}
    }

    pub fn set_should_update(&self, value: bool) {}

    pub fn is_players_turn(&self) -> bool {
        true
    }

    pub fn can_change_state(&self) -> bool {
        true
    }

    pub fn map_holder(&self) -> Arc<WorldMapHolder> {
        Arc::new(WorldMapHolder {})
    }

    pub fn bottom_unit_table(&self) -> Arc<UnitTable> {
        Arc::new(UnitTable {})
    }

    pub fn viewing_civ(&self) -> Arc<Civilization> {
        Arc::new(Civilization {})
    }

    pub fn selected_civ(&self) -> Arc<Civilization> {
        Arc::new(Civilization {})
    }

    pub fn clear_undo_checkpoints(&self) {}

    pub fn get_restore_state(&self) -> WorldScreenRestoreState {
        WorldScreenRestoreState {}
    }

    pub fn auto_play(&self) -> Option<Arc<AutoPlay>> {
        None
    }

    pub fn game_info(&self) -> Arc<GameInfo> {
            Arc::new(GameInfo::new())
    }
}

pub struct WorldScreenRestoreState {
    // Implementation details
}

pub struct WorldMapHolder {
    // Implementation details
}

pub struct UnitTable {
    // Implementation details
}



pub struct PlayerReadyScreen {
    // Implementation details
}

impl PlayerReadyScreen {
    pub fn new(world_screen: Arc<WorldScreen>) -> Self {
        Self {}
    }
}

impl BaseScreen for PlayerReadyScreen {
    // Implementation would be provided elsewhere
}

pub struct LoadingScreen {
    // Implementation details
}

impl LoadingScreen {
    pub fn new(current_screen: Option<Arc<dyn BaseScreen>>) -> Self {
        Self {}
    }
}

impl BaseScreen for LoadingScreen {
    // Implementation would be provided elsewhere
}

pub struct MainMenuScreen {
    // Implementation details
}

impl MainMenuScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl BaseScreen for MainMenuScreen {
    // Implementation would be provided elsewhere
}

pub struct LanguagePickerScreen {
    // Implementation details
}

impl LanguagePickerScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl BaseScreen for LanguagePickerScreen {
    // Implementation would be provided elsewhere
}

pub struct CrashScreen {
    // Implementation details
}

impl CrashScreen {
    pub fn new(error: &dyn std::error::Error) -> Self {
        Self {}
    }
}

impl BaseScreen for CrashScreen {
    fn stage(&self) -> Option<Stage> {
        None
    }

    fn resize(&self, _width: i32, _height: i32) {
        // Empty implementation
    }

    fn render(&self) {
        // Empty implementation
    }

    fn resume(&self) {
        // Empty implementation
    }

    fn dispose(&self) {
        // Empty implementation
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct Popup {
    // Implementation details
}

impl Popup {
    pub fn new(screen: Arc<dyn BaseScreen>) -> Self {
        Self {}
    }

    pub fn add_good_sized_label(&mut self, text: &str) {}

    pub fn row(&mut self) {}

    pub fn add_close_button(&mut self) {}

    pub fn open(&self) {}
}

pub struct ConfirmPopup {
    // Implementation details
}

impl ConfirmPopup {
    pub fn new(
        screen: Arc<dyn BaseScreen>,
        question: String,
        confirm_text: String,
        restore_default: Box<dyn Fn()>,
        action: Box<dyn Fn()>
    ) -> Self {
        Self {}
    }

    pub fn open(&self, force: bool) {}
}

pub struct LoadGameScreen {
    // Implementation details
}

impl LoadGameScreen {
    pub fn get_load_exception_message(error: &dyn std::error::Error) -> (String, String) {
        (error.to_string(), String::new())
    }
}

pub struct AutoPlay {
    // Implementation details
}

impl AutoPlay {
    pub fn new(enabled: bool) -> Self {
        Self {}
    }

    pub fn stop_auto_play(&self) {}
}

pub struct FileHandle {
    // Implementation details
}

impl FileHandle {
    pub fn is_directory(&self) -> bool {
        false
    }

    pub fn name(&self) -> String {
        String::new()
    }

    pub fn delete(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn list(&self) -> Vec<FileHandle> {
        Vec::new()
    }
}

pub struct ImageGetter {
    // Implementation details
}

impl ImageGetter {
    pub fn reset_atlases() {}

    pub fn reload_images() {}

    pub fn get_available_tilesets() -> Vec<String> {
        Vec::new()
    }

    pub fn set_new_ruleset(ruleset: &Ruleset, reload: bool) {}

    pub fn set_ruleset(ruleset: &Ruleset) {}

    pub fn get_external_image(file_name: &str) -> Image {
        Image {}
    }
}

pub struct Image {
    // Implementation details
}

impl Image {
    pub fn center(&self, stage: &Stage) {}

    pub fn set_alpha(&mut self, alpha: f32) {}

    pub fn add_action(&self, action: Action) {}
}

pub struct Action {
    // Implementation details
}

pub struct Actions {
    // Implementation details
}

impl Actions {
    pub fn alpha(alpha: f32, duration: f32) -> Action {
        Action {}
    }
}

pub struct Ruleset {
    // Implementation details
}

pub struct RulesetCache {
    // Implementation details
}

impl RulesetCache {
    pub fn load_rulesets() {}

    pub fn get_vanilla_ruleset() -> Ruleset {
        Ruleset {}
    }
}

pub struct TileSetCache {
    // Implementation details
}

impl TileSetCache {
    pub fn get_available_tilesets(tilesets: &[String]) -> Vec<String> {
        Vec::new()
    }

    pub fn load_tile_set_configs() {}
}

pub struct SkinCache {
    // Implementation details
}

impl SkinCache {
    pub fn load_skin_configs() {}
}

pub struct Fonts {
    // Implementation details
}

impl Fonts {
    pub fn font_implementation() -> FontImplementation {
        FontImplementation {}
    }
}

pub struct FontImplementation {
    // Implementation details
}

impl FontImplementation {
    pub fn set_font_family(&self, font_family_data: &str, font_size: i32) {}
}

pub struct Display {
    // Implementation details
}

impl Display {
    pub fn set_screen_mode(screen_mode: i32, settings: &GameSettings) {}
}

pub struct DebugUtils {
    // Implementation details
}

impl DebugUtils {
    pub fn set_visible_map(visible: bool) {}
}

pub struct Concurrency {
    // Implementation details
}

impl Concurrency {
    pub fn run<F>(name: &str, f: F) where F: FnOnce() + Send + 'static {
        std::thread::spawn(f);
    }

    pub fn run_blocking<F>(f: F) where F: FnOnce() {
        f();
    }

    pub fn stop_thread_pools() {}
}

pub fn launch_on_gl_thread<F>(f: F) where F: FnOnce() {
    f();
}

pub fn with_gl_context<F, R>(f: F) -> R where F: FnOnce() -> R {
    f()
}

pub fn with_thread_pool_context<F, R>(f: F) -> R where F: FnOnce() -> R {
    f()
}

pub struct Gdx {
    // Implementation details
}

impl Gdx {
    pub fn input() -> Option<Input> {
        Some(Input {})
    }

    pub fn app_type() -> ApplicationType {
        ApplicationType::Desktop
    }

    pub fn graphics() -> Graphics {
        Graphics {}
    }

    pub fn app() -> Application {
        Application {}
    }
}

pub struct Input {
    // Implementation details
}

impl Input {
    pub fn set_catch_key(&self, key: InputKey, catch: bool) {}

    pub fn set_input_processor(&self, processor: Option<InputProcessor>) {}
}

pub struct InputProcessor {
    // Implementation details
}

pub enum InputKey {
    Back,
    // Other keys would be defined here
}

pub struct Graphics {
    // Implementation details
}

impl Graphics {
    pub fn set_continuous_rendering(&self, continuous: bool) {}

    pub fn request_rendering(&self) {}
}

pub struct Application {
    // Implementation details
}

impl Application {
    pub fn exit(&self) {}

    pub fn post_runnable(&self, runnable: Box<dyn FnOnce() + Send>) {}
}

pub enum ApplicationType {
    Desktop,
    // Other types would be defined here
}

pub struct SoundPlayer {
    // Implementation details
}

impl SoundPlayer {
    pub fn clear_cache() {}
}

pub struct Log {
    // Implementation details
}

impl Log {
    pub fn error(message: &str, error: &dyn std::error::Error) {}
}

pub struct CancellationException {
    // Implementation details
}

impl std::error::Error for CancellationException {}

impl std::fmt::Display for CancellationException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CancellationException")
    }
}

pub trait IsPartOfGameInfoSerialization {}

impl IsPartOfGameInfoSerialization for Version {}

pub enum MusicMood {
    Menu,
    Ambient,
    // Other moods would be defined here
}

pub enum MusicTrackChooserFlags {
    SuffixMustMatch,
    // Other flags would be defined here
}

pub enum PlayerType {
    Human,
    // Other types would be defined here
}

pub trait Is {
    fn is<T: 'static>(&self) -> bool;
}

impl<T: 'static> Is for T {
    fn is<U: 'static>(&self) -> bool {
        std::any::TypeId::of::<T>() == std::any::TypeId::of::<U>()
    }
}

pub fn debug(message: &str) {
    println!("{}", message);
}