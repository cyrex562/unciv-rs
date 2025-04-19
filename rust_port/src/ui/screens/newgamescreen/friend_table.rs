use egui::{Color32, Ui, Align};
use egui_extras::Size;

use crate::logic::multiplayer::friend_list::Friend;
use crate::ui::components::widgets::WrappableLabel;
use crate::utils::constants::Constants;

/// Table that displays a friend in the UI
pub struct FriendTable {
    friend: Friend,
    width: f32,
    min_height: f32,
    inner_table: egui::Frame,
}

impl FriendTable {
    /// Create a new FriendTable
    pub fn new(friend: Friend, width: f32, min_height: f32) -> Self {
        Self {
            friend,
            width,
            min_height,
            inner_table: egui::Frame::none(),
        }
    }

    /// Initialize the table UI
    pub fn init(&mut this, ui: &mut Ui) {
        let inner_color = Color32::WHITE; // because 0xFFFFFFFF doesn't work for some reason
        let total_padding = 30.0;
        let internal_width = this.width - total_padding;

        // Create title table
        let mut title_table = egui::Frame::none();

        let title_text = &this.friend.name;
        let friend_display_name_max_width = internal_width - 70.0; // for the friend indicator with padding
        let mut friend_display_label = WrappableLabel::new(
            title_text,
            friend_display_name_max_width,
            inner_color,
            Constants::HEADING_FONT_SIZE,
        );

        if friend_display_label.preferred_width() > friend_display_name_max_width - 2.0 {
            friend_display_label.set_wrap(true);
            title_table.add(ui, friend_display_label.with_width(friend_display_name_max_width));
        } else {
            title_table.add(ui, friend_display_label.with_alignment(Align::Center).with_padding(10.0, 0.0));
        }

        this.inner_table.add(ui, title_table);
        this.inner_table.set_min_size([this.width, this.min_height - total_padding]);
    }

    /// Show the table
    pub fn show(&mut this, ui: &mut Ui) {
        this.init(ui);
        this.inner_table.show(ui);
    }
}