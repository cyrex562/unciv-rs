use std::collections::HashSet;
use egui::{Ui, Checkbox};
use crate::ui::components::widgets::ExpanderTab;

/// A widget containing one expander for multiple checkboxes.
pub struct MultiCheckboxTable {
    title: String,
    persistence_id: String,
    values: HashSet<String>,
    checkboxes: Vec<(String, Checkbox)>,
}

impl MultiCheckboxTable {
    pub fn new(title: String, persistence_id: String, values: HashSet<String>) -> Self {
        let mut checkboxes = Vec::new();
        for value in values.iter() {
            checkboxes.push((value.clone(), Checkbox::new(value.clone(), true)));
        }

        Self {
            title,
            persistence_id,
            values,
            checkboxes,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ExpanderTab::new(&self.title, &self.persistence_id, false)
            .show(ui, |ui| {
                ui.add_space(5.0);
                for (value, checkbox) in &mut self.checkboxes {
                    if ui.checkbox(&mut checkbox.checked, value.clone()).clicked() {
                        if checkbox.checked {
                            self.values.insert(value.clone());
                        } else {
                            self.values.remove(value);
                        }
                    }
                    ui.add_space(5.0);
                }
            });
    }

    pub fn get_values(&self) -> &HashSet<String> {
        &self.values
    }
}