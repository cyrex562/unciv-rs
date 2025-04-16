use ggez::graphics::{Color, DrawParam, Mesh, MeshBuilder, Text};
use ggez::mint::Point2;
use std::collections::HashMap;

use crate::models::stats::Stats;
use crate::ui::components::extensions::{add_to_center, surround_with_circle, to_label};
use crate::ui::images::ImageGetter;

/// A horizontal group for displaying yield statistics
pub struct YieldGroup {
    /// The current stats being displayed
    current_stats: Stats,

    /// The children of this group
    children: Vec<Box<dyn Drawable>>,

    /// The position of this group
    position: Point2<f32>,

    /// The size of this group
    size: Point2<f32>,

    /// Whether this group is visible
    visible: bool,

    /// Whether this group is enabled
    enabled: bool,

    /// Whether this group is transformed
    is_transform: bool,
}

/// Trait for drawable objects
pub trait Drawable: Send + Sync {
    /// Draws this object
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult;

    /// Gets the position of this object
    fn position(&self) -> Point2<f32>;

    /// Sets the position of this object
    fn set_position(&mut self, position: Point2<f32>);

    /// Gets the size of this object
    fn size(&self) -> Point2<f32>;

    /// Sets the size of this object
    fn set_size(&mut self, size: Point2<f32>);

    /// Gets whether this object is visible
    fn visible(&self) -> bool;

    /// Sets whether this object is visible
    fn set_visible(&mut self, visible: bool);

    /// Gets whether this object is enabled
    fn enabled(&self) -> bool;

    /// Sets whether this object is enabled
    fn set_enabled(&mut self, enabled: bool);
}

impl YieldGroup {
    /// Creates a new YieldGroup
    pub fn new() -> Self {
        Self {
            current_stats: Stats::new(),
            children: Vec::new(),
            position: Point2 { x: 0.0, y: 0.0 },
            size: Point2 { x: 0.0, y: 0.0 },
            visible: true,
            enabled: true,
            is_transform: false, // performance helper - nothing here is rotated or scaled
        }
    }

    /// Sets the stats to display
    pub fn set_stats(&mut self, stats: &Stats) {
        // Don't update if the stats are the same
        if self.current_stats.equals(stats) {
            return; // don't need to update - this is a memory and time saver!
        }

        self.current_stats = stats.clone();
        self.clear_children();

        // Add a stat icon table for each stat with a positive amount
        for (stat, amount) in stats.iter() {
            if amount > 0.0 { // Defense against upstream bugs - negatives would show as "lots"
                self.add_child(Box::new(self.get_stat_icons_table(stat.name(), amount as i32)));
            }
        }

        self.pack();
    }

    /// Gets an icon for the given stat name
    pub fn get_icon(&self, stat_name: &str) -> Box<dyn Drawable> {
        let mut icon = ImageGetter::get_stat_icon(stat_name);
        let mut circle = surround_with_circle(&mut icon, 12.0, Some("StatIcons/Circle"));
        circle.set_color(Color::new(0.2, 0.2, 0.2, 0.5)); // CHARCOAL with alpha 0.5
        Box::new(circle)
    }

    /// Gets a table of stat icons for the given stat name and number
    fn get_stat_icons_table(&self, stat_name: &str, number: i32) -> StatIconsTable {
        let mut table = StatIconsTable::new();

        match number {
            1 => {
                table.add(self.get_icon(stat_name));
            }
            2 => {
                table.add(self.get_icon(stat_name));
                table.add_row();
                table.add(self.get_icon(stat_name));
            }
            3 => {
                table.add_with_colspan(self.get_icon(stat_name), 2);
                table.add_row();
                table.add(self.get_icon(stat_name));
                table.add(self.get_icon(stat_name));
            }
            4 => {
                table.add(self.get_icon(stat_name));
                table.add(self.get_icon(stat_name));
                table.add_row();
                table.add(self.get_icon(stat_name));
                table.add(self.get_icon(stat_name));
            }
            _ => {
                let mut group = StatGroup::new(22.0, 22.0);
                let mut large_image = ImageGetter::get_stat_icon(stat_name);
                let mut circle = surround_with_circle(&mut large_image, 22.0, None);
                circle.set_color(Color::new(0.2, 0.2, 0.2, 0.5)); // CHARCOAL with alpha 0.5
                add_to_center(&mut group, Box::new(circle));

                if number > 5 {
                    let text = if number < 10 {
                        number.to_string()
                    } else {
                        "*".to_string()
                    };

                    let mut label = to_label(&text, 8, Color::WHITE, ggez::graphics::Align::Center);
                    let mut amount_group = surround_with_circle(&mut label, 10.0, Some("StatIcons/Circle"));
                    amount_group.set_color(Color::new(0.2, 0.2, 0.2, 1.0)); // CHARCOAL

                    // Adjust position
                    let mut pos = amount_group.position();
                    pos.y -= 0.5;
                    amount_group.set_position(pos);

                    let mut group_pos = group.position();
                    let mut amount_pos = amount_group.position();
                    amount_pos.x = group.size().x - amount_group.size().x * 3.0 / 4.0;
                    amount_pos.y = -amount_group.size().y / 4.0;
                    amount_group.set_position(amount_pos);

                    group.add_child(Box::new(amount_group));
                }

                table.add(Box::new(group));
            }
        }

        table.pack();
        table
    }

