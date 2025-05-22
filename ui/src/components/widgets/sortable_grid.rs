use ggez::graphics::{Color, DrawParam};
use ggez::mint::Point2;
use std::collections::HashMap;
use std::sync::Arc;
use std::cmp::Ordering;

use crate::constants::Constants;
use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::components::widgets::cell::Cell;
use crate::ui::components::widgets::label::Label;
use crate::ui::components::widgets::horizontal_group::HorizontalGroup;
use crate::ui::components::widgets::non_transform_group::NonTransformGroup;
use crate::ui::components::widgets::drawable::Drawable;
use crate::ui::components::widgets::actor::Actor;
use crate::ui::components::widgets::layout::Layout;
use crate::ui::components::widgets::image::Image;
use crate::ui::components::widgets::icon_circle_group::IconCircleGroup;
use crate::ui::components::widgets::tooltip::Tooltip;
use crate::ui::components::widgets::font::Fonts;
use crate::ui::components::widgets::widget_group::WidgetGroup;
use crate::ui::screens::basescreen::BaseScreen;

/// The direction a column may be sorted in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Natural order of underlying data - only available before using any sort-click
    None,
    /// Ascending order
    Ascending,
    /// Descending order
    Descending,
}

impl SortDirection {
    /// Returns the inverted sort direction
    fn inverted(&self, default_sort: SortDirection) -> SortDirection {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
            SortDirection::None => default_sort,
        }
    }
}

/// Defines what is needed to remember the sorting state of the grid
pub trait ISortState<CT> {
    /// Stores the column this grid is currently sorted by
    fn sorted_by(&self) -> &CT;
    /// Sets the column this grid is currently sorted by
    fn set_sorted_by(&mut self, column: CT);
    /// Stores the direction column sorted_by is sorted in
    fn direction(&self) -> SortDirection;
    /// Sets the direction column sorted_by is sorted in
    fn set_direction(&mut self, direction: SortDirection);
}

/// Default implementation used as default for the sort_state parameter
pub struct SortState<CT> {
    /// The column this grid is currently sorted by
    sorted_by: CT,
    /// The direction column sorted_by is sorted in
    direction: SortDirection,
}

impl<CT> SortState<CT> {
    /// Creates a new SortState with the given default column
    pub fn new(default: CT) -> Self {
        Self {
            sorted_by: default,
            direction: SortDirection::None,
        }
    }
}

impl<CT> ISortState<CT> for SortState<CT> {
    fn sorted_by(&self) -> &CT {
        &self.sorted_by
    }

    fn set_sorted_by(&mut self, column: CT) {
        self.sorted_by = column;
    }

    fn direction(&self) -> SortDirection {
        self.direction
    }

    fn set_direction(&mut self, direction: SortDirection) {
        self.direction = direction;
    }
}

/// Interface for sortable grid content providers
pub trait ISortableGridContentProvider<IT, ACT> {
    /// Gets the header actor for this column
    fn get_header_actor(&self, icon_size: f32) -> Option<Arc<dyn Actor>>;

    /// Gets the entry actor for this column and item
    fn get_entry_actor(&self, item: &IT, icon_size: f32, action_context: &ACT) -> Option<Arc<dyn Actor>>;

    /// Gets the totals actor for this column
    fn get_totals_actor(&self, data: &[IT]) -> Arc<dyn Actor>;

    /// Gets the comparator for this column
    fn get_comparator(&self) -> Box<dyn Fn(&IT, &IT) -> Ordering>;

    /// Gets the default sort direction for this column
    fn default_sort(&self) -> SortDirection;

    /// Gets the alignment for this column
    fn align(&self) -> Point2;

    /// Gets whether this column should fill horizontally
    fn fill_x(&self) -> bool;

    /// Gets whether this column should expand horizontally
    fn expand_x(&self) -> bool;

    /// Gets whether this column should equalize height
    fn equalize_height(&self) -> bool;

    /// Gets the header tip for this column
    fn header_tip(&self) -> String;

    /// Gets whether to hide icons in the header tip
    fn header_tip_hide_icons(&self) -> bool;
}

/// Wrap icon, label or other Actor and sort symbol for a header cell
pub trait IHeaderElement {
    /// The outer actor that goes into the actual header
    fn outer_actor(&self) -> &Arc<dyn Actor>;

    /// The header actor
    fn header_actor(&self) -> Option<&Arc<dyn Actor>>;

