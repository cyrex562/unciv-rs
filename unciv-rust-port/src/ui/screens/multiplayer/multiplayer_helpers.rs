use std::time::{Duration, Instant};
use egui::Ui;
use clipboard::{ClipboardContext, ClipboardProvider};

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::logic::multiplayer::{Multiplayer, MultiplayerGame};
use crate::ui::components::fonts::Fonts;
use crate::ui::popups::{Popup, ToastPopup};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::savescreens::LoadGameScreen;
use crate::utils::concurrency::Concurrency;
use crate::utils::format_duration;
use crate::utils::translations::tr;

/// Helper functions for multiplayer screens
pub struct MultiplayerHelpers;

impl MultiplayerHelpers {
    /// Load a multiplayer game
    pub fn load_multiplayer_game(screen: &BaseScreen, selected_game: &MultiplayerGame) {
        let loading_game_popup = Popup::new(screen);
        loading_game_popup.add_good_sized_label("Loading latest game state...");
        loading_game_popup.open();

        Concurrency::run("JoinMultiplayerGame", move || {
            match UncivGame::current().online_multiplayer.load_game(selected_game) {
                Ok(_) => {
                    Concurrency::launch_on_gl_thread(|| {
                        loading_game_popup.close();
                        UncivGame::current().pop_screen();
                    });
                }
                Err(ex) => {
                    let (message, _) = LoadGameScreen::get_load_exception_message(&ex);
                    Concurrency::launch_on_gl_thread(move || {
                        loading_game_popup.reuse_with(&message, true);
                    });
                }
            }
        });
    }

    /// Build description text for a multiplayer game
    pub fn build_description_text(multiplayer_game: &MultiplayerGame) -> String {
        let mut description_text = String::new();

        if let Some(ex) = &multiplayer_game.error {
            let (message, _) = LoadGameScreen::get_load_exception_message(ex, "Error while refreshing:");
            description_text.push_str(&format!("{}\n", message));
        }

        let last_update = multiplayer_game.get_last_update();
        let duration = Duration::between(last_update, Instant::now());
        description_text.push_str(&format!("Last refresh: [{}] ago\n", format_duration(duration)));

        if let Some(preview) = &multiplayer_game.preview {
            if let Some(current_player) = &preview.current_player {
                let current_turn_start_time = Instant::from_epoch_millis(preview.current_turn_start_time);
                let current_player_civ = preview.get_current_player_civ();

                let player_descriptor = if current_player_civ.player_id == UncivGame::current().settings.multiplayer.user_id {
                    "You".to_string()
                } else {
                    let friend = UncivGame::current().settings.multiplayer.friend_list
                        .iter()
                        .find(|f| f.player_id == current_player_civ.player_id)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    friend
                };

                let player_text = format!("{}{} ({})", preview.current_player, " ", player_descriptor);
                let duration = Duration::between(current_turn_start_time, Instant::now());

                description_text.push_str(&format!("Current Turn: [{}] since [{}] ago\n",
                    player_text, format_duration(duration)));

                let player_civ_name = preview.civilizations
                    .iter()
                    .find(|c| c.player_id == UncivGame::current().settings.multiplayer.user_id)
                    .map(|c| c.civ_name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                description_text.push_str(&format!("{}, {}, {}{}\n",
                    player_civ_name,
                    preview.difficulty.tr(),
                    Fonts::TURN,
                    preview.turns));

                description_text.push_str(&format!("Base ruleset: {}\n", preview.game_parameters.base_ruleset));

                if !preview.game_parameters.mods.is_empty() {
                    description_text.push_str(&format!("Mods: {}\n",
                        preview.game_parameters.mods.join(", ")));
                }
            }
        }

        description_text.tr()
    }

    /// Show a warning about using Dropbox for multiplayer
    pub fn show_dropbox_warning(screen: &BaseScreen) {
        if !Multiplayer::uses_dropbox() || UncivGame::current().settings.multiplayer.hide_dropbox_warning {
            return;
        }

        let dropbox_warning = Popup::new(screen);
        dropbox_warning.add_good_sized_label(
            "You're currently using the default multiplayer server, which is based on a free Dropbox account. \
            Because a lot of people use this, it is uncertain if you'll actually be able to access it consistently. \
            Consider using a custom server instead."
        ).colspan(2).row();

        dropbox_warning.add_button("Open Documentation", || {
            let url = format!("{}{}", Constants::WIKI_URL, "Other/Multiplayer/#hosting-a-multiplayer-server");
            open::that(url).unwrap_or_else(|e| eprintln!("Failed to open URL: {}", e));
        }).colspan(2).row();

        let mut check_box = false;
        dropbox_warning.add_checkbox("Don't show again", &mut check_box);
        dropbox_warning.add_close_button(|| {
            UncivGame::current().settings.multiplayer.hide_dropbox_warning = check_box;
        });

        dropbox_warning.open();
    }

    /// Copy text to clipboard
    pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx: ClipboardContext = ClipboardProvider::new()?;
        ctx.set_contents(text.to_string())?;
        Ok(())
    }
}