    /// Clears all children
    fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Adds a child
    fn add_child(&mut self, child: Box<dyn Drawable>) {
        self.children.push(child);
    }

    /// Packs the group
    fn pack(&mut self) {
        // Calculate the total width and maximum height
        let mut total_width = 0.0;
        let mut max_height = 0.0;

        for child in &self.children {
            let child_size = child.size();
            total_width += child_size.x;
            max_height = max_height.max(child_size.y);
        }

        // Set the size
        self.size = Point2 { x: total_width, y: max_height };

        // Position the children
        let mut x = 0.0;
        for child in &mut self.children {
            let mut pos = child.position();
            pos.x = x;
            child.set_position(pos);
            x += child.size().x;
        }
    }

    /// Draws the group
    pub fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        if !this.visible {
            return Ok(());
        }

        for child in &this.children {
            child.draw(ctx, param)?;
        }

        Ok(())
    }
}

/// A table for displaying stat icons
pub struct StatIconsTable {
    /// The children of this table
    children: Vec<Box<dyn Drawable>>,

    /// The position of this table
    position: Point2<f32>,

    /// The size of this table
    size: Point2<f32>,

    /// Whether this table is visible
    visible: bool,

    /// Whether this table is enabled
    enabled: bool,

    /// The current row
    current_row: Vec<Box<dyn Drawable>>,

    /// The rows of this table
    rows: Vec<Vec<Box<dyn Drawable>>>,

    /// The column spans of the current row
    column_spans: Vec<i32>,
}

impl StatIconsTable {
    /// Creates a new StatIconsTable
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            position: Point2 { x: 0.0, y: 0.0 },
            size: Point2 { x: 0.0, y: 0.0 },
            visible: true,
            enabled: true,
            current_row: Vec::new(),
            rows: Vec::new(),
            column_spans: Vec::new(),
        }
    }

    /// Adds a child to the current row
    pub fn add(&mut self, child: Box<dyn Drawable>) {
        self.current_row.push(child);
        self.column_spans.push(1);
    }

    /// Adds a child with a column span to the current row
    pub fn add_with_colspan(&mut self, child: Box<dyn Drawable>, colspan: i32) {
        self.current_row.push(child);
        self.column_spans.push(colspan);
    }

    /// Adds a row
    pub fn add_row(&mut self) {
        if !self.current_row.is_empty() {
            self.rows.push(self.current_row.clone());
            self.current_row.clear();
            self.column_spans.clear();
        }
    }

    /// Packs the table
    pub fn pack(&mut self) {
        // Add the current row if it's not empty
        if !self.current_row.is_empty() {
            self.rows.push(self.current_row.clone());
        }

        // Calculate the total width and height
        let mut total_width = 0.0;
        let mut total_height = 0.0;

        for row in &self.rows {
            let mut row_width = 0.0;
            let mut row_height = 0.0;

            for child in row {
                let child_size = child.size();
                row_width += child_size.x;
                row_height = row_height.max(child_size.y);
            }

            total_width = total_width.max(row_width);
            total_height += row_height;
        }

        // Set the size
        self.size = Point2 { x: total_width, y: total_height };

        // Position the children
        let mut y = 0.0;
        for row in &mut self.rows {
            let mut x = 0.0;
            let mut row_height = 0.0;

            for child in row {
                let mut pos = child.position();
                pos.x = x;
                pos.y = y;
                child.set_position(pos);

                let child_size = child.size();
                x += child_size.x;
                row_height = row_height.max(child_size.y);
            }

            y += row_height;
        }
    }
}

impl Drawable for StatIconsTable {
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        if !self.visible {
            return Ok(());
        }

        for row in &self.rows {
            for child in row {
                child.draw(ctx, param)?;
            }
        }

        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.position
    }

    fn set_position(&mut self, position: Point2<f32>) {
        self.position = position;
    }

    fn size(&self) -> Point2<f32> {
        self.size
    }

    fn set_size(&mut self, size: Point2<f32>) {
        self.size = size;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// A group for displaying a stat
pub struct StatGroup {
    /// The children of this group
    children: Vec<Box<dyn Drawable>>,

    /// The position of this group
    position: Point2<f32>,

    /// The size of this group
    size: Point2<f32>,

    /// Whether this group is visible
    visible: bool,

    /// Whether this group is enabled
    enabled: bool,
}

impl StatGroup {
    /// Creates a new StatGroup with the given width and height
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            children: Vec::new(),
            position: Point2 { x: 0.0, y: 0.0 },
            size: Point2 { x: width, y: height },
            visible: true,
            enabled: true,
        }
    }

    /// Adds a child
    pub fn add_child(&mut self, child: Box<dyn Drawable>) {
        self.children.push(child);
    }
}

impl Drawable for StatGroup {
    fn draw(&self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        if !self.visible {
            return Ok(());
        }

        for child in &self.children {
            child.draw(ctx, param)?;
        }

        Ok(())
    }

    fn position(&self) -> Point2<f32> {
        self.position
    }

    fn set_position(&mut self, position: Point2<f32>) {
        self.position = position;
    }

    fn size(&self) -> Point2<f32> {
        self.size
    }

    fn set_size(&mut self, size: Point2<f32>) {
        self.size = size;
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}