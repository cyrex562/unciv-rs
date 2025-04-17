use ggez::graphics::{Color, DrawParam, Drawable, Mesh, MeshBatch, Rect, Text};
use ggez::input::mouse::MouseButton;
use ggez::mint::Point2;
use ggez::{Context, GameResult};
use std::collections::HashMap;
use std::rc::Rc;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::ui::components::button::Button;
use crate::ui::components::label::Label;
use crate::ui::components::scroll_pane::ScrollPane;
use crate::ui::components::table::Table;
use crate::ui::components::text_field::TextField;
use crate::ui::screens::base_screen::BaseScreen;
use crate::ui::skin::Skin;
use crate::ui::widget::Widget;

/// Controls how content may scroll in a popup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scrollability {
    /// No scrolling
    None,
    /// Entire content wrapped in a ScrollPane so it can scroll if larger than maximum dimensions
    All,
    /// Content separated into scrollable upper part and static lower part containing the buttons
    WithoutButtons,
}

/// Base class for all Popups, i.e. Tables that get rendered in the middle of a screen and on top of everything else
pub struct Popup {
    /// The stage that will be used for open, measurements or finding other instances
    stage_to_show_on: Rc<BaseScreen>,
    /// Controls how content can scroll if too large
    scrollability: Scrollability,
    /// Causes inner_table to limit its width/height - useful if scrollability is on
    max_size_percentage: f32,
    /// Maximum width of the popup
    max_popup_width: f32,
    /// Maximum height of the popup
    max_popup_height: f32,
    /// The inner table that contains the actual popup content
    inner_table: Table,
    /// This contains most of the Popup content
    top_table: Table,
    /// This contains the bottom row buttons and does not participate in scrolling
    bottom_table: Table,
    /// Callbacks that will be called whenever this Popup is shown
    show_listeners: Vec<Box<dyn Fn()>>,
    /// Callbacks that will be called whenever this Popup is closed, no matter how
    close_listeners: Vec<Box<dyn Fn()>>,
    /// Enables/disables closing by clicking/tapping outside inner_table
    click_behind_to_close: bool,
    /// Unlike close_listeners this is only fired on "click-behind" closing
    on_close_callback: Option<Box<dyn Fn()>>,
    /// Whether the popup is currently visible
    is_visible: bool,
    /// Whether the popup is currently touchable
    touchable: bool,
    /// The background mesh for the popup
    background: Option<Mesh>,
    /// The inner table background mesh
    inner_table_background: Option<Mesh>,
    /// The current keyboard focus
    keyboard_focus: Option<Box<dyn Widget>>,
    /// The current scroll pane if any
    scroll_pane: Option<ScrollPane>,
}

impl Popup {
    /// Creates a new Popup with the given stage, scrollability, and max size percentage
    pub fn new(
        stage_to_show_on: Rc<BaseScreen>,
        scrollability: Scrollability,
        max_size_percentage: f32,
    ) -> Self {
        let max_popup_width = stage_to_show_on.width() * max_size_percentage;
        let max_popup_height = stage_to_show_on.height() * max_size_percentage;

        let mut popup = Self {
            stage_to_show_on,
            scrollability,
            max_size_percentage,
            max_popup_width,
            max_popup_height,
            inner_table: Table::new(),
            top_table: Table::new(),
            bottom_table: Table::new(),
            show_listeners: Vec::new(),
            close_listeners: Vec::new(),
            click_behind_to_close: false,
            on_close_callback: None,
            is_visible: false,
            touchable: true,
            background: None,
            inner_table_background: None,
            keyboard_focus: None,
            scroll_pane: None,
        };

        popup.setup_ui();
        popup
    }

    /// Creates a new Popup with the given screen, scrollability, and max size percentage
    pub fn new_with_screen(
        screen: &Rc<BaseScreen>,
        scrollability: Scrollability,
        max_size_percentage: f32,
    ) -> Self {
        Self::new(screen.clone(), scrollability, max_size_percentage)
    }

