// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/UnitRenamePopup.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Button, TextEdit, RichText, Vec2};
use crate::models::civilization::Civilization;
use crate::models::unit::Unit;
use crate::models::unciv_sound::UncivSound;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::pickerscreen::PickerScreen;
use crate::utils::translation::tr;

/// Popup for renaming units
pub struct UnitRenamePopup {
    unit: Rc<RefCell<Unit>>,
    civ_info: Rc<RefCell<Civilization>>,
    picker_screen: PickerScreen,
    text_edit: TextEdit,
    name: String,
}

impl UnitRenamePopup {
    /// Creates a new unit rename popup
    pub fn new(unit: Rc<RefCell<Unit>>, civ_info: Rc<RefCell<Civilization>>) -> Self {
        let unit_ref = unit.borrow();
        let name = unit_ref.name.clone();

        let mut popup = Self {
            unit: Rc::clone(&unit),
            civ_info: Rc::clone(&civ_info),
            picker_screen: PickerScreen::new(false),
            text_edit: TextEdit::singleline(&mut String::new()),
            name,
        };

        popup.init();
        popup
    }

    /// Initializes the popup
    fn init(&mut self) {
        // Set default close action
        self.picker_screen.set_default_close_action();

        // Set description label text
        self.picker_screen.set_description_label_text(&tr("Enter new name"));

        // Set right side button text
        self.picker_screen.set_right_side_button_text(&tr("Rename"));

        // Set right side button click handler
        self.picker_screen.set_right_side_button_on_click(UncivSound::Paper, || {
            self.rename();
        });

        // Set up text edit
        self.text_edit = TextEdit::singleline(&mut self.name);
        self.text_edit.hint_text(tr("Enter new name"));
        self.text_edit.desired_width(f32::INFINITY);
        self.text_edit.desired_rows(1);
        self.text_edit.cursor_at_end();
        self.text_edit.text_style(egui::TextStyle::Body);
        self.text_edit.text_color(Color32::WHITE);
        self.text_edit.background_color(Color32::from_rgba_premultiplied(0, 0, 0, 0));
        self.text_edit.margin(Vec2::new(0.0, 0.0));
        self.text_edit.padding(Vec2::new(0.0, 0.0));
        self.text_edit.frame(false);
        self.text_edit.interactive(true);
        self.text_edit.readonly(false);
        self.text_edit.min_size(Vec2::new(0.0, 0.0));
        self.text_edit.max_size(Vec2::new(f32::INFINITY, f32::INFINITY));
        self.text_edit.horizontal_scroll(true);
        self.text_edit.vertical_scroll(false);
        self.text_edit.cursor_at_end();
        self.text_edit.single_line(true);
        self.text_edit.password(false);
        self.text_edit.layouter(&mut |ui, text, wrap_width| {
            let mut layout_job = egui::text::LayoutJob::default();
            layout_job.append(
                text,
                0.0,
                egui::text::TextStyle::new(egui::FontId::proportional(14.0), Color32::WHITE),
            );
            ui.fonts(|f| f.layout_job(layout_job))
        });
    }

    /// Renames the unit
    fn rename(&mut self) {
        let mut unit = self.unit.borrow_mut();
        unit.name = self.name.clone();

        // TODO: Implement pop_screen
        // self.game.pop_screen();
    }

    /// Shows the popup
    pub fn show(&mut self, ui: &mut Ui) {
        self.picker_screen.show(ui);

        // Add text edit to top table
        self.picker_screen.add_to_top_table(self.text_edit.clone());
    }
}