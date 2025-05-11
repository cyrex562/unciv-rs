// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/presenter/CityPresenter.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Vec2, Color32};
use crate::game::city::City;
use crate::game::battle::CityCombatant;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::pickerscreens::CityRenamePopup;
use crate::ui::screens::worldscreen::unit::UnitTable;
use crate::ui::screens::worldscreen::unit::presenter::UnitPresenter;
use crate::utils::translations::tr;

/// Presenter for city information in the unit table
pub struct CityPresenter {
    unit_table: Rc<RefCell<UnitTable>>,
    unit_presenter: Rc<RefCell<UnitPresenter>>,
    selected_city: Option<Rc<RefCell<City>>>,
}

impl CityPresenter {
    /// Creates a new CityPresenter
    pub fn new(unit_table: Rc<RefCell<UnitTable>>, unit_presenter: Rc<RefCell<UnitPresenter>>) -> Self {
        Self {
            unit_table,
            unit_presenter,
            selected_city: None,
        }
    }

    /// Gets the position of the selected city
    pub fn position(&self) -> Option<Vec2> {
        self.selected_city.as_ref().map(|city| city.borrow().location)
    }

    /// Selects a city and returns whether the selection changed
    pub fn select_city(&mut self, city: Option<Rc<RefCell<City>>>) -> bool {
        // If the last selected unit connecting a road, keep it selected. Otherwise, clear.
        let mut unit_presenter = self.unit_presenter.borrow_mut();
        if unit_presenter.selected_unit_is_connecting_road {
            if let Some(unit) = unit_presenter.selected_units.first() {
                unit_presenter.select_unit(Some(unit.clone()));
                unit_presenter.selected_unit_is_connecting_road = true; // select_unit resets this
            }
        } else {
            unit_presenter.select_unit(None);
        }

        if city == self.selected_city {
            return false;
        }

        self.selected_city = city;
        true
    }

    /// Updates the UI when needed
    pub fn update_when_needed(&self) {
        let mut unit_table = self.unit_table.borrow_mut();
        unit_table.separator_visible = true;

        if let Some(city) = &self.selected_city {
            let city = city.borrow();
            let mut name_label_text = tr(&city.name);

            if city.health < city.get_max_health() {
                name_label_text.push_str(&format!(" ({})", tr(&city.health.to_string())));
            }

            unit_table.unit_name_label.set_text(name_label_text);

            // Clear previous listeners
            unit_table.unit_name_label.clear_listeners();

            // Add click listener for renaming
            let world_screen = unit_table.world_screen.clone();
            let city_clone = city.clone();
            unit_table.unit_name_label.add_click_listener(move |_| {
                if !world_screen.borrow().can_change_state {
                    return;
                }

                let popup = CityRenamePopup::new(
                    world_screen.clone(),
                    city_clone.clone(),
                    Box::new(move |_| {
                        let mut unit_table = unit_table.borrow_mut();
                        unit_table.unit_name_label.set_text(tr(&city_clone.borrow().name));
                        world_screen.borrow_mut().should_update = true;
                    })
                );

                world_screen.borrow_mut().push_screen(Box::new(popup));
            });

            // Clear and update description table
            unit_table.description_table.clear();
            unit_table.description_table.defaults().pad(2.0).pad_right(5.0);

            // Add strength information
            let city_combatant = CityCombatant::new(city.clone());
            unit_table.description_table.add(tr("Strength"));
            unit_table.description_table.add(tr(&city_combatant.get_defending_strength().to_string())).row();
            unit_table.description_table.add(tr("Bombard strength"));
            unit_table.description_table.add(tr(&city_combatant.get_attacking_strength().to_string())).row();

            unit_table.world_screen.borrow_mut().should_update = true;
        }
    }
}