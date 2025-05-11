// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/presenter/SpyPresenter.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Vec2, Color32};
use crate::game::spy::Spy;
use crate::ui::images::ImageGetter;
use crate::ui::screens::worldscreen::unit::UnitTable;

/// Presenter for spy information in the unit table
pub struct SpyPresenter {
    unit_table: Rc<RefCell<UnitTable>>,
    selected_spy: Option<Rc<RefCell<Spy>>>,
}

impl SpyPresenter {
    /// Creates a new SpyPresenter
    pub fn new(unit_table: Rc<RefCell<UnitTable>>) -> Self {
        Self {
            unit_table,
            selected_spy: None,
        }
    }

    /// Gets the position of the selected spy's city
    pub fn position(&self) -> Option<Vec2> {
        self.selected_spy.as_ref().and_then(|spy| {
            spy.borrow().get_city_or_null().map(|city| city.borrow().location)
        })
    }

    /// Selects a spy
    pub fn select_spy(&mut self, spy: Option<Rc<RefCell<Spy>>>) {
        self.selected_spy = spy;
    }

    /// Checks if the presenter should be shown
    pub fn should_be_shown(&self) -> bool {
        self.selected_spy.is_some()
    }

    /// Updates the UI when needed
    pub fn update_when_needed(&self) {
        if let Some(spy) = &self.selected_spy {
            let spy = spy.borrow();
            let mut unit_table = self.unit_table.borrow_mut();

            // Clear previous listeners and set name
            unit_table.unit_name_label.clear_listeners();
            unit_table.unit_name_label.set_text(spy.name.clone());

            // Clear description table
            unit_table.description_table.clear();

            // Clear and update icon holder
            unit_table.unit_icon_holder.clear();

            // Add spy icon
            let spy_icon = ImageGetter::get_image("OtherIcons/Spy_White");
            spy_icon.set_color(Color32::WHITE);
            spy_icon.set_size(30.0);
            unit_table.unit_icon_holder.add(spy_icon);

            // Show separator
            unit_table.separator_visible = true;

            // Determine color based on rank
            let color = match spy.rank {
                1 => Color32::from_rgb(139, 69, 19), // BROWN
                2 => Color32::from_rgb(211, 211, 211), // LIGHT_GRAY
                3 => Color32::from_rgb(255, 215, 0), // GOLD
                _ => ImageGetter::CHARCOAL,
            };

            // Add stars based on rank
            for _ in 0..spy.rank {
                let star = ImageGetter::get_image("OtherIcons/Star");
                star.set_color(color);
                star.set_size(20.0);
                star.pad(1.0);
                unit_table.description_table.add(star);
            }
        }
    }
}