    /// The current sort direction shown
    fn sort_shown(&self) -> SortDirection;

    /// Sets the current sort direction shown
    fn set_sort_shown(&mut self, direction: SortDirection);

    /// Show or remove the sort symbol
    fn set_sort_state(&mut self, new_sort: SortDirection);

    /// Remove the sort symbol
    fn remove_sort_symbol(&mut self, sort_symbol: &Arc<Label>);

    /// Show the sort symbol
    fn show_sort_symbol(&mut self, sort_symbol: &Arc<Label>);

    /// Size the cell
    fn size_cell(&self, cell: &mut Cell);
}

/// Empty header element
pub struct EmptyHeaderElement {
    outer_actor: Arc<dyn Actor>,
    sort_shown: SortDirection,
}

impl EmptyHeaderElement {
    /// Creates a new EmptyHeaderElement
    pub fn new() -> Self {
        Self {
            outer_actor: Arc::new(Actor::new()),
            sort_shown: SortDirection::None,
        }
    }
}

impl IHeaderElement for EmptyHeaderElement {
    fn outer_actor(&self) -> &Arc<dyn Actor> {
        &self.outer_actor
    }

    fn header_actor(&self) -> Option<&Arc<dyn Actor>> {
        None
    }

    fn sort_shown(&self) -> SortDirection {
        self.sort_shown
    }

    fn set_sort_shown(&mut self, direction: SortDirection) {
        self.sort_shown = direction;
    }

    fn set_sort_state(&mut self, _new_sort: SortDirection) {
        // Do nothing
    }

    fn remove_sort_symbol(&mut self, _sort_symbol: &Arc<Label>) {
        // Do nothing
    }

    fn show_sort_symbol(&mut self, _sort_symbol: &Arc<Label>) {
        // Do nothing
    }

    fn size_cell(&self, _cell: &mut Cell) {
        // Do nothing
    }
}

/// Version of IHeaderElement that works fine for Image or IconCircleGroup and overlays the sort symbol on its lower right
pub struct IconHeaderElement<CT> {
    outer_actor: Arc<NonTransformGroup>,
    header_actor: Arc<dyn Actor>,
    sort_shown: SortDirection,
    column: CT,
    icon_size: f32,
}

impl<CT> IconHeaderElement<CT> {
    /// Creates a new IconHeaderElement
    pub fn new(column: CT, header_actor: Arc<dyn Actor>, icon_size: f32) -> Self {
        let outer_actor = Arc::new(NonTransformGroup::new());
        outer_actor.set_size(icon_size, icon_size);
        outer_actor.add_child(header_actor.clone());
        header_actor.set_size(icon_size, icon_size);
        header_actor.center(outer_actor.clone());

        Self {
            outer_actor,
            header_actor,
            sort_shown: SortDirection::None,
            column,
            icon_size,
        }
    }

    /// Initializes activation and tooltip
    fn init_activation_and_tooltip(&mut self, action_context: &impl std::any::Any) {
        // This would be implemented by the SortableGrid to handle clicks and tooltips
    }
}

impl<CT> IHeaderElement for IconHeaderElement<CT> {
    fn outer_actor(&self) -> &Arc<dyn Actor> {
        &self.outer_actor
    }

    fn header_actor(&self) -> Option<&Arc<dyn Actor>> {
        Some(&self.header_actor)
    }

    fn sort_shown(&self) -> SortDirection {
        self.sort_shown
    }

    fn set_sort_shown(&mut self, direction: SortDirection) {
        self.sort_shown = direction;
    }

    fn set_sort_state(&mut self, new_sort: SortDirection) {
        if new_sort == self.sort_shown {
            return;
        }

        // This would be implemented by the SortableGrid to handle sort symbols
        self.sort_shown = new_sort;
    }

    fn remove_sort_symbol(&mut self, sort_symbol: &Arc<Label>) {
        self.outer_actor.remove_child(sort_symbol);
    }

    fn show_sort_symbol(&mut self, sort_symbol: &Arc<Label>) {
        sort_symbol.set_position(self.icon_size - 2.0, 0.0);
        self.outer_actor.add_child(sort_symbol.clone());
    }

    fn size_cell(&self, cell: &mut Cell) {
        cell.set_size(self.icon_size);
    }
}

