use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Layout, Rect, Ui, Align};
use std::collections::HashMap;

use crate::constants::DEFAULT_FONT_SIZE;
use crate::gui::GUI;
use crate::logic::city::{City, CityConstructions};
use crate::logic::city::constructions::IConstruction;
use crate::models::building::Building;
use crate::models::unit::BaseUnit;
use crate::models::ruleset::unique::UniqueType;
use crate::models::ruleset::perpetual_construction::{PerpetualConstruction, PerpetualStatConversion};
use crate::ui::components::extensions::{darken, disable, is_enabled, to_checkbox, to_label, to_text_button};
use crate::ui::components::input::{key_shortcuts, on_activation, onClick, KeyboardBinding};
use crate::ui::fonts::Fonts;
use crate::ui::images::ImageGetter;
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::ui::screens::cityscreen::buy_button_factory::BuyButtonFactory;
use crate::ui::popups::confirm_popup::ConfirmPopup;
use crate::ui::popups::close_all_popups;
use crate::utils::translations::tr;
use crate::utils::sound::UncivSound;

/// This is the bottom-right table in the city screen that shows the currently selected construction
pub struct ConstructionInfoTable {
    /// Reference to the parent city screen
    city_screen: Box<dyn CityScreen>,

    /// The inner table containing the selected construction information
    selected_construction_table: egui::Frame,

    /// Factory for creating buy buttons
    buy_button_factory: BuyButtonFactory,
}

impl ConstructionInfoTable {
    /// Create a new ConstructionInfoTable
    pub fn new(city_screen: Box<dyn CityScreen>) -> Self {
        let mut selected_construction_table = egui::Frame::none();
        selected_construction_table.set_style(egui::Style {
            background: Some(darken(BaseScreen::get_skin_color(), 0.5)),
            padding: egui::style::Margin::same(10.0),
            ..Default::default()
        });

        let buy_button_factory = BuyButtonFactory::new(city_screen.clone());

        Self {
            city_screen,
            selected_construction_table,
            buy_button_factory,
        }
    }

    /// Update the table with the current selected construction
    pub fn update(&mut self, selected_construction: Option<Box<dyn IConstruction>>) {
        self.selected_construction_table.clear();

        if selected_construction.is_none() {
            self.selected_construction_table.set_visible(false);
            return;
        }

        self.selected_construction_table.set_visible(true);

        if let Some(construction) = selected_construction {
            self.update_selected_construction_table(construction);
        }

        self.selected_construction_table.pack();
    }

    /// Update the selected construction table with the given construction
    fn update_selected_construction_table(&mut self, construction: Box<dyn IConstruction>) {
        let city = self.city_screen.get_city();
        let city_constructions = &city.city_constructions;

        // Add construction portrait
        let mut portrait = ImageGetter::get_construction_portrait(&construction.name(), 50.0);

        // Add link to civilopedia if available
        if let Some(ruleset_object) = construction.ruleset_object() {
            let link = ruleset_object.make_link();
            if !link.is_empty() {
                portrait.on_click(move || {
                    self.city_screen.open_civilopedia(&link);
                });
            }
        }

        self.selected_construction_table.add(portrait).pad(5.0);

        // Add construction name and turns to build
        let mut building_text = tr(&construction.name(), true);

        let special_construction = PerpetualConstruction::get_perpetual_constructions_map()
            .get(&construction.name())
            .cloned();

        building_text += special_construction
            .map(|sc| sc.get_production_tooltip(city))
            .unwrap_or_else(|| city_constructions.get_turns_to_construction_string(&construction));

        self.selected_construction_table.add(to_label(&building_text)).expand_x().row();

        // Add description
        let description = match construction.type_name() {
            t if t.contains("BaseUnit") => {
                if let Some(unit) = construction.as_any().downcast_ref::<BaseUnit>() {
                    unit.get_description(city)
                } else {
                    String::new()
                }
            },
            t if t.contains("Building") => {
                if let Some(building) = construction.as_any().downcast_ref::<Building>() {
                    building.get_description(city, true)
                } else {
                    String::new()
                }
            },
            t if t.contains("PerpetualStatConversion") => {
                if let Some(conversion) = construction.as_any().downcast_ref::<PerpetualStatConversion>() {
                    let rate = conversion.get_conversion_rate(city);
                    tr(&conversion.description.replace("[rate]", &format!("[{}]", rate)))
                } else {
                    String::new()
                }
            },
            t if t.contains("PerpetualConstruction") => {
                if let Some(perpetual) = construction.as_any().downcast_ref::<PerpetualConstruction>() {
                    tr(&perpetual.description)
                } else {
                    String::new()
                }
            },
            _ => String::new(), // Should never happen
        };

        let mut description_label = to_label(&description);
        description_label.set_wrap(true);

        let width = if self.city_screen.is_cramped_portrait() {
            self.city_screen.stage().width() / 3.0
        } else {
            self.city_screen.stage().width() / 4.0
        };

        self.selected_construction_table.add(description_label).colspan(2).width(width);

        // Show sell button if construction is built
        if city_constructions.is_built(&construction.name()) {
            self.show_sell_button(&construction);
        } else if self.buy_button_factory.has_buy_buttons(&construction) {
            self.selected_construction_table.row();

            for button in self.buy_button_factory.get_buy_buttons(&construction) {
                self.selected_construction_table.add(button).pad_top(5.0).colspan(2).center().row();
            }
        }

        // Handle unit promotions
        if let Some(unit) = construction.as_any().downcast_ref::<BaseUnit>() {
            let base_unit = unit.name.clone();
            let build_unit_with_promotions = city.unit_should_use_saved_promotion.get(&base_unit).cloned();

            if build_unit_with_promotions.is_some() {
                self.selected_construction_table.row();

                let mut checkbox = to_checkbox(
                    "Use default promotions",
                    build_unit_with_promotions.unwrap_or(false),
                    move |checked| {
                        city.unit_should_use_saved_promotion.insert(base_unit.clone(), checked);
                    }
                );

                self.selected_construction_table.add(checkbox).colspan(2).center();
            }
        }
    }

