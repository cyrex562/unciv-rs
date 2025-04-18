use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, Ui, Align, Layout, Button, Frame};

use crate::ui::screens::basescreen::BaseScreen;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::images::ImageGetter;
use crate::ui::popups::ConfirmPopup;
use crate::ui::audio::{MusicMood, MusicTrackChooserFlags};
use crate::ui::components::input::{KeyCharAndCode, key_shortcuts};
use crate::ui::components::fonts::Fonts;
use crate::logic::civilization::Civilization;
use crate::logic::civilization::diplomacy::{DiplomacyManager, DiplomaticStatus, RelationshipLevel, DiplomacyFlags, DiplomaticModifiers};
use crate::logic::trade::{Trade, TradeOffer, TradeOfferType};
use crate::utils::translations::tr;
use crate::utils::constants::Constants;
use crate::game::UncivGame;
use crate::gui::GUI;

use super::diplomacy_screen::DiplomacyScreen;
use super::leader_intro_table::LeaderIntroTable;
use super::trade_table::TradeTable;

/// Handles the diplomacy interface for major civilizations
pub struct MajorCivDiplomacyTable {
    diplomacy_screen: Arc<DiplomacyScreen>,
    viewing_civ: Arc<Civilization>,
}

impl MajorCivDiplomacyTable {
    /// Creates a new MajorCivDiplomacyTable
    pub fn new(diplomacy_screen: Arc<DiplomacyScreen>) -> Self {
        Self {
            viewing_civ: diplomacy_screen.viewing_civ.clone(),
            diplomacy_screen,
        }
    }

    /// Gets the major civilization diplomacy table
    pub fn get_major_civ_diplomacy_table(&self, other_civ: &Civilization) -> egui::Frame {
        let other_civ_diplomacy_manager = other_civ.get_diplomacy_manager(&self.viewing_civ).unwrap();

        let mut diplomacy_table = egui::Frame::none();
        diplomacy_table.set_padding(egui::style::Spacing::new(10.0));

        // Determine greeting text and voice based on relationship
        let (hello_text, hello_voice) = if other_civ_diplomacy_manager.is_relationship_level_le(RelationshipLevel::Enemy) {
            (other_civ.nation.hate_hello.clone(), format!("{}.hateHello", other_civ.civ_name))
        } else {
            (other_civ.nation.neutral_hello.clone(), format!("{}.neutralHello", other_civ.civ_name))
        };

        // Add leader introduction
        let leader_intro_table = LeaderIntroTable::new(other_civ.clone(), hello_text);
        diplomacy_table.add(leader_intro_table.build(&mut Ui::default()));
        diplomacy_table.add_separator();

        // Check if diplomatic relationships can change
        let diplomatic_relationships_can_change = !self.viewing_civ.game_info.ruleset.mod_options.has_unique("DiplomaticRelationshipsCannotChange");

        let diplomacy_manager = self.viewing_civ.get_diplomacy_manager(other_civ).unwrap();

        // Add diplomacy buttons based on war status
        if !self.viewing_civ.is_at_war_with(other_civ) {
            diplomacy_table.add(self.get_trade_button(other_civ));

            if !diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
                diplomacy_table.add(self.get_declare_friendship_button(other_civ));
            }

            if !diplomacy_manager.has_flag(DiplomacyFlags::Denunciation)
                && !diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
                diplomacy_table.add(self.get_denounce_button(other_civ, &diplomacy_manager));
            }

