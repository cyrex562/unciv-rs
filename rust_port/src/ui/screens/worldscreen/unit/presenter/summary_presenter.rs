// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/presenter/SummaryPresenter.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::Vec2;
use crate::ui::screens::worldscreen::unit::UnitTable;
use crate::utils::translations::tr;

/// Presenter for unit summary information in the unit table
pub struct SummaryPresenter {
    unit_table: Rc<RefCell<UnitTable>>,
}

impl SummaryPresenter {
    /// Creates a new SummaryPresenter
    pub fn new(unit_table: Rc<RefCell<UnitTable>>) -> Self {
        Self {
            unit_table,
        }
    }

    /// Gets the position (always None for summary)
    pub fn position(&self) -> Option<Vec2> {
        None
    }

    /// Updates the UI
    pub fn update(&self) {
        let mut unit_table = self.unit_table.borrow_mut();
        unit_table.close_button_visible = false;
    }

    /// Updates the UI when needed
    pub fn update_when_needed(&self) {
        let mut unit_table = self.unit_table.borrow_mut();

        // Clear description table
        unit_table.description_table.clear();

        // Set unit name label
        unit_table.unit_name_label.set_text(tr("Units"));

        // Count idle and waiting units
        let viewing_civ = unit_table.world_screen.borrow().viewing_civ.clone();
        let idle_count = viewing_civ.borrow().units.get_idle_units()
            .iter()
            .filter(|unit| unit.borrow().due)
            .count();

        let waiting_count = viewing_civ.borrow().units.get_idle_units()
            .iter()
            .filter(|unit| !unit.borrow().due)
            .count();

        // Build subtext
        let mut sub_text_parts = Vec::new();
        if idle_count > 0 {
            sub_text_parts.push(format!("[{}] {}", idle_count, tr("idle")));
        }
        if waiting_count > 0 {
            sub_text_parts.push(format!("[{}] {}", waiting_count, tr("skipping")));
        }

        let sub_text = sub_text_parts.join(", ");

        if !sub_text.is_empty() {
            unit_table.separator_visible = true;
            unit_table.description_table.add(sub_text);
        } else {
            unit_table.separator_visible = false;
        }
    }
}