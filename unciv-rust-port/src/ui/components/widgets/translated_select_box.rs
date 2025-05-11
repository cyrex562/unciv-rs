use std::collections::HashMap;
use std::sync::Arc;

use ggez::graphics::Color;
use ggez::mint::Point2;

use crate::ui::components::widgets::select_box::SelectBox;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::translations::TranslationManager;

/// A select box that displays translated strings
pub struct TranslatedSelectBox {
    /// The base SelectBox that this TranslatedSelectBox extends
    base: SelectBox<TranslatedString>,

    /// The items in the select box
    items: Vec<TranslatedString>,

    /// The currently selected item
    selected: Option<TranslatedString>,
}

/// A string that can be translated
#[derive(Debug, Clone)]
pub struct TranslatedString {
    /// The original value
    pub value: String,

    /// The translated value
    pub translation: String,
}

impl TranslatedString {
    /// Creates a new TranslatedString with the given value
    pub fn new(value: String) -> Self {
        let translation = TranslationManager::translate(&value, true);
        Self {
            value,
            translation,
        }
    }
}

// Implement equality for TranslatedString
impl PartialEq for TranslatedString {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for TranslatedString {}

// Implement hash for TranslatedString
impl std::hash::Hash for TranslatedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

// Implement display for TranslatedString
impl std::fmt::Display for TranslatedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.translation)
    }
}

impl TranslatedSelectBox {
    /// Creates a new TranslatedSelectBox with the given values and default selection
    pub fn new(values: &[String], default: &str) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        // Create translated strings for each value
        let items: Vec<TranslatedString> = values
            .iter()
            .map(|value| TranslatedString::new(value.clone()))
            .collect();

        // Find the default item
        let selected = items
            .iter()
            .find(|item| item.value == default)
            .cloned()
            .or_else(|| items.first().cloned());

        let mut select_box = Self {
            base: SelectBox::new(skin),
            items: items.clone(),
            selected: selected.clone(),
        };

        // Set the items in the base select box
        select_box.base.set_items(&items);

        // Set the selected item
        if let Some(selected) = selected {
            select_box.base.set_selected(&selected);
        }

        select_box
    }

    /// Sets the selected item by value
    pub fn set_selected(&mut self, new_value: &str) {
        // Find the item with the given value
        if let Some(item) = self.items.iter().find(|item| item.value == new_value) {
            self.selected = Some(item.clone());
            self.base.set_selected(item);
        }
    }

    /// Gets the selected value
    pub fn get_selected(&self) -> Option<&str> {
        self.selected.as_ref().map(|item| item.value.as_str())
    }

    /// Gets the selected translated string
    pub fn get_selected_translation(&self) -> Option<&str> {
        self.selected.as_ref().map(|item| item.translation.as_str())
    }
}

// Implement the necessary traits for TranslatedSelectBox
impl std::ops::Deref for TranslatedSelectBox {
    type Target = SelectBox<TranslatedString>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for TranslatedSelectBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for TranslatedSelectBox {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            items: self.items.clone(),
            selected: self.selected.clone(),
        }
    }
}