    /// Creates a new Popup with the given screen and scrollability
    pub fn new_with_scrollability(screen: &Rc<BaseScreen>, scrollability: Scrollability) -> Self {
        Self::new_with_screen(screen, scrollability, 0.9)
    }

    /// Sets up the UI for the popup
    fn setup_ui(&mut self) {
        // Set up the background
        let mut gray = Color::GRAY;
        gray.a = 0.5;
        self.background = Some(Mesh::new_rectangle(
            &self.stage_to_show_on.graphics_context(),
            DrawParam::default(),
            Rect::new(0.0, 0.0, self.stage_to_show_on.width(), self.stage_to_show_on.height()),
            gray,
        ).unwrap());

        // Set up the inner table background
        let base_color = self.stage_to_show_on.skin().base_color();
        let darkened_color = base_color.darken(0.5);
        self.inner_table_background = Some(Mesh::new_rectangle(
            &self.stage_to_show_on.graphics_context(),
            DrawParam::default(),
            Rect::new(0.0, 0.0, self.max_popup_width, self.max_popup_height),
            darkened_color,
        ).unwrap());

        // Set up the tables based on scrollability
        match self.scrollability {
            Scrollability::None => {
                self.top_table = self.inner_table.clone();
                self.bottom_table = self.inner_table.clone();
            }
            Scrollability::All => {
                self.top_table = self.inner_table.clone();
                self.bottom_table = self.inner_table.clone();
                self.scroll_pane = Some(ScrollPane::new(self.inner_table.clone()));
            }
            Scrollability::WithoutButtons => {
                self.top_table = Table::new();
                self.top_table.pad(20.0, 0.0, 20.0, 0.0);
                self.top_table.defaults().fill_x().pad(5.0);
                self.bottom_table = Table::new();
                self.scroll_pane = Some(ScrollPane::new(self.top_table.clone()));
                self.inner_table.add(self.scroll_pane.as_ref().unwrap().clone());
                self.inner_table.defaults().fill_x();
                self.inner_table.row();
                self.inner_table.add(self.bottom_table.clone());
            }
        }

        self.bottom_table.pad(20.0);
        self.bottom_table.defaults().pad(5.0);
    }

    /// Recalculates the maximum height of the inner table
    fn recalculate_inner_table_max_height(&mut self) {
        if self.top_table.id() == self.bottom_table.id() {
            return;
        }
        // In Rust, we'd need to implement this based on the actual table implementation
    }

    /// Recalculates the maximum width of the inner table
    fn recalculate_inner_table_max_width(&mut self) {
        let min_width = self.inner_table.min_width().min(self.stage_to_show_on.width());
        if min_width < self.max_popup_width {
            return;
        }
        // In Rust, we'd need to implement this based on the actual table implementation
    }

    /// Displays the Popup on the screen
    pub fn open(&mut self, force: bool) {
        self.stage_to_show_on.add_widget(Box::new(self.clone()));
        self.recalculate_inner_table_max_height();
        self.inner_table.pack();
        self.recalculate_inner_table_max_width();
        self.pack();
        self.center();

        if force || !self.stage_to_show_on.has_open_popups() {
            self.show();
        }
    }

    /// Centers the popup on the screen
    fn center(&mut self) {
        let x = (self.stage_to_show_on.width() - self.width()) / 2.0;
        let y = (self.stage_to_show_on.height() - self.height()) / 2.0;
        self.set_position(x, y);
    }

    /// Shows the popup
    fn show(&mut self) {
        self.is_visible = true;
        for listener in &self.show_listeners {
            listener();
        }
    }

    /// Closes the popup
    pub fn close(&mut self) {
        for listener in &self.close_listeners {
            listener();
        }
        self.stage_to_show_on.remove_widget(self.id());

        // Show the next popup if any
        if let Some(next_popup) = self.stage_to_show_on.popups().last() {
            if let Some(popup) = next_popup.downcast_ref::<Popup>() {
                popup.show();
            }
        }
    }

