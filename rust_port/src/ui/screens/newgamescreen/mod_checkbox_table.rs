use std::collections::{HashSet, LinkedHashSet};
use std::rc::Rc;
use std::cell::RefCell;
use egui::{Color32, Ui, Checkbox, ScrollArea};
use crate::models::metadata::GameParameters;
use crate::models::ruleset::{Ruleset, RulesetCache};
use crate::models::ruleset::validation::ModCompatibility;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::ExpanderTab;
use crate::ui::popups::ToastPopup;
use crate::utils::concurrency::Concurrency;

/// A widget containing one expander for extension mods.
/// Manages compatibility checks, warns or prevents incompatibilities.
pub struct ModCheckboxTable {
    mods: Rc<RefCell<LinkedHashSet<String>>>,
    base_ruleset_name: String,
    base_ruleset: Ruleset,
    mod_widgets: Vec<ModWithCheckbox>,
    saved_modcheck_result: Option<String>,
    disable_change_events: bool,
    expander_pad_top: f32,
    expander_pad_other: f32,
    screen: Rc<RefCell<BaseScreen>>,
    on_update: Box<dyn Fn(String)>,
}

struct ModWithCheckbox {
    mod_: Ruleset,
    widget: Checkbox,
}

impl ModCheckboxTable {
    pub fn new(
        mods: LinkedHashSet<String>,
        initial_base_ruleset: String,
        screen: Rc<RefCell<BaseScreen>>,
        is_portrait: bool,
        on_update: Box<dyn Fn(String)>,
    ) -> Self {
        let mod_rulesets: Vec<Ruleset> = RulesetCache::values()
            .filter(|it| ModCompatibility::is_extension_mod(it))
            .collect();

        let mut mod_widgets = Vec::new();
        for mod_ in mod_rulesets.iter().sorted_by(|a, b| a.name.cmp(&b.name)) {
            let checkbox = Checkbox::new(mod_.name.clone(), mods.contains(&mod_.name));
            mod_widgets.push(ModWithCheckbox {
                mod_: mod_.clone(),
                widget: checkbox,
            });
        }

        let mut table = Self {
            mods: Rc::new(RefCell::new(mods)),
            base_ruleset_name: String::new(),
            base_ruleset: Ruleset::default(),
            mod_widgets,
            saved_modcheck_result: None,
            disable_change_events: false,
            expander_pad_top: if is_portrait { 0.0 } else { 16.0 },
            expander_pad_other: if is_portrait { 0.0 } else { 10.0 },
            screen,
            on_update,
        };

        table.set_base_ruleset(initial_base_ruleset);
        table
    }

    pub fn update_selection(&mut self) {
        self.saved_modcheck_result = None;
        self.disable_change_events = true;
        for mod_widget in &mut self.mod_widgets {
            mod_widget.widget.checked = self.mods.borrow().contains(&mod_widget.mod_.name);
        }
        self.disable_change_events = false;
        self.deselect_incompatible_mods(None);
    }

    pub fn set_base_ruleset(&mut self, new_base_ruleset_name: String) {
        let new_base_ruleset = match RulesetCache::get(&new_base_ruleset_name) {
            Some(ruleset) => ruleset,
            None => {
                // We're calling this from init, base_ruleset is lateinit, and the mod may have been deleted
                return self.set_base_ruleset("Civ_V_GnK".to_string());
            }
        };

        self.base_ruleset_name = new_base_ruleset_name;
        self.base_ruleset = new_base_ruleset;
        self.saved_modcheck_result = None;

        let mut mods = self.mods.borrow_mut();
        mods.clear(); // We'll regenerate this from checked widgets

        let compatible_mods: Vec<&ModWithCheckbox> = self.mod_widgets
            .iter()
            .filter(|it| ModCompatibility::meets_base_requirements(&it.mod_, &self.base_ruleset))
            .collect();

        if compatible_mods.is_empty() {
            return;
        }

        for mod_widget in &compatible_mods {
            if mod_widget.widget.checked {
                mods.insert(mod_widget.mod_.name.clone());
            }
        }

        self.disable_incompatible_mods();

        Concurrency::run(|| {
            self.complex_mod_check_returns_errors();
        });
    }

