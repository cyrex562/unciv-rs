use ggez::graphics::Color;
use egui_extras::Size;
use uuid::Uuid;
use rules::constants;
/// Popup that shows mod errors and asks the user if they want to proceed
pub struct AcceptModErrorsPopup {
    screen_id: Uuid,
    mod_check_result: String,
    restore_default: dyn Fn(),
    action: dyn Fn(),
    confirm_popup: ConfirmPopup,
}

impl AcceptModErrorsPopup {
    /// Create a new AcceptModErrorsPopup
    pub fn new(
        screen_id: Uuid,
        mod_check_result: String,
        restore_default: Box<dyn Fn()>,
        action: Box<dyn Fn()>,
    ) -> Self {
        let confirm_popup = ConfirmPopup::new(
            screen_id.clone(),
            "", // We'll use a colored label instead
            "Accept".to_string(),
            false,
            restore_default,
            action,
        );

        Self {
            screen_id,
            mod_check_result,
            restore_default,
            action,
            confirm_popup,
        }
    }

    /// Initialize the popup UI
    pub fn init(&mut self, ui: &mut Ui) {
        self.confirm_popup.set_click_behind_to_close(false);

        // Skip the empty question label
        ui.add_space(10.0);

        let max_row_width = self.screen_id.stage_width() * 0.9 - 50.0; // total padding is 2*(20+5)
        self.confirm_popup.set_scrolling_disabled(true, false);

        // Note - using the version of ColorMarkupLabel that supports «color» but it was too garish.
        let question = "Are you really sure you want to play with the following known problems?";
        let mut label1 = ColorMarkupLabel::new(question, constants::HEADING_FONT_SIZE);
        let wrap_width = label1
            .preferred_width()
            .min(max_row_width / 2.0)
            .max(max_row_width);
        label1.set_alignment(egui::Align::Center);

        if label1.preferred_width() > wrap_width {
            label1.set_wrap(true);
            ui.add(label1.with_width(wrap_width));
            ui.add_space(15.0);
        } else {
            ui.add(label1);
            ui.add_space(15.0);
        }

        let warnings = self
            .mod_check_result
            .replace("Error:", "«RED»Error«»:")
            .replace("Warning:", "«GOLD»Warning«»:")
            .replace("OK:", "«GREEN»OK«»:");

        let mut label2 = ColorMarkupLabel::new(&warnings, Constants::DEFAULT_FONT_SIZE);
        label2.set_wrap(true);
        ui.add(label2.with_width(wrap_width));

        // Close all popups (including toasts)
        self.screen_id.close_all_popups();

        // Open the popup
        self.confirm_popup.open(true);
    }

    /// Show the popup
    pub fn show(&mut self, ui: &mut Ui) {
        self.init(ui);
        self.confirm_popup.show(ui);
    }
}
