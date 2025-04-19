use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align};
use crate::models::civilization::Civilization;
use crate::models::civilization::Notification;
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::components::widgets::TabbedPager;
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_categories::{EmpireOverviewCategories, EmpireOverviewTabState};

pub struct EmpireOverviewScreen {
    viewing_player: Rc<RefCell<Civilization>>,
    center_area_height: f32,
    tabbed_pager: Rc<RefCell<TabbedPager>>,
    page_objects: HashMap<EmpireOverviewCategories, Box<dyn EmpireOverviewTab>>,
    persist_state: Rc<RefCell<OverviewPersistableData>>,
}

impl EmpireOverviewScreen {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        default_category: Option<EmpireOverviewCategories>,
        selection: &str,
    ) -> Self {
        let center_area_height = UncivGame::current().stage.height - 82.0;
        let persist_state = Rc::new(RefCell::new(UncivGame::current().settings.overview.clone()));
        let select_category = default_category.unwrap_or(persist_state.borrow().last);
        let icon_size = Constants::DEFAULT_FONT_SIZE as f32;

        let mut screen = Self {
            viewing_player,
            center_area_height,
            tabbed_pager: Rc::new(RefCell::new(TabbedPager::new(
                UncivGame::current().stage.width,
                UncivGame::current().stage.width,
                center_area_height,
                center_area_height,
                Color32::WHITE,
                EmpireOverviewCategories::iter().count(),
            ))),
            page_objects: HashMap::new(),
            persist_state,
        };

        screen.init(select_category, icon_size, selection);
        screen
    }

    fn init(&mut self, select_category: EmpireOverviewCategories, icon_size: f32, selection: &str) {
        for category in EmpireOverviewCategories::iter() {
            let tab_state = category.test_state(&self.viewing_player);
            if tab_state == EmpireOverviewTabState::Hidden {
                continue;
            }

            let icon = if category.icon_name().is_empty() {
                None
            } else {
                Some(ImageGetter::get_image(category.icon_name()))
            };

            let page_object = category.create_tab(
                self.viewing_player.clone(),
                Rc::new(RefCell::new(self.clone())),
                self.persist_state.borrow().get(category).cloned(),
            );

            let index = self.tabbed_pager.borrow_mut().add_page(
                category.to_string(),
                page_object.clone(),
                icon,
                icon_size,
                tab_state != EmpireOverviewTabState::Normal,
                category.shortcut_key(),
                category.scroll_align(),
            );

            self.page_objects.insert(category, page_object);

            if category == select_category {
                self.tabbed_pager.borrow_mut().select_page(index);
                self.select(category, selection);
            }
        }

        self.persist_state.borrow_mut().update(&self.page_objects);

        let close_button = self.get_close_button(|| {
            UncivGame::current().pop_screen();
        });
        self.tabbed_pager.borrow_mut().decorate_header(close_button);

        self.tabbed_pager.borrow_mut().set_fill_parent(true);
        UncivGame::current().stage.add_actor(self.tabbed_pager.clone());
    }

    pub fn resize_page(&mut self, tab: &dyn EmpireOverviewTab) {
        if let Some((category, _)) = self.page_objects.iter().find(|(_, t)| t.as_any().type_id() == tab.as_any().type_id()) {
            self.tabbed_pager.borrow_mut().replace_page(category.to_string(), tab.clone());
        }
    }

    pub fn select(&mut self, category: EmpireOverviewCategories, selection: &str) {
        self.tabbed_pager.borrow_mut().select_page(category.to_string());
        if let Some(tab) = self.page_objects.get(&category) {
            if let Some(scroll_y) = tab.select(selection) {
                self.tabbed_pager.borrow_mut().set_page_scroll_y(
                    self.tabbed_pager.borrow().active_page(),
                    scroll_y,
                );
            }
        }
    }

    /// Helper to show the world screen with a temporary "one-time" notification
    /// Here because it's common to notification history, resource finder, and city WLTK demanded resource
    pub fn show_one_time_notification(&self, notification: Option<Notification>) {
        if let Some(notification) = notification {
            let world_screen = UncivGame::current().get_world_screen();
            world_screen.notifications_scroll.borrow_mut().one_time_notification = Some(notification.clone());
            UncivGame::current().reset_to_world_screen();
            notification.reset_execute_round_robin();
            notification.execute(world_screen);
        }
    }

    pub fn resume(&mut self) {
        // This is called by UncivGame.popScreen - e.g. after City Tab opened a City and the user closes that CityScreen...
        // Notify the current tab via its IPageExtensions.activated entry point so it can refresh if needed
        let index = self.tabbed_pager.borrow().active_page();
        if let Some(category) = EmpireOverviewCategories::iter().nth(index) {
            if let Some(tab) = self.page_objects.get(&category) {
                tab.activated(index, "", &self.tabbed_pager); // Fake caption marks this as popScreen-triggered
            }
        }
    }
}

impl RecreateOnResize for EmpireOverviewScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        self.tabbed_pager.borrow_mut().select_page(-1); // trigger deselect on _old_ instance so the tabs can persist their stuff
        Box::new(Self::new(
            self.viewing_player.clone(),
            Some(self.persist_state.borrow().last),
            "",
        ))
    }
}