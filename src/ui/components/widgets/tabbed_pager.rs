use ggez::graphics::{Color, DrawParam};
use ggez::mint::Point2;
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::Constants;
use crate::ui::components::input::{KeyCharAndCode, KeyShortcutDispatcher};
use crate::ui::components::widgets::scroll_pane::ScrollPane;
use crate::ui::components::widgets::table::Table;
use crate::ui::components::widgets::widget::Widget;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;

/// A trait for pages that need to be notified of activation/deactivation
pub trait PageExtensions {
    /// Called after a page is shown
    fn activated(&mut self, index: i32, caption: &str, pager: &TabbedPager);

    /// Called before a page is hidden
    fn deactivated(&mut self, index: i32, caption: &str, pager: &TabbedPager) {}

    /// Optional second content widget, placed outside the tab's main ScrollPane
    fn get_fixed_content(&self) -> Option<Arc<dyn Widget>> {
        None
    }
}

/// Represents the state of a page in the TabbedPager
struct PageState {
    /// The caption of the page
    caption: String,

    /// The main content of the page
    content: Arc<dyn Widget>,

    /// Optional fixed content
    fixed_content: Option<Arc<dyn Widget>>,

    /// Whether the page is disabled
    disabled: bool,

    /// Optional icon for the page
    icon: Option<Arc<dyn Widget>>,

    /// Size of the icon
    icon_size: f32,

    /// Keyboard shortcut for the page
    shortcut_key: KeyCharAndCode,

    /// Scroll alignment
    scroll_align: i32,

    /// Whether to sync scroll with fixed content
    sync_scroll: bool,

    /// Fixed height of the page
    fixed_height: f32,

    /// Scroll position X
    scroll_x: f32,

    /// Scroll position Y
    scroll_y: f32,

    /// Button X position
    button_x: f32,

    /// Button width
    button_width: f32,
}

impl PageState {
    fn new(
        caption: String,
        content: Arc<dyn Widget>,
        fixed_content: Option<Arc<dyn Widget>>,
        disabled: bool,
        icon: Option<Arc<dyn Widget>>,
        icon_size: f32,
        shortcut_key: KeyCharAndCode,
        scroll_align: i32,
        sync_scroll: bool,
    ) -> Self {
        Self {
            caption,
            content,
            fixed_content,
            disabled,
            icon,
            icon_size,
            shortcut_key,
            scroll_align,
            sync_scroll,
            fixed_height: 0.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            button_x: 0.0,
            button_width: 0.0,
        }
    }
}

/// A widget that implements a tabbed interface with scrollable content
pub struct TabbedPager {
    /// The base Table that this TabbedPager extends
    base: Table,

    /// The pages in this pager
    pages: Vec<PageState>,

    /// The currently active page index
    active_page: i32,

    /// The header table
    header: Table,

    /// The header scroll pane
    header_scroll: ScrollPane,

    /// The height of the header
    header_height: f32,

    /// The fixed content scroll pane
    fixed_content_scroll: ScrollPane,

    /// The content scroll pane
    content_scroll: ScrollPane,

    /// Callback for page selection
    on_selection_callback: Option<Box<dyn Fn(i32, &str, &TabbedPager) + Send + Sync>>,

    /// The header font size
    header_font_size: i32,

    /// The header font color
    header_font_color: Color,

    /// The highlight color for active tabs
    highlight_color: Color,

    /// The background color
    background_color: Color,

    /// The header padding
    header_padding: f32,

    /// The separator color
    separator_color: Color,
}

impl TabbedPager {
    /// Creates a new TabbedPager with the given parameters
    pub fn new(
        minimum_width: f32,
        maximum_width: f32,
        minimum_height: f32,
        maximum_height: f32,
        header_font_size: i32,
        header_font_color: Color,
        highlight_color: Color,
        background_color: Color,
        header_padding: f32,
        separator_color: Color,
    ) -> Self {
        let base_screen = BaseScreen::get_instance();
        let skin_strings = base_screen.skin_strings();

        let mut pager = Self {
            base: Table::new(),
            pages: Vec::new(),
            active_page: -1,
            header: Table::new(),
            header_scroll: ScrollPane::new(true),
            header_height: 0.0,
            fixed_content_scroll: ScrollPane::new(true),
            content_scroll: ScrollPane::new(false),
            on_selection_callback: None,
            header_font_size,
            header_font_color,
            highlight_color,
            background_color,
            header_padding,
            separator_color,
        };

        // Set up the base table
        pager.base.set_background(
            skin_strings.get_ui_background("General/TabbedPager", Some(background_color))
        );

        // Set up the header
        pager.header.defaults().pad(header_padding, header_padding * 0.5);

        // Add components to base table
        pager.base.add_child(pager.header_scroll.clone()).grow_x().min_height(pager.header_height);

        if separator_color != Color::CLEAR {
            pager.base.add_separator(separator_color);
        }

        pager.base.add_child(pager.fixed_content_scroll.clone());
        pager.base.add_child(pager.content_scroll.clone()).grow();

        pager
    }

