use ggez::{
    graphics::{Color, DrawParam, Drawable, Mesh, Rect, Text},
    input::keyboard::KeyCode,
    Context, GameResult,
};
use std::collections::HashMap;

// Constants
const DEFAULT_SEPARATOR_HEIGHT: f32 = 1.0;
const DEFAULT_SEPARATOR_WIDTH: f32 = 2.0;

// Color utilities
pub fn color_from_hex(hex_color: u32) -> Color {
    let r = ((hex_color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex_color >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex_color & 0xFF) as f32 / 255.0;
    Color::new(r, g, b, 1.0)
}

pub fn color_from_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

pub trait ColorExt {
    fn darken(&self, t: f32) -> Color;
    fn brighten(&self, t: f32) -> Color;
}

impl ColorExt for Color {
    fn darken(&self, t: f32) -> Color {
        Color::new(
            self.r * (1.0 - t),
            self.g * (1.0 - t),
            self.b * (1.0 - t),
            self.a,
        )
    }

    fn brighten(&self, t: f32) -> Color {
        Color::new(
            self.r + (1.0 - self.r) * t,
            self.g + (1.0 - self.g) * t,
            self.b + (1.0 - self.b) * t,
            self.a,
        )
    }
}

// Actor trait for common functionality
pub trait Actor: Drawable {
    fn get_x(&self) -> f32;
    fn get_y(&self) -> f32;
    fn get_width(&self) -> f32;
    fn get_height(&self) -> f32;
    fn set_position(&mut self, x: f32, y: f32);
    fn center_x(&mut self, parent_width: f32);
    fn center_y(&mut self, parent_height: f32);
    fn center(&mut self, parent_width: f32, parent_height: f32);
}

impl<T: Actor> ActorExt for T {}

pub trait ActorExt: Actor {
    fn surround_with_circle(
        &self,
        size: f32,
        resize_actor: bool,
        color: Color,
    ) -> CircleGroup {
        CircleGroup::new(size, self, resize_actor, color)
    }

    fn add_border(&self, size: f32, color: Color, expand_cell: bool) -> Table {
        let mut table = Table::new();
        table.set_padding(size);
        table.set_background(color);

        let cell = table.add_actor(self);
        if expand_cell {
            cell.expand();
        }
        cell.fill();
        table.pack();
        table
    }
}

// Table implementation
pub struct Table {
    actors: Vec<Box<dyn Actor>>,
    padding: f32,
    background_color: Option<Color>,
    cells: Vec<Cell>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            actors: Vec::new(),
            padding: 0.0,
            background_color: None,
            cells: Vec::new(),
        }
    }

    pub fn set_padding(&mut self, padding: f32) {
        self.padding = padding;
    }

    pub fn set_background(&mut self, color: Color) {
        self.background_color = Some(color);
    }

    pub fn add_actor<T: Actor + 'static>(&mut self, actor: T) -> Cell {
        let cell = Cell::new(Box::new(actor));
        self.cells.push(cell.clone());
        cell
    }

    pub fn add_separator(
        &mut self,
        color: Color,
        col_span: usize,
        height: f32,
    ) -> Cell<Image> {
        let separator = Image::new_separator(color, height);
        let cell = self.add_actor(separator)
            .colspan(col_span)
            .height(height)
            .fill_x();
        cell
    }

    pub fn add_separator_vertical(
        &mut self,
        color: Color,
        width: f32,
    ) -> Cell<Image> {
        let separator = Image::new_separator(color, width);
        self.add_actor(separator)
            .width(width)
            .fill_y()
    }

    pub fn pack(&mut self) {
        // Implementation for packing the table layout
    }
}

// Cell implementation
#[derive(Clone)]
pub struct Cell {
    actor: Box<dyn Actor>,
    colspan: usize,
    rowspan: usize,
    width: Option<f32>,
    height: Option<f32>,
    expand_x: bool,
    expand_y: bool,
    fill_x: bool,
    fill_y: bool,
}

