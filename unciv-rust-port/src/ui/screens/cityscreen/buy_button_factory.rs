use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashMap;

use crate::constants::Constants;
use crate::logic::city::CityConstructions;
use crate::logic::map::tile::Tile;
use crate::models::Religion;
use crate::models::ruleset::{Building, IConstruction, INonPerpetualConstruction, PerpetualConstruction};
use crate::models::ruleset::unique::UniqueType;
use crate::models::stats::Stat;
use crate::ui::audio::SoundPlayer;
use crate::ui::components::UncivTooltip;
use crate::ui::components::extensions::{disable, is_enabled, to_text_button};
use crate::ui::components::input::{KeyboardBinding, on_activation};
use crate::ui::popups::{Popup, close_all_popups};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::screens::cityscreen::CityScreen;
use crate::utils::translations::tr;

/// This struct handles everything related to buying constructions. This includes
/// showing and handling [ConfirmBuyPopup] and the actual purchase in [purchase_construction].
pub struct BuyButtonFactory<'a> {
    city_screen: &'a CityScreen,
    preferred_buy_stat: Stat, // Used for keyboard buy
}

impl<'a> BuyButtonFactory<'a> {
    /// Create a new BuyButtonFactory
    pub fn new(city_screen: &'a CityScreen) -> Self {
        Self {
            city_screen,
            preferred_buy_stat: Stat::Gold,
        }
    }

    /// Check if there are buy buttons for a construction
    pub fn has_buy_buttons(&self, construction: Option<&dyn IConstruction>) -> bool {
        !self.get_buy_buttons(construction).is_empty()
    }

