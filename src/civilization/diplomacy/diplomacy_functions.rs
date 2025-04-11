use crate::civilization::{
    AlertType, Civilization, NotificationCategory, NotificationIcon, PopupAlert,
};
use crate::game::UncivGame;
use crate::logic::map::mapunit::movement::UnitMovement;
use crate::logic::map::tile::Tile;
use crate::models::ruleset::unique::UniqueType;
use crate::models::stats::{Stat, Stats};
use std::cmp::max;

/// Functions related to diplomacy between civilizations
pub struct DiplomacyFunctions<'a> {
    pub civ_info: &'a mut Civilization,
}

impl<'a> DiplomacyFunctions<'a> {
    /// Creates a new DiplomacyFunctions instance
    pub fn new(civ_info: &'a mut Civilization) -> Self {
        Self { civ_info }
    }

    /// A sorted sequence of all other civs we know (excluding barbarians and spectators)
    pub fn get_known_civs_sorted(&self, include_city_states: bool, include_defeated: bool) -> Vec<&Civilization> {
        self.civ_info.game_info.get_civs_sorted(include_city_states, include_defeated, |civ| {
            civ != self.civ_info && self.civ_info.knows(civ)
        })
    }

    /// Makes two civilizations meet each other
    pub fn make_civilizations_meet(&mut self, other_civ: &mut Civilization, war_on_contact: bool) {
        self.meet_civ(other_civ, war_on_contact);
        other_civ.diplomacy_functions.meet_civ(self.civ_info, war_on_contact);
    }

    /// Internal function to make this civilization meet another
    fn meet_civ(&mut self, other_civ: &mut Civilization, war_on_contact: bool) {
        self.civ_info.diplomacy.insert(
            other_civ.civ_name.clone(),
            DiplomacyManager::new(self.civ_info, other_civ.civ_name.clone())
                .apply(|dm| dm.diplomatic_status = DiplomaticStatus::Peace)
        );

        if !other_civ.is_spectator() {
            other_civ.popup_alerts.add(PopupAlert::new(AlertType::FirstContact, self.civ_info.civ_name.clone()));
        }

        if self.civ_info.is_current_player() {
            UncivGame::current().settings.add_completed_tutorial_task("Meet another civilization");
        }

        if self.civ_info.is_city_state && other_civ.is_major_civ() {
            if war_on_contact || other_civ.is_minor_civ_aggressor() {
                return; // No gift if they are bad people, or we are just about to be at war
            }

            let city_state_location = if self.civ_info.cities.is_empty() {
                None
            } else {
                self.civ_info.get_capital().map(|cap| cap.location)
            };

            let mut gift_amount = Stats::new().with_gold(15.0);
            let faith_amount = Stats::new().with_faith(4.0);

            // Later, religious city-states will also gift gold, making this the better implementation
            // For now, it might be overkill though.
            let mut meet_string = format!(
                "[{}] has given us [{}] as a token of goodwill for meeting us",
                self.civ_info.civ_name,
                gift_amount.to_string_for_notifications()
            );

            let religion_meet_string = format!(
                "[{}] has also given us [{}]",
                self.civ_info.civ_name,
                faith_amount.to_string_for_notifications()
            );

            if self.civ_info.diplomacy.values().filter(|dm| dm.other_civ().is_major_civ()).count() == 1 {
                gift_amount.times_in_place(2.0);
                meet_string = format!(
                    "[{}] has given us [{}] as we are the first major civ to meet them",
                    self.civ_info.civ_name,
                    gift_amount.to_string_for_notifications()
                );
            }

            if let Some(location) = city_state_location {
                other_civ.add_notification(
                    meet_string,
                    location,
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Gold,
                );
            } else {
                other_civ.add_notification(
                    meet_string,
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Gold,
                );
            }

            if other_civ.is_city_state && other_civ.city_state_functions.can_provide_stat(Stat::Faith) {
                other_civ.add_notification(
                    religion_meet_string,
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Faith,
                );

                for (key, value) in faith_amount.iter() {
                    other_civ.add_stat(*key, value.to_int());
                }
            }

            for (key, value) in gift_amount.iter() {
                other_civ.add_stat(*key, value.to_int());
            }

            if !self.civ_info.cities.is_empty() {
                if let Some(capital) = self.civ_info.get_capital() {
                    if let Some(center_tile) = capital.get_center_tile() {
                        center_tile.set_explored(other_civ, true);
                    }
                }
            }

            self.civ_info.quest_manager.just_met(other_civ); // Include them in war with major pseudo-quest
        }
    }

    /// Checks if this civilization is at war with another civilization
    pub fn is_at_war_with(&self, other_civ: &Civilization) -> bool {
        if other_civ == self.civ_info {
            return false;
        }

        if other_civ.is_barbarian || self.civ_info.is_barbarian {
            return true;
        }

        match self.civ_info.diplomacy.get(&other_civ.civ_name) {
            Some(diplomacy_manager) => diplomacy_manager.diplomatic_status == DiplomaticStatus::War,
            None => false, // not encountered yet
        }
    }

