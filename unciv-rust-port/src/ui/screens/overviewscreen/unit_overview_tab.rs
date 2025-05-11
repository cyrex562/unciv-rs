// Source: orig_src/core/src/com/unciv/ui/screens/overviewscreen/UnitOverviewTab.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align, ScrollArea, Button, Image, Response};
use crate::models::civilization::Civilization;
use crate::models::unit::Unit;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::{TabbedPager, ExpanderTab};
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;
use super::unit_overview_tab_column::UnitOverviewTabColumn;
use super::unit_overview_tab_helpers::UnitOverviewTabHelpers;

pub struct UnitOverviewTab {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<UnitOverviewTabPersistableData>>,
    game: Rc<RefCell<UncivGame>>,
    table: Ui,
    columns: Vec<UnitOverviewTabColumn>,
    helpers: UnitOverviewTabHelpers,
}

impl UnitOverviewTab {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<UnitOverviewTabPersistableData>,
    ) -> Self {
        let game = Rc::clone(&overview_screen.borrow().game);

        let mut tab = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            game,
            table: Ui::default(),
            columns: Vec::new(),
            helpers: UnitOverviewTabHelpers::new(),
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        let mut table = Ui::default();
        table.defaults().pad(10.0);

        // Initialize columns
        self.columns = vec![
            UnitOverviewTabColumn::new("Name", |unit| unit.name.clone()),
            UnitOverviewTabColumn::new("Type", |unit| unit.base_unit.name.clone()),
            UnitOverviewTabColumn::new("Location", |unit| {
                if let Some(tile) = unit.get_tile() {
                    format!("({}, {})", tile.position.x, tile.position.y)
                } else {
                    "Unknown".to_string()
                }
            }),
            UnitOverviewTabColumn::new("Health", |unit| format!("{}%", unit.health)),
            UnitOverviewTabColumn::new("Movement", |unit| format!("{}/{}", unit.current_movement, unit.max_movement)),
            UnitOverviewTabColumn::new("Experience", |unit| format!("{}", unit.experience)),
            UnitOverviewTabColumn::new("Promotions", |unit| {
                unit.promotions.iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
        ];

        // Add column headers
        for column in &self.columns {
            table.add_label(&column.name, Constants::heading_font_size(), false).row();
        }

        // Add unit rows
        let units: Vec<_> = self.viewing_player.borrow().units.iter().collect();
        for unit in units {
            for column in &self.columns {
                let text = (column.getter)(&unit);
                table.add_label(&text, 0, false).row();
            }
        }

        self.table = table;
    }

    pub fn update(&mut self) {
        self.init();
    }
}

impl EmpireOverviewTab for UnitOverviewTab {
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
pub struct UnitOverviewTabPersistableData {
    // Add any persistent data fields here
}

impl EmpireOverviewTabPersistableData for UnitOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        true // Implement based on actual fields
    }
}