    /// Get the buy buttons for a construction
    pub fn get_buy_buttons(&self, construction: Option<&dyn IConstruction>) -> Vec<egui::Button> {
        let selection = self.city_screen.selected_construction.is_some() ||
                        self.city_screen.selected_queue_entry >= 0;

        if selection && construction.is_some() && !construction.unwrap().is::<PerpetualConstruction>() {
            Stat::stats_usable_to_buy()
                .iter()
                .filter_map(|&stat| {
                    self.get_buy_button(
                        construction.unwrap().as_any().downcast_ref::<dyn INonPerpetualConstruction>(),
                        stat
                    )
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get a buy button for a construction and stat
    fn get_buy_button(&self, construction: Option<&dyn INonPerpetualConstruction>, stat: Stat) -> Option<egui::Button> {
        if !Stat::stats_usable_to_buy().contains(&stat) || construction.is_none() {
            return None;
        }

        let construction = construction.unwrap();
        let city = &self.city_screen.city;
        let mut button = egui::Button::new("");

        if !self.is_construction_purchase_shown(construction, stat) {
            // This can't ever be bought with the given currency.
            // We want one disabled "buy" button without a price for "priceless" buildings such as wonders
            // We don't want such a button when the construction can be bought using a different currency
            if stat != Stat::Gold || construction.can_be_purchased_with_any_stat(city) {
                return None;
            }
            button.set_text(tr("Buy"));
            disable(&mut button);
        } else {
            let construction_buy_cost = construction.get_stat_buy_cost(city, stat).unwrap();
            button.set_text(format!("{} {} {}", tr("Buy"), tr(&construction_buy_cost.to_string()), stat.character));

            let construction_clone = construction.clone();
            let stat_clone = stat;
            let city_screen = self.city_screen;

            on_activation(&mut button, KeyboardBinding::BuyConstruction, move || {
                disable(&mut button);
                city_screen.buy_button_factory.buy_button_on_click(construction_clone, stat_clone);
            });

            // allow puppets, since is_construction_purchase_allowed handles that and exceptions to that rule
            button.set_enabled(
                self.city_screen.can_change_state &&
                city.city_constructions.is_construction_purchase_allowed(construction, stat, construction_buy_cost)
            );

            // Not very intelligent, but the least common currency "wins"
            self.preferred_buy_stat = stat;

            if city.city_constructions.is_construction_purchase_blocked_by_unit(construction) {
                UncivTooltip::add_tooltip(&mut button, "Move unit out of city first", 26.0, false);
            }
        }

        button.set_padding(egui::style::Padding::same(5.0));

        Some(button)
    }

    /// Handle a buy button click
    fn buy_button_on_click(&self, construction: &dyn INonPerpetualConstruction, stat: Stat) {
        if !construction.is::<Building>() || !construction.has_unique(UniqueType::CreatesOneImprovement) {
            self.ask_to_buy_construction(construction, stat, None);
            return;
        }

        if self.city_screen.selected_queue_entry < 0 {
            self.city_screen.start_pick_tile_for_creates_one_improvement(construction, stat, true);
            return;
        }

        // Buying a UniqueType.CreatesOneImprovement building from queue must pass down
        // the already selected tile, otherwise a new one is chosen from Automation code.
        let improvement = construction.get_improvement_to_create(
            self.city_screen.city.get_ruleset(),
            &self.city_screen.city.civ
        ).unwrap();

        let tile_for_improvement = self.city_screen.city.city_constructions.get_tile_for_improvement(&improvement.name);
        self.ask_to_buy_construction(construction, stat, tile_for_improvement);
    }

    /// Ask whether user wants to buy [construction] for [stat].
    ///
    /// Used from onClick and keyboard dispatch, thus only minimal parameters are passed,
    /// and it needs to do all checks and the sound as appropriate.
    pub fn ask_to_buy_construction(
        &self,
        construction: &dyn INonPerpetualConstruction,
        stat: Stat,
        tile: Option<&Tile>
    ) {
        if !self.is_construction_purchase_shown(construction, stat) {
            return;
        }

        let city = &self.city_screen.city;
        let construction_stat_buy_cost = construction.get_stat_buy_cost(city, stat).unwrap();

        if !city.city_constructions.is_construction_purchase_allowed(construction, stat, construction_stat_buy_cost) {
            return;
        }

        close_all_popups();
        ConfirmBuyPopup::new(
            self.city_screen,
            construction.clone(),
            stat,
            construction_stat_buy_cost,
            tile.cloned(),
        );
    }

    /// This tests whether the buy button should be _shown_
    fn is_construction_purchase_shown(&self, construction: &dyn INonPerpetualConstruction, stat: Stat) -> bool {
        let city = &self.city_screen.city;
        construction.can_be_purchased_with_stat(city, stat)
    }

    /// Called only by ask_to_buy_construction's Yes answer - not to be confused with [CityConstructions.purchase_construction]
    ///
    /// # Arguments
    ///
    /// * `construction` - The construction to purchase
    /// * `stat` - The stat to use for purchase (default: Gold)
    /// * `tile` - The tile to place the construction on (supports [UniqueType.CreatesOneImprovement])
    fn purchase_construction(
        &self,
        construction: &dyn INonPerpetualConstruction,
        stat: Stat,
        tile: Option<&Tile>
    ) {
        SoundPlayer::play(stat.purchase_sound);
        let city = &self.city_screen.city;

        if !city.city_constructions.purchase_construction(
            construction,
            self.city_screen.selected_queue_entry,
            false,
            stat,
            tile
        ) {
            let mut popup = Popup::new(self.city_screen);
            popup.add(format!(
                "No space available to place [{}] near [{}]",
                tr(&construction.name),
                tr(&city.name)
            ));
            popup.add_close_button();
            popup.open();
            return;
        }

        if self.city_screen.selected_queue_entry >= 0 ||
           self.city_screen.selected_construction.as_ref()
               .map_or(true, |c| !c.is_buildable(&city.city_constructions))
        {
            self.city_screen.selected_queue_entry = -1;
            self.city_screen.clear_selection();

            // Allow buying next queued or auto-assigned construction right away
            city.city_constructions.choose_next_construction();

            if !city.city_constructions.current_construction_from_queue.is_empty() {
                if let Some(new_construction) = city.city_constructions.get_current_construction()
                    .and_then(|c| c.as_any().downcast_ref::<dyn INonPerpetualConstruction>())
                {
                    self.city_screen.select_construction(new_construction);
                }
            }
        }

        self.city_screen.city.reassign_population();
        self.city_screen.update();
    }
}

/// Popup for confirming a construction purchase
pub struct ConfirmBuyPopup<'a> {
    city_screen: &'a CityScreen,
    construction: Box<dyn INonPerpetualConstruction>,
    stat: Stat,
    construction_stat_buy_cost: i32,
    tile: Option<Box<Tile>>,
}

impl<'a> ConfirmBuyPopup<'a> {
    /// Create a new ConfirmBuyPopup
    pub fn new(
        city_screen: &'a CityScreen,
        construction: Box<dyn INonPerpetualConstruction>,
        stat: Stat,
        construction_stat_buy_cost: i32,
        tile: Option<Box<Tile>>,
    ) -> Self {
        let mut popup = Self {
            city_screen,
            construction,
            stat,
            construction_stat_buy_cost,
            tile,
        };

        popup.show();
        popup
    }

    /// Show the popup
    fn show(&mut self) {
        let city = &self.city_screen.city;
        let balance = city.get_stat_reserve(self.stat);
        let majority_religion = city.religion.get_majority_religion();
        let your_religion = city.civ.religion_manager.religion;

        let is_buying_with_faith_for_foreign_religion =
            self.construction.has_unique(UniqueType::ReligiousUnit) &&
            !self.construction.has_unique(UniqueType::TakeReligionOverBirthCity) &&
            majority_religion != your_religion;

        let mut popup = Popup::new(self.city_screen.stage);

        popup.add_good_sized_label(&format!(
            "Currently you have [{}] [{}].",
            balance,
            self.stat.name
        )).pad_bottom(10.0).row();

        if is_buying_with_faith_for_foreign_religion {
            // Earlier tests should forbid this Popup unless both religions are non-null, but to be safe:
            let get_religion_name = |religion: Option<&Religion>| {
                religion.map(|r| r.get_religion_display_name()).unwrap_or(Constants::UNKNOWN_CITY_NAME)
            };

            popup.add_good_sized_label(&format!(
                "You are buying a religious unit in a city that doesn't follow the religion you founded ([{}]). \
                This means that the unit is tied to that foreign religion ([{}]) and will be less useful.",
                get_religion_name(your_religion),
                get_religion_name(majority_religion)
            )).row();

            let mut warning_label = popup.add_good_sized_label(
                "Are you really sure you want to purchase this unit?",
                Constants::HEADING_FONT_SIZE
            );
            warning_label.set_color(egui::Color32::from_rgb(178, 34, 34)); // FIREBRICK
            warning_label.pad_bottom(10.0).row();
        }

        popup.add_good_sized_label(&format!(
            "Would you like to purchase [{}] for [{}] [{}]?",
            self.construction.name,
            self.construction_stat_buy_cost,
            self.stat.character
        )).row();

        let city_screen = self.city_screen;
        let construction = self.construction.clone();
        let stat = self.stat;
        let tile = self.tile.clone();

        popup.add_close_button(Constants::CANCEL, KeyboardBinding::Cancel, move || {
            city_screen.update();
        });

        let confirm_style = BaseScreen::skin().get("positive");

        popup.add_ok_button("Purchase", KeyboardBinding::Confirm, confirm_style, move || {
            city_screen.buy_button_factory.purchase_construction(&*construction, stat, tile.as_deref());
        });

        popup.equalize_last_two_button_widths();
        popup.open(true);
    }
}