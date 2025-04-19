use egui::{Color32, Ui};
use egui_extras::Size;

use crate::game::UncivGame;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::popups::ConfirmPopup;
use crate::ui::screens::basescreen::BaseScreen;
use crate::utils::constants::Constants;

/// Popup that shows mod errors and asks the user if they want to proceed
pub struct AcceptModErrorsPopup {
    screen: Box<dyn BaseScreen>,
    mod_check_result: String,
    restore_default: Box<dyn Fn()>,
    action: Box<dyn Fn()>,
    confirm_popup: ConfirmPopup,
}

impl AcceptModErrorsPopup {
    /// Create a new AcceptModErrorsPopup
    pub fn new(
        screen: Box<dyn BaseScreen>,
        mod_check_result: String,
        restore_default: Box<dyn Fn()>,
        action: Box<dyn Fn()>,
    ) -> Self {
        let confirm_popup = ConfirmPopup::new(
            screen.clone(),
            "", // We'll use a colored label instead
            "Accept".to_string(),
            false,
            restore_default,
            action,
        );

        Self {
            screen,
            mod_check_result,
            restore_default,
            action,
            confirm_popup,
        }
    }

    /// Initialize the popup UI
    pub fn init(&mut this, ui: &mut Ui) {
        this.confirm_popup.set_click_behind_to_close(false);

        // Skip the empty question label
        ui.add_space(10.0);

        let max_row_width = this.screen.stage_width() * 0.9 - 50.0; // total padding is 2*(20+5)
        this.confirm_popup.set_scrolling_disabled(true, false);

        // Note - using the version of ColorMarkupLabel that supports «color» but it was too garish.
        let question = "Are you really sure you want to play with the following known problems?";
        let mut label1 = ColorMarkupLabel::new(question, Constants::HEADING_FONT_SIZE);
        let wrap_width = label1.preferred_width().min(max_row_width / 2.0).max(max_row_width);
        label1.set_alignment(egui::Align::Center);

        if label1.preferred_width() > wrap_width {
            label1.set_wrap(true);
            ui.add(label1.with_width(wrap_width));
            ui.add_space(15.0);
        } else {
            ui.add(label1);
            ui.add_space(15.0);
        }

        let warnings = this.mod_check_result
            .replace("Error:", "«RED»Error«»:")
            .replace("Warning:", "«GOLD»Warning«»:")
            .replace("OK:", "«GREEN»OK«»:");

        let mut label2 = ColorMarkupLabel::new(&warnings, Constants::DEFAULT_FONT_SIZE);
        label2.set_wrap(true);
        ui.add(label2.with_width(wrap_width));

        // Close all popups (including toasts)
        this.screen.close_all_popups();

        // Open the popup
        this.confirm_popup.open(true);
    }

    /// Show the popup
    pub fn show(&mut this, ui: &mut Ui) {
        this.init(ui);
        this.confirm_popup.show(ui);
    }
}