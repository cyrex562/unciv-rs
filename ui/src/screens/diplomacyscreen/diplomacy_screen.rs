use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Align, Layout, Split};

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::basescreen::{BaseScreen, RecreateOnResize};
use crate::ui::screens::diplomacyscreen::city_state_diplomacy_table::CityStateDiplomacyTable;
use crate::ui::screens::diplomacyscreen::major_civ_diplomacy_table::MajorCivDiplomacyTable;
use crate::ui::screens::diplomacyscreen::leader_intro_table::LeaderIntroTable;
use crate::ui::screens::diplomacyscreen::trade_table::TradeTable;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::components::tilegroups::InfluenceTable;
use crate::ui::images::ImageGetter;
use crate::ui::popups::ConfirmPopup;
use crate::ui::audio::{MusicMood, MusicTrackChooserFlags};
use crate::ui::components::input::{KeyCharAndCode, key_shortcuts};
use crate::ui::components::fonts::Fonts;
use crate::logic::civilization::Civilization;
use crate::logic::civilization::diplomacy::{DiplomacyManager, DiplomaticStatus, RelationshipLevel};
use crate::logic::trade::Trade;
use crate::utils::translations::tr;
use crate::utils::constants::Constants;
use crate::game::UncivGame;
use crate::gui::GUI;

/// Creates the diplomacy screen for viewing_civ.
///
/// When select_civ is given and select_trade is not, that Civilization is selected as if clicked on the left side.
/// When select_civ is given and select_trade is not but show_trade is set, the TradeTable for that Civilization is shown.
/// When select_civ and select_trade are supplied, that Trade for that Civilization is selected, used for the counter-offer option from `TradePopup`.
/// Note calling this with select_civ a City State and select_trade supplied is **not allowed**.
pub struct DiplomacyScreen {
    viewing_civ: Arc<Civilization>,
    select_civ: Option<Arc<Civilization>>,
    select_trade: Option<Trade>,
    show_trade: bool,

    // UI components
    left_side_table: egui::Frame,
    left_side_scroll: ScrollArea,
    highlighted_civ_button: Option<egui::Frame>,
    highlight_background: egui::Color32,
    right_side_table: egui::Frame,
    close_button: egui::Button,

    // Constants
    nation_icon_size: f32,
    nation_icon_pad: f32,
    close_button_size: f32,
    close_button_pad: f32,
}

impl DiplomacyScreen {
    /// Creates a new DiplomacyScreen
    pub fn new(
        viewing_civ: Arc<Civilization>,
        select_civ: Option<Arc<Civilization>>,
        select_trade: Option<Trade>,
        show_trade: bool,
    ) -> Self {
        let nation_icon_size = 100.0;
        let nation_icon_pad = 10.0;
        let close_button_size = 50.0;
        let close_button_pad = 10.0;

        let highlight_color = Color32::from_rgba_premultiplied(255, 255, 255, 85); // 0.333f alpha

        let mut left_side_table = egui::Frame::none();
        left_side_table.set_padding(egui::style::Spacing::new(2.5));

        let mut right_side_table = egui::Frame::none();
        right_side_table.set_padding(egui::style::Spacing::new(2.5));

        let mut close_button = egui::Button::new("X")
            .on_click(|| {
                UncivGame::current().pop_screen();
            });

        Self {
            viewing_civ,
            select_civ,
            select_trade,
            show_trade,

            left_side_table,
            left_side_scroll: ScrollArea::vertical(),
            highlighted_civ_button: None,
            highlight_background: highlight_color,
            right_side_table,
            close_button,

            nation_icon_size,
            nation_icon_pad,
            close_button_size,
            close_button_pad,
        }
    }

