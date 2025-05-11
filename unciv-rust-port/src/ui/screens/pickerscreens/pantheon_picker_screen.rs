// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PantheonPickerScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea};
use crate::models::civilization::Civilization;
use crate::models::ruleset::belief::{Belief, BeliefType};
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::utils::translation::tr;
use super::religion_picker_screen_common::ReligionPickerScreenCommon;

pub struct PantheonPickerScreen {
    choosing_civ: Rc<RefCell<Civilization>>,
    selected_pantheon: Option<Rc<Belief>>,
    picker_screen: PickerScreen,
    common: ReligionPickerScreenCommon,
}

impl PantheonPickerScreen {
    pub fn new(choosing_civ: Rc<RefCell<Civilization>>) -> Self {
        let mut screen = Self {
            choosing_civ: Rc::clone(&choosing_civ),
            selected_pantheon: None,
            picker_screen: PickerScreen::new(),
            common: ReligionPickerScreenCommon::new(Rc::clone(&choosing_civ)),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Set up the top table with padding
        self.picker_screen.set_top_table_padding(10.0);

        // Get the ruleset from the civilization
        let ruleset = self.choosing_civ.borrow().game_info.ruleset.clone();

        // Iterate through beliefs to find pantheon beliefs
        for belief in ruleset.beliefs.values() {
            if belief.belief_type != BeliefType::Pantheon {
                continue;
            }

            // Create a button for this belief
            let belief_button = self.common.get_belief_button(belief.clone(), false);

            // Check if the belief is available to the civilization
            let choosing_civ = self.choosing_civ.borrow();
            let is_available = choosing_civ.religion_manager.get_religion_with_belief(belief.clone()).is_none()
                && belief.get_matching_uniques(UniqueType::OnlyAvailable, StateForConditionals::IgnoreConditionals)
                    .iter()
                    .all(|unique| unique.conditionals_apply(&choosing_civ.state));

            if is_available {
                // Set up the button for selection
                let belief_clone = belief.clone();
                belief_button.on_click(move |_| {
                    self.selected_pantheon = Some(belief_clone.clone());
                    self.picker_screen.pick(&tr(&format!("Follow [{}]", belief_clone.name), false));
                });
            } else {
                // Disable the button if not available
                belief_button.disable(Color32::from_rgba_premultiplied(255, 0, 0, 128));
            }

            // Add the button to the top table
            self.picker_screen.add_to_top_table(belief_button);
        }

        // Set up the OK action
        self.picker_screen.set_ok_action("Choose a pantheon", move || {
            if let Some(pantheon) = &self.selected_pantheon {
                self.common.choose_beliefs(vec![pantheon.clone()], self.common.using_free_beliefs());
            }
        });
    }

    pub fn show(&self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}