    pub fn disable_all_checkboxes(&mut self) {
        self.disable_change_events = true;
        for mod_widget in &mut self.mod_widgets {
            mod_widget.widget.checked = false;
        }
        self.mods.borrow_mut().clear();
        self.disable_change_events = false;

        self.saved_modcheck_result = None;
        self.disable_incompatible_mods();
        (self.on_update)("-".to_string()); // should match no mod
    }

    fn complex_mod_check_returns_errors(&mut self) -> bool {
        // Check over complete combination of selected mods
        let complex_mod_link_check = RulesetCache::check_combined_mod_links(&self.mods.borrow(), &self.base_ruleset_name);
        if !complex_mod_link_check.is_warn_user() {
            self.saved_modcheck_result = None;
            return false;
        }
        self.saved_modcheck_result = Some(complex_mod_link_check.get_error_text());
        complex_mod_link_check.show_warn_or_error_toast(&self.screen);
        complex_mod_link_check.is_error()
    }

    fn check_box_changed(&mut self, checkbox: &mut Checkbox, mod_: &Ruleset) {
        if self.disable_change_events {
            return;
        }

        if checkbox.checked {
            // First the quick standalone check
            let mod_link_errors = mod_.get_error_list();
            if mod_link_errors.is_error() {
                mod_link_errors.show_warn_or_error_toast(&self.screen);
                checkbox.checked = false; // Cancel event to reset to previous state
                return;
            }

            self.mods.borrow_mut().insert(mod_.name.clone());

            // Check over complete combination of selected mods
            if self.complex_mod_check_returns_errors() {
                // Cancel event to reset to previous state
                checkbox.checked = false;
                self.mods.borrow_mut().remove(&mod_.name);
                self.saved_modcheck_result = None; // we just fixed it
                return;
            }
        } else {
            self.mods.borrow_mut().remove(&mod_.name);

            if self.complex_mod_check_returns_errors() {
                // Cancel event to reset to previous state
                checkbox.checked = true;
                self.mods.borrow_mut().insert(mod_.name.clone());
                self.saved_modcheck_result = None; // we just fixed it
                return;
            }
        }

        self.disable_incompatible_mods();
        (self.on_update)(mod_.name.clone());
    }

    fn deselect_incompatible_mods(&mut self, skip_checkbox: Option<&Checkbox>) {
        self.disable_change_events = true;
        for mod_widget in &mut self.mod_widgets {
            if let Some(skip) = skip_checkbox {
                if std::ptr::eq(&mod_widget.widget, skip) {
                    continue;
                }
            }
            if !ModCompatibility::meets_all_requirements(&mod_widget.mod_, &this.base_ruleset, this.get_selected_mods()) {
                mod_widget.widget.checked = false;
                this.mods.borrow_mut().remove(&mod_widget.mod_.name);
            }
        }
        this.disable_change_events = false;
    }

    fn disable_incompatible_mods(&mut this) {
        for mod_widget in &mut this.mod_widgets {
            let enable = ModCompatibility::meets_all_requirements(&mod_widget.mod_, &this.base_ruleset, this.get_selected_mods());
            if !enable && mod_widget.widget.checked {
                mod_widget.widget.checked = false;
            }
            mod_widget.widget.enabled = enable;
        }
    }

    fn get_selected_mods(&self) -> Vec<&Ruleset> {
        self.mod_widgets
            .iter()
            .filter(|it| it.widget.checked)
            .map(|it| &it.mod_)
            .collect()
    }

    pub fn change_game_parameters(&mut this, new_game_parameters: &GameParameters) {
        *this.mods.borrow_mut() = new_game_parameters.mods.clone();
    }

    pub fn show(&mut this, ui: &mut Ui) {
        let compatible_mods: Vec<&ModWithCheckbox> = this.mod_widgets
            .iter()
            .filter(|it| ModCompatibility::meets_base_requirements(&it.mod_, &this.base_ruleset))
            .collect();

        if compatible_mods.is_empty() {
            return;
        }

        ExpanderTab::new("Extension mods", "NewGameExpansionMods", false)
            .show(ui, |ui| {
                ui.add_space(5.0);
                for mod_widget in compatible_mods {
                    if ui.checkbox(&mut mod_widget.widget.checked, mod_widget.mod_.name.clone()).clicked() {
                        this.check_box_changed(&mut mod_widget.widget, &mod_widget.mod_);
                    }
                    ui.add_space(5.0);
                }
            });
    }
}