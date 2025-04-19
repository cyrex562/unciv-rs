use egui::{Color32, Ui};
use egui_extras::Size;
use uuid::Uuid;

use crate::game::UncivGame;
use crate::logic::id_checker::IdChecker;
use crate::logic::multiplayer::friend_list::{FriendList, Friend, ErrorType};
use crate::ui::components::widgets::UncivTextField;
use crate::ui::popups::ToastPopup;
use crate::ui::screens::multiplayer::view_friends_list_screen::ViewFriendsListScreen;
use crate::ui::screens::pickerscreens::PickerScreen;
use crate::utils::translations::tr;

/// Screen for editing an existing friend in the friend list
pub struct EditFriendScreen {
    friend_name_text_field: UncivTextField,
    player_id_text_field: UncivTextField,
    paste_player_id_button: egui::Button,
    friend: Friend,
}

impl EditFriendScreen {
    /// Create a new EditFriendScreen
    pub fn new(friend: Friend) -> Self {
        Self {
            friend_name_text_field: UncivTextField::new(&friend.name),
            player_id_text_field: UncivTextField::new(&friend.player_id),
            paste_player_id_button: egui::Button::new("Paste player ID from clipboard"),
            friend,
        }
    }
}

impl PickerScreen for EditFriendScreen {
    fn init(&mut this, ui: &mut Ui) {
        // Add friend name field
        ui.heading("Friend name");
        ui.add_space(10.0);
        this.friend_name_text_field.show(ui);
        ui.add_space(30.0);

        // Add player ID field with paste button
        ui.heading("Player ID");
        ui.horizontal(|ui| {
            this.player_id_text_field.show(ui);
            if ui.button("Paste player ID from clipboard").clicked() {
                if let Ok(clipboard_text) = crate::ui::screens::multiplayer::multiplayer_helpers::MultiplayerHelpers::copy_to_clipboard("") {
                    this.player_id_text_field.set_text(&clipboard_text);
                }
            }
        });
        ui.add_space(30.0);

        // Set up close button
        this.set_close_button_text("Back".tr());
        this.set_close_button_action(|| {
            UncivGame::current().pop_screen();
        });

        // Set up right side button
        this.set_right_side_button_text("Edit friend".tr());
        this.set_right_side_button_enabled(true);
        this.set_right_side_button_action(|| {
            let friend_name = this.friend_name_text_field.text();
            let player_id = this.player_id_text_field.text();

            // Validate player ID
            match Uuid::parse_str(&IdChecker::check_and_return_player_uuid(&player_id)) {
                Ok(_) => {
                    let mut friend_list = FriendList::new();

                    match friend_list.edit(&this.friend.name, friend_name, player_id) {
                        ErrorType::Name => {
                            ToastPopup::new("Friend name is already in your friends list!", this);
                            return;
                        }
                        ErrorType::Id => {
                            ToastPopup::new("Player ID is already in your friends list!", this);
                            return;
                        }
                        ErrorType::NoName => {
                            ToastPopup::new("You have to write a name for your friend!", this);
                            return;
                        }
                        ErrorType::NoId => {
                            ToastPopup::new("You have to write an ID for your friend!", this);
                            return;
                        }
                        ErrorType::Yourself => {
                            ToastPopup::new("You cannot add your own player ID in your friend list!", this);
                            return;
                        }
                        _ => {
                            // Success
                            let new_screen = UncivGame::current().pop_screen();
                            if let Some(view_friends_list_screen) = new_screen.downcast_ref::<ViewFriendsListScreen>() {
                                view_friends_list_screen.refresh_friends_list();
                            }
                        }
                    }
                }
                Err(_) => {
                    ToastPopup::new("Player ID is incorrect", this);
                    return;
                }
            }
        });
    }

    fn show(&mut this, ui: &mut Ui) {
        this.init(ui);
    }
}