    /// Adds a new page to the pager
    pub fn add_page(
        &mut self,
        caption: String,
        content: Option<Arc<dyn Widget>>,
        icon: Option<Arc<dyn Widget>>,
        icon_size: f32,
        insert_before: i32,
        disabled: bool,
        shortcut_key: KeyCharAndCode,
        scroll_align: i32,
        sync_scroll: bool,
    ) -> i32 {
        let content = content.unwrap_or_else(|| Arc::new(Table::new()));
        let fixed_content = if let Some(page_ext) = content.as_any().downcast_ref::<dyn PageExtensions>() {
            page_ext.get_fixed_content()
        } else {
            None
        };

        let page = PageState::new(
            caption,
            content,
            fixed_content,
            disabled,
            icon,
            icon_size,
            shortcut_key,
            scroll_align,
            sync_scroll,
        );

        // Add the page button to the header
        let button = self.create_page_button(&page);
        self.header.add_child(button);

        // Insert the page at the specified position
        if insert_before >= 0 && insert_before < self.pages.len() as i32 {
            self.pages.insert(insert_before as usize, page);
            insert_before
        } else {
            self.pages.push(page);
            self.pages.len() as i32 - 1
        }
    }

    /// Creates a button for a page
    fn create_page_button(&self, page: &PageState) -> Arc<dyn Widget> {
        // Implementation for creating the page button
        // This would create a button with the page's caption and icon
        unimplemented!("Page button creation not yet implemented")
    }

    /// Selects a page by index
    pub fn select_page(&mut self, index: i32, center_button: bool) -> bool {
        if index < -1 || index >= self.pages.len() as i32 {
            return false;
        }
        if self.active_page == index {
            return false;
        }
        if index >= 0 && self.pages[index as usize].disabled {
            return false;
        }

        // Deactivate current page
        if self.active_page >= 0 {
            let page = &mut self.pages[self.active_page as usize];
            if let Some(page_ext) = page.content.as_any().downcast_ref::<dyn PageExtensions>() {
                page_ext.deactivated(self.active_page, &page.caption, this);
            }
            page.scroll_x = self.content_scroll.scroll_x();
            page.scroll_y = self.content_scroll.scroll_y();
        }

        self.active_page = index;

        // Activate new page
        if index >= 0 {
            let page = &mut self.pages[index as usize];

            // Set up scroll alignment
            if page.scroll_align != 0 {
                if page.scroll_align & 1 != 0 { // Center horizontally
                    page.scroll_x = (page.content.width() - self.base.width()) / 2.0;
                } else if page.scroll_align & 2 != 0 { // Right
                    page.scroll_x = f32::MAX;
                }
                if page.scroll_align & 4 != 0 { // Center vertically
                    page.scroll_y = (page.content.height() - self.base.height()) / 2.0;
                } else if page.scroll_align & 8 != 0 { // Bottom
                    page.scroll_y = f32::MAX;
                }
                page.scroll_align = 0;
            }

            // Set up fixed content
            if let Some(fixed_content) = &page.fixed_content {
                self.fixed_content_scroll.set_content(fixed_content.clone());
                self.fixed_content_scroll.set_height(page.fixed_height);
                self.fixed_content_scroll.set_scroll_x(page.scroll_x);
                self.fixed_content_scroll.set_sync_scroll(page.sync_scroll);
            }

            // Set up main content
            self.content_scroll.set_content(page.content.clone());
            self.content_scroll.set_scroll_x(page.scroll_x);
            self.content_scroll.set_scroll_y(page.scroll_y);
            self.content_scroll.set_sync_scroll(page.sync_scroll);

            // Center or ensure visibility of the button
            if center_button {
                self.header_scroll.set_scroll_x(
                    page.button_x + (page.button_width - self.header_scroll.width()) / 2.0
                );
            } else {
                let scroll_x = self.header_scroll.scroll_x();
                let min_x = page.button_x + page.button_width - self.header_scroll.width();
                let max_x = page.button_x;
                self.header_scroll.set_scroll_x(scroll_x.clamp(min_x, max_x));
            }

            // Activate the page
            if let Some(page_ext) = page.content.as_any().downcast_ref::<dyn PageExtensions>() {
                page_ext.activated(index, &page.caption, this);
            }

            // Call selection callback
            if let Some(callback) = &self.on_selection_callback {
                callback(index, &page.caption, this);
            }
        }

        true
    }

    /// Sets a callback for page selection
    pub fn on_selection<F>(&mut self, callback: F)
    where
        F: Fn(i32, &str, &TabbedPager) + Send + Sync + 'static,
    {
        self.on_selection_callback = Some(Box::new(callback));
    }
}

// Implement the necessary traits for TabbedPager
impl std::ops::Deref for TabbedPager {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for TabbedPager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for TabbedPager {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            pages: self.pages.clone(),
            active_page: self.active_page,
            header: self.header.clone(),
            header_scroll: self.header_scroll.clone(),
            header_height: self.header_height,
            fixed_content_scroll: self.fixed_content_scroll.clone(),
            content_scroll: self.content_scroll.clone(),
            on_selection_callback: None, // Callbacks are not cloned
            header_font_size: self.header_font_size,
            header_font_color: self.header_font_color,
            highlight_color: self.highlight_color,
            background_color: self.background_color,
            header_padding: self.header_padding,
            separator_color: self.separator_color,
        }
    }
}