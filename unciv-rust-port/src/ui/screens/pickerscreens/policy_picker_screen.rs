// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PolicyPickerScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea, RichText};
use crate::models::civilization::Civilization;
use crate::models::ruleset::policy::{Policy, PolicyBranch, PolicyBranchType};
use crate::models::tutorial_trigger::TutorialTrigger;
use crate::models::unciv_sound::UncivSound;
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::ui::images::ImageGetter;
use crate::ui::popups::confirm_popup::ConfirmPopup;
use crate::utils::translation::tr;
use super::picker_pane::PickerPane;

// Define policy colors
#[derive(Clone, Copy)]
pub enum PolicyColors {
    ButtonBGPickable,
    ButtonBGPickableSelected,
    ButtonBGNotPickable,
    ButtonBGNotPickableSelected,
    ButtonBGAdopted,
    ButtonBGAdoptedSelected,
    ButtonIconPickable,
    ButtonIconPickableSelected,
    ButtonIconNotPickable,
    ButtonIconNotPickableSelected,
    ButtonIconAdopted,
    ButtonIconAdoptedSelected,
    BranchBGCompleted,
    BranchBGNotAdopted,
    BranchHeaderBG,
    BranchLabelAdopted,
    BranchLabelPickable,
    BranchLabelNotPickable,
}

impl PolicyColors {
    pub fn color(&self) -> Color32 {
        match self {
            PolicyColors::ButtonBGPickable => Color32::from_rgb(32, 46, 64),
            PolicyColors::ButtonBGPickableSelected => Color32::from_rgb(1, 17, 19),
            PolicyColors::ButtonBGNotPickable => Color32::from_rgb(5, 45, 65),
            PolicyColors::ButtonBGNotPickableSelected => Color32::from_rgb(1, 17, 19),
            PolicyColors::ButtonBGAdopted => Color32::from_rgb(1, 17, 19),
            PolicyColors::ButtonBGAdoptedSelected => Color32::from_rgb(1, 17, 19),
            PolicyColors::ButtonIconPickable => Color32::WHITE,
            PolicyColors::ButtonIconPickableSelected => Color32::WHITE,
            PolicyColors::ButtonIconNotPickable => Color32::from_rgba_premultiplied(255, 255, 255, 51),
            PolicyColors::ButtonIconNotPickableSelected => Color32::from_rgba_premultiplied(255, 255, 255, 51),
            PolicyColors::ButtonIconAdopted => Color32::GOLD,
            PolicyColors::ButtonIconAdoptedSelected => Color32::GOLD,
            PolicyColors::BranchBGCompleted => Color32::from_rgb(255, 205, 0),
            PolicyColors::BranchBGNotAdopted => Color32::from_rgb(5, 45, 65),
            PolicyColors::BranchHeaderBG => Color32::from_rgb(5, 45, 65),
            PolicyColors::BranchLabelAdopted => Color32::from_rgb(150, 70, 40),
            PolicyColors::BranchLabelPickable => Color32::WHITE,
            PolicyColors::BranchLabelNotPickable => Color32::from_rgba_premultiplied(255, 255, 255, 127),
        }
    }
}

pub struct PolicyPickerScreen {
    viewing_civ: Rc<RefCell<Civilization>>,
    can_change_state: bool,
    select: Option<String>,
    picker_screen: PickerScreen,
    policy_name_to_button: HashMap<String, PolicyButton>,
    selected_policy_button: Option<PolicyButton>,
}

