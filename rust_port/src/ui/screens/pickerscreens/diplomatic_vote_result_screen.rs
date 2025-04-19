// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/DiplomaticVoteResultScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea, RichText};
use crate::models::civilization::Civilization;
use crate::models::unciv_sound::UncivSound;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;
use super::picker_pane::PickerPane;

pub struct DiplomaticVoteResultScreen {
    voting_civ: Rc<RefCell<Civilization>>,
    picker_screen: PickerScreen,
}

impl DiplomaticVoteResultScreen {
    pub fn new(voting_civ: Rc<RefCell<Civilization>>) -> Self {
        let mut screen = Self {
            voting_civ,
            picker_screen: PickerScreen::new(),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        self.picker_screen.set_default_close_action();
        self.picker_screen.set_right_side_button_text(tr("Close", false));

        let voting_civ = self.voting_civ.borrow();
        let vote = voting_civ.diplomacy_functions.get_diplomatic_vote();

        if let Some(vote) = vote {
            let voted_civ = vote.voted_civ.as_ref().map(|c| c.borrow().civ_name.clone());
            let vote_text = match voted_civ {
                Some(civ_name) => tr(&format!("You voted for [{}]", civ_name), false),
                None => tr("You abstained", false),
            };

            self.picker_screen.set_description_label_text(vote_text);

            // Add vote results table
            let mut total_votes = 0;
            let mut votes_by_civ = Vec::new();

            for civ in voting_civ.diplomacy_functions.get_known_civs_sorted(false) {
                let civ_name = civ.borrow().civ_name.clone();
                let nation = civ.borrow().nation.clone();
                let votes = civ.borrow().diplomacy_functions.get_diplomatic_vote()
                    .map(|v| v.votes)
                    .unwrap_or(0);

                total_votes += votes;
                votes_by_civ.push((civ_name, nation, votes));
            }

            // Sort by votes (descending)
            votes_by_civ.sort_by(|a, b| b.2.cmp(&a.2));

            // Add vote results to the screen
            for (civ_name, nation, votes) in votes_by_civ {
                let percentage = if total_votes > 0 {
                    (votes as f32 / total_votes as f32 * 100.0) as i32
                } else {
                    0
                };

                let button = PickerPane::get_picker_option_button(
                    ImageGetter::get_nation_portrait(&nation, PickerPane::PICKER_OPTION_ICON_SIZE),
                    &format!("{}: {} votes ({}%)", civ_name, votes, percentage)
                );

                self.picker_screen.add_to_top_table(button);
            }
        } else {
            self.picker_screen.set_description_label_text(tr("No diplomatic vote is currently in progress", false));
        }

        // Set up right side button
        self.picker_screen.set_right_side_button_on_click(UncivSound::Chimes, || {
            // Close the screen
            // In Rust, we'll need to handle this differently based on the game's screen management
            // This is a placeholder for the actual implementation
            // game.pop_screen();
        });
    }

    pub fn show(&self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}