// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PickerScreen.kt

use std::rc::Rc;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::input::key_char_and_code::KeyCharAndCode;
use crate::ui::components::input::key_shortcuts::KeyShortcuts;
use crate::ui::components::input::on_activation::OnActivation;
use super::picker_pane::PickerPane;

pub struct PickerScreen {
    pub picker_pane: PickerPane,
    pub close_button: Button,
    pub description_label: DescriptionLabel,
    pub description_scroll: AutoScrollPane,
    pub right_side_group: VerticalGroup,
    pub right_side_button: Button,
    pub top_table: BorderedTable,
    pub bottom_table: BorderedTable,
    pub scroll_pane: AutoScrollPane,
    pub split_pane: SplitPane,
}

impl PickerScreen {
    pub fn new(disable_scroll: bool) -> Self {
        let mut screen = Self {
            picker_pane: PickerPane::new(disable_scroll),
            close_button: Button::new(""),
            description_label: DescriptionLabel::new(""),
            description_scroll: AutoScrollPane::new(),
            right_side_group: VerticalGroup::new(),
            right_side_button: Button::new(""),
            top_table: BorderedTable::new(),
            bottom_table: BorderedTable::new(),
            scroll_pane: AutoScrollPane::new(),
            split_pane: SplitPane::new(),
        };

        screen.init();
        screen
    }

    fn init(&mut self) {
        // Set up references to picker pane components
        self.close_button = self.picker_pane.close_button.clone();
        self.description_label = self.picker_pane.description_label.clone();
        self.description_scroll = self.picker_pane.description_scroll.clone();
        self.right_side_group = self.picker_pane.right_side_group.clone();
        self.right_side_button = self.picker_pane.right_side_button.clone();
        self.top_table = self.picker_pane.top_table.clone();
        self.bottom_table = self.picker_pane.bottom_table.clone();
        self.scroll_pane = self.picker_pane.scroll_pane.clone();
        self.split_pane = self.picker_pane.split_pane.clone();

        // Set up the picker pane
        self.picker_pane.set_fill_parent(true);
        self.ensure_layout();
    }

    fn ensure_layout(&mut self) {
        // Make sure that anyone relying on sizes of the tables within this class during construction gets correct size readings
        self.picker_pane.validate();
    }

    pub fn set_default_close_action(&mut self) {
        // Set up the close button action and the Back/ESC handler
        self.picker_pane.close_button.on_activation(|| {
            // In Rust, we'll need to handle this differently based on the game's screen management
            // This is a placeholder for the actual implementation
            // game.pop_screen();
        });

        self.picker_pane.close_button.key_shortcuts.add(KeyCharAndCode::BACK);
    }

    pub fn set_right_side_button_enabled(&mut self, enabled: bool) {
        self.picker_pane.set_right_side_button_enabled(enabled);
    }

    pub fn pick(&mut self, right_button_text: &str) {
        self.picker_pane.pick(right_button_text);
    }

    pub fn show(&self, ui: &mut Ui) {
        // In Rust with egui, we'll need to implement this differently
        // This is a placeholder for the actual implementation
    }
}

// Re-export these types for convenience
pub use super::picker_pane::{DescriptionLabel, AutoScrollPane, VerticalGroup, BorderedTable, SplitPane};