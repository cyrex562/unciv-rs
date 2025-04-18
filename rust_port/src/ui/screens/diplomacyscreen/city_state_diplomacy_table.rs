use std::collections::HashMap;
use std::sync::Arc;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::{Color32, RichText, ScrollArea, TextEdit, Ui, Align, Layout};

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::diplomacyscreen::DiplomacyScreen;
use crate::ui::screens::diplomacyscreen::leader_intro_table::LeaderIntroTable;
use crate::ui::components::extensions::*;
use crate::ui::components::widgets::ColorMarkupLabel;
use crate::ui::images::ImageGetter;
use crate::ui::popups::ConfirmPopup;
use crate::logic::civilization::Civilization;
use crate::logic::civilization::AlertType;
use crate::logic::civilization::PopupAlert;
use crate::logic::civilization::diplomacy::DiplomacyFlags;
use crate::logic::civilization::diplomacy::DiplomacyManager;
use crate::logic::civilization::diplomacy::DiplomaticStatus;
use crate::logic::civilization::diplomacy::RelationshipLevel;
use crate::logic::civilization::managers::AssignedQuest;
use crate::logic::trade::TradeLogic;
use crate::logic::trade::TradeOffer;
use crate::logic::trade::TradeOfferType;
use crate::models::ruleset::Quest;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::unique::UniqueType;
use crate::utils::constants::Constants;
use crate::utils::translations::tr;

/// Table for displaying and managing city-state diplomacy
pub struct CityStateDiplomacyTable {
    diplomacy_screen: Arc<DiplomacyScreen>,
    viewing_civ: Arc<Civilization>,
}

impl CityStateDiplomacyTable {
    /// Creates a new CityStateDiplomacyTable
    pub fn new(diplomacy_screen: Arc<DiplomacyScreen>) -> Self {
        let viewing_civ = diplomacy_screen.viewing_civ.clone();
        Self {
            diplomacy_screen,
            viewing_civ,
        }
    }

