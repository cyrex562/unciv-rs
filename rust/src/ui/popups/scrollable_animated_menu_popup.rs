use ggez::mint::Point2;
use std::rc::Rc;

use crate::ui::components::scroll_pane::ScrollPane;
use crate::ui::components::table::Table;
use crate::ui::screens::base_screen::BaseScreen;

use super::animated_menu_popup::AnimatedMenuPopup;

/// Adds (partial) scrollability to AnimatedMenuPopup.
///
/// Provide content by implementing `create_scrollable_content` and `create_fixed_content`.
/// If you need to modify outer wrapper styling, override `create_wrapper_table`.
pub trait ScrollableAnimatedMenuPopup: AnimatedMenuPopup {
    /// Creates the wrapper table for the popup content
    ///
    /// Override only to change styling.
    /// By default, a rounded edge dark gray background and 5.0 vertical / 15.0 horizontal padding for the two halves is used.
    fn create_wrapper_table(&self) -> Table;

    /// Provides the scrollable top part
    ///
    /// Returns `None` to abort opening the entire Popup
    fn create_scrollable_content(&self) -> Option<Table>;

    /// Provides the fixed bottom part
    ///
    /// Returns `None` to make the entire Popup scrollable (so the fixed part takes no vertical space, not even the default padding)
    fn create_fixed_content(&self) -> Option<Table>;

    /// Determines maximum usable width
    ///
    /// Use `stage_to_show_on` to measure the Stage (from the underlying Popup).
    /// Do not use `stage`, it is uninitialized at this point.
    fn max_popup_width(&self) -> f32 {
        0.95 * self.stage_to_show_on().width() - 5.0
    }

    /// Determines maximum usable height
    ///
    /// Use `stage_to_show_on` to measure the Stage (from the underlying Popup).
    /// Do not use `stage`, it is uninitialized at this point.
    fn max_popup_height(&self) -> f32 {
        0.95 * self.stage_to_show_on().height() - 5.0
    }

    /// Creates the content table for the popup
    ///
    /// This method is overridden to provide scrollability.
    fn create_content_table(&self) -> Option<Table> {
        // Get the scrollable content
        let top = self.create_scrollable_content()?;

        // Build content by wrapping scrollable and fixed parts
        let mut table = self.create_wrapper_table();
        let scroll = ScrollPane::new(top);
        let scroll_cell = table.add(scroll).grow_x();
        table.row();

        // Add the fixed content if any
        if let Some(bottom) = self.create_fixed_content() {
            table.add(bottom);
        }

        // ScrollBars need to be told their size
        let padded_max_height = self.max_popup_height();
        let desired_total_height = table.pref_height();
        let desired_scroll_height = table.get_row_pref_height(0);
        let scroll_height = if desired_total_height <= padded_max_height {
            desired_scroll_height
        } else {
            padded_max_height - (desired_total_height - desired_scroll_height)
        };

        let padded_max_width = self.max_popup_width();
        let desired_total_width = table.pref_width();
        let desired_content_width = table.get_column_pref_width(0);
        let scroll_width = if desired_total_width <= padded_max_height {
            desired_content_width
        } else {
            padded_max_width - (desired_total_width - desired_content_width)
        };

        scroll_cell.size(scroll_width, scroll_height);

        Some(table)
    }
}

/// A concrete implementation of ScrollableAnimatedMenuPopup
pub struct ScrollableAnimatedMenuPopupImpl {
    /// The base AnimatedMenuPopup
    base: AnimatedMenuPopup,
    /// The scrollable content creator
    scrollable_content_creator: Box<dyn Fn() -> Option<Table>>,
    /// The fixed content creator
    fixed_content_creator: Box<dyn Fn() -> Option<Table>>,
    /// The wrapper table creator
    wrapper_table_creator: Box<dyn Fn() -> Table>,
}

impl ScrollableAnimatedMenuPopupImpl {
    /// Creates a new ScrollableAnimatedMenuPopupImpl
    pub fn new(
        stage: Rc<BaseScreen>,
        position: Point2<f32>,
        scrollable_content_creator: Box<dyn Fn() -> Option<Table>>,
        fixed_content_creator: Box<dyn Fn() -> Option<Table>>,
        wrapper_table_creator: Option<Box<dyn Fn() -> Table>>,
    ) -> Self {
        Self {
            base: AnimatedMenuPopup::new(stage.clone(), position),
            scrollable_content_creator,
            fixed_content_creator,
            wrapper_table_creator: wrapper_table_creator.unwrap_or_else(|| {
                Box::new(move || {
                    let mut table = Table::new();
                    table.pad(5.0, 15.0, 5.0, 15.0);
                    table
                })
            }),
        }
    }
}

impl AnimatedMenuPopup for ScrollableAnimatedMenuPopupImpl {
    fn stage_to_show_on(&self) -> &Rc<BaseScreen> {
        self.base.stage_to_show_on()
    }

    fn position(&self) -> Point2<f32> {
        self.base.position()
    }

    fn create_content_table(&self) -> Option<Table> {
        // Get the scrollable content
        let top = (self.scrollable_content_creator)()?;

        // Build content by wrapping scrollable and fixed parts
        let mut table = (self.wrapper_table_creator)();
        let scroll = ScrollPane::new(top);
        let scroll_cell = table.add(scroll).grow_x();
        table.row();

        // Add the fixed content if any
        if let Some(bottom) = (self.fixed_content_creator)() {
            table.add(bottom);
        }

        // ScrollBars need to be told their size
        let padded_max_height = self.max_popup_height();
        let desired_total_height = table.pref_height();
        let desired_scroll_height = table.get_row_pref_height(0);
        let scroll_height = if desired_total_height <= padded_max_height {
            desired_scroll_height
        } else {
            padded_max_height - (desired_total_height - desired_scroll_height)
        };

        let padded_max_width = self.max_popup_width();
        let desired_total_width = table.pref_width();
        let desired_content_width = table.get_column_pref_width(0);
        let scroll_width = if desired_total_width <= padded_max_height {
            desired_content_width
        } else {
            padded_max_width - (desired_total_width - desired_content_width)
        };

        scroll_cell.size(scroll_width, scroll_height);

        Some(table)
    }
}

impl ScrollableAnimatedMenuPopup for ScrollableAnimatedMenuPopupImpl {
    fn create_wrapper_table(&self) -> Table {
        (self.wrapper_table_creator)()
    }

    fn create_scrollable_content(&self) -> Option<Table> {
        (self.scrollable_content_creator)()
    }

    fn create_fixed_content(&self) -> Option<Table> {
        (self.fixed_content_creator)()
    }
}