    /// Initializes the diplomacy screen
    pub fn init(&mut self, ui: &mut Ui) {
        // Create split pane
        let mut split = Split::vertical();

        // In cramped conditions, start the left side with enough width for nation icon and padding
        let split_amount = 0.2f32.max(self.left_side_scroll.min_width() / ui.available_rect_before_wrap().width());
        split.set_split_amount(split_amount);

        self.update_left_side_table(ui);

        // Position close button
        self.position_close_button(ui);

        // Add close button to UI
        ui.add(self.close_button.clone());

        // Handle selected civilization
        if let Some(select_civ) = &self.select_civ {
            if self.show_trade {
                let trade_table = self.set_trade(select_civ);
                if let Some(select_trade) = &self.select_trade {
                    trade_table.trade_logic.current_trade.set(select_trade.clone());
                }
                trade_table.offer_columns_table.update();
            } else {
                self.update_right_side(select_civ);
            }
        }
    }

    /// Gets the civilopedia ruleset
    pub fn get_civilopedia_ruleset(&self) -> &Ruleset {
        &self.viewing_civ.game_info.ruleset
    }

    /// Positions the close button
    fn position_close_button(&mut self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        self.close_button.set_position(
            rect.right() - self.close_button_pad,
            rect.top() + self.close_button_pad,
            Align::TOP_RIGHT
        );
    }

    /// Updates the left side table
    pub fn update_left_side_table(&mut self, ui: &mut Ui) {
        self.left_side_table.clear();
        self.left_side_table.add_space(self.close_button_pad);

        let mut select_civ_y = 0.0;

        for civ in self.viewing_civ.diplomacy_functions.get_known_civs_sorted() {
            if let Some(select_civ) = &self.select_civ {
                if civ.civ_name == select_civ.civ_name {
                    select_civ_y = self.left_side_table.min_height();
                }
            }

            let civ_indicator = ImageGetter::get_nation_portrait(&civ.nation, self.nation_icon_size);

            let relation_level = civ.get_diplomacy_manager(&self.viewing_civ).unwrap().relationship_level();
            let relationship_icon = if civ.is_city_state && relation_level == RelationshipLevel::Ally {
                let mut star = ImageGetter::get_image("OtherIcons/Star")
                    .surround_with_circle(size: 30.0, color: relation_level.color);
                star.color = Color32::GOLD;
                star
            } else {
                let color = if self.viewing_civ.is_at_war_with(&civ) {
                    Color32::RED
                } else {
                    relation_level.color
                };
                ImageGetter::get_circle(color, 30.0)
            };

            civ_indicator.add_actor(relationship_icon);

            if civ.is_city_state {
                let inner_color = civ.game_info.ruleset.nations.get(&civ.civ_name).unwrap().get_inner_color();
                let mut type_icon = ImageGetter::get_image(&format!("CityStateIcons/{}", civ.city_state_type.name))
                    .surround_with_circle(size: 35.0, color: inner_color);
                type_icon.color = ImageGetter::CHARCOAL;
                civ_indicator.add_actor(type_icon);
                type_icon.y = (civ_indicator.height - type_icon.height).floor();
                type_icon.x = (civ_indicator.width - type_icon.width).floor();
            }

            if civ.is_city_state && civ.quest_manager.have_quests_for(&self.viewing_civ) {
                let quest_icon = ImageGetter::get_image("OtherIcons/Quest")
                    .surround_with_circle(size: 30.0, color: Color32::GOLDENROD);
                civ_indicator.add_actor(quest_icon);
                quest_icon.x = (civ_indicator.width - quest_icon.width).floor();
            }

            let civ_name_label = civ.civ_name.to_label(hide_icons: true);

            // The wrapper serves only to highlight the selected civ better
            let mut civ_button = egui::Frame::none();
            civ_button.set_padding(egui::style::Spacing::new(self.nation_icon_pad));
            civ_button.add(civ_indicator);
            civ_button.add(civ_name_label);

            civ_button.on_click(|| {
                self.update_right_side(&civ);
                self.highlight_civ(&civ_button);
            });

            if let Some(select_civ) = &self.select_civ {
                if civ.civ_name == select_civ.civ_name {
                    self.highlight_civ(&civ_button);
                }
            }

            self.left_side_table.add(civ_button).pad_bottom(20.0 - self.nation_icon_pad).grow_x();
        }

        if select_civ_y != 0.0 {
            self.left_side_scroll.layout();
            self.left_side_scroll.scroll_y = select_civ_y + (self.nation_icon_size + 2.0 * self.nation_icon_pad - ui.available_rect_before_wrap().height()) / 2.0;
            self.left_side_scroll.update_visual_scroll();
        }
    }