    /// Show sell button if construction is a currently sellable building
    fn show_sell_button(&mut self, construction: &Box<dyn IConstruction>) {
        if let Some(building) = construction.as_any().downcast_ref::<Building>() {
            if building.is_sellable() {
                let sell_amount = self.city_screen.get_city().get_gold_for_selling_building(&building.name);
                let sell_text = format!("{{Sell}} {} {}", sell_amount, Fonts::gold());
                let mut sell_building_button = to_text_button(&sell_text);

                self.selected_construction_table.row();
                self.selected_construction_table.add(sell_building_button).pad_top(5.0).colspan(2).center();

                let is_free = self.city_screen.has_free_building(building);
                let enable_sell = !is_free &&
                    !self.city_screen.get_city().is_puppet &&
                    self.city_screen.can_change_state() &&
                    (!self.city_screen.get_city().has_sold_building_this_turn ||
                     self.city_screen.get_city().civ.game_info.game_parameters.god_mode);

                sell_building_button.set_enabled(enable_sell);

                if enable_sell {
                    let construction_clone = construction.clone();
                    let sell_text_clone = sell_text.clone();

                    sell_building_button.on_click(UncivSound::Coin, move || {
                        sell_building_button.disable();
                        self.sell_building_clicked(building, &sell_text_clone);
                    });
                }

                if (self.city_screen.get_city().has_sold_building_this_turn &&
                    !self.city_screen.get_city().civ.game_info.game_parameters.god_mode) ||
                    self.city_screen.get_city().is_puppet ||
                    !self.city_screen.can_change_state() {
                    sell_building_button.disable();
                }
            }
        }
    }

    /// Handle sell building button click
    fn sell_building_clicked(&mut self, construction: &Building, sell_text: &str) {
        close_all_popups();

        let construction_clone = construction.clone();
        let city_screen_clone = self.city_screen.clone();

        ConfirmPopup::new(
            self.city_screen.clone(),
            &format!("Are you sure you want to sell this [{}]?", construction.name),
            sell_text,
            move || {
                city_screen_clone.update();
            },
            move || {
                self.sell_building_confirmed(&construction_clone);
            }
        ).open();
    }

    /// Handle sell building confirmation
    fn sell_building_confirmed(&mut self, construction: &Building) {
        self.city_screen.get_city().sell_building(construction);
        self.city_screen.clear_selection();
        self.city_screen.update();
    }

    /// Pack the table to calculate its size
    pub fn pack(&mut self) {
        self.selected_construction_table.pack();
    }

    /// Render the table
    pub fn render(&mut self, ui: &mut Ui) {
        self.selected_construction_table.render(ui);
    }
}