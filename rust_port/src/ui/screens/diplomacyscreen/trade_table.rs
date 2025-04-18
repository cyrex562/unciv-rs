use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, Ui, Align, Layout, Button, Frame, ScrollArea};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::logic::civilization::Civilization;
use crate::logic::trade::{TradeLogic, TradeRequest, TradeOfferType};
use crate::utils::translations::tr;
use crate::utils::constants::Constants;

use super::diplomacy_screen::DiplomacyScreen;
use super::offer_columns_table::OfferColumnsTable;

/// Table that holds the trade interface between civilizations
pub struct TradeTable {
    civ: Arc<Civilization>,
    other_civilization: Arc<Civilization>,
    diplomacy_screen: Arc<DiplomacyScreen>,
    trade_logic: Arc<TradeLogic>,
    offer_columns_table: OfferColumnsTable,
    offer_trade_text: String,
    offer_button: Button,
    is_trade_offered: bool,
}

impl TradeTable {
    /// Creates a new TradeTable
    pub fn new(
        civ: Arc<Civilization>,
        other_civilization: Arc<Civilization>,
        diplomacy_screen: Arc<DiplomacyScreen>
    ) -> Self {
        let trade_logic = Arc::new(TradeLogic::new(civ.clone(), other_civilization.clone()));

        let offer_trade_text = "{Offer trade}\n({They'll decide on their turn})".to_string();
        let mut offer_button = Button::new(offer_trade_text.clone());
        offer_button.set_enabled(false);

        let mut table = Self {
            civ,
            other_civilization,
            diplomacy_screen,
            trade_logic: trade_logic.clone(),
            offer_columns_table: OfferColumnsTable::new(
                trade_logic.clone(),
                diplomacy_screen.clone(),
                trade_logic.our_civilization.clone(),
                trade_logic.other_civilization.clone(),
                Box::new(move || {
                    // This closure will be called when the trade changes
                    // We'll implement this in the build method
                })
            ),
            offer_trade_text,
            offer_button,
            is_trade_offered: false,
        };

        // Initialize the table
        table.init();

        table
    }

    /// Initializes the trade table
    fn init(&mut self) {
        // Check if there's an existing offer
        let existing_offer = self.other_civilization.trade_requests.iter()
            .find(|request| request.requesting_civ == self.civ.civ_name);

        if let Some(offer) = existing_offer {
            self.trade_logic.current_trade.set(offer.trade.reverse());
            self.offer_columns_table.update();
        }

        // Check if a trade is already offered
        self.is_trade_offered = self.other_civilization.trade_requests.iter()
            .any(|request| request.requesting_civ == self.civ.civ_name);

        if self.is_trade_offered {
            self.offer_button.set_text("Retract offer".tr());
        } else {
            self.offer_button.set_text(self.offer_trade_text.tr());
        }

        // Set up the offer button click handler
        let civ_clone = self.civ.clone();
        let other_civ_clone = self.other_civilization.clone();
        let trade_logic_clone = self.trade_logic.clone();
        let offer_columns_table_clone = self.offer_columns_table.clone();
        let offer_trade_text_clone = self.offer_trade_text.clone();

        self.offer_button.on_click(move || {
            if self.is_trade_offered {
                self.retract_offer();
                return;
            }

            // If there is a research agreement trade, make sure both civilizations should be able to pay for it.
            // If not lets add an extra gold offer to satisfy this.
            // There must be enough gold to add to the offer to satisfy this, otherwise the research agreement button would be disabled
            if trade_logic_clone.current_trade.our_offers.iter().any(|offer| offer.name == Constants::RESEARCH_AGREEMENT) {
                let research_cost = civ_clone.diplomacy_functions.get_research_agreement_cost(&other_civ_clone);

                let current_player_offered_gold = trade_logic_clone.current_trade.our_offers.iter()
                    .find(|offer| offer.offer_type == TradeOfferType::Gold)
                    .map(|offer| offer.amount)
                    .unwrap_or(0);

                let other_civ_offered_gold = trade_logic_clone.current_trade.their_offers.iter()
                    .find(|offer| offer.offer_type == TradeOfferType::Gold)
                    .map(|offer| offer.amount)
                    .unwrap_or(0);

                let new_current_player_gold = civ_clone.gold + other_civ_offered_gold - research_cost;
                let new_other_civ_gold = other_civ_clone.gold + current_player_offered_gold - research_cost;

                // Check if we require more gold from them
                if new_current_player_gold < 0 {
                    if let Some(gold_offer) = trade_logic_clone.their_available_offers.iter()
                        .find(|offer| offer.offer_type == TradeOfferType::Gold) {
                        let mut modified_offer = gold_offer.clone();
                        modified_offer.amount = -new_current_player_gold;
                        offer_columns_table_clone.add_offer(
                            &modified_offer,
                            &mut trade_logic_clone.current_trade.their_offers,
                            &mut trade_logic_clone.current_trade.our_offers
                        );
                    }
                }

                // Check if they require more gold from us
                if new_other_civ_gold < 0 {
                    if let Some(gold_offer) = trade_logic_clone.our_available_offers.iter()
                        .find(|offer| offer.offer_type == TradeOfferType::Gold) {
                        let mut modified_offer = gold_offer.clone();
                        modified_offer.amount = -new_other_civ_gold;
                        offer_columns_table_clone.add_offer(
                            &modified_offer,
                            &mut trade_logic_clone.current_trade.our_offers,
                            &mut trade_logic_clone.current_trade.their_offers
                        );
                    }
                }
            }

            // Add the trade request
            let trade_request = TradeRequest::new(
                self.civ.civ_name.clone(),
                self.trade_logic.current_trade.reverse()
            );
            self.other_civilization.trade_requests.push(trade_request);
            self.civ.cache.update_civ_resources();

            // Update the button text
            self.offer_button.set_text("Retract offer".tr());
            self.is_trade_offered = true;
        });
    }

    /// Retracts the current trade offer
    fn retract_offer(&mut self) {
        self.other_civilization.trade_requests.retain(|request| request.requesting_civ != self.civ.civ_name);
        self.civ.cache.update_civ_resources();
        self.offer_button.set_text(self.offer_trade_text.tr());
        self.is_trade_offered = false;
    }

    /// Called when the trade changes
    fn on_change(&mut self) {
        self.offer_columns_table.update();
        self.retract_offer();

        // Enable the offer button if there are offers
        let has_offers = !self.trade_logic.current_trade.their_offers.is_empty() ||
                         !self.trade_logic.current_trade.our_offers.is_empty();
        self.offer_button.set_enabled(has_offers);
    }

    /// Enables or disables the offer button
    pub fn enable_offer_button(&mut self, is_enabled: bool) {
        self.offer_button.set_enabled(is_enabled);
    }

    /// Builds the trade table UI
    pub fn build(&self, ui: &mut Ui) -> egui::Frame {
        let mut frame = egui::Frame::none();
        frame.set_padding(egui::style::Spacing::new(5.0));

        // Add the offer columns table
        frame.add(self.offer_columns_table.build(ui));

        // Add a separator
        frame.add_separator();

        // Add the offer button
        frame.add(self.offer_button.clone());

        frame
    }
}