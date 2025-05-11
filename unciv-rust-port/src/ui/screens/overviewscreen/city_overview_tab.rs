use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, ScrollArea};
use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::TabbedPager;
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;

pub struct CityOverviewTab {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<CityOverviewTabPersistableData>>,
    city_list: Vec<Rc<RefCell<City>>>,
    selected_city: Option<Rc<RefCell<City>>>,
}

impl CityOverviewTab {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<CityOverviewTabPersistableData>,
    ) -> Self {
        let mut tab = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            city_list: Vec::new(),
            selected_city: None,
        };
        tab.update_city_list();
        tab
    }

    fn update_city_list(&mut self) {
        self.city_list = self.viewing_player.borrow().cities.iter()
            .map(|city| city.clone())
            .collect();
        self.city_list.sort_by(|a, b| a.borrow().name.cmp(&b.borrow().name));
    }

    pub fn show(&self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            for city in &self.city_list {
                let city_ref = city.borrow();
                let is_selected = self.selected_city.as_ref().map_or(false, |selected| {
                    selected.borrow().id == city_ref.id
                });

                if ui.selectable_label(is_selected, &city_ref.name).clicked() {
                    self.selected_city = Some(city.clone());
                    if let Some(persist_data) = &mut *self.persist_data.borrow_mut() {
                        persist_data.selected_city_id = Some(city_ref.id.clone());
                    }
                }
            }
        });
    }

    pub fn select(&self, selection: &str) -> Option<f32> {
        if let Some(city_id) = selection.strip_prefix("city/") {
            if let Some(city) = self.city_list.iter().find(|c| c.borrow().id == city_id) {
                self.selected_city = Some(city.clone());
                if let Some(persist_data) = &mut *self.persist_data.borrow_mut() {
                    persist_data.selected_city_id = Some(city_id.to_string());
                }
                return Some(0.0); // Scroll to top
            }
        }
        None
    }
}

impl EmpireOverviewTab for CityOverviewTab {
    fn viewing_player(&self) -> &Rc<RefCell<Civilization>> {
        &self.viewing_player
    }

    fn overview_screen(&self) -> &Rc<RefCell<dyn BaseScreen>> {
        &self.overview_screen
    }

    fn persist_data(&self) -> &Rc<RefCell<dyn EmpireOverviewTabPersistableData>> {
        &self.persist_data
    }
}

#[derive(Default)]
pub struct CityOverviewTabPersistableData {
    selected_city_id: Option<String>,
}

impl EmpireOverviewTabPersistableData for CityOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        self.selected_city_id.is_none()
    }
}