    /// Gets the city-state diplomacy table for a civilization
    pub fn get_city_state_diplomacy_table(&self, other_civ: &Civilization) -> egui::Frame {
        let other_civ_diplomacy_manager = other_civ.get_diplomacy_manager(&self.viewing_civ).unwrap();

        let mut diplomacy_table = self.get_city_state_diplomacy_table_header(other_civ);

        diplomacy_table.add_separator();

        let give_gift_button = egui::Button::new("Give a Gift")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_gold_gift_table(other_civ)
                }));
            });
        diplomacy_table.add(give_gift_button);
        if self.diplomacy_screen.is_not_players_turn() || self.viewing_civ.is_at_war_with(other_civ) {
            give_gift_button.disable();
        }

        if let Some(improve_tile_button) = self.get_improve_tiles_button(other_civ, &other_civ_diplomacy_manager) {
            diplomacy_table.add(improve_tile_button);
        }

        if other_civ_diplomacy_manager.diplomatic_status != DiplomaticStatus::Protector {
            diplomacy_table.add(self.get_pledge_to_protect_button(other_civ));
        } else {
            diplomacy_table.add(self.get_revoke_protection_button(other_civ));
        }

        let demand_tribute_button = egui::Button::new("Demand Tribute")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_demand_tribute_table(other_civ)
                }));
            });
        diplomacy_table.add(demand_tribute_button);
        if self.diplomacy_screen.is_not_players_turn() || self.viewing_civ.is_at_war_with(other_civ) {
            demand_tribute_button.disable();
        }

        let diplomacy_manager = self.viewing_civ.get_diplomacy_manager(other_civ).unwrap();
        if !self.viewing_civ.game_info.ruleset.mod_options.has_unique(UniqueType::DiplomaticRelationshipsCannotChange) {
            if self.viewing_civ.is_at_war_with(other_civ) {
                diplomacy_table.add(self.get_negotiate_peace_city_state_button(other_civ, &diplomacy_manager));
            } else {
                diplomacy_table.add(self.diplomacy_screen.get_declare_war_button(&diplomacy_manager, other_civ));
            }
        }

        if let Some(capital) = other_civ.get_capital() {
            if self.viewing_civ.has_explored(capital.get_center_tile()) {
                diplomacy_table.add(self.diplomacy_screen.get_go_to_on_map_button(other_civ));
            }
        }

        if let Some(diplomatic_marriage_button) = self.get_diplomatic_marriage_button(other_civ) {
            diplomacy_table.add(diplomatic_marriage_button);
        }

        for assigned_quest in other_civ.quest_manager.get_assigned_quests_for(&self.viewing_civ.civ_name) {
            diplomacy_table.add_separator();
            diplomacy_table.add(self.get_quest_table(&assigned_quest));
        }

        for target in other_civ.get_known_civs().filter(|c| other_civ.quest_manager.war_with_major_active(c) && &self.viewing_civ != c) {
            diplomacy_table.add_separator();
            diplomacy_table.add(self.get_war_with_major_table(target, other_civ));
        }

        diplomacy_table
    }

    /// Gets the city-state diplomacy table header
    fn get_city_state_diplomacy_table_header(&self, other_civ: &Civilization) -> egui::Frame {
        let other_civ_diplomacy_manager = other_civ.get_diplomacy_manager(&self.viewing_civ).unwrap();

        let mut diplomacy_table = egui::Frame::none();
        diplomacy_table.set_padding(egui::style::Spacing::new(2.5));

        diplomacy_table.add(LeaderIntroTable::new(other_civ)).pad_bottom(15.0);

        diplomacy_table.add(format!("Type: {}", other_civ.city_state_type.name).to_label());
        diplomacy_table.add(format!("Personality: {}", other_civ.city_state_personality).to_label());

        if other_civ.detailed_civ_resources.iter().any(|r| r.resource.resource_type != ResourceType::Bonus) {
            let mut resources_table = egui::Frame::none();
            resources_table.add("Resources: ".to_label()).pad_right(10.0);
            let city_state_resources = other_civ.city_state_functions.get_city_state_resources_for_ally();
            for supply_list in city_state_resources {
                if supply_list.resource.resource_type == ResourceType::Bonus {
                    continue;
                }
                let name = supply_list.resource.name.clone();
                let mut wrapper = egui::Frame::none();
                let image = ImageGetter::get_resource_portrait(&name, 30.0);
                wrapper.add(image).pad_right(5.0);
                wrapper.add(supply_list.amount.to_string().to_label());
                resources_table.add(wrapper).pad_right(20.0);
                wrapper.add_tooltip(&name, 18.0);
                wrapper.on_click(|| {
                    self.diplomacy_screen.open_civilopedia(supply_list.resource.make_link());
                });
            }
            diplomacy_table.add(resources_table);
        }
        diplomacy_table.row().pad_top(15.0);

        other_civ.city_state_functions.update_ally_civ_for_city_state();
        let mut ally = other_civ.get_ally_civ();
        if let Some(ally_name) = ally {
            let ally_influence = other_civ.get_diplomacy_manager(ally_name).unwrap().get_influence() as i32;
            if !self.viewing_civ.knows(ally_name) && ally_name != self.viewing_civ.civ_name {
                ally = Some("Unknown civilization".to_string());
            }
            diplomacy_table.add(format!("Ally: [{}] with [{}] Influence", ally.unwrap(), ally_influence).to_label());
        }

        let protectors = other_civ.city_state_functions.get_protector_civs();
        if !protectors.is_empty() {
            let mut new_protectors = Vec::new();
            for protector in protectors {
                if !self.viewing_civ.knows(&protector.civ_name) && protector.civ_name != self.viewing_civ.civ_name {
                    new_protectors.push(tr("Unknown civilization"));
                } else {
                    new_protectors.push(tr(&protector.civ_name));
                }
            }
            let protector_string = format!("Protected by: {}", new_protectors.join(", "));
            diplomacy_table.add(protector_string.to_label());
        }

        let at_war = other_civ.is_at_war_with(&self.viewing_civ);

        let next_level_string = if at_war {
            "".to_string()
        } else if other_civ_diplomacy_manager.get_influence() < 30.0 {
            "Reach 30 for friendship.".to_string()
        } else if ally == Some(self.viewing_civ.civ_name.clone()) {
            "".to_string()
        } else {
            "Reach highest influence above 60 for alliance.".to_string()
        };
        diplomacy_table.add(self.diplomacy_screen.get_relationship_table(&other_civ_diplomacy_manager));
        if !next_level_string.is_empty() {
            diplomacy_table.add(next_level_string.to_label());
        }
        diplomacy_table.row().pad_top(15.0);

        let relation_level = other_civ_diplomacy_manager.relationship_ignore_afraid();
        if relation_level >= RelationshipLevel::Friend {
            // RelationshipChange = Ally -> Friend or Friend -> Favorable
            let turns_to_relationship_change = other_civ_diplomacy_manager.get_turns_to_relationship_change();
            if turns_to_relationship_change != 0 {
                diplomacy_table.add(format!("Relationship changes in another [{}] turns", turns_to_relationship_change).to_label());
            }
        }

        fn get_bonus_text(header: &str, level: RelationshipLevel, viewing_civ: &Civilization, other_civ: &Civilization) -> String {
            let bonuses = viewing_civ.city_state_functions
                .get_city_state_bonuses(&other_civ.city_state_type, level)
                .filter(|b| !b.is_hidden_to_users());
            if bonuses.is_empty() {
                return "".to_string();
            }
            let mut result = vec![header.to_string()];
            result.extend(bonuses.iter().map(|b| b.get_display_text()));
            result.iter().map(|s| tr(s)).collect::<Vec<_>>().join("\n")
        }

        fn add_bonus_label(header: &str, bonus_level: RelationshipLevel, relation_level: RelationshipLevel,
                          viewing_civ: &Civilization, other_civ: &Civilization, diplomacy_table: &mut egui::Frame) {
            let bonus_label_color = if relation_level == bonus_level { Color32::GREEN } else { Color32::GRAY };
            let bonus_text = get_bonus_text(header, bonus_level, viewing_civ, other_civ);
            let bonus_label = ColorMarkupLabel::new(&bonus_text, bonus_label_color)
                .with_alignment(Align::Center);
            diplomacy_table.add(bonus_label);
        }

        add_bonus_label("When Friends:", RelationshipLevel::Friend, relation_level,
                       &self.viewing_civ, other_civ, &mut diplomacy_table);
        add_bonus_label("When Allies:", RelationshipLevel::Ally, relation_level,
                       &self.viewing_civ, other_civ, &mut diplomacy_table);

        if let Some(unit_name) = &other_civ.city_state_unique_unit {
            let tech_names = self.viewing_civ.game_info.ruleset.units.get(unit_name).unwrap().required_techs();
            let tech_and_tech = tech_names.join(" and ");
            let is_or_are = if tech_names.len() == 1 { "is" } else { "are" };
            diplomacy_table.add(format!("[{}] is able to provide [{}] once [{}] [{}] researched.",
                                      other_civ.civ_name, unit_name, tech_and_tech, is_or_are)
                .to_label().with_font_size(Constants::DEFAULT_FONT_SIZE));
        }

        diplomacy_table
    }

    /// Gets the revoke protection button
    fn get_revoke_protection_button(&self, other_civ: &Civilization) -> egui::Button {
        let mut revoke_protection_button = egui::Button::new("Revoke Protection")
            .on_click(|| {
                ConfirmPopup::new(
                    &self.diplomacy_screen,
                    &format!("Revoke protection for [{}]?", other_civ.civ_name),
                    "Revoke Protection",
                    false,
                    || {
                        other_civ.city_state_functions.remove_protector_civ(&self.viewing_civ);
                        self.diplomacy_screen.update_left_side_table(other_civ);
                        self.diplomacy_screen.update_right_side(other_civ);
                    }
                ).open();
            });

        if self.diplomacy_screen.is_not_players_turn() || !other_civ.city_state_functions.other_civ_can_withdraw_protection(&self.viewing_civ) {
            revoke_protection_button.disable();
        }

        revoke_protection_button
    }

    /// Gets the pledge to protect button
    fn get_pledge_to_protect_button(&self, other_civ: &Civilization) -> egui::Button {
        let mut protection_button = egui::Button::new("Pledge to protect")
            .on_click(|| {
                ConfirmPopup::new(
                    &self.diplomacy_screen,
                    &format!("Declare Protection of [{}]?", other_civ.civ_name),
                    "Pledge to protect",
                    true,
                    || {
                        other_civ.city_state_functions.add_protector_civ(&self.viewing_civ);
                        self.diplomacy_screen.update_left_side_table(other_civ);
                        self.diplomacy_screen.update_right_side(other_civ);
                    }
                ).open();
            });

        if self.diplomacy_screen.is_not_players_turn() || !other_civ.city_state_functions.other_civ_can_pledge_protection(&self.viewing_civ) {
            protection_button.disable();
        }

        protection_button
    }

    /// Gets the negotiate peace city-state button
    fn get_negotiate_peace_city_state_button(
        &self,
        other_civ: &Civilization,
        other_civ_diplomacy_manager: &DiplomacyManager
    ) -> egui::Button {
        let mut peace_button = egui::Button::new("Negotiate Peace")
            .on_click(|| {
                ConfirmPopup::new(
                    &self.diplomacy_screen,
                    &format!("Peace with [{}]?", other_civ.civ_name),
                    "Negotiate Peace",
                    true,
                    || {
                        let mut trade_logic = TradeLogic::new(&self.viewing_civ, other_civ);
                        trade_logic.current_trade.our_offers.push(
                            TradeOffer::new(Constants::PEACE_TREATY, TradeOfferType::Treaty, self.viewing_civ.game_info.speed)
                        );
                        trade_logic.current_trade.their_offers.push(
                            TradeOffer::new(Constants::PEACE_TREATY, TradeOfferType::Treaty, self.viewing_civ.game_info.speed)
                        );
                        trade_logic.accept_trade();
                        self.diplomacy_screen.update_left_side_table(other_civ);
                        self.diplomacy_screen.update_right_side(other_civ);
                    }
                ).open();
            });

        let city_states_ally = other_civ.get_ally_civ();
        let at_war_with_its_ally = self.viewing_civ.get_known_civs()
            .iter()
            .any(|c| c.civ_name == city_states_ally && c.is_at_war_with(&self.viewing_civ));

        if self.diplomacy_screen.is_not_players_turn() || at_war_with_its_ally {
            peace_button.disable();
        }

        if other_civ_diplomacy_manager.has_flag(DiplomacyFlags::DeclaredWar) {
            peace_button.disable(); // Can't trade for 10 turns after war was declared
            let turns_left = other_civ_diplomacy_manager.get_flag(DiplomacyFlags::DeclaredWar);
            peace_button.set_text(format!("{}\n{} turns", peace_button.text(), tr(&turns_left.to_string())));
        }

        peace_button
    }

    /// Gets the improve tiles button
    fn get_improve_tiles_button(
        &self,
        other_civ: &Civilization,
        other_civ_diplomacy_manager: &DiplomacyManager
    ) -> Option<egui::Button> {
        if other_civ.cities.is_empty() {
            return None;
        }

        let improvable_resource_tiles = self.get_improvable_resource_tiles(other_civ);
        let improvements = other_civ.game_info.ruleset.tile_improvements.iter()
            .filter(|(_, imp)| imp.turns_to_build != -1);

        let mut needs_improvements = false;

        for improvable_tile in &improvable_resource_tiles {
            for (_, tile_improvement) in &improvements {
                if improvable_tile.tile_resource.is_improved_by(&tile_improvement.name)
                    && improvable_tile.improvement_functions.can_build_improvement(tile_improvement, other_civ) {
                    needs_improvements = true;
                    break;
                }
            }
            if needs_improvements {
                break;
            }
        }

        if !needs_improvements {
            return None;
        }

        let mut improve_tile_button = egui::Button::new("Gift Improvement")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_improvement_gift_table(other_civ)
                }));
            });

        if self.diplomacy_screen.is_not_players_turn() || other_civ_diplomacy_manager.get_influence() < 60.0 {
            improve_tile_button.disable();
        }

        Some(improve_tile_button)
    }

    /// Gets the diplomatic marriage button
    fn get_diplomatic_marriage_button(&self, other_civ: &Civilization) -> Option<egui::Button> {
        if !self.viewing_civ.has_unique(UniqueType::CityStateCanBeBoughtForGold)
            && !self.viewing_civ.has_unique(UniqueType::CityStateCanBeBoughtForGoldOld) {
            return None;
        }

        let mut diplomatic_marriage_button = egui::Button::new(
            format!("Diplomatic Marriage ([{}] Gold)", other_civ.city_state_functions.get_diplomatic_marriage_cost())
        )
        .on_click(|| {
            let new_cities = other_civ.cities.clone();
            other_civ.city_state_functions.diplomatic_marriage(&self.viewing_civ);
            // The other civ will no longer exist
            for city in new_cities {
                self.viewing_civ.popup_alerts.push(PopupAlert::new(AlertType::DiplomaticMarriage, city.id));
                // Player gets to choose between annex and puppet
            }
        });

        if self.diplomacy_screen.is_not_players_turn() || !other_civ.city_state_functions.can_be_married_by(&self.viewing_civ) {
            diplomatic_marriage_button.disable();
        }

        Some(diplomatic_marriage_button)
    }

    /// Gets the gold gift table
    fn get_gold_gift_table(&self, other_civ: &Civilization) -> egui::Frame {
        let mut diplomacy_table = self.get_city_state_diplomacy_table_header(other_civ);
        diplomacy_table.add_separator();

        for gift_amount in [250, 500, 1000] {
            let influence_amount = other_civ.city_state_functions.influence_gained_by_gift(&self.viewing_civ, gift_amount);
            let mut gift_button = egui::Button::new(
                format!("Gift [{}] gold (+[{}] influence)", gift_amount, influence_amount)
            )
            .on_click(|| {
                other_civ.city_state_functions.receive_gold_gift(&self.viewing_civ, gift_amount);
                self.diplomacy_screen.update_left_side_table(other_civ);
                self.diplomacy_screen.update_right_side(other_civ);
            });

            diplomacy_table.add(gift_button);
            if self.viewing_civ.gold < gift_amount || self.diplomacy_screen.is_not_players_turn() {
                gift_button.disable();
            }
        }

        let back_button = egui::Button::new("Back")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_city_state_diplomacy_table(other_civ)
                }));
            });

        diplomacy_table.add(back_button);
        diplomacy_table
    }

    /// Gets improvable resource tiles
    fn get_improvable_resource_tiles(&self, other_civ: &Civilization) -> Vec<&Tile> {
        other_civ.get_capital().unwrap().get_tiles().iter()
            .filter(|tile| {
                tile.has_viewable_resource(other_civ)
                    && tile.tile_resource.resource_type != ResourceType::Bonus
                    && (tile.improvement.is_none() || !tile.tile_resource.is_improved_by(&tile.improvement.unwrap()))
            })
            .collect()
    }

    /// Gets the improvement gift table
    fn get_improvement_gift_table(&self, other_civ: &Civilization) -> egui::Frame {
        let mut improvement_gift_table = self.get_city_state_diplomacy_table_header(other_civ);
        improvement_gift_table.add_separator();

        let improvable_resource_tiles = self.get_improvable_resource_tiles(other_civ);
        let tile_improvements = &other_civ.game_info.ruleset.tile_improvements;

        for improvable_tile in improvable_resource_tiles {
            for (_, tile_improvement) in tile_improvements {
                if improvable_tile.tile_resource.is_improved_by(&tile_improvement.name)
                    && improvable_tile.improvement_functions.can_build_improvement(tile_improvement, other_civ) {
                    let mut improve_tile_button = egui::Button::new(
                        format!("Build [{}] on [{}] (200 Gold)", tile_improvement.name, improvable_tile.tile_resource.name)
                    )
                    .on_click(|| {
                        self.viewing_civ.add_gold(-200);
                        improvable_tile.stop_working_on_improvement();
                        improvable_tile.set_improvement(&tile_improvement.name);
                        other_civ.cache.update_civ_resources();
                        self.diplomacy_screen.right_side_table.clear();
                        self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                            self.get_city_state_diplomacy_table(other_civ)
                        }));
                    });

                    if self.viewing_civ.gold < 200 {
                        improve_tile_button.disable();
                    }

                    improvement_gift_table.add(improve_tile_button);
                }
            }
        }

        let back_button = egui::Button::new("Back")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_city_state_diplomacy_table(other_civ)
                }));
            });

        improvement_gift_table.add(back_button);
        improvement_gift_table
    }

    /// Gets the demand tribute table
    fn get_demand_tribute_table(&self, other_civ: &Civilization) -> egui::Frame {
        let mut diplomacy_table = self.get_city_state_diplomacy_table_header(other_civ);
        diplomacy_table.add_separator();
        diplomacy_table.add("Tribute Willingness".to_label());

        let mut modifier_table = egui::Frame::none();
        let tribute_modifiers = other_civ.city_state_functions.get_tribute_modifiers(&self.viewing_civ, true);
        for (key, value) in tribute_modifiers {
            let color = if value >= 0.0 { Color32::GREEN } else { Color32::RED };
            modifier_table.add(key.to_label().with_color(color));
            modifier_table.add(tr(&value.to_string()).to_label().with_color(color));
        }

        modifier_table.add("Sum:".to_label());
        modifier_table.add(tribute_modifiers.values().sum::<f32>().to_string().to_label());
        diplomacy_table.add(modifier_table);
        diplomacy_table.add("At least 0 to take gold, at least 30 and size 4 city for worker".to_label());
        diplomacy_table.add_separator();

        let gold_amount = other_civ.city_state_functions.gold_gained_by_tribute();
        let mut demand_gold_button = egui::Button::new(
            format!("Take [{}] gold (-15 Influence)", gold_amount)
        )
        .on_click(|| {
            other_civ.city_state_functions.tribute_gold(&self.viewing_civ);
            self.diplomacy_screen.right_side_table.clear();
            self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                self.get_city_state_diplomacy_table(other_civ)
            }));
        });

        diplomacy_table.add(demand_gold_button);
        if other_civ.city_state_functions.get_tribute_willingness(&self.viewing_civ, false) < 0.0 {
            demand_gold_button.disable();
        }

        let mut demand_worker_button = egui::Button::new("Take worker (-50 Influence)")
            .on_click(|| {
                other_civ.city_state_functions.tribute_worker(&self.viewing_civ);
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_city_state_diplomacy_table(other_civ)
                }));
            });

        diplomacy_table.add(demand_worker_button);
        if other_civ.city_state_functions.get_tribute_willingness(&self.viewing_civ, true) < 0.0 {
            demand_worker_button.disable();
        }

        let back_button = egui::Button::new("Back")
            .on_click(|| {
                self.diplomacy_screen.right_side_table.clear();
                self.diplomacy_screen.right_side_table.add(ScrollArea::vertical().show(ui, |ui| {
                    self.get_city_state_diplomacy_table(other_civ)
                }));
            });

        diplomacy_table.add(back_button);
        diplomacy_table
    }

    /// Gets the quest table
    fn get_quest_table(&self, assigned_quest: &AssignedQuest) -> egui::Frame {
        let mut quest_table = egui::Frame::none();
        quest_table.set_padding(egui::style::Spacing::new(10.0));

        let quest = self.viewing_civ.game_info.ruleset.quests.get(&assigned_quest.quest_name).unwrap();
        let remaining_turns = assigned_quest.get_remaining_turns();
        let title = if quest.influence > 0.0 {
            format!("[{}] (+[{}] influence)", quest.name, quest.influence as i32)
        } else {
            quest.name.clone()
        };
        let description = assigned_quest.get_description();

        quest_table.add(title.to_label().with_font_size(Constants::HEADING_FONT_SIZE));
        quest_table.add(description.to_label().with_wrap(true).with_alignment(Align::Center))
            .with_width(self.diplomacy_screen.stage.width() / 2.0);

        if quest.duration > 0 {
            quest_table.add(format!("[{}] turns remaining", remaining_turns).to_label());
        }

        if quest.is_global() {
            let leader_string = self.viewing_civ.game_info.get_civilization(&assigned_quest.assigner)
                .quest_manager.get_score_string_for_global_quest(assigned_quest);
            if !leader_string.is_empty() {
                quest_table.add(leader_string.to_label());
            }
        }

        quest_table.on_click(|| {
            assigned_quest.on_click_action();
        });

        quest_table
    }

    /// Gets the war with major table
    fn get_war_with_major_table(&self, target: &Civilization, other_civ: &Civilization) -> egui::Frame {
        let mut war_table = egui::Frame::none();
        war_table.set_padding(egui::style::Spacing::new(10.0));

        let title = format!("War against [{}]", target.civ_name);
        let description = format!("We need you to help us defend against [{}]. Killing [{}] of their military units would slow their offensive.",
                                target.civ_name, other_civ.quest_manager.units_to_kill(target));
        let progress = if self.viewing_civ.knows(target) {
            format!("Currently you have killed [{}] of their military units.",
                   other_civ.quest_manager.units_killed_so_far(target, &self.viewing_civ))
        } else {
            "You need to find them first!".to_string()
        };

        war_table.add(title.to_label().with_font_size(Constants::HEADING_FONT_SIZE));
        war_table.add(description.to_label().with_wrap(true).with_alignment(Align::Center))
            .with_width(self.diplomacy_screen.stage.width() / 2.0);
        war_table.add(progress.to_label().with_wrap(true).with_alignment(Align::Center))
            .with_width(self.diplomacy_screen.stage.width() / 2.0);

        war_table
    }
}