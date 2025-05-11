// Source: orig_src/core/src/com/unciv/ui/screens/pickerscreens/PickerPane.kt

use std::rc::Rc;
use egui::{Ui, Color32, Align, Response, Button, Image, ScrollArea, RichText, Vec2};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::bordered_table::BorderedTable;
use crate::ui::components::widgets::auto_scroll_pane::AutoScrollPane;
use crate::ui::components::widgets::vertical_group::VerticalGroup;
use crate::ui::components::widgets::icon_text_button::IconTextButton;
use crate::ui::images::ImageGetter;
use crate::utils::constants::Constants;

pub struct PickerPane {
    pub close_button: Button,
    pub description_label: DescriptionLabel,
    pub description_scroll: AutoScrollPane,
    pub right_side_group: VerticalGroup,
    pub right_side_button: Button,
    pub top_table: BorderedTable,
    pub bottom_table: BorderedTable,
    pub scroll_pane: AutoScrollPane,
    pub split_pane: SplitPane,
    disable_scroll: bool,
}

impl PickerPane {
    pub const PICKER_OPTION_ICON_SIZE: f32 = 30.0;
    const SCREEN_SPLIT: f32 = 0.85;
    const MAX_BOTTOM_TABLE_HEIGHT: f32 = 150.0; // about 7 lines of normal text

    pub fn new(disable_scroll: bool) -> Self {
        let mut pane = Self {
            close_button: Button::new("Close"),
            description_label: DescriptionLabel::new(""),
            description_scroll: AutoScrollPane::new(),
            right_side_group: VerticalGroup::new(),
            right_side_button: Button::new(""),
            top_table: BorderedTable::new(),
            bottom_table: BorderedTable::new(),
            scroll_pane: AutoScrollPane::new(),
            split_pane: SplitPane::new(),
            disable_scroll,
        };

        pane.init();
        pane
    }

    fn init(&mut self) {
        // Add close button to bottom table
        self.bottom_table.add(&self.close_button).pad(10.0);

        // Set up description label
        self.description_label.set_wrap(true);
        let mut description_with_pad = BorderedTable::new();
        description_with_pad.add(&self.description_label).pad(10.0).grow();
        self.description_scroll = AutoScrollPane::new_with_content(description_with_pad);
        self.bottom_table.add(&self.description_scroll).grow();

        // Set up right side button
        self.right_side_button.disable();
        self.right_side_group.add(&self.right_side_button);
        self.bottom_table.add(&self.right_side_group).pad(10.0).right();

        // Set up scroll pane
        self.scroll_pane.set_scrolling_disabled(self.disable_scroll, self.disable_scroll);
        if self.disable_scroll {
            self.scroll_pane.clear_listeners();
        }

        // Set up split pane
        self.split_pane = SplitPane::new_with_content(
            self.scroll_pane.clone(),
            self.bottom_table.clone(),
            true,
            BaseScreen::get_skin(),
        );

        // Add split pane to the main layout
        self.add(&self.split_pane).expand().fill();
    }

    pub fn layout(&mut self) {
        // Ensure bottom table height doesn't exceed maximum
        let bottom_height = self.bottom_table.height().min(Self::MAX_BOTTOM_TABLE_HEIGHT);
        self.bottom_table.set_height(bottom_height);

        // Calculate split amount
        let scroll_height = self.scroll_pane.height();
        let split_amount = (scroll_height / (scroll_height + bottom_height))
            .max(Self::SCREEN_SPLIT);
        self.split_pane.set_split_amount(split_amount);
    }

    pub fn set_right_side_button_enabled(&mut self, enabled: bool) {
        if enabled {
            self.right_side_button.enable();
        } else {
            self.right_side_button.disable();
        }
    }

    pub fn pick(&mut self, right_button_text: &str) {
        if crate::ui::gui::GUI::is_my_turn() {
            self.right_side_button.enable();
        }
        self.right_side_button.set_text(right_button_text);
    }

    pub fn get_picker_option_button(icon: Image, label: &str) -> Button {
        let mut button = IconTextButton::new(label, icon);
        button.icon_cell.size(Self::PICKER_OPTION_ICON_SIZE).pad(10.0);
        button.label_cell.pad(10.0);
        button
    }
}

pub struct DescriptionLabel {
    text: String,
    wrap: bool,
}

impl DescriptionLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            wrap: false,
        }
    }

    pub fn set_text(&mut self, new_text: &str) {
        self.text = new_text.to_string();
        // In Rust, we'll need to handle scroll position updates differently
        // This is a placeholder for the actual implementation
    }

    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }
}

pub struct SplitPane {
    first: AutoScrollPane,
    second: BorderedTable,
    vertical: bool,
    skin: Rc<BaseScreen>,
    split_amount: f32,
}

impl SplitPane {
    pub fn new() -> Self {
        Self {
            first: AutoScrollPane::new(),
            second: BorderedTable::new(),
            vertical: true,
            skin: Rc::new(BaseScreen::new()),
            split_amount: 0.5,
        }
    }

    pub fn new_with_content(
        first: AutoScrollPane,
        second: BorderedTable,
        vertical: bool,
        skin: Rc<BaseScreen>,
    ) -> Self {
        Self {
            first,
            second,
            vertical,
            skin,
            split_amount: 0.5,
        }
    }

    pub fn set_split_amount(&mut self, amount: f32) {
        self.split_amount = amount;
    }
}