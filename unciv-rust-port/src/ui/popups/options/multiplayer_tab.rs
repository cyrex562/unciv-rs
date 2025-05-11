use std::collections::HashMap;
use std::time::Duration;
use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::{GameSettings, GameSetting};
use crate::ui::components::widgets::{Button, Checkbox, SettingsSelect, TabbedPager, UncivTextField};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::popups::{AuthPopup, Popup};
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::media_finder::IMediaFinder;
use crate::utils::multiplayer::{Multiplayer, MultiplayerAuthException, FileStorageRateLimitReached};
use crate::utils::translation::tr;

// Helper struct for refresh select options
struct RefreshSelect {
    label_text: String,
    extra_custom_server_options: Vec<SelectItem<Duration>>,
    dropbox_options: Vec<SelectItem<Duration>>,
    setting: GameSetting,
    settings: GameSettings,
    custom_server_items: Vec<SelectItem<Duration>>,
    dropbox_items: Vec<SelectItem<Duration>>,
}

impl RefreshSelect {
    fn new(
        label_text: String,
        extra_custom_server_options: Vec<SelectItem<Duration>>,
        dropbox_options: Vec<SelectItem<Duration>>,
        setting: GameSetting,
        settings: GameSettings
    ) -> Self {
        let custom_server_items = [&extra_custom_server_options[..], &dropbox_options[..]].concat();
        let dropbox_items = dropbox_options.clone();

        Self {
            label_text,
            extra_custom_server_options,
            dropbox_options,
            setting,
            settings,
            custom_server_items,
            dropbox_items,
        }
    }

    fn update(&mut self, is_custom_server: bool) {
        if is_custom_server && self.items.len() != self.custom_server_items.len() {
            self.replace_items(self.custom_server_items.clone());
        } else if !is_custom_server && self.items.len() != self.dropbox_items.len() {
            self.replace_items(self.dropbox_items.clone());
        }
    }
}

// Helper struct for select items
struct SelectItem<T> {
    label: String,
    value: T,
}

impl<T> SelectItem<T> {
    fn new(label: String, value: T) -> Self {
        Self { label, value }
    }
}

// Helper function to create refresh options
fn create_refresh_options(unit: chrono::Duration, options: &[i64]) -> Vec<SelectItem<Duration>> {
    options.iter().map(|&option| {
        let duration = match unit {
            d if d.num_seconds() == 1 => Duration::from_secs(option as u64),
            d if d.num_minutes() == 1 => Duration::from_secs(option as u64 * 60),
            _ => Duration::from_secs(option as u64),
        };
        SelectItem::new(format_duration(duration), duration)
    }).collect()
}

// Helper function to format duration
fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else {
        let minutes = seconds / 60;
        format!("{} minutes", minutes)
    }
}

// Helper function to get initial options
fn get_initial_options(
    extra_custom_server_options: &[SelectItem<Duration>],
    dropbox_options: &[SelectItem<Duration>]
) -> Vec<SelectItem<Duration>> {
    let custom_server_items = [extra_custom_server_options, dropbox_options].concat();
    let dropbox_items = dropbox_options.to_vec();

    if Multiplayer::uses_custom_server() {
        custom_server_items
    } else {
        dropbox_items
    }
}

// Helper function to fix text field URL on type
fn fix_text_field_url_on_type(text_field: &mut UncivTextField) {
    let mut text = text_field.get_text();
    let mut cursor = text_field.get_cursor_position().min(text.len());

    let text_before_cursor = &text[..cursor];

    // Replace multiple slashes with a single one, except when it's a ://
    let multiple_slashes = regex::Regex::new(r"(?<!:)/{2,}").unwrap();
    let new_text = multiple_slashes.replace_all(&text, "/").to_string();

    // Calculate updated cursor
    let new_cursor = multiple_slashes.replace_all(text_before_cursor, "/").len();

    // Update TextField
    if new_text != text {
        text_field.set_text(new_text);
        text_field.set_cursor_position(new_cursor);
    }
}

// Helper function to add select as separate table
fn add_select_as_separate_table(tab: &mut BaseScreen, settings_select: &mut SettingsSelect<Duration>) {
    let mut table = BaseScreen::new();
    settings_select.add_to(&mut table);
    tab.add(table).grow_x().fill_x().row();
}

