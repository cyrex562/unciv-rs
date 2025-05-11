use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, Ui, Align, Layout, Button, Frame, ScrollArea};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::images::ImageGetter;
use crate::ui::popups::AskNumberPopup;
use crate::logic::civilization::Civilization;
use crate::logic::trade::{TradeLogic, TradeOffer, TradeOffersList, TradeOfferType};
use crate::utils::translations::tr;
use crate::utils::constants::Constants;

use super::diplomacy_screen::DiplomacyScreen;
use super::offers_list_scroll::OffersListScroll;

/// This is the class that holds the 4 columns of the offers (ours/theirs/ offered/available) in trade
pub struct OfferColumnsTable {
    trade_logic: Arc<TradeLogic>,
    screen: Arc<DiplomacyScreen>,
    our_civ: Arc<Civilization>,
    their_civ: Arc<Civilization>,
    on_change: Box<dyn Fn() + Send + Sync>,

    // UI components
    our_available_offers_table: OffersListScroll,
    our_offers_table: OffersListScroll,
    their_offers_table: OffersListScroll,
    their_available_offers_table: OffersListScroll,

    // Layout state
    is_portrait_mode: bool,
    column_width: f32,
}

impl OfferColumnsTable {
    /// Creates a new OfferColumnsTable
    pub fn new(
        trade_logic: Arc<TradeLogic>,
        screen: Arc<DiplomacyScreen>,
        our_civ: Arc<Civilization>,
        their_civ: Arc<Civilization>,
        on_change: Box<dyn Fn() + Send + Sync>
    ) -> Self {
        let is_portrait_mode = screen.is_narrower_than_4to3();
        let column_width = screen.get_trade_columns_width() - 20.0; // Subtract padding: ours and OffersListScroll's

        let our_available_offers_table = OffersListScroll::new(
            "OurAvail".to_string(),
            Box::new(move |offer| {
                trade_logic.current_trade.run(|trade| {
                    Self::offer_click_implementation(
                        offer,
                        false,
                        &mut trade.our_offers,
                        &mut trade.their_offers,
                        &trade_logic.our_civilization
                    );
                });
            })
        );

        let our_offers_table = OffersListScroll::new(
            "OurTrade".to_string(),
            Box::new(move |offer| {
                trade_logic.current_trade.run(|trade| {
                    Self::offer_click_implementation(
                        offer,
                        true,
                        &mut trade.our_offers,
                        &mut trade.their_offers,
                        &trade_logic.our_civilization
                    );
                });
            })
        );

        let their_offers_table = OffersListScroll::new(
            "TheirTrade".to_string(),
            Box::new(move |offer| {
                trade_logic.current_trade.run(|trade| {
                    Self::offer_click_implementation(
                        offer,
                        true,
                        &mut trade.their_offers,
                        &mut trade.our_offers,
                        &trade_logic.other_civilization
                    );
                });
            })
        );

        let their_available_offers_table = OffersListScroll::new(
            "TheirAvail".to_string(),
            Box::new(move |offer| {
                trade_logic.current_trade.run(|trade| {
                    Self::offer_click_implementation(
                        offer,
                        false,
                        &mut trade.their_offers,
                        &mut trade.our_offers,
                        &trade_logic.other_civilization
                    );
                });
            })
        );

        Self {
            trade_logic,
            screen,
            our_civ,
            their_civ,
            on_change,
            our_available_offers_table,
            our_offers_table,
            their_offers_table,
            their_available_offers_table,
            is_portrait_mode,
            column_width,
        }
    }

    /// Adds an offer to the specified offer list and optionally to the corresponding offer list
    pub fn add_offer(&self, offer: &TradeOffer, offer_list: &mut TradeOffersList, corresponding_offer_list: &mut TradeOffersList) {
        offer_list.push(offer.clone());
        if offer.offer_type == TradeOfferType::Treaty {
            corresponding_offer_list.push(offer.clone());
        }
        (self.on_change)();
    }

    /// Handles the click on an offer
    fn offer_click_implementation(
        offer: &TradeOffer,
        invert: bool,
        list: &mut TradeOffersList,
        counter_list: &mut TradeOffersList,
        civ: &Civilization
    ) {
        match offer.offer_type {
            TradeOfferType::Gold => {
                Self::open_gold_selection_popup(
                    offer,
                    list,
                    civ.gold,
                    Box::new(move |user_input| {
                        offer.amount = user_input;
                        if list.iter().any(|o| o.offer_type == offer.offer_type) {
                            if let Some(existing) = list.iter_mut().find(|o| o.offer_type == offer.offer_type) {
                                existing.amount = offer.amount;
                            }
                        } else {
                            list.push(offer.clone());
                        }
                        if offer.amount == 0 {
                            list.retain(|o| o.offer_type != offer.offer_type);
                        }
                    })
                );
            },
            TradeOfferType::GoldPerTurn => {
                Self::open_gold_selection_popup(
                    offer,
                    list,
                    civ.stats.stats_for_next_turn.gold as i32,
                    Box::new(move |user_input| {
                        offer.amount = user_input;
                        if list.iter().any(|o| o.offer_type == offer.offer_type) {
                            if let Some(existing) = list.iter_mut().find(|o| o.offer_type == offer.offer_type) {
                                existing.amount = offer.amount;
                            }
                        } else {
                            list.push(offer.clone());
                        }
                        if offer.amount == 0 {
                            list.retain(|o| o.offer_type != offer.offer_type);
                        }
                    })
                );
            },
            _ => {
                let mut modified_offer = offer.clone();
                if invert {
                    modified_offer.amount = -modified_offer.amount;
                }
                list.push(modified_offer);
                if offer.offer_type == TradeOfferType::Treaty {
                    counter_list.push(offer.clone());
                }
            }
        }
    }

