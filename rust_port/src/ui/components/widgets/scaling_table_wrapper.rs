use ggez::graphics::{DrawParam, Rect};
use ggez::mint::Point2;
use std::sync::Arc;

use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::components::widgets::cell::Cell;
use crate::ui::components::widgets::drawable::Drawable;

/// Allows inserting a scaled Table into another, such that the outer Table "sees" correct inner Table dimensions.
///
/// Note: Delegates only the basic Table API: background, columns, rows, add, row and defaults.
/// Add to these as needed.
pub struct ScalingTableWrapper {
    /// The minimum scale factor allowed
    min_scale: f32,
    /// The inner table that will be scaled
    inner_table: Table,
    /// The current width of the wrapper
    width: f32,
    /// The current height of the wrapper
    height: f32,
}

impl ScalingTableWrapper {
    /// Creates a new ScalingTableWrapper with the given minimum scale
    pub fn new(min_scale: f32) -> Self {
        Self {
            min_scale,
            inner_table: Table::new(),
            width: 0.0,
            height: 0.0,
        }
    }

    /// Resets the scale of the inner table to 1.0
    pub fn reset_scale(&mut self) {
        self.inner_table.set_scale(1.0);
        self.inner_table.set_transform(false);
    }

    /// Scales the inner table to fit within the given maximum width
    pub fn scale_to(&mut self, max_width: f32) {
        self.inner_table.pack();
        let scale = (max_width / self.inner_table.get_pref_width()).clamp(self.min_scale, 1.0);
        if scale >= 1.0 {
            return;
        }
        self.inner_table.set_transform(true);
        self.inner_table.set_scale(scale);
        if !self.inner_table.needs_layout() {
            self.inner_table.invalidate();
            self.invalidate();
        }
    }

    /// Gets a reference to the inner table
    pub fn inner_table(&self) -> &Table {
        &self.inner_table
    }

    /// Gets a mutable reference to the inner table
    pub fn inner_table_mut(&mut self) -> &mut Table {
        &mut self.inner_table
    }

    /// Sets the background of the inner table
    pub fn set_background(&mut self, background: Option<Arc<dyn Drawable>>) {
        self.inner_table.set_background(background);
    }

    /// Gets the background of the inner table
    pub fn background(&self) -> Option<&Arc<dyn Drawable>> {
        self.inner_table.background()
    }

    /// Gets the number of columns in the inner table
    pub fn columns(&self) -> i32 {
        self.inner_table.columns()
    }

    /// Gets the number of rows in the inner table
    pub fn rows(&self) -> i32 {
        self.inner_table.rows()
    }

    /// Gets the defaults cell of the inner table
    pub fn defaults(&mut self) -> &mut Cell {
        self.inner_table.defaults()
    }

    /// Adds a widget to the inner table
    pub fn add<W: Widget + 'static>(&mut self, widget: Arc<W>) -> &mut Cell {
        self.inner_table.add(widget)
    }

    /// Adds an empty cell to the inner table
    pub fn add_empty(&mut self) -> &mut Cell {
        self.inner_table.add_empty()
    }

    /// Adds a new row to the inner table
    pub fn row(&mut self) -> &mut Cell {
        self.inner_table.row()
    }
}

impl WidgetGroup for ScalingTableWrapper {
    fn add_child<W: Widget + 'static>(&mut self, child: Arc<W>) {
        // The inner table is the only child
        if Arc::ptr_eq(&child, &Arc::new(self.inner_table.clone())) {
            return;
        }
        // Otherwise, add to the inner table
        self.inner_table.add(child);
    }

    fn remove_child<W: Widget + 'static>(&mut self, child: &Arc<W>) -> bool {
        // Can only remove the inner table
        if Arc::ptr_eq(child, &Arc::new(self.inner_table.clone())) {
            return true;
        }
        false
    }

    fn children(&self) -> &[Arc<dyn Widget>] {
        // Return a slice containing only the inner table
        std::slice::from_ref(&Arc::new(self.inner_table.clone()) as &Arc<dyn Widget>)
    }

    fn children_mut(&mut self) -> &mut [Arc<dyn Widget>] {
        // This is a bit tricky since we need to return a mutable slice
        // In practice, this should never be called since we only have one child
        unimplemented!("ScalingTableWrapper only supports the inner table as a child")
    }
}

impl Widget for ScalingTableWrapper {
    fn draw(&mut self, ctx: &mut ggez::Context, param: DrawParam) -> ggez::GameResult {
        self.inner_table.draw(ctx, param)
    }

    fn update(&mut self, ctx: &mut ggez::Context, delta: f32) -> ggez::GameResult {
        self.inner_table.update(ctx, delta)
    }

    fn get_min_width(&self) -> f32 {
        self.inner_table.get_min_width() * self.inner_table.scale_x()
    }

    fn get_pref_width(&self) -> f32 {
        self.inner_table.get_pref_width() * self.inner_table.scale_x()
    }

    fn get_max_width(&self) -> f32 {
        self.inner_table.get_pref_width()
    }

    fn get_min_height(&self) -> f32 {
        self.inner_table.get_min_height() * self.inner_table.scale_y()
    }

    fn get_pref_height(&self) -> f32 {
        self.inner_table.get_pref_height() * self.inner_table.scale_y()
    }

    fn get_max_height(&self) -> f32 {
        self.inner_table.get_pref_height()
    }

    fn layout(&mut self) {
        // Set the bounds of the inner table based on the wrapper's size and the inner table's scale
        self.inner_table.set_bounds(
            Rect::new(
                0.0,
                0.0,
                self.width / self.inner_table.scale_x(),
                self.height / self.inner_table.scale_y(),
            ),
        );
    }

    fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.layout();
    }

    fn invalidate(&mut self) {
        self.inner_table.invalidate();
    }
}

impl Clone for ScalingTableWrapper {
    fn clone(&self) -> Self {
        Self {
            min_scale: self.min_scale,
            inner_table: self.inner_table.clone(),
            width: self.width,
            height: self.height,
        }
    }
}