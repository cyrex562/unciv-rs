use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::fmt;

/// Settings that apply across all games, stored in GameSettings.json
#[derive(Clone, Debug)]
pub struct GameSettings {
    /// Allows panning the map by moving the pointer to the screen edges
    pub map_auto_scroll: bool,
    /// How fast the map pans using keyboard or with map_auto_scroll and mouse
    pub map_panning_speed: f32,

    pub show_worked_tiles: bool,
    pub show_resources_and_improvements: bool,
    pub show_tile_yields: bool,
    pub show_unit_movements: bool,
    pub show_settlers_suggested_city_locations: bool,

    pub check_for_due_units: bool,
    pub check_for_due_units_cycles: bool,
    pub auto_unit_cycle: bool,
    pub small_unit_button: bool,
    pub single_tap_move: bool,
    pub long_tap_move: bool,
    pub language: String,
    pub locale: Option<String>,
    pub screen_size: ScreenSize,
    pub screen_mode: i32,
    pub tutorials_shown: HashSet<String>,
    pub tutorial_tasks_completed: HashSet<String>,

    pub sound_effects_volume: f32,
    pub city_sounds_volume: f32,
    pub music_volume: f32,
    pub voices_volume: f32,
    pub pause_between_tracks: i32,

    pub turns_between_autosaves: i32,
    pub max_autosaves_stored: i32,
    pub tile_set: String,
    pub unit_set: Option<String>,
    pub skin: String,
    pub show_tutorials: bool,
    pub auto_assign_city_production: bool,

    /// This set of construction names has two effects:
    /// * Matching constructions are no longer candidates for auto_assign_city_production
    /// * Matching constructions are offered in a separate 'Disabled' category in CityScreen
    pub disabled_auto_assign_constructions: HashSet<String>,

    pub auto_building_roads: bool,
    pub automated_workers_replace_improvements: bool,
    pub automated_units_move_on_turn_start: bool,
    pub automated_units_can_upgrade: bool,
    pub automated_units_choose_promotions: bool,
    pub cities_auto_bombard_at_end_of_turn: bool,

    pub show_minimap: bool,
    pub minimap_size: i32,    // default corresponds to 15% screen space
    pub unit_icon_opacity: f32, // default corresponds to fully opaque
    pub show_pixel_improvements: bool,
    pub continuous_rendering: bool,
    pub order_trade_offers_by_amount: bool,
    pub confirm_next_turn: bool,
    pub window_state: WindowState,
    pub is_freshly_created: bool,
    pub visual_mods: HashSet<String>,
    pub use_demographics: bool,
    pub show_zoom_buttons: bool,
    pub forbid_popup_click_behind_to_close: bool,

    pub notifications_log_max_turns: i32,

    pub show_autosaves: bool,

    pub android_cutout: bool,
    pub android_hide_system_ui: bool,

    pub multiplayer: GameSettingsMultiplayer,

    pub auto_play: GameSettingsAutoPlay,

    /// Holds EmpireOverviewScreen per-page persistable states
    pub overview: OverviewPersistableData,

    /// Orientation for mobile platforms
    pub display_orientation: ScreenOrientation,

    /// Saves the last successful new game's setup
    pub last_game_setup: Option<GameSetupInfo>,

    pub font_family_data: FontFamilyData,
    pub font_size_multiplier: f32,

    pub enable_easter_eggs: bool,

    /// Maximum zoom-out of the map - performance heavy
    pub max_world_zoom_out: f32,

    pub key_bindings: KeyboardBindings,

    /// NotificationScroll on Word Screen visibility control - mapped to NotificationsScroll.UserSetting enum
    /// Defaulting this to "" - and implement the fallback only in NotificationsScroll leads to Options popup and actual effect being in disagreement!
    pub notification_scroll: String,

    /// If on, selected notifications are drawn enlarged with wider padding
    pub enlarge_selected_notification: bool,

    /// Whether the Nation Picker shows icons only or the horizontal "civBlocks" with leader/nation name
    pub nation_picker_list_mode: NationPickerListMode,

    /// Size of automatic display of UnitSet art in Civilopedia - 0 to disable
    pub pedia_unit_art_size: f32,

    /// Don't close developer console after a successful command
    pub keep_console_open: bool,
    /// Persist the history of successful developer console commands
    pub console_command_history: Vec<String>,

    /// used to migrate from older versions of the settings
    pub version: Option<i32>,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            map_auto_scroll: false,
            map_panning_speed: 6.0,

            show_worked_tiles: false,
            show_resources_and_improvements: true,
            show_tile_yields: false,
            show_unit_movements: false,
            show_settlers_suggested_city_locations: true,

            check_for_due_units: true,
            check_for_due_units_cycles: false,
            auto_unit_cycle: true,
            small_unit_button: true,
            single_tap_move: false,
            long_tap_move: true,
            language: "English".to_string(),
            locale: None,
            screen_size: ScreenSize::Small,
            screen_mode: 0,
            tutorials_shown: HashSet::new(),
            tutorial_tasks_completed: HashSet::new(),

            sound_effects_volume: 0.5,
            city_sounds_volume: 0.5,
            music_volume: 0.5,
            voices_volume: 0.5,
            pause_between_tracks: 10,