// Helper function to add multiplayer server options
fn add_multiplayer_server_options(
    tab: &mut BaseScreen,
    options_popup: &mut OptionsPopup,
    to_update: &[&mut RefreshSelect]
) {
    let settings = &mut options_popup.settings;

    let mut connection_to_server_button = Button::new("Check connection");

    let text_to_show_for_online_multiplayer_address = if Multiplayer::uses_custom_server() {
        settings.multiplayer.server.clone()
    } else {
        "https://".to_string()
    };

    let mut multiplayer_server_text_field = UncivTextField::new("Server address", text_to_show_for_online_multiplayer_address);
    multiplayer_server_text_field.set_text_field_filter(|_, c| !" \r\n\t\\".contains(c));
    multiplayer_server_text_field.set_programmatic_change_events(true);

    let mut server_ip_table = BaseScreen::new();

    let mut server_address_label = Text::new("Server address");
    server_address_label.on_click(Box::new(move || {
        // In Rust, we would need to implement this functionality
        // multiplayer_server_text_field.set_text(Gdx::app().clipboard().contents());
    }));

    server_ip_table.add(server_address_label).colspan(2).pad_bottom(Constants::DEFAULT_FONT_SIZE as f32 / 2.0).row();

    multiplayer_server_text_field.on_change(Box::new(move || {
        fix_text_field_url_on_type(&mut multiplayer_server_text_field);
        // We can't trim on 'fix_text_field_url_on_type' for reasons
        settings.multiplayer.server = multiplayer_server_text_field.get_text().trim_end_matches('/').to_string();

        let is_custom_server = Multiplayer::uses_custom_server();
        connection_to_server_button.set_enabled(is_custom_server);

        for refresh_select in to_update {
            refresh_select.update(is_custom_server);
        }
    }));

    server_ip_table.add(multiplayer_server_text_field)
        .min_width(options_popup.get_stage_width() / 3.0)
        .pad_right(Constants::DEFAULT_FONT_SIZE as f32)
        .grow_x();

    connection_to_server_button.on_click(Box::new(move || {
        let mut popup = Popup::new(options_popup.get_stage());
        popup.add_good_sized_label("Awaiting response...").row();
        popup.open(true);

        successfully_connected_to_server(Box::new(move |connection_success, auth_success| {
            if connection_success && auth_success {
                popup.reuse_with("Success!", true);
            } else if connection_success {
                popup.close();
                let mut auth_popup = AuthPopup::new(options_popup.get_stage(), Some(Box::new(move |success| {
                    if success {
                        popup.reuse_with("Success!", true);
                    } else {
                        popup.reuse_with("Failed!", true);
                    }
                    popup.open(true);
                })));
                auth_popup.open(true);
            } else {
                popup.reuse_with("Failed!", true);
            }
        }));
    }));

    server_ip_table.add(connection_to_server_button).row();

    if UncivGame::current().online_multiplayer.multiplayer_server.feature_set.auth_version > 0 {
        let password = settings.multiplayer.passwords.get(&settings.multiplayer.server)
            .cloned()
            .unwrap_or_else(|| "Password".to_string());

        let mut password_text_field = UncivTextField::new("", password);
        let mut set_password_button = Button::new("Set password");

        server_ip_table.add(Text::new("Set password")).pad_top(16.0).colspan(2).row();
        server_ip_table.add(password_text_field).colspan(2).grow_x().pad_bottom(8.0).row();

        let mut password_status_table = BaseScreen::new();

        let status_text = if settings.multiplayer.passwords.contains_key(&settings.multiplayer.server) {
            "Your userId is password secured"
        } else {
            "Set a password to secure your userId"
        };

        password_status_table.add(Text::new(status_text));

        set_password_button.on_click(Box::new(move || {
            set_password(password_text_field.get_text(), options_popup);
        }));

        password_status_table.add(set_password_button).pad_left(16.0);

        server_ip_table.add(password_status_table).colspan(2).row();
    }

    tab.add(server_ip_table).colspan(2).fill_x().row();
}

// Helper function to add turn checker options
fn add_turn_checker_options(
    tab: &mut BaseScreen,
    options_popup: &mut OptionsPopup
) -> Option<&mut RefreshSelect> {
    let settings = &mut options_popup.settings;

    options_popup.add_checkbox(
        tab,
        "Enable out-of-game turn notifications",
        &mut settings.multiplayer.turn_checker_enabled
    );

    if !settings.multiplayer.turn_checker_enabled {
        return None;
    }

    let mut turn_checker_select = RefreshSelect::new(
        "Out-of-game, update status of all games every:".to_string(),
        create_refresh_options(chrono::Duration::seconds(30), &[30]),
        create_refresh_options(chrono::Duration::minutes(1), &[1, 2, 5, 15]),
        GameSetting::MULTIPLAYER_TURN_CHECKER_DELAY,
        settings.clone()
    );

    add_select_as_separate_table(tab, &mut turn_checker_select);

    options_popup.add_checkbox(
        tab,
        "Show persistent notification for turn notifier service",
        &mut settings.multiplayer.turn_checker_persistent_notification_enabled
    );

    Some(&mut turn_checker_select)
}

// Helper function to successfully connect to server
fn successfully_connected_to_server(action: Box<dyn Fn(bool, bool)>) {
    Concurrency::run("TestIsAlive", move || {
        let mut connection_success = false;
        let mut auth_success = false;

        match UncivGame::current().online_multiplayer.multiplayer_server.check_server_status() {
            Ok(success) => {
                connection_success = success;
                if success {
                    match UncivGame::current().online_multiplayer.multiplayer_server.authenticate(None) {
                        Ok(success) => {
                            auth_success = success;
                        },
                        Err(_) => {
                            // We ignore the exception here, because we handle the failed auth onGLThread
                        }
                    }
                }
            },
            Err(_) => {
                // Connection failed
            }
        }

        // Execute the action on the main thread
        action(connection_success, auth_success);
    });
}