    /// Highlights a civilization button
    fn highlight_civ(&mut self, civ_button: &egui::Frame) {
        if let Some(highlighted) = &mut self.highlighted_civ_button {
            highlighted.background = None;
        }
        civ_button.background = Some(self.highlight_background);
        self.highlighted_civ_button = Some(civ_button.clone());
    }

    /// Updates the right side of the screen
    pub fn update_right_side(&mut self, other_civ: &Civilization) {
        self.right_side_table.clear();

        UncivGame::current().music_controller.choose_track(
            &other_civ.civ_name,
            MusicMood::peace_or_war(self.viewing_civ.is_at_war_with(other_civ)),
            MusicTrackChooserFlags::SET_SELECT_NATION
        );

        let content = if other_civ.is_city_state {
            CityStateDiplomacyTable::new(self).get_city_state_diplomacy_table(other_civ)
        } else {
            MajorCivDiplomacyTable::new(self).get_major_civ_diplomacy_table(other_civ)
        };

        self.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
            ui.add(content);
        }));
    }

    /// Sets up the trade table
    pub fn set_trade(&mut self, other_civ: &Civilization) -> TradeTable {
        self.right_side_table.clear();
        let trade_table = TradeTable::new(&self.viewing_civ, other_civ, self);
        self.right_side_table.add(trade_table.clone());
        trade_table
    }

    /// Gets the relationship table
    pub fn get_relationship_table(&self, other_civ_diplomacy_manager: &DiplomacyManager) -> egui::Frame {
        let mut relationship_table = egui::Frame::none();

        let opinion_of_us = if other_civ_diplomacy_manager.civ_info.is_city_state {
            other_civ_diplomacy_manager.get_influence() as i32
        } else {
            other_civ_diplomacy_manager.opinion_of_other_civ() as i32
        };

        relationship_table.add("Our relationship: ".to_label());
        let relationship_level = other_civ_diplomacy_manager.relationship_level();
        let relationship_text = format!("{} ({})", relationship_level.name.tr(), opinion_of_us);

        let relationship_color = match relationship_level {
            RelationshipLevel::Neutral => Color32::WHITE,
            RelationshipLevel::Favorable | RelationshipLevel::Friend | RelationshipLevel::Ally => Color32::GREEN,
            RelationshipLevel::Afraid => Color32::YELLOW,
            _ => Color32::RED,
        };

        relationship_table.add(relationship_text.to_label().with_color(relationship_color));

        if other_civ_diplomacy_manager.civ_info.is_city_state {
            relationship_table.add(
                InfluenceTable::new(
                    other_civ_diplomacy_manager.get_influence(),
                    relationship_level,
                    200.0,
                    10.0
                )
            ).colspan(2).pad(5.0);
        }

        relationship_table
    }

    /// Gets the declare war button
    pub fn get_declare_war_button(
        &self,
        diplomacy_manager: &DiplomacyManager,
        other_civ: &Civilization
    ) -> egui::Button {
        let mut declare_war_button = egui::Button::new("Declare war")
            .with_style(egui::style::ButtonStyle::negative());

        let turns_to_peace_treaty = diplomacy_manager.turns_to_peace_treaty();
        if turns_to_peace_treaty > 0 {
            declare_war_button.disable();
            declare_war_button.set_text(format!("{} ({} turns)", declare_war_button.text(), turns_to_peace_treaty.tr()));
        }

        declare_war_button.on_click(|| {
            ConfirmPopup::new(
                self,
                &self.get_declare_war_button_text(other_civ),
                "Declare war",
                false,
                || {
                    diplomacy_manager.declare_war();
                    self.set_right_side_flavor_text(other_civ, &other_civ.nation.attacked, "Very well.");
                    self.update_left_side_table(ui);

                    let music = UncivGame::current().music_controller;
                    music.choose_track(&other_civ.civ_name, MusicMood::WAR, MusicTrackChooserFlags::SET_SPECIFIC);
                    music.play_voice(&format!("{}.attacked", other_civ.civ_name));
                }
            ).open();
        });

        if self.is_not_players_turn() {
            declare_war_button.disable();
        }

        declare_war_button
    }

    /// Gets the declare war button text
    fn get_declare_war_button_text(&self, other_civ: &Civilization) -> String {
        let mut message_lines = Vec::new();
        message_lines.push(format!("Declare war on [{}]?", other_civ.civ_name));

        // Tell the player who all will join the other side from defensive pacts
        let other_civ_defensive_pact_list: Vec<&Civilization> = other_civ.diplomacy.values()
            .filter(|other_civ_diplo_manager| {
                other_civ_diplo_manager.other_civ().civ_name != self.viewing_civ.civ_name
                    && other_civ_diplo_manager.diplomatic_status == DiplomaticStatus::DefensivePact
                    && !other_civ_diplo_manager.other_civ().is_at_war_with(&self.viewing_civ)
            })
            .map(|it| it.other_civ())
            .collect();

        // Defensive pact chains are not allowed now
        for civ in &other_civ_defensive_pact_list {
            if self.viewing_civ.knows(civ) {
                message_lines.push(format!("[{}] will also join them in the war", civ.civ_name));
            } else {
                message_lines.push("An unknown civilization will also join them in the war".to_string());
            }
        }

        // Tell the player that their defensive pacts will be canceled.
        for civ_diplo_manager in self.viewing_civ.diplomacy.values() {
            if civ_diplo_manager.other_civ().civ_name != other_civ.civ_name
                && civ_diplo_manager.diplomatic_status == DiplomaticStatus::DefensivePact
                && !other_civ_defensive_pact_list.iter().any(|c| c.civ_name == civ_diplo_manager.other_civ().civ_name) {
                message_lines.push(format!("This will cancel your defensive pact with [{}]", civ_diplo_manager.other_civ_name));
            }
        }

        message_lines.iter().map(|line| format!("{{{}}}", line)).collect::<Vec<_>>().join("\n")
    }

    /// Sets the right side flavor text
    pub fn set_right_side_flavor_text(
        &mut self,
        other_civ: &Civilization,
        flavor_text: &str,
        response: &str
    ) {
        let mut diplomacy_table = egui::Frame::none();
        diplomacy_table.set_padding(egui::style::Spacing::new(10.0));

        diplomacy_table.add(LeaderIntroTable::new(other_civ));
        diplomacy_table.add_separator();
        diplomacy_table.add(flavor_text.to_label());

        let mut response_button = egui::Button::new(response);
        response_button.on_click(|| {
            self.update_right_side(other_civ);
        });
        response_button.key_shortcuts.add(KeyCharAndCode::SPACE);
        diplomacy_table.add(response_button);

        self.right_side_table.clear();
        self.right_side_table.add(diplomacy_table);
    }

    /// Gets the go to on map button
    pub fn get_go_to_on_map_button(&self, civilization: &Civilization) -> egui::Button {
        let mut go_to_on_map_button = egui::Button::new("Go to on map");
        go_to_on_map_button.on_click(|| {
            let world_screen = UncivGame::current().reset_to_world_screen();
            world_screen.map_holder.set_center_position(civilization.get_capital().unwrap().location, false);
        });
        go_to_on_map_button
    }

    /// Gets the trade columns width
    pub fn get_trade_columns_width(&self) -> f32 {
        (self.stage.width() * 0.8 - 3.0) / 2.0  // 3 for SplitPane handle
    }

    /// Checks if it's not the player's turn
    pub fn is_not_players_turn(&self) -> bool {
        !GUI::is_allowed_change_state()
    }
}

impl RecreateOnResize for DiplomacyScreen {
    fn recreate(&self) -> Box<dyn BaseScreen> {
        Box::new(DiplomacyScreen::new(
            self.viewing_civ.clone(),
            self.select_civ.clone(),
            self.select_trade.clone(),
            self.show_trade
        ))
    }
}

impl BaseScreen for DiplomacyScreen {
    fn resize(&mut self, width: i32, height: i32) {
        // Position close button
        self.position_close_button(ui);
    }
}