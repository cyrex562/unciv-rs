// Source: orig_src/core/src/com/unciv/ui/screens/UniqueBuilderScreen.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Button, TextEdit, ScrollArea, RichText, Grid, ComboBox};
use crate::logic::ruleset::{Ruleset, Unique, UniqueTarget, UniqueType, UniqueValidator};
use crate::ui::screens::picker_screen::PickerScreen;
use crate::ui::images::ImageGetter;

/// A PickerScreen to select a language, used once on the initial run after a fresh install.
/// After that, OptionsPopup provides the functionality.
/// Reusable code is in LanguageTable and add_language_tables.
pub struct UniqueBuilderScreen {
    ruleset: Rc<RefCell<Ruleset>>,
    main_unique_table: UniqueTable,
    modifier_tables: Vec<UniqueTable>,
    current_unique_text: String,
    stage_width: f32,
    stage_height: f32,
    right_side_button_visible: bool,
    right_side_button_text: String,
    right_side_button_enabled: bool,
    description_label_text: String,
}

impl UniqueBuilderScreen {
    pub fn new(ruleset: Rc<RefCell<Ruleset>>, stage_width: f32, stage_height: f32) -> Self {
        let mut screen = Self {
            ruleset: ruleset.clone(),
            main_unique_table: UniqueTable::new(true, ruleset.clone(), stage_width, stage_height, |screen| {
                screen.update_current_unique_text();
            }),
            modifier_tables: Vec::new(),
            current_unique_text: String::new(),
            stage_width,
            stage_height,
            right_side_button_visible: false,
            right_side_button_text: "Copy to clipboard".to_string(),
            right_side_button_enabled: false,
            description_label_text: String::new(),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Set default close action
        // TODO: Implement set_default_close_action

        self.right_side_button_visible = false;
        self.right_side_button_text = "Copy to clipboard".to_string();

        // Initialize main unique table
        self.main_unique_table.initialize();

        // Add modifier button will be handled in the show method
    }

    fn update_current_unique_text(&mut self) {
        let main_text = self.main_unique_table.unique_text.clone();
        let modifier_texts: Vec<String> = self.modifier_tables.iter()
            .map(|table| format!(" <{}>", table.unique_text))
            .collect();

        self.current_unique_text = format!("{}{}", main_text, modifier_texts.join(""));
        self.description_label_text = self.current_unique_text.clone();
        self.right_side_button_visible = true;
        self.right_side_button_enabled = true;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Draw main unique table
        self.main_unique_table.show(ui);

        // Draw modifier tables
        for table in &mut self.modifier_tables {
            table.show(ui);
        }

        // Draw add modifier button
        if ui.button("Add Modifier").clicked() {
            let modifier_table = UniqueTable::new(false, self.ruleset.clone(), self.stage_width, self.stage_height, |screen| {
                screen.update_current_unique_text();
            });
            self.modifier_tables.push(modifier_table);
        }

        // Draw description label
        ui.label(RichText::new(self.description_label_text.clone()));

        // Draw right side button
        if self.right_side_button_visible {
            if ui.button(RichText::new(self.right_side_button_text.clone())).enabled(self.right_side_button_enabled).clicked() {
                // TODO: Implement clipboard functionality
                // In Kotlin: Gdx.app.clipboard.contents = currentUniqueText
            }
        }
    }
}

pub struct UniqueTable {
    is_main_unique: bool,
    ruleset: Rc<RefCell<Ruleset>>,
    stage_width: f32,
    stage_height: f32,
    on_unique_change: Box<dyn FnMut(&mut UniqueBuilderScreen)>,
    unique_text: String,
    unique_target: UniqueTarget,
    unique_type: Option<UniqueType>,
    unique_search_text: String,
    parameter_values: HashMap<usize, String>,
    unique_errors: Vec<String>,
}

impl UniqueTable {
    pub fn new(
        is_main_unique: bool,
        ruleset: Rc<RefCell<Ruleset>>,
        stage_width: f32,
        stage_height: f32,
        on_unique_change: impl FnMut(&mut UniqueBuilderScreen) + 'static,
    ) -> Self {
        Self {
            is_main_unique,
            ruleset,
            stage_width,
            stage_height,
            on_unique_change: Box::new(on_unique_change),
            unique_text: "Unique".to_string(),
            unique_target: UniqueTarget::Global,
            unique_type: None,
            unique_search_text: String::new(),
            parameter_values: HashMap::new(),
            unique_errors: Vec::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.on_unique_target_change();
    }

    fn on_unique_target_change(&mut self) {
        // Filter unique targets based on whether this is a main unique or a modifier
        let unique_targets = if self.is_main_unique {
            UniqueTarget::entries().iter()
                .filter(|target| target.modifier_type == UniqueTarget::ModifierType::None)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            UniqueTarget::entries().iter()
                .filter(|target| target.modifier_type != UniqueTarget::ModifierType::None)
                .cloned()
                .collect::<Vec<_>>()
        };

        // Set default unique target
        self.unique_target = unique_targets.first().cloned().unwrap_or(UniqueTarget::Global);

        // Update unique type options
        self.update_unique_type_options();
    }

    fn update_unique_type_options(&mut self) {
        // Filter unique types based on the selected target
        let uniques_for_target = UniqueType::entries().iter()
            .filter(|unique_type| unique_type.can_accept_unique_target(&self.unique_target))
            .filter(|unique_type| unique_type.get_deprecation_annotation().is_none())
            .filter(|unique_type| {
                if self.unique_search_text.is_empty() {
                    true
                } else {
                    unique_type.text.to_lowercase().contains(&self.unique_search_text.to_lowercase())
                }
            })
            .cloned()
            .collect::<Vec<_>>();

        // Set default unique type if available
        if let Some(first_unique) = uniques_for_target.first() {
            self.unique_type = Some(first_unique.clone());
            self.unique_text = first_unique.text.clone();
            self.update_parameter_options();
        }
    }

    fn update_parameter_options(&mut self) {
        if let Some(unique_type) = &self.unique_type {
            let parameters = unique_type.text.get_placeholder_parameters();

            for (index, parameter) in parameters.iter().enumerate() {
                let known_values = unique_type.parameter_type_map.get(&index)
                    .map(|types| types.iter()
                        .flat_map(|t| t.get_known_values_for_autocomplete(&self.ruleset.borrow()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default();

                if !known_values.is_empty() {
                    // Set default value if not already set
                    if !self.parameter_values.contains_key(&index) {
                        self.parameter_values.insert(index, known_values[0].clone());
                    }
                }
            }

            // Update unique text with parameter values
            self.update_unique_text_with_parameters();
        }
    }

    fn update_unique_text_with_parameters(&mut self) {
        if let Some(unique_type) = &self.unique_type {
            let parameters = unique_type.text.get_placeholder_parameters();
            let mut current_params = Vec::new();

            for (index, _) in parameters.iter().enumerate() {
                let value = self.parameter_values.get(&index).cloned().unwrap_or_default();
                current_params.push(value);
            }

            self.unique_text = unique_type.text.fill_placeholders(&current_params);
        }
    }

    fn update_unique(&mut self) {
        self.unique_errors.clear();

        let unique = Unique::new(self.unique_text.clone());
        let validator = UniqueValidator::new(&self.ruleset.borrow());
        let errors = validator.check_unique(&unique, true, None, true);

        for error in errors {
            self.unique_errors.push(error.text.clone());
        }

        // Call the callback to update the parent screen
        // This is a bit tricky in Rust, as we need to pass a reference to the parent
        // For now, we'll just store the errors and let the parent handle the update
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Draw unique target selector
        let mut selected_target = self.unique_target.clone();
        ComboBox::from_label("Unique Target")
            .selected_text(format!("{:?}", selected_target))
            .show_ui(ui, |ui| {
                for target in UniqueTarget::entries() {
                    if (self.is_main_unique && target.modifier_type == UniqueTarget::ModifierType::None) ||
                       (!self.is_main_unique && target.modifier_type != UniqueTarget::ModifierType::None) {
                        ui.selectable_value(&mut selected_target, target.clone(), format!("{:?}", target));
                    }
                }
            });

        if selected_target != self.unique_target {
            self.unique_target = selected_target;
            self.on_unique_target_change();
        }

        // Draw unique type selector
        if let Some(unique_type) = &self.unique_type {
            let mut selected_type = unique_type.clone();
            ComboBox::from_label("Unique Type")
                .selected_text(selected_type.text.clone())
                .show_ui(ui, |ui| {
                    for unique_type in UniqueType::entries() {
                        if unique_type.can_accept_unique_target(&self.unique_target) &&
                           unique_type.get_deprecation_annotation().is_none() &&
                           (self.unique_search_text.is_empty() ||
                            unique_type.text.to_lowercase().contains(&self.unique_search_text.to_lowercase())) {
                            ui.selectable_value(&mut selected_type, unique_type.clone(), unique_type.text.clone());
                        }
                    }
                });

            if selected_type != *unique_type {
                self.unique_type = Some(selected_type);
                self.unique_text = self.unique_type.as_ref().unwrap().text.clone();
                self.update_parameter_options();
            }
        }

        // Draw unique search field
        ui.text_edit_singleline(&mut self.unique_search_text);

        // Draw unique text field
        ui.text_edit_singleline(&mut self.unique_text);

        // Draw parameter selectors
        if let Some(unique_type) = &self.unique_type {
            let parameters = unique_type.text.get_placeholder_parameters();

            for (index, parameter) in parameters.iter().enumerate() {
                ui.label(format!("Parameter {}: {}", index + 1, parameter));

                let known_values = unique_type.parameter_type_map.get(&index)
                    .map(|types| types.iter()
                        .flat_map(|t| t.get_known_values_for_autocomplete(&self.ruleset.borrow()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default();

                if !known_values.is_empty() {
                    let mut selected_value = self.parameter_values.get(&index).cloned().unwrap_or_default();
                    ComboBox::from_label(format!("Value for {}", parameter))
                        .selected_text(selected_value.clone())
                        .show_ui(ui, |ui| {
                            for value in &known_values {
                                ui.selectable_value(&mut selected_value, value.clone(), value.clone());
                            }
                        });

                    if selected_value != self.parameter_values.get(&index).cloned().unwrap_or_default() {
                        self.parameter_values.insert(index, selected_value);
                        self.update_unique_text_with_parameters();
                    }
                } else {
                    ui.label("No known values");
                }
            }
        }

        // Draw error messages
        if !self.unique_errors.is_empty() {
            ui.label("Errors:");
            for error in &self.unique_errors {
                ui.label(error);
            }
        } else {
            ui.label("No errors!");
        }
    }
}