// Helper function to set password
fn set_password(password: String, options_popup: &mut OptionsPopup) {
    if password.trim().is_empty() {
        return;
    }

    let mut popup = Popup::new(options_popup.get_stage());
    popup.add_good_sized_label("Awaiting response...").row();
    popup.open(true);

    if password.len() < 6 {
        popup.reuse_with("Password must be at least 6 characters long", true);
        return;
    }

    if UncivGame::current().online_multiplayer.multiplayer_server.feature_set.auth_version == 0 {
        popup.reuse_with("This server does not support authentication", true);
        return;
    }

    successfully_set_password(password, Box::new(move |success, ex| {
        if success {
            popup.reuse_with(
                format!("Password set successfully for server [{}]", options_popup.settings.multiplayer.server),
                true
            );
        } else {
            if let Some(ex) = ex.downcast_ref::<MultiplayerAuthException>() {
                let mut auth_popup = AuthPopup::new(options_popup.get_stage(), Some(Box::new(move |auth_success| {
                    // If auth was successful, try to set password again
                    if auth_success {
                        popup.close();
                        set_password(password, options_popup);
                    } else {
                        popup.reuse_with("Failed to set password!", true);
                    }
                })));
                auth_popup.open(true);
                return;
            }

            let message = if let Some(ex) = ex.downcast_ref::<FileStorageRateLimitReached>() {
                format!("Server limit reached! Please wait for [{}] seconds", ex.limit_remaining_seconds)
            } else {
                "Failed to set password!".to_string()
            };

            popup.reuse_with(message, true);
        }
    }));
}

// Helper function to successfully set password
fn successfully_set_password(password: String, action: Box<dyn Fn(bool, Option<Box<dyn std::error::Error>>)>) {
    Concurrency::run("SetPassword", move || {
        match UncivGame::current().online_multiplayer.multiplayer_server.set_password(&password) {
            Ok(success) => {
                action(success, None);
            },
            Err(ex) => {
                action(false, Some(Box::new(ex)));
            }
        }
    });
}

// Main function to create the multiplayer tab
pub fn create_multiplayer_tab(options_popup: &mut OptionsPopup) -> BaseScreen {
    let mut tab = BaseScreen::new();
    tab.pad(10.0);
    tab.defaults().pad(5.0);

    let settings = &mut options_popup.settings;

    options_popup.add_checkbox(
        &mut tab,
        "Enable multiplayer status button in singleplayer games",
        &mut settings.multiplayer.status_button_in_single_player,
        true
    );

    tab.add_separator();

    let mut cur_refresh_select = RefreshSelect::new(
        "Update status of currently played game every:".to_string(),
        create_refresh_options(chrono::Duration::seconds(1), &[3, 5]),
        create_refresh_options(chrono::Duration::seconds(1), &[10, 20, 30, 60]),
        GameSetting::MULTIPLAYER_CURRENT_GAME_REFRESH_DELAY,
        settings.clone()
    );

    add_select_as_separate_table(&mut tab, &mut cur_refresh_select);

    let mut all_refresh_select = RefreshSelect::new(
        "In-game, update status of all games every:".to_string(),
        create_refresh_options(chrono::Duration::seconds(1), &[15, 30]),
        create_refresh_options(chrono::Duration::minutes(1), &[1, 2, 5, 15]),
        GameSetting::MULTIPLAYER_ALL_GAME_REFRESH_DELAY,
        settings.clone()
    );

    add_select_as_separate_table(&mut tab, &mut all_refresh_select);

    tab.add_separator();

    // At the moment the notification service only exists on Android
    let turn_checker_select = if cfg!(target_os = "android") {
        let select = add_turn_checker_options(&mut tab, options_popup);
        tab.add_separator();
        select
    } else {
        None
    };

    let sounds = IMediaFinder::labeled_sounds().get_labeled_sounds();

    let mut current_game_sound_select = SettingsSelect::new(
        "Sound notification for when it's your turn in your currently open game:".to_string(),
        sounds.clone(),
        GameSetting::MULTIPLAYER_CURRENT_GAME_TURN_NOTIFICATION_SOUND,
        settings.clone()
    );

    add_select_as_separate_table(&mut tab, &mut current_game_sound_select);

    let mut other_game_sound_select = SettingsSelect::new(
        "Sound notification for when it's your turn in any other game:".to_string(),
        sounds,
        GameSetting::MULTIPLAYER_OTHER_GAME_TURN_NOTIFICATION_SOUND,
        settings.clone()
    );

    add_select_as_separate_table(&mut tab, &mut other_game_sound_select);

    tab.add_separator();

    let to_update = vec![&mut cur_refresh_select, &mut all_refresh_select];
    let to_update = if let Some(select) = turn_checker_select {
        let mut v = to_update;
        v.push(select);
        v
    } else {
        to_update
    };

    add_multiplayer_server_options(&mut tab, options_popup, &to_update);

    tab
}