            turns_between_autosaves: 1,
            max_autosaves_stored: 10,
            tile_set: "Default".to_string(),
            unit_set: Some("Default".to_string()),
            skin: "Default".to_string(),
            show_tutorials: true,
            auto_assign_city_production: false,

            disabled_auto_assign_constructions: HashSet::new(),

            auto_building_roads: true,
            automated_workers_replace_improvements: true,
            automated_units_move_on_turn_start: false,
            automated_units_can_upgrade: false,
            automated_units_choose_promotions: false,
            cities_auto_bombard_at_end_of_turn: false,

            show_minimap: true,
            minimap_size: 6,
            unit_icon_opacity: 1.0,
            show_pixel_improvements: true,
            continuous_rendering: false,
            order_trade_offers_by_amount: true,
            confirm_next_turn: false,
            window_state: WindowState::default(),
            is_freshly_created: false,
            visual_mods: HashSet::new(),
            use_demographics: false,
            show_zoom_buttons: false,
            forbid_popup_click_behind_to_close: false,

            notifications_log_max_turns: 5,

            show_autosaves: false,

            android_cutout: false,
            android_hide_system_ui: true,

            multiplayer: GameSettingsMultiplayer::default(),

            auto_play: GameSettingsAutoPlay::default(),

            overview: OverviewPersistableData::default(),

            display_orientation: ScreenOrientation::Landscape,

            last_game_setup: None,

            font_family_data: FontFamilyData::default(),
            font_size_multiplier: 1.0,

            enable_easter_eggs: true,

            max_world_zoom_out: 2.0,

            key_bindings: KeyboardBindings::default(),

            notification_scroll: "Default".to_string(),

            enlarge_selected_notification: true,

            nation_picker_list_mode: NationPickerListMode::List,

            pedia_unit_art_size: 0.0,

            keep_console_open: false,
            console_command_history: Vec::new(),

            version: None,
        }
    }
}

impl GameSettings {
    /// Returns whether pixel units are shown based on unit_set
    pub fn show_pixel_units(&self) -> bool {
        self.unit_set.is_some()
    }

    /// Saves the current settings
    pub fn save(&mut self) {
        // In a real implementation, this would save to a file
        // For now, we'll just update the window size
        self.refresh_window_size();
    }

    /// Refreshes the window size
    pub fn refresh_window_size(&mut self) {
        if self.is_freshly_created {
            return;
        }
        // In a real implementation, this would get the current window size
        // For now, we'll just use the default
        self.window_state = WindowState::current();
    }

    /// Adds a completed tutorial task
    pub fn add_completed_tutorial_task(&mut self, tutorial_task: String) -> bool {
        if !self.tutorial_tasks_completed.insert(tutorial_task) {
            return false;
        }
        // In a real implementation, this would update the tutorial task collapsed state
        self.save();
        true
    }

    /// Updates the locale from the language
    pub fn update_locale_from_language(&mut self) {
        self.locale = Some(get_locale_from_language(&self.language));
    }

    /// Gets the font size
    pub fn get_font_size(&self) -> i32 {
        (FontFamilyData::ORIGINAL_FONT_SIZE as f32 * self.font_size_multiplier) as i32
    }

    /// Gets the current locale
    fn get_current_locale(&self) -> String {
        self.locale.clone().unwrap_or_else(|| {
            get_locale_from_language(&self.language)
        })
    }

    /// Gets the collator from the locale
    pub fn get_collator_from_locale(&self) -> String {
        // In a real implementation, this would return a Collator
        // For now, we'll just return the locale
        self.get_current_locale()
    }

    /// Gets the current number format
    pub fn get_current_number_format(&self) -> String {
        // In a real implementation, this would return a NumberFormat
        // For now, we'll just return the language
        self.language.clone()
    }
}

/// Represents the state of a window
#[derive(Clone, Debug, Default)]
pub struct WindowState {
    pub width: i32,
    pub height: i32,
}

impl WindowState {
    /// Our choice of minimum window width
    pub const MINIMUM_WIDTH: i32 = 120;
    /// Our choice of minimum window height
    pub const MINIMUM_HEIGHT: i32 = 80;

    /// Creates a new WindowState with the current window dimensions
    pub fn current() -> Self {
        // In a real implementation, this would get the current window dimensions
        // For now, we'll just use the default
        WindowState::default()
    }

    /// Constrains the dimensions to be within the given limits
    pub fn coerce_in(&self, maximum_width: i32, maximum_height: i32) -> Self {
        if self.width >= Self::MINIMUM_WIDTH && self.width <= maximum_width &&
           self.height >= Self::MINIMUM_HEIGHT && self.height <= maximum_height {
            return self.clone();
        }

        WindowState {
            width: self.width.clamp(Self::MINIMUM_WIDTH, maximum_width),
            height: self.height.clamp(Self::MINIMUM_HEIGHT, maximum_height),
        }
    }
}

/// Represents the size of the screen
#[derive(Clone, Debug, PartialEq)]
pub enum ScreenSize {
    /// Tiny screen size
    Tiny,
    /// Small screen size
    Small,
    /// Medium screen size
    Medium,
    /// Large screen size
    Large,
    /// Huge screen size
    Huge,
}

