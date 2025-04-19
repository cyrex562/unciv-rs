// Source: orig_src/core/src/com/unciv/ui/screens/overviewscreen/TradesOverviewTab.kt

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use egui::{Ui, Color32, Align, ScrollArea, Button, Image, Response};
use crate::models::civilization::Civilization;
use crate::models::trade::{Trade, TradeOffersList};
use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::widgets::{TabbedPager, ExpanderTab};
use crate::ui::images::ImageGetter;
use crate::constants::Constants;
use crate::game::UncivGame;
use super::empire_overview_tab::EmpireOverviewTab;
use super::empire_overview_categories::EmpireOverviewCategories;

pub struct TradesOverviewTab {
    viewing_player: Rc<RefCell<Civilization>>,
    overview_screen: Rc<RefCell<dyn BaseScreen>>,
    persist_data: Rc<RefCell<TradesOverviewTabPersistableData>>,
    game: Rc<RefCell<UncivGame>>,
}

impl TradesOverviewTab {
    pub fn new(
        viewing_player: Rc<RefCell<Civilization>>,
        overview_screen: Rc<RefCell<dyn BaseScreen>>,
        persist_data: Option<TradesOverviewTabPersistableData>,
    ) -> Self {
        let game = Rc::clone(&overview_screen.borrow().game);

        let mut tab = Self {
            viewing_player,
            overview_screen,
            persist_data: Rc::new(RefCell::new(persist_data.unwrap_or_default())),
            game,
        };

        tab.init();
        tab
    }

    fn init(&mut self) {
        let mut table = Ui::default();
        table.defaults().pad(10.0);

        // Get diplomacies with pending trade
        let diplomacies_with_pending_trade: Vec<_> = self.viewing_player.borrow().diplomacy.values()
            .filter(|diplomacy| {
                diplomacy.other_civ().trade_requests.iter()
                    .any(|request| request.requesting_civ == self.viewing_player.borrow().civ_name)
            })
            .collect();

        if !diplomacies_with_pending_trade.is_empty() {
            table.add_label("Pending trades", Constants::heading_font_size(), false)
                .pad_top(10.0).row();

            for diplomacy in diplomacies_with_pending_trade {
                let other_civ = diplomacy.other_civ();
                let trade_requests: Vec<_> = other_civ.trade_requests.iter()
                    .filter(|request| request.requesting_civ == self.viewing_player.borrow().civ_name)
                    .collect();

                for trade_request in trade_requests {
                    let reversed_trade = trade_request.trade.reverse();
                    table.add(self.create_trade_table(&reversed_trade, &other_civ)).row();
                }
            }
        }

        // Get diplomacies with existing trade
        let mut diplomacies_with_existing_trade: Vec<_> = self.viewing_player.borrow().diplomacy.values()
            .filter(|diplomacy| !diplomacy.trades.is_empty())
            .collect();

        // Sort by trade duration
        diplomacies_with_existing_trade.sort_by(|d1, d2| {
            let d1_offers = d1.trades.first().map(|t| &t.our_offers).unwrap_or(&Vec::new());
            let d2_offers = d2.trades.first().map(|t| &t.our_offers).unwrap_or(&Vec::new());

            let d1_max_duration = d1_offers.iter()
                .map(|offer| offer.duration)
                .max()
                .unwrap_or(0);

            let d2_max_duration = d2_offers.iter()
                .map(|offer| offer.duration)
                .max()
                .unwrap_or(0);

            d2_max_duration.cmp(&d1_max_duration) // Descending order
        });

        if !diplomacies_with_existing_trade.is_empty() {
            table.add_label("Current trades", Constants::heading_font_size(), false)
                .pad_top(10.0).row();

            for diplomacy in diplomacies_with_existing_trade {
                let other_civ = diplomacy.other_civ();
                for trade in &diplomacy.trades {
                    table.add(self.create_trade_table(trade, &other_civ)).row();
                }
            }
        }

        self.table = table;
    }

    fn create_trade_table(&self, trade: &Trade, other_civ: &Rc<RefCell<Civilization>>) -> Ui {
        let mut table = Ui::default();

        let our_offers_table = self.create_offers_table(
            &self.viewing_player,
            &trade.our_offers,
            trade.their_offers.len()
        );

        let their_offers_table = self.create_offers_table(
            other_civ,
            &trade.their_offers,
            trade.our_offers.len()
        );

        table.add(our_offers_table)
            .min_width(self.overview_screen.borrow().stage.width / 4.0)
            .fill_y();

        table.add(their_offers_table)
            .min_width(self.overview_screen.borrow().stage.width / 4.0)
            .fill_y();

        table
    }

    fn create_offers_table(
        &self,
        civ: &Rc<RefCell<Civilization>>,
        offers_list: &TradeOffersList,
        number_of_other_sides_offers: usize
    ) -> Ui {
        let mut table = Ui::default();
        table.defaults().pad(10.0);

        // Set background color based on civilization
        let outer_color = civ.borrow().nation.get_outer_color();
        table.background = self.overview_screen.borrow().skin_strings.get_ui_background(
            "OverviewScreen/TradesOverviewTab/OffersTable",
            outer_color
        );

        // Add civilization name
        let inner_color = civ.borrow().nation.get_inner_color();
        table.add_label(&civ.borrow().civ_name, 0, false, inner_color).row();
        table.add_separator();

        // Add offers
        for offer in offers_list {
            let mut offer_text = offer.get_offer_text();
            if !offer_text.contains('\n') {
                offer_text.push('\n');
            }
            table.add_label(&offer_text, 0, false, inner_color).row();
        }

        // Add empty rows to match the other side's number of offers
        for _ in 0..(number_of_other_sides_offers - offers_list.len()) {
            table.add_label("\n", 0, false).row();
        }

        table
    }
}

impl EmpireOverviewTab for TradesOverviewTab {
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
pub struct TradesOverviewTabPersistableData {
    // Add any persistent data fields here
}

impl EmpireOverviewTabPersistableData for TradesOverviewTabPersistableData {
    fn is_empty(&self) -> bool {
        true // Implement based on actual fields
    }
}