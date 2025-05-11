use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align};
use crate::models::civilization::Civilization;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::TabbedPager;

/// Abstract container for persistable data
/// - default struct does nothing
/// - If persistence should end when quitting the game - do not override is_empty() and EmpireOverviewCategories.get_persist_data_class()
/// - For persistence in GameSettings.json, override both
pub struct EmpireOverviewTabPersistableData {
    /// Used by serialization to detect when a default state can be omitted
    pub fn is_empty(&self) -> bool {
        true
    }
}

/// Base trait for all overview tabs
pub trait EmpireOverviewTab: TabbedPagerPageExtensions {
    fn viewing_player(&self) -> &Rc<RefCell<Civilization>>;
    fn overview_screen(&self) -> &Rc<RefCell<EmpireOverviewScreen>>;
    fn persistable_data(&self) -> &Rc<RefCell<EmpireOverviewTabPersistableData>>;

    /// Called when the tab is activated
    /// Returns the scroll position if the tab can select something specific
    fn select(&self, selection: &str) -> Option<f32> {
        None
    }

    /// Get the game info from the viewing player
    fn game_info(&self) -> &Rc<RefCell<GameInfo>> {
        &self.viewing_player().borrow().game_info
    }
}

/// Trait for tab page extensions
pub trait TabbedPagerPageExtensions {
    /// Called when the tab is activated
    fn activated(&self, index: i32, caption: &str, pager: &Rc<RefCell<TabbedPager>>);
}