    /// Adds a good sized label to the popup
    pub fn add_good_sized_label(&mut self, text: &str, size: i32, hide_icons: bool) -> &mut Self {
        let mut label = Label::new(text);
        label.set_wrap(true);
        label.set_alignment(ggez::graphics::Align::Center);
        label.set_font_size(size);
        label.set_hide_icons(hide_icons);

        self.add(label);
        self.width(self.stage_to_show_on.width() / 2.0);

        self
    }

    /// Adds a button to the popup
    pub fn add_button<F>(&mut self, text: &str, key: Option<char>, style: Option<&str>, action: F) -> &mut Self
    where
        F: Fn() + 'static,
    {
        let mut button = Button::new(text);
        if let Some(style) = style {
            button.set_style(style);
        }

        button.on_click(Box::new(action));
        if let Some(key) = key {
            button.add_key_shortcut(key);
        }

        self.bottom_table.add(button);

        self
    }

    /// Adds a close button to the popup
    pub fn add_close_button<F>(&mut self, text: &str, additional_key: Option<char>, style: Option<&str>, action: Option<F>) -> &mut Self
    where
        F: Fn() + 'static,
    {
        self.click_behind_to_close = true;

        if let Some(action) = action {
            self.on_close_callback = Some(Box::new(action));
        }

        let close_action = move || {
            self.close();
            if let Some(action) = &self.on_close_callback {
                action();
            }
        };

        self.add_button(text, additional_key, style, close_action);

        // Add back key shortcut
        if let Some(button) = self.bottom_table.children().last() {
            if let Some(button) = button.downcast_ref::<Button>() {
                button.add_key_shortcut('\u{1b}'); // ESC key
            }
        }

        self
    }

    /// Adds an OK button to the popup
    pub fn add_ok_button<F, V>(&mut self, text: &str, additional_key: Option<char>, style: Option<&str>, validate: V, action: F) -> &mut Self
    where
        F: Fn() + 'static,
        V: Fn() -> bool + 'static,
    {
        let ok_action = move || {
            if validate() {
                self.close();
                action();
            }
        };

        self.add_button(text, additional_key, style, ok_action);

        // Add return key shortcut
        if let Some(button) = self.bottom_table.children().last() {
            if let Some(button) = button.downcast_ref::<Button>() {
                button.add_key_shortcut('\n'); // Return key
            }
        }

        self
    }

    /// Equalizes the width of the last two buttons
    pub fn equalize_last_two_button_widths(&mut self) {
        let children = self.bottom_table.children();
        if children.len() < 2 {
            return;
        }

        let button1 = &children[children.len() - 2];
        let button2 = &children[children.len() - 1];

        if let (Some(button1), Some(button2)) = (
            button1.downcast_ref::<Button>(),
            button2.downcast_ref::<Button>()
        ) {
            let width1 = button1.width();
            let width2 = button2.width();

            button1.set_min_width(width2);
            button2.set_min_width(width1);
        }
    }

    /// Reuses this popup with a new text and optionally a close button
    pub fn reuse_with(&mut self, new_text: &str, with_close_button: bool) {
        self.clear();
        self.add_good_sized_label(new_text, DEFAULT_FONT_SIZE, false);

        if with_close_button {
            self.add_close_button("Close", None, None, None::<Box<dyn Fn()>>);
        }
    }

    /// Clears the popup
    pub fn clear(&mut self) {
        self.top_table.clear();
        self.bottom_table.clear();
        self.click_behind_to_close = false;
        self.on_close_callback = None;
    }

    /// Gets the scroll pane if any
    pub fn get_scroll_pane(&self) -> Option<&ScrollPane> {
        self.scroll_pane.as_ref()
    }

    /// Gets the keyboard focus
    pub fn keyboard_focus(&self) -> Option<&dyn Widget> {
        self.keyboard_focus.as_ref().map(|w| w.as_ref())
    }

