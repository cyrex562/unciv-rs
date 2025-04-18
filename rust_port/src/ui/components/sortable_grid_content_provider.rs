use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use ggez::graphics::{Color, DrawParam, Drawable, Mesh, Rect, Text};
use ggez::Context;
use ggez::mint::Point2;
use ggez::input::mouse::MouseInput;
use ggez::event::MouseButton;

use crate::game::GameInfo;
use crate::ui::components::widgets::SortableGrid;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::surround_with_circle;
use crate::ui::components::extensions::to_label;

/// This defines all behaviour of a sortable Grid per column through overridable parts:
/// - `is_visible` can hide a column
/// - `align`, `fill_x`, `expand_x`, `equalize_height` control geometry
/// - `get_comparator` or `get_entry_value` control sorting, `default_sort` the initial order
/// - `get_header_actor`, `header_tip` and `header_tip_hide_icons` define how the header row looks
/// - `get_entry_value` or `get_entry_actor` define what the cells display
/// - `get_entry_value` or `get_totals_actor` define what the totals row displays
///
/// `IT` - The item type - what defines the row
/// `ACT` - Action context type - The Type of any object you need passed to `get_entry_actor` for potential OnClick calls
pub trait SortableGridContentProvider<IT, ACT> {
    /// Tooltip for the column header, typically overridden to default to enum name, will be auto-translated
    fn header_tip(&self) -> String;

    /// Passed to add_tooltip(hide_icons) - override to true to prevent autotranslation from inserting icons
    fn header_tip_hide_icons(&self) -> bool {
        false
    }

    /// Cell alignment - used on header, entry and total cells
    fn align(&self) -> i32;

    /// Cell fill_x - used on header, entry and total cells
    fn fill_x(&self) -> bool;

    /// Cell expand_x - used on header, entry and total cells
    fn expand_x(&self) -> bool;

    /// When overridden `true`, the entry cells of this column will be equalized to their max height
    fn equalize_height(&self) -> bool;

    /// Default sort direction when a column is first sorted - can be None to disable sorting entirely for this column.
    /// Relevant for visuals (simply inverting the comparator would leave the displayed arrow not matching)
    fn default_sort(&self) -> Option<SortableGrid::SortDirection>;

    /// Whether the column should be rendered
    fn is_visible(&self, game_info: &GameInfo) -> bool {
        true
    }

    /// Comparator Factory used for sorting.
    /// - The default will sort by `get_entry_value` ascending.
    /// Returns positive to sort second lambda argument before first lambda argument
    fn get_comparator(&self) -> Box<dyn Fn(&IT, &IT) -> Ordering> {
        Box::new(|a: &IT, b: &IT| {
            self.get_entry_value(a).cmp(&self.get_entry_value(b))
        })
    }

    /// Factory for the header cell Actor
    /// `icon_size` Suggestion for icon size passed down from SortableGrid constructor, intended to scale the grid header.
    /// If the actor is not an icon, treat as height.
    fn get_header_actor(&self, icon_size: f32) -> Option<Box<dyn Drawable>>;

    /// A getter for the numeric value to display in a cell
    fn get_entry_value(&self, item: &IT) -> i32;

    /// Factory for entry cell Actor
    /// - By default displays the (numeric) result of `get_entry_value`.
    /// - `action_context` can be used to define `on_click` actions.
    fn get_entry_actor(&self, item: &IT, icon_size: f32, action_context: &ACT) -> Option<Box<dyn Drawable>> {
        Some(to_label(self.get_entry_value(item).to_string(), Color::WHITE, 16, false))
    }

    /// Factory for totals cell Actor
    /// - By default displays the sum over `get_entry_value`.
    /// - Note a count may be meaningful even if entry cells display something other than a number,
    ///   In that case _not_ overriding this and supply a meaningful `get_entry_value` may be easier.
    /// - On the other hand, a sum may not be meaningful even if the cells are numbers - to leave
    ///   the total empty override to return `None`.
    fn get_totals_actor(&self, items: &[IT]) -> Option<Box<dyn Drawable>> {
        let sum: i32 = items.iter().map(|item| self.get_entry_value(item)).sum();
        Some(to_label(sum.to_string(), Color::WHITE, 16, false))
    }
}

impl<IT, ACT> SortableGridContentProvider<IT, ACT> {
    /// Get a circled icon with the specified path, size, and color
    pub fn get_circled_icon(path: &str, icon_size: f32, circle_color: Color) -> Box<dyn Drawable> {
        let image = ImageGetter::get_image(path);
        let mut image = image.clone();
        image.set_color(Color::new(0.2, 0.2, 0.2, 1.0)); // CHARCOAL color
        surround_with_circle(&image, icon_size, circle_color)
    }

    /// Convert an integer to a centered label
    pub fn int_to_centered_label(value: i32) -> Box<dyn Drawable> {
        let mut label = to_label(value.to_string(), Color::WHITE, 16, false);
        // Set alignment to center
        // In Rust/ggez, we would need to implement this differently
        // This is a placeholder for the actual implementation
        label
    }
}