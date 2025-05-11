use std::collections::HashMap;
use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, Ui, Align, Layout, Button, Frame, ScrollArea, Image};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::{ColorMarkupLabel, ExpanderTab};
use crate::ui::images::ImageGetter;
use crate::logic::civilization::Civilization;
use crate::logic::trade::{TradeOffer, TradeOffersList, TradeOfferType};
use crate::models::ruleset::tile::ResourceSupplyList;
use crate::utils::translations::tr;
use crate::utils::constants::Constants;
use crate::game::UncivGame;

/// Widget for one fourth of an OfferColumnsTable - instantiated for ours/theirs × available/traded
///
/// # Arguments
///
/// * `persistence_id` - Part of ID added to ExpanderTab.persistence_id to distinguish the four usecases
/// * `on_offer_clicked` - What to do when a trade button is clicked
pub struct OffersListScroll {
    persistence_id: String,
    on_offer_clicked: Box<dyn Fn(&TradeOffer) + Send + Sync>,
    expander_tabs: HashMap<TradeOfferType, ExpanderTab>,
}

impl OffersListScroll {
    /// Creates a new OffersListScroll
    pub fn new(
        persistence_id: String,
        on_offer_clicked: Box<dyn Fn(&TradeOffer) + Send + Sync>
    ) -> Self {
        Self {
            persistence_id,
            on_offer_clicked,
            expander_tabs: HashMap::new(),
        }
    }

    /// Updates the offers list with new data
    ///
    /// # Arguments
    ///
    /// * `offers_to_display` - The offers which should be displayed as buttons
    /// * `other_side_offers` - The list of other side's offers to compare with whether these offers are unique
    /// * `untradable_offers` - Things we got from sources that we can't trade on, displayed for completeness - should be aggregated per resource to "All" origin
    /// * `our_civ` - Our civilization
    /// * `their_civ` - Their civilization
    pub fn update(
        &mut self,
        offers_to_display: &TradeOffersList,
        other_side_offers: &TradeOffersList,
        untradable_offers: &ResourceSupplyList,
        our_civ: &Civilization,
        their_civ: &Civilization
    ) {
        self.expander_tabs.clear();

        // Create expander tabs for each offer type that has offers
        for offer_type in TradeOfferType::iter() {
            let label_name = match offer_type {
                TradeOfferType::Gold | TradeOfferType::GoldPerTurn | TradeOfferType::Treaty |
                TradeOfferType::Agreement | TradeOfferType::Introduction => "",
                TradeOfferType::LuxuryResource => "Luxury resources",
                TradeOfferType::StrategicResource => "Strategic resources",
                TradeOfferType::StockpiledResource => "Stockpiled resources",
                TradeOfferType::Technology => "Technologies",
                TradeOfferType::WarDeclaration => "Declarations of war",
                TradeOfferType::City => "Cities",
            };

            let offers_of_type: Vec<&TradeOffer> = offers_to_display.iter()
                .filter(|offer| offer.offer_type == offer_type)
                .collect();

            if !label_name.is_empty() && !offers_of_type.is_empty() {
                let persistence_id = format!("Trade.{}.{:?}", self.persistence_id, offer_type);
                let mut expander_tab = ExpanderTab::new(label_name, persistence_id);
                expander_tab.set_padding(5.0);
                self.expander_tabs.insert(offer_type, expander_tab);
            }
        }

        // Process each offer type
        for offer_type in TradeOfferType::iter() {
            let mut offers_of_type: Vec<&TradeOffer> = offers_to_display.iter()
                .filter(|offer| offer.offer_type == offer_type)
                .collect();

            // Sort offers based on settings
            if UncivGame::current().settings.order_trade_offers_by_amount {
                offers_of_type.sort_by(|a, b| b.amount.cmp(&a.amount));
            } else {
                offers_of_type.sort_by(|a, b| {
                    if a.offer_type == TradeOfferType::City {
                        a.get_offer_text().cmp(&b.get_offer_text())
                    } else {
                        a.name.tr().cmp(&b.name.tr())
                    }
                });
            }

            // If this offer type has an expander tab, add it to the UI
            if let Some(expander_tab) = self.expander_tabs.get_mut(&offer_type) {
                expander_tab.clear();
            }

            // Process each offer
            for offer in offers_of_type {
                let trade_label = offer.get_offer_text(untradable_offers.sum_by(&offer.name));

                // Get the appropriate icon for the offer type
                let trade_icon = match offer.offer_type {
                    TradeOfferType::LuxuryResource | TradeOfferType::StrategicResource => {
                        Some(ImageGetter::get_resource_portrait(&offer.name, 30.0))
                    },
                    TradeOfferType::WarDeclaration => {
                        Some(ImageGetter::get_nation_portrait(
                            &our_civ.game_info.ruleset.nations[&offer.name].unwrap(),
                            30.0
                        ))
                    },
                    _ => None,
                };

                // Create the trade button
                let mut trade_button = Button::new(trade_label);

                // Set up the button with icon if available
                if let Some(icon) = &trade_icon {
                    trade_button.set_icon(icon.clone());
                    trade_button.set_icon_size(30.0);
                }

                trade_button.set_text_align(Align::Center);
                trade_button.set_padding(5.0);

                // Determine amount per click based on offer type
                let amount_per_click = match offer.offer_type {
                    TradeOfferType::Gold => 50,
                    TradeOfferType::Treaty => i32::MAX,
                    _ => 1,
                };

                // Check if the offer is tradable
                let is_tradable = offer.is_tradable()
                    && offer.name != Constants::PEACE_TREATY // can't disable peace treaty!
                    && (offer.name != Constants::RESEARCH_AGREEMENT
                        // If we have a research agreement make sure the total gold of both Civs is higher than the total cost
                        // If both civs combined can pay for the research agreement, don't disable it. One can offer the other it's gold.
                        || (our_civ.gold + their_civ.gold > our_civ.diplomacy_functions.get_research_agreement_cost(their_civ) * 2));

                if is_tradable {
                    // Highlight unique suggestions
                    if (offer_type == TradeOfferType::LuxuryResource || offer_type == TradeOfferType::StrategicResource)
                        && other_side_offers.iter().all(|o| o.offer_type != offer.offer_type || o.name != offer.name || o.amount < 0) {
                        trade_button.set_text_color(Color32::GREEN);
                    }

                    // Set up click handler
                    let on_click = self.on_offer_clicked.clone();
                    let offer_clone = offer.clone();
                    trade_button.on_click(move || {
                        let amount_transferred = amount_per_click.min(offer_clone.amount);
                        let mut modified_offer = offer_clone.clone();
                        modified_offer.amount = amount_transferred;
                        on_click(&modified_offer);
                    });
                } else {
                    trade_button.disable(); // for instance, we have negative gold
                }

                // Add the button to the appropriate container
                if let Some(expander_tab) = self.expander_tabs.get_mut(&offer_type) {
                    expander_tab.add(trade_button);
                }
            }
        }
    }

    /// Builds the offers list UI
    pub fn build(&self, ui: &mut Ui) -> egui::Frame {
        let mut frame = egui::Frame::none();
        frame.set_padding(egui::style::Spacing::new(5.0));

        // Add all expander tabs to the frame
        for expander_tab in self.expander_tabs.values() {
            frame.add(expander_tab.build(ui));
        }

        frame
    }
}