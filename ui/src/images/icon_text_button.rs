use ggez::graphics::{self, Color, DrawParam, Image, Text};
use ggez::mint::Point2;
use ggez::Context;
use ggez::GameResult;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::ui::components::button::Button;
use crate::ui::components::fonts::Fonts;
use crate::ui::components::table::Table;
use crate::ui::components::table_cell::TableCell;
use crate::ui::screens::base_screen::BaseScreen;

/// A button that displays an optional icon and text label.
///
/// This class translates a string and makes a button widget from it, with control over
/// font size, font color, an optional icon, and custom formatting.
pub struct IconTextButton {
    button: Button,
    table: Table,
    icon_cell: Option<TableCell>,
    label_cell: TableCell,
    icon: Option<Box<dyn Actor>>,
    label: Text,
    font_size: i32,
    font_color: Color,
}

/// Trait for actors that can be drawn and positioned.
pub trait Actor: Send + Sync {
    /// Draws the actor.
    fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult;

    /// Gets the width of the actor.
    fn get_width(&self) -> f32;

    /// Gets the height of the actor.
    fn get_height(&self) -> f32;

    /// Sets the position of the actor.
    fn set_position(&mut self, x: f32, y: f32);

    /// Sets the size of the actor.
    fn set_size(&mut self, width: f32, height: f32);

    /// Sets the origin of the actor.
    fn set_origin(&mut self, x: f32, y: f32);
}

impl IconTextButton {
    /// Creates a new IconTextButton with the given text and optional parameters.
    pub fn new(
        text: &str,
        icon: Option<Box<dyn Actor>>,
        font_size: i32,
        font_color: Color,
    ) -> Self {
        let mut button = Button::new(BaseScreen::get_skin());
        let mut table = Table::new();

        // Create the label
        let label = Self::create_label(text, font_color, font_size, true);

        // Create the icon cell if an icon is provided
        let icon_cell = if let Some(icon_clone) = icon.clone() {
            let size = font_size as f32;
            let mut icon_actor = icon_clone;
            icon_actor.set_size(size, size);
            icon_actor.set_origin(size / 2.0, size / 2.0);

            let mut cell = TableCell::new(Some(Box::new(icon_actor)));
            cell.set_size(size, size);
            cell.set_padding_right(size / 3.0);
            Some(cell)
        } else {
            let mut cell = TableCell::new(None);
            cell.set_padding_right(font_size as f32 / 2.0);
            Some(cell)
        };

        // Create the label cell
        let mut label_cell = TableCell::new(Some(Box::new(TextActor::new(label.clone()))));

        // Add padding to the top of the label cell to align with the icon
        Self::pad_top_descent(&mut label_cell, font_size);

        // Add cells to the table
        if let Some(icon_cell_clone) = icon_cell.clone() {
            table.add_cell(icon_cell_clone);
        }
        table.add_cell(label_cell.clone());

        // Add padding to the table
        table.set_padding(10.0);

        // Add the table to the button
        button.set_child(Box::new(table.clone()));

        Self {
            button,
            table,
            icon_cell,
            label_cell,
            icon,
            label,
            font_size,
            font_color,
        }
    }

    /// Creates a label with the given text and formatting.
    fn create_label(text: &str, color: Color, font_size: i32, hide_icons: bool) -> Text {
        // In a real implementation, this would use a more sophisticated text creation method
        // that handles translations and formatting
        let mut text_obj = Text::new(text);
        text_obj.set_font_size(font_size as f32);
        text_obj.set_color(color);
        text_obj
    }

    /// Adds padding to the top of a label cell to align it with an icon.
    fn pad_top_descent(cell: &mut TableCell, font_size: i32) {
        let descender_height = Fonts::get_descender_height(font_size);
        cell.set_padding_top(descender_height);
    }

    /// Draws the IconTextButton.
    pub fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult {
        self.button.draw(ctx, parent_alpha)
    }

    /// Gets the width of the IconTextButton.
    pub fn get_width(&self) -> f32 {
        self.button.get_width()
    }

    /// Gets the height of the IconTextButton.
    pub fn get_height(&self) -> f32 {
        self.button.get_height()
    }

    /// Sets the position of the IconTextButton.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.button.set_position(x, y);
    }

    /// Sets the size of the IconTextButton.
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.button.set_size(width, height);
    }

    /// Sets the color of the IconTextButton.
    pub fn set_color(&mut self, color: Color) {
        self.font_color = color;
        self.label.set_color(color);
    }
}

/// A simple actor that draws a Text object.
struct TextActor {
    text: Text,
}

impl TextActor {
    /// Creates a new TextActor with the given text.
    fn new(text: Text) -> Self {
        Self { text }
    }
}

impl Actor for TextActor {
    fn draw(&self, ctx: &mut Context, parent_alpha: f32) -> GameResult {
        graphics::draw(
            ctx,
            &self.text,
            DrawParam::new()
                .color([1.0, 1.0, 1.0, parent_alpha]),
        )
    }

    fn get_width(&self) -> f32 {
        self.text.width()
    }

    fn get_height(&self) -> f32 {
        self.text.height()
    }

    fn set_position(&mut self, _x: f32, _y: f32) {
        // Text position is handled by the parent
    }

    fn set_size(&mut self, _width: f32, _height: f32) {
        // Text size is handled by font size
    }

    fn set_origin(&mut self, _x: f32, _y: f32) {
        // Text origin is handled by the parent
    }
}