            if diplomatic_relationships_can_change {
                diplomacy_table.add(self.diplomacy_screen.get_declare_war_button(&diplomacy_manager, other_civ));
            }
        } else if diplomatic_relationships_can_change {
            let negotiate_peace_button = self.get_negotiate_peace_major_civ_button(other_civ, &other_civ_diplomacy_manager);
            diplomacy_table.add(negotiate_peace_button);
        }

        // Add demands button
        let mut demands_button = egui::Button::new("Demands");
        demands_button.on_click(|| {
            self.diplomacy_screen.right_side_table.clear();
            self.diplomacy_screen.right_side_table.add(self.get_demands_table(&self.viewing_civ, other_civ));
        });
        diplomacy_table.add(demands_button);

        if self.diplomacy_screen.is_not_players_turn() {
            demands_button.disable();
        }

        // Add go to on map button if capital is explored
        if let Some(capital) = other_civ.get_capital() {
            if self.viewing_civ.has_explored(capital.get_center_tile()) {
                diplomacy_table.add(self.diplomacy_screen.get_go_to_on_map_button(other_civ));
            }
        }

        // Add relationship info for AI civilizations
        if !other_civ.is_human() {
            diplomacy_table.add(self.diplomacy_screen.get_relationship_table(&other_civ_diplomacy_manager));
            diplomacy_table.add(self.get_diplomacy_modifiers_table(&other_civ_diplomacy_manager));

            if let Some(promises_table) = self.get_promises_table(&diplomacy_manager, &other_civ_diplomacy_manager) {
                diplomacy_table.add(promises_table);
            }
        }

        // Play greeting voice
        UncivGame::current().music_controller.play_voice(&hello_voice);

        diplomacy_table
    }

    /// Gets the negotiate peace button for major civilizations
    fn get_negotiate_peace_major_civ_button(
        &self,
        other_civ: &Civilization,
        other_civ_diplomacy_manager: &DiplomacyManager
    ) -> egui::Button {
        let mut negotiate_peace_button = egui::Button::new("Negotiate Peace");
        negotiate_peace_button.on_click(|| {
            let trade_table = self.diplomacy_screen.set_trade(other_civ);
            let peace_treaty = TradeOffer::new(
                Constants::PEACE_TREATY,
                TradeOfferType::Treaty,
                speed: self.viewing_civ.game_info.speed
            );
            trade_table.trade_logic.current_trade.their_offers.push(peace_treaty.clone());
            trade_table.trade_logic.current_trade.our_offers.push(peace_treaty);
            trade_table.offer_columns_table.update();
            trade_table.enable_offer_button(true);
        });

        if self.diplomacy_screen.is_not_players_turn() {
            negotiate_peace_button.disable();
        }

        if other_civ_diplomacy_manager.has_flag(DiplomacyFlags::DeclaredWar) {
            negotiate_peace_button.disable(); // Can't trade for 10 turns after war was declared
            let turns_left = other_civ_diplomacy_manager.get_flag(DiplomacyFlags::DeclaredWar);
            negotiate_peace_button.set_text(format!("{}\n{} {}",
                negotiate_peace_button.text(),
                turns_left.tr(),
                Fonts::TURN
            ));
        }

        negotiate_peace_button
    }

    /// Gets the denounce button
    fn get_denounce_button(
        &self,
        other_civ: &Civilization,
        diplomacy_manager: &DiplomacyManager
    ) -> egui::Button {
        let mut denounce_button = egui::Button::new("Denounce ([30] turns)");
        denounce_button.on_click(|| {
            ConfirmPopup::new(
                &self.diplomacy_screen,
                &format!("Denounce [{}]?", other_civ.civ_name),
                "Denounce ([30] turns)",
                false,
                || {
                    diplomacy_manager.denounce();
                    self.diplomacy_screen.update_left_side_table(other_civ);
                    self.diplomacy_screen.set_right_side_flavor_text(
                        other_civ,
                        "We will remember this.",
                        "Very well."
                    );
                }
            ).open();
        });

        if self.diplomacy_screen.is_not_players_turn() {
            denounce_button.disable();
        }

        denounce_button
    }

    /// Gets the declare friendship button
    fn get_declare_friendship_button(&self, other_civ: &Civilization) -> egui::Button {
        let mut declare_friendship_button = egui::Button::new("Offer Declaration of Friendship ([30] turns)");
        declare_friendship_button.on_click(|| {
            other_civ.popup_alerts.push(
                PopupAlert::new(
                    AlertType::DeclarationOfFriendship,
                    self.viewing_civ.civ_name.clone()
                )
            );
            declare_friendship_button.disable();
        });

        if self.diplomacy_screen.is_not_players_turn() ||
           other_civ.popup_alerts.iter().any(|alert|
               alert.alert_type == AlertType::DeclarationOfFriendship &&
               alert.value == self.viewing_civ.civ_name
           ) {
            declare_friendship_button.disable();
        }

        declare_friendship_button
    }

    /// Gets the trade button
    fn get_trade_button(&self, other_civ: &Civilization) -> egui::Button {
        let mut trade_button = egui::Button::new("Trade");
        trade_button.on_click(|| {
            let trade_table = self.diplomacy_screen.set_trade(other_civ);
            trade_table.offer_columns_table.update();
        });

        if self.diplomacy_screen.is_not_players_turn() {
            trade_button.disable();
        }

        trade_button
    }

    /// Gets the promises table
    fn get_promises_table(
        &self,
        diplomacy_manager: &DiplomacyManager,
        other_civ_diplomacy_manager: &DiplomacyManager
    ) -> Option<egui::Frame> {
        let mut promises_table = egui::Frame::none();

        if other_civ_diplomacy_manager.has_flag(DiplomacyFlags::AgreedToNotSettleNearUs) {
            let text = format!(
                "We promised not to settle near them ([{}] turns remaining)",
                other_civ_diplomacy_manager.get_flag(DiplomacyFlags::AgreedToNotSettleNearUs)
            );
            promises_table.add(text.to_label().with_color(Color32::from_rgb(211, 211, 211)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::AgreedToNotSettleNearUs) {
            let text = format!(
                "They promised not to settle near us ([{}] turns remaining)",
                diplomacy_manager.get_flag(DiplomacyFlags::AgreedToNotSettleNearUs)
            );
            promises_table.add(text.to_label().with_color(Color32::from_rgb(211, 211, 211)));
        }

        if other_civ_diplomacy_manager.has_flag(DiplomacyFlags::AgreedToNotSpreadReligion) {
            let text = format!(
                "We promised not to spread religion to them ([{}] turns remaining)",
                other_civ_diplomacy_manager.get_flag(DiplomacyFlags::AgreedToNotSpreadReligion)
            );
            promises_table.add(text.to_label().with_color(Color32::from_rgb(211, 211, 211)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::AgreedToNotSpreadReligion) {
            let text = format!(
                "They promised not to spread religion to us ([{}] turns remaining)",
                diplomacy_manager.get_flag(DiplomacyFlags::AgreedToNotSpreadReligion)
            );
            promises_table.add(text.to_label().with_color(Color32::from_rgb(211, 211, 211)));
        }

        if promises_table.is_empty() {
            None
        } else {
            Some(promises_table)
        }
    }

    /// Gets the diplomacy modifiers table
    fn get_diplomacy_modifiers_table(&self, other_civ_diplomacy_manager: &DiplomacyManager) -> egui::Frame {
        let mut diplomacy_modifiers_table = egui::Frame::none();

        for (key, value) in &other_civ_diplomacy_manager.diplomatic_modifiers {
            // Angry about attacked CS and destroyed CS do not stack
            if key == &DiplomaticModifiers::AttackedProtectedMinor.name()
                && other_civ_diplomacy_manager.has_modifier(DiplomaticModifiers::DestroyedProtectedMinor) {
                continue;
            }

            let diplomatic_modifier = match DiplomaticModifiers::from_str(key) {
                Ok(modifier) => modifier,
                Err(_) => continue, // This modifier is from the future, you cannot understand it yet
            };

            let mut text = format!("{} ", diplomatic_modifier.text().tr());
            if *value > 0.0 {
                text.push('+');
            }
            text.push_str(&value.round().to_string());

            let color = if *value < 0.0 {
                Color32::RED
            } else {
                Color32::GREEN
            };

            diplomacy_modifiers_table.add(text.to_label().with_color(color));
        }

        diplomacy_modifiers_table
    }

    /// Gets the demands table
    fn get_demands_table(&self, viewing_civ: &Civilization, other_civ: &Civilization) -> egui::Frame {
        let mut demands_table = egui::Frame::none();
        demands_table.set_padding(egui::style::Spacing::new(10.0));

        let mut dont_settle_cities_button = egui::Button::new("Please don't settle new cities near us.");
        if other_civ.popup_alerts.iter().any(|alert|
            alert.alert_type == AlertType::DemandToStopSettlingCitiesNear &&
            alert.value == viewing_civ.civ_name
        ) {
            dont_settle_cities_button.disable();
        }
        dont_settle_cities_button.on_click(|| {
            other_civ.popup_alerts.push(
                PopupAlert::new(
                    AlertType::DemandToStopSettlingCitiesNear,
                    viewing_civ.civ_name.clone()
                )
            );
            dont_settle_cities_button.disable();
        });
        demands_table.add(dont_settle_cities_button);

        let mut dont_spread_religion_button = egui::Button::new("Please don't spread your religion to us.");
        if other_civ.popup_alerts.iter().any(|alert|
            alert.alert_type == AlertType::DemandToStopSpreadingReligion &&
            alert.value == viewing_civ.civ_name
        ) {
            dont_spread_religion_button.disable();
        }
        dont_spread_religion_button.on_click(|| {
            other_civ.popup_alerts.push(
                PopupAlert::new(
                    AlertType::DemandToStopSpreadingReligion,
                    viewing_civ.civ_name.clone()
                )
            );
            dont_spread_religion_button.disable();
        });
        demands_table.add(dont_spread_religion_button);

        let mut close_button = egui::Button::new(Constants::CLOSE);
        close_button.on_click(|| {
            self.diplomacy_screen.update_right_side(other_civ);
        });
        demands_table.add(close_button);

        demands_table
    }
}