// Source: orig_src/core/src/com/unciv/ui/screens/overviewscreen/UnitSupplyTable.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::{Ui, Color32, Align, Response, Image};
use crate::models::civilization::Civilization;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::ExpanderTab;
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use super::unit_overview_tab::UnitOverviewTab;

/// This is a static factory to avoid making ExpanderTab open. UnitSupplyTable object used purely as namespace.
pub struct UnitSupplyTable;

impl UnitSupplyTable {
    pub fn create(
        overview_screen: &Rc<RefCell<dyn BaseScreen>>,
        unit_overview_tab: &UnitOverviewTab,
        viewing_player: &Rc<RefCell<Civilization>>,
        supply_table_width: f32
    ) -> ExpanderTab {
        let stats = viewing_player.borrow().stats;
        let deficit = stats.get_unit_supply_deficit();

        // Create warning icon if there's a deficit
        let icon = if deficit <= 0 {
            None
        } else {
            let mut icon_group = Ui::default();
            icon_group.set_size(36.0, 36.0);

            let mut image = ImageGetter::get_image("OtherIcons/ExclamationMark");
            image.color = Color32::FIREBRICK;
            image.set_size(36.0, 36.0);
            image.center(&icon_group);
            image.set_origin(Align::Center);
            icon_group.add(image);

            Some(icon_group)
        };

        // Create the expander tab
        let mut expander = ExpanderTab::new(
            "Unit Supply",
            Constants::default_font_size(),
            icon,
            deficit > 0, // Start expanded if there's a deficit
            0.0, // Default padding
            supply_table_width,
            Box::new(move || {
                overview_screen.borrow().resize_page(unit_overview_tab);
            })
        );

        // Add content to the expander
        expander.content(|ui| {
            ui.defaults().pad(5.0).fill(false);
            ui.background = overview_screen.borrow().skin_strings.get_ui_background(
                "OverviewScreen/UnitOverviewTab/UnitSupplyTable",
                overview_screen.borrow().skin_strings.skin_config.base_color.darken(0.6)
            );

            Self::add_labeled_value(ui, "Base Supply", stats.get_base_unit_supply());
            Self::add_labeled_value(ui, "Cities", stats.get_unit_supply_from_cities());
            Self::add_labeled_value(ui, "Population", stats.get_unit_supply_from_pop());
            ui.add_separator();
            Self::add_labeled_value(ui, "Total Supply", stats.get_unit_supply());
            Self::add_labeled_value(ui, "In Use", viewing_player.borrow().units.get_civ_units_size());
            ui.add_separator();
            Self::add_labeled_value(ui, "Supply Deficit", deficit);
            Self::add_labeled_value(ui, "Production Penalty", format!("{}%", stats.get_unit_supply_production_penalty().to_int()));

            if deficit > 0 {
                let penalty_label = ui.add_label(
                    "Increase your supply or reduce the amount of units to remove the production penalty",
                    Color32::FIREBRICK
                );
                penalty_label.wrap = true;
                ui.add(penalty_label).colspan(2).left()
                    .width(supply_table_width).row();
            }
        });

        expander
    }

    // Helper methods for adding labeled values
    fn add_labeled_value(ui: &mut Ui, label: &str, value: i32) {
        ui.add_label(label).left();
        ui.add_label(value.to_string()).right().row();
    }

    fn add_labeled_value(ui: &mut Ui, label: &str, value: String) {
        ui.add_label(label).left();
        ui.add_label(value).right().row();
    }
}