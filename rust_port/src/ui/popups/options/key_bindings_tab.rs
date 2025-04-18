use std::collections::HashMap;
use std::collections::LinkedHashMap;
use ggez::graphics::{Color, DrawParam, Text};
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::models::metadata::GameSettings;
use crate::ui::components::input::{KeyCharAndCode, KeyboardBinding};
use crate::ui::components::widgets::{Checkbox, ExpanderTab, KeyCapturingButton};
use crate::ui::popups::options::OptionsPopup;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::screens::civilopedia_screen::{FormattedLine, MarkupRenderer};
use crate::ui::screens::tabbed_pager::TabbedPager;
use crate::utils::translation::tr;

pub struct KeyBindingsTab {
    options_popup: OptionsPopup,
    label_width: f32,
    key_bindings: HashMap<KeyboardBinding, KeyCharAndCode>,
    grouped_widgets: LinkedHashMap<KeyboardBinding::Category, LinkedHashMap<KeyboardBinding, KeyCapturingButton>>,
    disclaimer: Vec<FormattedLine>,
}

impl KeyBindingsTab {
    pub fn new(options_popup: OptionsPopup, label_width: f32) -> Self {
        let key_bindings = options_popup.settings.key_bindings.clone();
        let grouped_widgets = Self::create_grouped_widgets();

        let disclaimer = vec![
            FormattedLine::new("This is a work in progress.", Some("FIREBRICK".to_string()), true, None),
            FormattedLine::empty(),
            // FormattedLine::new("Do not pester the developers for missing entries!", None, false, None), // little joke
            FormattedLine::new("Please see the Tutorial.", None, false, Some("Tutorial/Keyboard Bindings".to_string())),
            FormattedLine::separator(),
        ];

        Self {
            options_popup,
            label_width,
            key_bindings,
            grouped_widgets,
            disclaimer,
        }
    }

    pub fn render(&mut self, ctx: &mut Context, screen: &BaseScreen) -> GameResult<()> {
        let mut table = screen.create_table();
        table.pad(10.0);
        table.defaults().pad(5.0);

        // Add disclaimer
        let disclaimer_text = MarkupRenderer::render(&self.disclaimer, self.label_width, |link| {
            // Handle link click - would need to implement GUI.open_civilopedia
            println!("Opening civilopedia: {}", link);
        });
        table.add_label(disclaimer_text).center().row();

        // Add each category
        for (category, bindings) in &self.grouped_widgets {
            let category_widget = self.get_category_widget(category, bindings);
            table.add(category_widget).row();
        }

        // Render the table
        table.render(ctx, screen)?;

        Ok(())
    }

    fn create_grouped_widgets() -> LinkedHashMap<KeyboardBinding::Category, LinkedHashMap<KeyboardBinding, KeyCapturingButton>> {
        // We want: For each category, sorted by their translated label,
        //     a sorted (by translated label) collection of all visible bindings in that category,
        //     associated with the actual UI widget (a KeyCapturingButton),
        //     and we want to easily index that by binding, so it should be a order-preserving map.

        let mut result = LinkedHashMap::new();

        // Get all keyboard bindings and filter out hidden ones
        let all_bindings: Vec<KeyboardBinding> = KeyboardBinding::values()
            .into_iter()
            .filter(|binding| !binding.hidden)
            .collect();

        // Group by category
        let mut bindings_by_category: HashMap<KeyboardBinding::Category, Vec<KeyboardBinding>> = HashMap::new();
        for binding in all_bindings {
            bindings_by_category.entry(binding.category).or_insert_with(Vec::new).push(binding);
        }

        // Sort categories by translated label
        let mut sorted_categories: Vec<KeyboardBinding::Category> = bindings_by_category.keys().cloned().collect();
        sorted_categories.sort_by(|a, b| tr(&a.label).cmp(&tr(&b.label)));

        // For each category, create a map of bindings to buttons
        for category in sorted_categories {
            let mut bindings_map = LinkedHashMap::new();

            // Get bindings for this category and sort by translated label
            let mut category_bindings = bindings_by_category.remove(&category).unwrap_or_default();
            category_bindings.sort_by(|a, b| tr(&a.label).cmp(&tr(&b.label)));

            // Create a button for each binding
            for binding in category_bindings {
                let button = KeyCapturingButton::new(binding.default_key);
                bindings_map.insert(binding, button);
            }

            result.insert(category, bindings_map);
        }

        result
    }

    fn get_category_widget(
        &self,
        category: &KeyboardBinding::Category,
        bindings: &LinkedHashMap<KeyboardBinding, KeyCapturingButton>
    ) -> ExpanderTab {
        let mut expander = ExpanderTab::new(
            &category.label,
            false,
            0.0,
            5.0,
            format!("KeyBindings.{}", category.name)
        );

        expander.defaults().pad_top(5.0);

        for (binding, widget) in bindings {
            expander.add_label(&binding.label).pad_right(10.0).min_width(self.label_width / 2.0);
            expander.add(widget.clone()).row();

            // Set the current key from settings
            if let Some(key) = self.key_bindings.get(binding) {
                widget.set_current(*key);
            }
        }

        expander
    }

    pub fn on_key_hit(&mut self) {
        for (category, bindings) in &mut self.grouped_widgets {
            let scope = category.check_conflicts_in();
            if scope.is_empty() {
                continue;
            }

            let mut used_keys = Vec::new();
            let mut conflicting_keys = Vec::new();

            // Collect all widgets in scope
            let mut widgets_in_scope = Vec::new();
            for scope_category in &scope {
                if let Some(scope_bindings) = self.grouped_widgets.get(scope_category) {
                    for (binding, widget) in scope_bindings {
                        widgets_in_scope.push((binding.category, widget.clone()));
                    }
                }
            }

            // Check for conflicts
            for (scope_category, widget) in &widgets_in_scope {
                let key = widget.current();

                // We shall not use any key of a different category in scope,
                // nor use a key within this category twice - if this category _is_ in scope.
                if used_keys.contains(&key) || *scope_category != *category {
                    conflicting_keys.push(key);
                } else {
                    used_keys.push(key);
                }
            }

            // Mark conflicts in the current category
            for widget in bindings.values_mut() {
                widget.set_mark_conflict(conflicting_keys.contains(&widget.current()));
            }
        }
    }

    pub fn save(&self) {
        if self.grouped_widgets.is_empty() {
            return; // We never initialized the current values, better not save
        }

        let mut settings = self.options_popup.settings.clone();

        for (_, bindings) in &self.grouped_widgets {
            for (binding, widget) in bindings {
                settings.key_bindings.insert(*binding, widget.current());
            }
        }

        self.options_popup.settings = settings;
    }
}

impl TabbedPager for KeyBindingsTab {
    fn activated(&mut self, _index: i32, _caption: &str) {
        // Update the UI when activated
        // In Rust, we don't need to do anything special here as the render method
        // will be called automatically
    }

    fn deactivated(&mut self, _index: i32, _caption: &str) {
        // Save settings when deactivated
        self.save();
    }
}