impl PolicyPickerScreen {
    pub fn new(
        viewing_civ: Rc<RefCell<Civilization>>,
        can_change_state: bool,
        select: Option<String>,
    ) -> Self {
        let mut screen = Self {
            viewing_civ,
            can_change_state,
            select,
            picker_screen: PickerScreen::new(false),
            policy_name_to_button: HashMap::new(),
            selected_policy_button: None,
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Set up the screen
        self.picker_screen.set_default_close_action();

        // Set up the right side button text based on policy state
        let viewing_civ = self.viewing_civ.borrow();
        let policies = &viewing_civ.policies;

        let button_text = if policies.all_policies_adopted(false) {
            "All policies adopted"
        } else if policies.free_policies > 0 {
            "Adopt free policy"
        } else {
            &format!("{Adopt policy}\n({}/{})",
                policies.stored_culture,
                policies.get_culture_needed_for_next_policy())
        };

        self.picker_screen.set_right_side_button_text(&tr(button_text, false));

        // Set up the right side button action
        self.picker_screen.set_right_side_button_on_click(UncivSound::Policy, || {
            // In Rust, we'll need to handle this differently
            // This is a placeholder for the actual implementation
        });

        // Disable the right side button if can't change state
        if !self.can_change_state {
            self.picker_screen.set_right_side_button_enabled(false);
        }

        // Create policy branches
        self.create_policy_branches();

        // Handle selection if provided
        if let Some(select) = &self.select {
            // In Rust, we'll need to handle this differently
            // This is a placeholder for the actual implementation
        }
    }

    fn create_policy_branches(&mut self) {
        // This is a placeholder for the actual implementation
        // The original Kotlin code is quite complex and would need to be adapted to Rust
    }

    fn pick_policy(&mut self, button: PolicyButton) {
        // This is a placeholder for the actual implementation
        // The original Kotlin code is quite complex and would need to be adapted to Rust
    }

    fn confirm_action(&self) {
        // This is a placeholder for the actual implementation
        // The original Kotlin code is quite complex and would would need to be adapted to Rust
    }

    pub fn show(&self, ui: &mut Ui) {
        self.picker_screen.show(ui);
    }
}

impl RecreateOnResize for PolicyPickerScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        // This is a placeholder for the actual implementation
        // The original Kotlin code is quite complex and would need to be adapted to Rust
        Box::new(PolicyPickerScreen::new(
            Rc::clone(&self.viewing_civ),
            self.can_change_state,
            self.selected_policy_button.as_ref().map(|b| b.policy.name.clone()),
        ))
    }
}

// Define the PolicyButton struct
pub struct PolicyButton {
    pub policy: Rc<Policy>,
    is_pickable: bool,
    is_adopted: bool,
    is_selected: bool,
    icon: Image,
    bg_color: Color32,
}

impl PolicyButton {
    pub fn new(
        viewing_civ: Rc<RefCell<Civilization>>,
        can_change_state: bool,
        policy: Rc<Policy>,
        size: f32,
    ) -> Self {
        let viewing_civ_ref = viewing_civ.borrow();
        let is_pickable = policy.is_pickable(&viewing_civ_ref, can_change_state);
        let is_adopted = viewing_civ_ref.policies.is_adopted(&policy.name);

        let mut button = Self {
            policy,
            is_pickable,
            is_adopted,
            is_selected: false,
            icon: ImageGetter::get_image(&format!("PolicyIcons/{}", policy.name)),
            bg_color: Color32::TRANSPARENT,
        };

        button.init(size);
        button
    }

    fn init(&mut self, size: f32) {
        // Set up the button
        self.icon.set_size(size * 0.7, size * 0.7);
        self.update_state();
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        self.update_state();
    }

    fn update_state(&mut self) {
        // Update button appearance based on state
        let (bg_color, icon_color) = if self.is_selected && self.is_pickable {
            (PolicyColors::ButtonBGPickableSelected, PolicyColors::ButtonIconPickableSelected)
        } else if self.is_pickable {
            (PolicyColors::ButtonBGPickable, PolicyColors::ButtonIconPickable)
        } else if self.is_selected && self.is_adopted {
            (PolicyColors::ButtonBGAdoptedSelected, PolicyColors::ButtonIconAdoptedSelected)
        } else if self.is_adopted {
            (PolicyColors::ButtonBGAdopted, PolicyColors::ButtonIconAdopted)
        } else if self.is_selected {
            (PolicyColors::ButtonBGNotPickableSelected, PolicyColors::ButtonIconNotPickableSelected)
        } else {
            (PolicyColors::ButtonBGNotPickable, PolicyColors::ButtonIconNotPickable)
        };

        self.bg_color = bg_color.color();
        self.icon.set_color(icon_color.color());
    }
}

// Helper function to check if a policy is pickable
fn is_policy_pickable(policy: &Policy, viewing_civ: &Civilization, can_change_state: bool) -> bool {
    viewing_civ.is_current_player()
        && can_change_state
        && !viewing_civ.is_defeated()
        && !viewing_civ.policies.is_adopted(&policy.name)
        && policy.policy_branch_type != PolicyBranchType::BranchComplete
        && viewing_civ.policies.is_adoptable(policy)
        && viewing_civ.policies.can_adopt_policy()
}