// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/DiplomaticVotePickerScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea};
use crate::models::civilization::Civilization;
use crate::models::unciv_sound::UncivSound;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;
use super::picker_pane::PickerPane;

pub struct DiplomaticVotePickerScreen {
    voting_civ: Rc<RefCell<Civilization>>,
    chosen_civ: Option<String>,
    picker_screen: PickerScreen,
}

impl DiplomaticVotePickerScreen {
    pub fn new(voting_civ: Rc<RefCell<Civilization>>) -> Self {
        let mut screen = Self {
            voting_civ,
            chosen_civ: None,
            picker_screen: PickerScreen::new(),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        self.picker_screen.set_default_close_action();
        self.picker_screen.set_right_side_button_text(tr("Choose a civ to vote for", false));
        self.picker_screen.set_description_label_text(tr("Choose who should become the world leader and win a Diplomatic Victory!", false));

        let choosable_civs = self.voting_civ.borrow().diplomacy_functions.get_known_civs_sorted(false);

        for civ in choosable_civs {
            let civ_name = civ.borrow().civ_name.clone();
            let nation = civ.borrow().nation.clone();

            let button = PickerPane::get_picker_option_button(
                ImageGetter::get_nation_portrait(&nation, PickerPane::PICKER_OPTION_ICON_SIZE),
                &civ_name
            );

            let civ_name_clone = civ_name.clone();
            button.on_click(UncivSound::Chimes, move || {
                self.chosen_civ = Some(civ_name_clone.clone());
                self.picker_screen.pick(&tr(&format!("Vote for [{}]", civ_name_clone), false));
            });

            self.picker_screen.add_to_top_table(button);
        }

        // Add abstain button
        let abstain_button = PickerPane::get_picker_option_button(
            ImageGetter::get_image("OtherIcons/Stop").with_size(PickerPane::PICKER_OPTION_ICON_SIZE, PickerPane::PICKER_OPTION_ICON_SIZE),
            "Abstain"
        );

        abstain_button.on_click(UncivSound::Chimes, || {
            self.chosen_civ = None;
            self.picker_screen.pick(&tr("Abstain", false));
        });

        self.picker_screen.add_to_top_table(abstain_button);

        // Set up right side button
        self.picker_screen.set_right_side_button_on_click(UncivSound::Chimes, || {
            self.vote_and_close();
        });
    }

    fn vote_and_close(&self) {
        if let Some(chosen_civ) = &self.chosen_civ {
            self.voting_civ.borrow_mut().diplomatic_vote_for_civ(Some(chosen_civ.clone()));
        } else {
            self.voting_civ.borrow_mut().diplomatic_vote_for_civ(None);
        }

        // Close the screen
        // In Rust, we'll need to handle this differently based on the game's screen management
        // This is a placeholder for the actual implementation
        // game.pop_screen();
    }

    pub fn show(&self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}