    /// Sets the keyboard focus
    pub fn set_keyboard_focus(&mut self, widget: Option<Box<dyn Widget>>) {
        self.keyboard_focus = widget;

        if let Some(text_field) = self.keyboard_focus.as_ref().and_then(|w| w.downcast_ref::<TextField>()) {
            text_field.select_all();
        }
    }
}

impl Widget for Popup {
    fn id(&self) -> u64 {
        // Generate a unique ID for this popup
        0 // This would need to be implemented properly
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        if !self.is_visible {
            return Ok(());
        }

        // Draw the background
        if let Some(background) = &self.background {
            background.draw(ctx, DrawParam::default())?;
        }

        // Draw the inner table background
        if let Some(inner_table_background) = &self.inner_table_background {
            inner_table_background.draw(ctx, DrawParam::default())?;
        }

        // Draw the inner table
        self.inner_table.draw(ctx)?;

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if !self.is_visible {
            return Ok(());
        }

        // Update the inner table
        self.inner_table.update(ctx)?;

        Ok(())
    }

    fn width(&self) -> f32 {
        self.inner_table.width()
    }

    fn height(&self) -> f32 {
        self.inner_table.height()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.inner_table.set_position(x, y);
    }

    fn position(&self) -> Point2<f32> {
        self.inner_table.position()
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.touchable {
            return false;
        }

        self.inner_table.contains_point(x, y)
    }

    fn on_mouse_button_down(&mut self, button: MouseButton, x: f32, y: f32) -> bool {
        if !self.touchable {
            return false;
        }

        if button == MouseButton::Left {
            // Check if click is outside the inner table
            if !self.inner_table.contains_point(x, y) && self.click_behind_to_close {
                self.close();
                if let Some(callback) = &self.on_close_callback {
                    callback();
                }
                return true;
            }
        }

        self.inner_table.on_mouse_button_down(button, x, y)
    }

    fn on_mouse_button_up(&mut self, button: MouseButton, x: f32, y: f32) -> bool {
        if !self.touchable {
            return false;
        }

        self.inner_table.on_mouse_button_up(button, x, y)
    }

    fn on_mouse_motion(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> bool {
        if !self.touchable {
            return false;
        }

        self.inner_table.on_mouse_motion(x, y, dx, dy)
    }

    fn on_key_down(&mut self, keycode: ggez::input::keyboard::KeyCode, keymods: ggez::input::keyboard::KeyMods, repeat: bool) -> bool {
        self.inner_table.on_key_down(keycode, keymods, repeat)
    }

    fn on_key_up(&mut self, keycode: ggez::input::keyboard::KeyCode, keymods: ggez::input::keyboard::KeyMods) -> bool {
        self.inner_table.on_key_up(keycode, keymods)
    }

    fn on_text_input(&mut self, character: char) -> bool {
        self.inner_table.on_text_input(character)
    }
}

impl Clone for Popup {
    fn clone(&self) -> Self {
        // This would need to be implemented properly
        Self::new(
            self.stage_to_show_on.clone(),
            self.scrollability,
            self.max_size_percentage,
        )
    }
}

/// Extension trait for BaseScreen to add popup-related functionality
pub trait PopupExt {
    /// Returns a list of currently active or pending Popup screens
    fn popups(&self) -> Vec<&Popup>;

    /// Returns the currently active Popup or None if none
    fn active_popup(&self) -> Option<&Popup>;

    /// Checks if there are visible Popups
    fn has_open_popups(&self) -> bool;

    /// Closes all Popups
    fn close_all_popups(&mut self);
}

impl PopupExt for BaseScreen {
    fn popups(&self) -> Vec<&Popup> {
        self.widgets()
            .iter()
            .filter_map(|w| w.downcast_ref::<Popup>())
            .collect()
    }

    fn active_popup(&self) -> Option<&Popup> {
        self.popups().into_iter().find(|p| p.is_visible)
    }

    fn has_open_popups(&self) -> bool {
        self.popups().iter().any(|p| p.is_visible)
    }

    fn close_all_popups(&mut self) {
        for popup in self.popups() {
            popup.close();
        }
    }
}