    /// Checks if this civilization can sign a declaration of friendship with another civilization
    pub fn can_sign_declaration_of_friendship_with(&self, other_civ: &Civilization) -> bool {
        other_civ.is_major_civ()
            && !other_civ.is_at_war_with(self.civ_info)
            && !self.civ_info.get_diplomacy_manager(other_civ).unwrap().has_flag(DiplomacyFlags::Denunciation)
            && !self.civ_info.get_diplomacy_manager(other_civ).unwrap().has_flag(DiplomacyFlags::DeclarationOfFriendship)
    }

    /// Checks if this civilization can sign a research agreement
    pub fn can_sign_research_agreement(&self) -> bool {
        if !self.civ_info.is_major_civ() {
            return false;
        }

        if !self.civ_info.has_unique(UniqueType::EnablesResearchAgreements) {
            return false;
        }

        if self.civ_info.tech.all_techs_are_researched() {
            return false;
        }

        true
    }

    /// Checks if this civilization can sign a research agreement with another civilization without cost
    pub fn can_sign_research_agreement_no_cost_with(&self, other_civ: &Civilization) -> bool {
        let diplomacy_manager = self.civ_info.get_diplomacy_manager(other_civ).unwrap();

        self.can_sign_research_agreement()
            && other_civ.diplomacy_functions.can_sign_research_agreement()
            && diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship)
            && !diplomacy_manager.has_flag(DiplomacyFlags::ResearchAgreement)
            && !diplomacy_manager.other_civ_diplomacy().has_flag(DiplomacyFlags::ResearchAgreement)
    }

    /// Checks if this civilization can sign research agreements with another civilization
    pub fn can_sign_research_agreements_with(&self, other_civ: &Civilization) -> bool {
        let cost = self.get_research_agreement_cost(other_civ);

        self.can_sign_research_agreement_no_cost_with(other_civ)
            && self.civ_info.gold >= cost && other_civ.gold >= cost
    }

    /// Gets the cost of a research agreement with another civilization
    pub fn get_research_agreement_cost(&self, other_civ: &Civilization) -> i32 {
        // https://forums.civfanatics.com/resources/research-agreements-bnw.25568/
        (max(
            self.civ_info.get_era().research_agreement_cost,
            other_civ.get_era().research_agreement_cost
        ) * self.civ_info.game_info.speed.gold_cost_modifier) as i32
    }

    /// Checks if this civilization can sign a defensive pact
    pub fn can_sign_defensive_pact(&self) -> bool {
        if !self.civ_info.is_major_civ() {
            return false;
        }

        if !self.civ_info.has_unique(UniqueType::EnablesDefensivePacts) {
            return false;
        }

        true
    }

    /// Checks if this civilization can sign a defensive pact with another civilization
    pub fn can_sign_defensive_pact_with(&self, other_civ: &Civilization) -> bool {
        let diplomacy_manager = self.civ_info.get_diplomacy_manager(other_civ).unwrap();

        self.can_sign_defensive_pact()
            && other_civ.diplomacy_functions.can_sign_defensive_pact()
            && (diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship)
                || diplomacy_manager.other_civ_diplomacy().has_flag(DiplomacyFlags::DeclarationOfFriendship))
            && !diplomacy_manager.has_flag(DiplomacyFlags::DefensivePact)
            && !diplomacy_manager.other_civ_diplomacy().has_flag(DiplomacyFlags::DefensivePact)
            && diplomacy_manager.diplomatic_status != DiplomaticStatus::DefensivePact
    }

    /// Checks whether units of this civilization can pass through the tiles owned by another civilization,
    /// considering only civ-wide filters.
    ///
    /// Use `Tile::can_civ_pass_through` to check whether units of a civilization can pass through
    /// a specific tile, considering only civ-wide filters.
    ///
    /// Use `UnitMovement::can_pass_through` to check whether a specific unit can pass through
    /// a specific tile.
    pub fn can_pass_through_tiles(&self, other_civ: &Civilization) -> bool {
        if other_civ == self.civ_info {
            return true;
        }

        if other_civ.is_barbarian {
            return true;
        }

        if self.civ_info.is_barbarian && self.civ_info.game_info.turns >= self.civ_info.game_info.get_difficulty().turn_barbarians_can_enter_player_tiles {
            return true;
        }

        if let Some(diplomacy_manager) = self.civ_info.diplomacy.get(&other_civ.civ_name) {
            if diplomacy_manager.has_open_borders || diplomacy_manager.diplomatic_status == DiplomaticStatus::War {
                return true;
            }
        }

        // Players can always pass through city-state tiles
        if !self.civ_info.is_ai_or_auto_playing() && other_civ.is_city_state {
            return true;
        }

        false
    }
}