impl Default for ScreenSize {
    fn default() -> Self {
        ScreenSize::Small
    }
}

/// Represents the mode of the nation picker list
#[derive(Clone, Debug, PartialEq)]
pub enum NationPickerListMode {
    /// Icons mode
    Icons,
    /// List mode
    List,
}

impl Default for NationPickerListMode {
    fn default() -> Self {
        NationPickerListMode::List
    }
}

/// Represents the orientation of the screen
#[derive(Clone, Debug, PartialEq)]
pub enum ScreenOrientation {
    /// Landscape orientation
    Landscape,
    /// Portrait orientation
    Portrait,
}

impl Default for ScreenOrientation {
    fn default() -> Self {
        ScreenOrientation::Landscape
    }
}

/// Represents the multiplayer settings
#[derive(Clone, Debug, Default)]
pub struct GameSettingsMultiplayer {
    pub user_id: String,
    pub passwords: HashMap<String, String>,
    pub user_name: String,
    pub server: String,
    pub friend_list: Vec<Friend>,
    pub turn_checker_enabled: bool,
    pub turn_checker_persistent_notification_enabled: bool,
    pub turn_checker_delay: Duration,
    pub status_button_in_single_player: bool,
    pub current_game_refresh_delay: Duration,
    pub all_game_refresh_delay: Duration,
    pub current_game_turn_notification_sound: String,
    pub other_game_turn_notification_sound: String,
    pub hide_dropbox_warning: bool,
}

impl GameSettingsMultiplayer {
    /// Gets the authentication header
    pub fn get_auth_header(&self) -> String {
        let server_password = self.passwords.get(&self.server).unwrap_or(&String::new());
        let pre_encoded_auth_value = format!("{}:{}", self.user_id, server_password);
        format!("Basic {}", base64::encode(pre_encoded_auth_value))
    }
}

/// Represents a friend in the friend list
#[derive(Clone, Debug)]
pub struct Friend {
    pub id: String,
    pub name: String,
}

/// Represents the auto-play settings
#[derive(Clone, Debug, Default)]
pub struct GameSettingsAutoPlay {
    pub show_auto_play_button: bool,
    pub auto_play_until_end: bool,
    pub auto_play_max_turns: i32,
    pub full_auto_play_ai: bool,
    pub auto_play_military: bool,
    pub auto_play_civilian: bool,
    pub auto_play_economy: bool,
    pub auto_play_technology: bool,
    pub auto_play_policies: bool,
    pub auto_play_religion: bool,
    pub auto_play_diplomacy: bool,
}

/// Represents the overview persistable data
#[derive(Clone, Debug, Default)]
pub struct OverviewPersistableData {
    // This would contain fields for the overview screen
}

/// Represents the game setup info
#[derive(Clone, Debug)]
pub struct GameSetupInfo {
    // This would contain fields for the game setup
}

/// Represents the font family data
#[derive(Clone, Debug, Default)]
pub struct FontFamilyData {
    /// The original font size
    pub const ORIGINAL_FONT_SIZE: i32 = 16;
    // This would contain fields for the font family
}

/// Represents the keyboard bindings
#[derive(Clone, Debug, Default)]
pub struct KeyboardBindings {
    // This would contain fields for the keyboard bindings
}

/// Gets the locale from the language
fn get_locale_from_language(language: &str) -> String {
    // In a real implementation, this would return a Locale
    // For now, we'll just return the language
    language.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_game_settings() {
        let settings = GameSettings::default();
        assert_eq!(settings.map_auto_scroll, false);
        assert_eq!(settings.map_panning_speed, 6.0);
        assert_eq!(settings.language, "English");
        assert_eq!(settings.screen_size, ScreenSize::Small);
    }

    #[test]
    fn test_show_pixel_units() {
        let mut settings = GameSettings::default();
        assert!(settings.show_pixel_units());

        settings.unit_set = None;
        assert!(!settings.show_pixel_units());
    }

    #[test]
    fn test_add_completed_tutorial_task() {
        let mut settings = GameSettings::default();
        assert!(settings.add_completed_tutorial_task("task1".to_string()));
        assert!(!settings.add_completed_tutorial_task("task1".to_string()));
        assert!(settings.tutorial_tasks_completed.contains("task1"));
    }

    #[test]
    fn test_window_state_coerce_in() {
        let window_state = WindowState { width: 100, height: 60 };
        let coerced = window_state.coerce_in(200, 100);
        assert_eq!(coerced.width, WindowState::MINIMUM_WIDTH);
        assert_eq!(coerced.height, WindowState::MINIMUM_HEIGHT);

        let window_state = WindowState { width: 150, height: 80 };
        let coerced = window_state.coerce_in(200, 100);
        assert_eq!(coerced.width, 150);
        assert_eq!(coerced.height, 80);

        let window_state = WindowState { width: 250, height: 120 };
        let coerced = window_state.coerce_in(200, 100);
        assert_eq!(coerced.width, 200);
        assert_eq!(coerced.height, 100);
    }
}