/// Version of IHeaderElement for all Layout returns from ISortableGridContentProvider.getHeaderActor
pub struct LayoutHeaderElement<CT> {
    outer_actor: Arc<HorizontalGroup>,
    header_actor: Arc<dyn Actor>,
    sort_shown: SortDirection,
    column: CT,
}

impl<CT> LayoutHeaderElement<CT> {
    /// Creates a new LayoutHeaderElement
    pub fn new(column: CT, header_actor: Arc<dyn Actor>) -> Self {
        let outer_actor = Arc::new(HorizontalGroup::new());
        outer_actor.set_transform(false);
        outer_actor.set_align(column.align());
        outer_actor.add_child(header_actor.clone());

        Self {
            outer_actor,
            header_actor,
            sort_shown: SortDirection::None,
            column,
        }
    }

    /// Initializes activation and tooltip
    fn init_activation_and_tooltip(&mut self, action_context: &impl std::any::Any) {
        // This would be implemented by the SortableGrid to handle clicks and tooltips
    }
}

impl<CT> IHeaderElement for LayoutHeaderElement<CT> {
    fn outer_actor(&self) -> &Arc<dyn Actor> {
        &self.outer_actor
    }

    fn header_actor(&self) -> Option<&Arc<dyn Actor>> {
        Some(&self.header_actor)
    }

    fn sort_shown(&self) -> SortDirection {
        self.sort_shown
    }

    fn set_sort_shown(&mut self, direction: SortDirection) {
        self.sort_shown = direction;
    }

    fn set_sort_state(&mut self, new_sort: SortDirection) {
        if new_sort == self.sort_shown {
            return;
        }

        // This would be implemented by the SortableGrid to handle sort symbols
        self.sort_shown = new_sort;
    }

    fn remove_sort_symbol(&mut self, sort_symbol: &Arc<Label>) {
        self.outer_actor.remove_child(sort_symbol);
    }

    fn show_sort_symbol(&mut self, sort_symbol: &Arc<Label>) {
        self.outer_actor.add_child(sort_symbol.clone());
    }

    fn size_cell(&self, _cell: &mut Cell) {
        // Do nothing
    }
}

/// A generic sortable grid Widget
///
/// Note this only remembers one sort criterion. Sorts like compareBy(type).thenBy(name) aren't supported.
pub struct SortableGrid<IT, ACT, CT>
where
    CT: ISortableGridContentProvider<IT, ACT> + Clone + PartialEq + 'static,
{
    /// The base Table that this SortableGrid extends
    base: Table,
    /// The columns to render as ISortableGridContentProvider instances
    columns: Vec<CT>,
    /// The actual "data" as in one object per row
    data: Vec<IT>,
    /// Passed to ISortableGridContentProvider.getEntryActor where it can be used to define onClick actions
    action_context: ACT,
    /// Sorting state will be kept here
    sort_state: Box<dyn ISortState<CT>>,
    /// Size for header icons
    icon_size: f32,
    /// Vertical padding for all Cells
    padding_vert: f32,
    /// Horizontal padding for all Cells
    padding_horz: f32,
    /// When true, the header row isn't part of the widget but delivered through get_header
    separate_header: bool,
    /// Called after every update - during init and re-sort
    update_callback: Option<Box<dyn Fn(&Table, &Table, &Table)>>,
    /// The header row
    header_row: Table,
    /// The header elements
    header_elements: HashMap<CT, Box<dyn IHeaderElement>>,
    /// The sort symbols
    sort_symbols: HashMap<bool, Arc<Label>>,
    /// The details table
    details: Table,
    /// The totals row
    totals_row: Table,
}

