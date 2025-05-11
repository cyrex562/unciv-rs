use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, Ui, Align, Layout};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::*;
use crate::ui::images::ImageGetter;
use crate::logic::civilization::Civilization;
use crate::utils::constants::Constants;

/// This is meant to be used for any kind of civ introduction - DiplomacyScreen,
/// AlertPopup WarDeclaration, FirstContact etc.
///
/// # Arguments
///
/// * `civ_info` - The civilization to display
/// * `hello` - Optional additional message
pub struct LeaderIntroTable {
    civ_info: Civilization,
    hello: String,
}

impl LeaderIntroTable {
    /// Creates a new LeaderIntroTable
    pub fn new(civ_info: Civilization, hello: String) -> Self {
        Self {
            civ_info,
            hello,
        }
    }

    /// Builds the leader introduction table
    ///
    /// Build either a Table(icon, leaderName <br> hello) or
    /// a Table(Portrait, Table(leaderName, icon <br> hello))
    ///
    /// City states in vanilla have leaderName=="" - but don't test CS, test leaderName to allow modding CS to have portraits
    pub fn build(&self, ui: &mut Ui) -> egui::Frame {
        let mut table = egui::Frame::none();
        table.set_padding(egui::style::Spacing::new(2.5));
        table.set_alignment(Align::CENTER);

        let nation = &self.civ_info.nation;
        let leader_portrait_file = format!("LeaderIcons/{}", nation.leader_name);
        let leader_label = self.civ_info.get_leader_display_name()
            .to_label()
            .with_font_size(Constants::HEADING_FONT_SIZE)
            .with_hide_icons(true);

        let nation_indicator = ImageGetter::get_nation_portrait(nation, 24.0);

        if !nation.leader_name.is_empty() && ImageGetter::image_exists(&leader_portrait_file) {
            // Layout with portrait
            let mut name_table = egui::Frame::none();
            name_table.add(leader_label);
            name_table.add(nation_indicator).pad(0.0, 10.0, 5.0, 0.0);

            if !self.hello.is_empty() {
                name_table.add(self.hello.to_label()).colspan(2);
            }

            table.add(ImageGetter::get_image(&leader_portrait_file))
                .with_size(100.0)
                .pad_right(10.0);

            table.add(name_table);
        } else {
            // Layout without portrait
            table.add(nation_indicator).pad(0.0, 0.0, 5.0, 10.0);
            table.add(leader_label);

            if !self.hello.is_empty() {
                table.add(self.hello.to_label()).colspan(2);
            }
        }

        table
    }
}