    /// Opens a popup for selecting gold amount
    fn open_gold_selection_popup(
        offer: &TradeOffer,
        our_offers: &mut TradeOffersList,
        max_gold: i32,
        action_on_ok: Box<dyn Fn(i32) + Send + Sync>
    ) {
        let existing_gold_offer = our_offers.iter().find(|o| o.offer_type == offer.offer_type);
        let default_value = if let Some(existing) = existing_gold_offer {
            existing.amount
        } else {
            offer.amount
        };

        let amount_buttons = if offer.offer_type == TradeOfferType::Gold {
            vec![50, 500]
        } else {
            vec![5, 15]
        };

        AskNumberPopup::new(
            "Enter the amount of gold".to_string(),
            ImageGetter::get_stat_icon("Gold").surround_with_circle(80.0),
            default_value,
            amount_buttons,
            0..=max_gold,
            action_on_ok
        ).open();
    }

    /// Builds the offer columns table UI
    pub fn build(&self, ui: &mut Ui) -> egui::Frame {
        let mut table = egui::Frame::none();
        table.set_padding(egui::style::Spacing::new(5.0));

        if !self.is_portrait_mode {
            // In landscape, arrange in 4 panels: ours left / theirs right ; items top / offers bottom.
            table.add("Our items".tr());
            table.add(format!("[{}]'s items", self.trade_logic.other_civilization.civ_name).tr());

            let mut our_available_scroll = ScrollArea::vertical();
            our_available_scroll.show(ui, |ui| {
                self.our_available_offers_table.build(ui);
            });
            table.add(our_available_scroll);

            let mut their_available_scroll = ScrollArea::vertical();
            their_available_scroll.show(ui, |ui| {
                self.their_available_offers_table.build(ui);
            });
            table.add(their_available_scroll);

            table.add_separator();

            table.add("Our trade offer".tr());
            table.add(format!("[{}]'s trade offer", self.trade_logic.other_civilization.civ_name).tr());

            let mut our_offers_scroll = ScrollArea::vertical();
            our_offers_scroll.show(ui, |ui| {
                self.our_offers_table.build(ui);
            });
            table.add(our_offers_scroll);

            let mut their_offers_scroll = ScrollArea::vertical();
            their_offers_scroll.show(ui, |ui| {
                self.their_offers_table.build(ui);
            });
            table.add(their_offers_scroll);
        } else {
            // In portrait, this will arrange the items lists vertically
            // and the offers still side-by-side below that
            table.add("Our items".tr()).colspan(2);

            let mut our_available_scroll = ScrollArea::vertical();
            our_available_scroll.show(ui, |ui| {
                self.our_available_offers_table.build(ui);
            });
            table.add(our_available_scroll).colspan(2);

            table.add_separator();

            table.add(format!("[{}]'s items", self.trade_logic.other_civilization.civ_name).tr()).colspan(2);

            let mut their_available_scroll = ScrollArea::vertical();
            their_available_scroll.show(ui, |ui| {
                self.their_available_offers_table.build(ui);
            });
            table.add(their_available_scroll).colspan(2);

            table.add_separator();

            table.add("Our trade offer".tr());
            table.add(format!("[{}]'s trade offer", self.trade_logic.other_civilization.civ_name).tr());

            let mut our_offers_scroll = ScrollArea::vertical();
            our_offers_scroll.show(ui, |ui| {
                self.our_offers_table.build(ui);
            });
            table.add(our_offers_scroll);

            let mut their_offers_scroll = ScrollArea::vertical();
            their_offers_scroll.show(ui, |ui| {
                self.their_offers_table.build(ui);
            });
            table.add(their_offers_scroll);
        }

        table
    }

    /// Updates the offer columns table
    pub fn update(&mut self) {
        let our_filtered_offers = self.trade_logic.our_available_offers.without(&self.trade_logic.current_trade.our_offers);
        let their_filtered_offers = self.trade_logic.their_available_offers.without(&self.trade_logic.current_trade.their_offers);

        let our_untradables = self.trade_logic.our_civilization.get_per_turn_resources_with_origins_for_trade()
            .remove_all(&Constants::TRADABLE);
        let their_untradables = self.trade_logic.other_civilization.get_per_turn_resources_with_origins_for_trade()
            .remove_all(&Constants::TRADABLE);

        self.our_available_offers_table.update(
            &our_filtered_offers,
            &self.trade_logic.their_available_offers,
            &our_untradables,
            &self.our_civ,
            &self.their_civ
        );

        self.our_offers_table.update(
            &self.trade_logic.current_trade.our_offers,
            &self.trade_logic.their_available_offers,
            &self.our_civ,
            &self.their_civ
        );

        self.their_offers_table.update(
            &self.trade_logic.current_trade.their_offers,
            &self.trade_logic.our_available_offers,
            &self.our_civ,
            &self.their_civ
        );

        self.their_available_offers_table.update(
            &their_filtered_offers,
            &self.trade_logic.our_available_offers,
            &their_untradables,
            &self.our_civ,
            &self.their_civ
        );
    }
}