impl Cell {
    pub fn new(actor: Box<dyn Actor>) -> Self {
        Self {
            actor,
            colspan: 1,
            rowspan: 1,
            width: None,
            height: None,
            expand_x: false,
            expand_y: false,
            fill_x: false,
            fill_y: false,
        }
    }

    pub fn colspan(mut self, colspan: usize) -> Self {
        self.colspan = colspan;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn fill_x(mut self) -> Self {
        self.fill_x = true;
        self
    }

    pub fn fill_y(mut self) -> Self {
        self.fill_y = true;
        self
    }

    pub fn expand(mut self) -> Self {
        self.expand_x = true;
        self.expand_y = true;
        self
    }
}

// Image implementation
pub struct Image {
    mesh: Mesh,
    color: Color,
}

impl Image {
    pub fn new_separator(color: Color, size: f32) -> Self {
        // Create a simple rectangle mesh for the separator
        let mesh = Mesh::new_rectangle(
            // Context would be needed here, but we'll handle that in the actual implementation
            DrawMode::Fill,
            Rect::new(0.0, 0.0, size, size),
            color,
        ).expect("Failed to create separator mesh");

        Self {
            mesh,
            color,
        }
    }
}

impl Actor for Image {
    fn get_x(&self) -> f32 {
        // Implementation
        0.0
    }

    fn get_y(&self) -> f32 {
        // Implementation
        0.0
    }

    fn get_width(&self) -> f32 {
        // Implementation
        0.0
    }

    fn get_height(&self) -> f32 {
        // Implementation
        0.0
    }

    fn set_position(&mut self, x: f32, y: f32) {
        // Implementation
    }

    fn center_x(&mut self, parent_width: f32) {
        let x = (parent_width - self.get_width()) / 2.0;
        self.set_position(x, self.get_y());
    }

    fn center_y(&mut self, parent_height: f32) {
        let y = (parent_height - self.get_height()) / 2.0;
        self.set_position(self.get_x(), y);
    }

    fn center(&mut self, parent_width: f32, parent_height: f32) {
        self.center_x(parent_width);
        self.center_y(parent_height);
    }
}

// CircleGroup implementation
pub struct CircleGroup {
    size: f32,
    actor: Box<dyn Actor>,
    resize_actor: bool,
    color: Color,
}

impl CircleGroup {
    pub fn new(
        size: f32,
        actor: &dyn Actor,
        resize_actor: bool,
        color: Color,
    ) -> Self {
        Self {
            size,
            actor: Box::new(actor.clone()),
            resize_actor,
            color,
        }
    }
}

impl Actor for CircleGroup {
    // Implement Actor trait methods
    // ... (similar to Image implementation)
}

// Button implementation
pub struct Button {
    enabled: bool,
    style: ButtonStyle,
}

pub struct ButtonStyle {
    normal: Color,
    disabled: Color,
}

impl Button {
    pub fn new() -> Self {
        Self {
            enabled: true,
            style: ButtonStyle {
                normal: Color::WHITE,
                disabled: Color::new(0.5, 0.5, 0.5, 1.0),
            },
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// Rectangle extensions
pub trait RectangleExt {
    fn get_overlap(&self, other: &Rect) -> Option<Rect>;
    fn top(&self) -> f32;
    fn right(&self) -> f32;
}

impl RectangleExt for Rect {
    fn get_overlap(&self, other: &Rect) -> Option<Rect> {
        let overlap_x = self.x.max(other.x);
        let overlap_y = self.y.max(other.y);
        let overlap_width = (self.x + self.w).min(other.x + other.w) - overlap_x;
        let overlap_height = (self.y + self.h).min(other.y + other.h) - overlap_y;

        if overlap_width <= 0.0 || overlap_height <= 0.0 {
            None
        } else {
            Some(Rect::new(overlap_x, overlap_y, overlap_width, overlap_height))
        }
    }

    fn top(&self) -> f32 {
        self.y + self.h
    }

    fn right(&self) -> f32 {
        self.x + self.w
    }
}