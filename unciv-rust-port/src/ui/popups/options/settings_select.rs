use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use ggez::graphics::{Text, DrawParam};
use ggez::{Context, GameResult};

use crate::models::UncivSound;
use crate::models::metadata::{GameSettings, GameSetting};
use crate::ui::audio::SoundPlayer;
use crate::ui::components::widgets::{Label, SelectBox};
use crate::ui::screens::base_screen::BaseScreen;
use crate::utils::translation::tr;

/// For creating a SelectBox that is automatically backed by a GameSettings property.
///
/// **Warning:** T has to be the same type as the GameSetting.kClass of the GameSetting argument.
///
/// This will also automatically send SettingsPropertyChanged events.
pub struct SettingsSelect<T> {
    settings_property: Box<dyn FnMut(&T)>,
    label: Label,
    refresh_select_box: SelectBox<SelectItem<T>>,
    items: Vec<SelectItem<T>>,
}

/// A selectable item with a label and value
pub struct SelectItem<T> {
    pub label: String,
    pub value: T,
}

impl<T> SelectItem<T> {
    pub fn new(label: String, value: T) -> Self {
        Self { label, value }
    }
}

impl<T: fmt::Display> fmt::Display for SelectItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", tr(&self.label))
    }
}

impl<T: PartialEq> PartialEq for SelectItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for SelectItem<T> {}

impl<T: Hash> Hash for SelectItem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: Clone + PartialEq + 'static> SettingsSelect<T> {
    /// Create a new SettingsSelect
    pub fn new(
        label_text: &str,
        items: impl IntoIterator<Item = SelectItem<T>>,
        setting: GameSetting,
        settings: &mut GameSettings
    ) -> Self {
        let items_vec: Vec<SelectItem<T>> = items.into_iter().collect();
        let settings_property = setting.get_property_mut(settings);

        let label = Self::create_label(label_text);
        let refresh_select_box = Self::create_select_box(&items_vec, &settings_property);

        Self {
            settings_property,
            label,
            refresh_select_box,
            items: items_vec,
        }
    }

    /// Create a label for the select box
    fn create_label(label_text: &str) -> Label {
        let mut select_label = Label::new(label_text);
        select_label.set_wrap(true);
        select_label
    }

    /// Create a select box with the given items
    fn create_select_box(
        initial_items: &[SelectItem<T>],
        settings_property: &Box<dyn FnMut(&T)>
    ) -> SelectBox<SelectItem<T>> {
        let mut select_box = SelectBox::new(BaseScreen::get_skin());
        select_box.set_items(initial_items);

        // Find the item that matches the current setting value
        let current_value = settings_property.get();
        let selected_item = initial_items.iter()
            .find(|item| item.value == *current_value)
            .unwrap_or_else(|| &initial_items[0]);

        select_box.set_selected(selected_item.clone());

        // Set up change handler
        select_box.set_on_change(Box::new(move |selected| {
            let new_value = selected.value.clone();
            settings_property.set(&new_value);

            // If the value is a sound, play it
            if let Some(sound) = (new_value as &dyn std::any::Any).downcast_ref::<UncivSound>() {
                SoundPlayer::play(sound);
            }
        }));

        select_box
    }

    /// Add an onChange listener to the select box
    pub fn on_change<F>(&mut self, listener: F) where F: FnMut(&SelectItem<T>) + 'static {
        self.refresh_select_box.set_on_change(Box::new(listener));
    }

    /// Add the select box to a table
    pub fn add_to(&self, table: &mut BaseScreen) {
        table.add(&self.label).grow_x().left();
        table.add(&self.refresh_select_box).row();
    }

    /// Replace the items in the select box while maintaining the currently selected item if possible
    pub fn replace_items(&mut self, options: &[SelectItem<T>]) {
        let prev = self.refresh_select_box.get_selected().clone();
        self.refresh_select_box.set_items(options);
        self.refresh_select_box.set_selected(prev);
        self.items = options.to_vec();
    }

    /// Get the current items
    pub fn get_items(&self) -> &[SelectItem<T>] {
        &self.items
    }

    /// Get the current selected item
    pub fn get_selected(&self) -> &SelectItem<T> {
        self.refresh_select_box.get_selected()
    }

    /// Set the selected item
    pub fn set_selected(&mut self, item: SelectItem<T>) {
        self.refresh_select_box.set_selected(item);
    }
}