impl<IT, ACT, CT> SortableGrid<IT, ACT, CT>
where
    CT: ISortableGridContentProvider<IT, ACT> + Clone + PartialEq + 'static,
{
    /// Creates a new SortableGrid with the given parameters
    pub fn new(
        columns: Vec<CT>,
        data: Vec<IT>,
        action_context: ACT,
        sort_state: Option<Box<dyn ISortState<CT>>>,
        icon_size: f32,
        padding_vert: f32,
        padding_horz: f32,
        separate_header: bool,
        update_callback: Option<Box<dyn Fn(&Table, &Table, &Table)>>,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin = base_screen.skin();

        let sort_state = sort_state.unwrap_or_else(|| {
            Box::new(SortState::new(columns[0].clone()))
        });

        let header_row = Table::new(skin.clone());
        let details = Table::new(skin.clone());
        let totals_row = Table::new(skin.clone());

        let mut grid = Self {
            base: Table::new(skin),
            columns,
            data,
            action_context,
            sort_state,
            icon_size,
            padding_vert,
            padding_horz,
            separate_header,
            update_callback,
            header_row,
            header_elements: HashMap::new(),
            sort_symbols: HashMap::new(),
            details,
            totals_row,
        };

        grid.init();

        grid
    }

    /// Initializes the grid
    fn init(&mut self) {
        // Check that separate_header is not combined with expanding columns
        if self.separate_header {
            for column in &self.columns {
                if column.expand_x() {
                    panic!("SortableGrid currently does not support separateHeader combined with expanding columns");
                }
            }
        }

        // Set defaults for all tables
        self.header_row.defaults().pad(self.padding_vert, self.padding_horz).min_width(self.icon_size);
        self.details.defaults().pad(self.padding_vert, self.padding_horz).min_width(self.icon_size);
        self.totals_row.defaults().pad(self.padding_vert, self.padding_horz).min_width(self.icon_size);

        // Initialize the grid
        self.init_header();
        self.update_header();
        self.update_details();
        self.init_totals();
        self.fire_callback();

        // Add the tables to the base table
        self.base.top();
        if !self.separate_header {
            self.base.add(self.header_row.clone()).row();
            self.base.add_separator(Color::GRAY).pad(self.padding_vert, 0.0);
        }
        self.base.add(self.details.clone()).row();
        self.base.add_separator(Color::GRAY).pad(self.padding_vert, 0.0);
        self.base.add(self.totals_row.clone());
    }

    /// Fires the update callback
    fn fire_callback(&self) {
        if let Some(callback) = &self.update_callback {
            self.header_row.pack();
            self.details.pack();
            self.totals_row.pack();
            callback(&self.header_row, &self.details, &self.totals_row);
        }
    }

    /// Initializes the header
    fn init_header(&mut self) {
        // Create sort symbols
        let sort_up_arrow = Arc::new(Label::new(
            &Fonts::sort_up_arrow().to_string(),
            Constants::default_font_size(),
            Color::WHITE,
            true,
            self.base.skin().clone(),
        ));

        let sort_down_arrow = Arc::new(Label::new(
            &Fonts::sort_down_arrow().to_string(),
            Constants::default_font_size(),
            Color::WHITE,
            true,
            self.base.skin().clone(),
        ));

        self.sort_symbols.insert(false, sort_up_arrow);
        self.sort_symbols.insert(true, sort_down_arrow);

        // Create header elements
        for column in &self.columns {
            let element = self.get_header_element(column.clone());
            self.header_elements.insert(column.clone(), element);

            let header_element = self.header_elements.get(column).unwrap();
            let mut cell = self.header_row.add(header_element.outer_actor().clone());
            header_element.size_cell(&mut cell);
            cell.align(column.align()).fill(column.fill_x(), false).expand(column.expand_x(), false);
        }
    }

    /// Gets the header element for the given column
    fn get_header_element(&self, column: CT) -> Box<dyn IHeaderElement> {
        if let Some(header_actor) = column.get_header_actor(self.icon_size) {
            if header_actor.is::<Image>() || header_actor.is::<IconCircleGroup>() {
                Box::new(IconHeaderElement::new(column, header_actor, self.icon_size))
            } else if header_actor.is::<Layout>() {
                Box::new(LayoutHeaderElement::new(column, header_actor))
            } else {
                Box::new(IconHeaderElement::new(column, header_actor, self.icon_size))
            }
        } else {
            Box::new(EmptyHeaderElement::new())
        }
    }

    /// Updates the grid
    pub fn update(&mut self) {
        self.update_header();
        self.update_details();
    }

    /// Updates the header
    fn update_header(&mut self) {
        for column in &self.columns {
            let sort_direction = if self.sort_state.sorted_by() == column {
                self.sort_state.direction()
            } else {
                SortDirection::None
            };

            if let Some(element) = self.header_elements.get_mut(column) {
                element.set_sort_state(sort_direction);
            }
        }
    }

    /// Updates the details
    fn update_details(&mut self) {
        self.details.clear();

        if self.data.is_empty() {
            return;
        }

        // Sort the data
        let comparator = self.sort_state.sorted_by().get_comparator();
        let mut sorted_data = self.data.clone();

        match self.sort_state.direction() {
            SortDirection::None => {
                // Keep original order
            }
            SortDirection::Ascending => {
                sorted_data.sort_by(|a, b| comparator(a, b));
            }
            SortDirection::Descending => {
                sorted_data.sort_by(|a, b| comparator(b, a));
            }
        }

        // Create cells to equalize
        let mut cells_to_equalize = Vec::new();

        // Add the data to the details table
        for item in sorted_data {
            for column in &self.columns {
                let actor = column.get_entry_actor(&item, self.icon_size, &self.action_context);

                if actor.is_none() {
                    self.details.add();
                    continue;
                }

                let actor = actor.unwrap();
                let mut cell = self.details.add(actor).align(column.align())
                    .fill(column.fill_x(), false).expand(column.expand_x(), false);

                if column.equalize_height() {
                    cells_to_equalize.push(cell);
                }
            }

            self.details.row();
        }

        // Equalize cell heights
        if !cells_to_equalize.is_empty() {
            let largest_label_height = cells_to_equalize.iter()
                .map(|cell| cell.pref_height())
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(0.0);

            for cell in cells_to_equalize {
                cell.min_height(largest_label_height);
            }
        }
    }

    /// Initializes the totals
    fn init_totals(&mut self) {
        for column in &self.columns {
            let totals_actor = column.get_totals_actor(&self.data);
            self.totals_row.add(totals_actor).align(column.align())
                .fill(column.fill_x(), false).expand(column.expand_x(), false);
        }
    }

    /// Toggles the sort for the given column
    fn toggle_sort(&mut self, sort_by: CT) {
        let direction = if self.sort_state.sorted_by() == &sort_by {
            self.sort_state.direction()
        } else {
            SortDirection::None
        };

        self.set_sort(sort_by, direction.inverted(sort_by.default_sort()));
    }

    /// Sets the sort for the given column and direction
    pub fn set_sort(&mut self, sort_by: CT, direction: SortDirection) {
        self.sort_state.set_sorted_by(sort_by);
        self.sort_state.set_direction(direction);

        // Rebuild header content to show sort state
        // And resort the table: clear and fill with sorted data
        self.update();
        self.fire_callback();
    }

    /// Gets the header table
    pub fn get_header(&self) -> &Table {
        if !self.separate_header {
            panic!("You can't call SortableGrid.get_header unless you set separate_header to true");
        }
        &self.header_row
    }

    /// Finds the first cell that contains an actor of type T that matches the predicate
    pub fn find_cell<T: Actor + 'static>(&self, predicate: impl Fn(&T) -> bool) -> Option<&Cell> {
        self.details.cells().iter()
            .find(|cell| {
                if let Some(actor) = cell.actor() {
                    if let Some(t_actor) = actor.downcast_ref::<T>() {
                        predicate(t_actor)
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
    }

    /// Finds the first cell that contains an actor of type T with the given name
    pub fn find_cell_by_name<T: Actor + 'static>(&self, name: &str) -> Option<&Cell> {
        self.find_cell(|actor| actor.name() == name)
    }
}

// Implement the necessary traits for SortableGrid
impl<IT, ACT, CT> std::ops::Deref for SortableGrid<IT, ACT, CT>
where
    CT: ISortableGridContentProvider<IT, ACT> + Clone + PartialEq + 'static,
{
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<IT, ACT, CT> std::ops::DerefMut for SortableGrid<IT, ACT, CT>
where
    CT: ISortableGridContentProvider<IT, ACT> + Clone + PartialEq + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<IT, ACT, CT> Clone for SortableGrid<IT, ACT, CT>
where
    IT: Clone,
    ACT: Clone,
    CT: ISortableGridContentProvider<IT, ACT> + Clone + PartialEq + 'static,
{
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            columns: self.columns.clone(),
            data: self.data.clone(),
            action_context: self.action_context.clone(),
            sort_state: Box::new(SortState::new(self.sort_state.sorted_by().clone())),
            icon_size: self.icon_size,
            padding_vert: self.padding_vert,
            padding_horz: self.padding_horz,
            separate_header: self.separate_header,
            update_callback: None, // Callbacks can't be cloned
            header_row: self.header_row.clone(),
            header_elements: HashMap::new(), // Will be recreated in init
            sort_symbols: self.sort_symbols.clone(),
            details: self.details.clone(),
            totals_row: self.totals_row.clone(),
        }
    }
}