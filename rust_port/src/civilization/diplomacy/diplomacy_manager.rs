use crate::civilization::{Civilization, NotificationCategory, NotificationIcon};
use crate::models::ruleset::unique::{StateForConditionals, UniqueTriggerActivation, UniqueType};
use crate::models::stats::ResourceSupplyList;
use crate::models::trade::{Trade, TradeEvaluation, TradeOffer, TradeOfferType};
use crate::utils::color::Color;
use std::collections::HashMap;
use std::cmp::max;
use std::f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipLevel {
    Unforgivable,
    Enemy,
    Afraid,
    Competitor,
    Neutral,
    Favorable,
    Friend,
    Ally,
}

impl RelationshipLevel {
    pub fn color(&self) -> Color {
        match self {
            Self::Unforgivable => Color::FIREBRICK,
            Self::Enemy => Color::YELLOW,
            Self::Afraid => Color::new(0x5300ffff),     // HSV(260,100,100)
            Self::Competitor => Color::new(0x1f998fff), // HSV(175,80,60)
            Self::Neutral => Color::new(0x1bb371ff),    // HSV(154,85,70)
            Self::Favorable => Color::new(0x14cc3cff),  // HSV(133,90,80)
            Self::Friend => Color::new(0x2ce60bff),     // HSV(111,95,90)
            Self::Ally => Color::CHARTREUSE,            // HSV(90,100,100)
        }
    }

    pub fn add(&self, delta: i32) -> Self {
        let values = [
            Self::Unforgivable,
            Self::Enemy,
            Self::Afraid,
            Self::Competitor,
            Self::Neutral,
            Self::Favorable,
            Self::Friend,
            Self::Ally,
        ];
        let new_ordinal = (self as i32 + delta).clamp(0, values.len() as i32 - 1) as usize;
        values[new_ordinal]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomacyFlags {
    DeclinedLuxExchange,
    DeclinedPeace,
    DeclinedResearchAgreement,
    DeclinedOpenBorders,
    DeclaredWar,
    DeclarationOfFriendship,
    DeclinedDeclarationOfFriendship,
    DefensivePact,
    DeclinedDefensivePact,
    DeclinedJoinWarOffer,
    ResearchAgreement,
    BorderConflict,
    TilesStolen,
    SettledCitiesNearUs,
    AgreedToNotSettleNearUs,
    IgnoreThemSettlingNearUs,
    SpreadReligionInOurCities,
    AgreedToNotSpreadReligion,
    IgnoreThemSpreadingReligion,
    ProvideMilitaryUnit,
    MarriageCooldown,
    NotifiedAfraid,
    RecentlyPledgedProtection,
    RecentlyWithdrewProtection,
    AngerFreeIntrusion,
    RememberDestroyedProtectedMinor,
    RememberAttackedProtectedMinor,
    RememberBulliedProtectedMinor,
    RememberSidedWithProtectedMinor,
    Denunciation,
    WaryOf,
    Bullied,
    RecentlyAttacked,
    ResourceTradesCutShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomaticModifiers {
    // Negative
    DeclaredWarOnUs,
    WarMongerer,
    CapturedOurCities,
    DeclaredFriendshipWithOurEnemies,
    BetrayedDeclarationOfFriendship,
    SignedDefensivePactWithOurEnemies,
    BetrayedDefensivePact,
    Denunciation,
    DenouncedOurAllies,
    RefusedToNotSettleCitiesNearUs,
    RefusedToNotSpreadReligionToUs,
    BetrayedPromiseToNotSettleCitiesNearUs,
    BetrayedPromiseToNotSpreadReligionToUs,
    UnacceptableDemands,
    UsedNuclearWeapons,
    StealingTerritory,
    DestroyedProtectedMinor,
    AttackedProtectedMinor,
    AttackedAlliedMinor,
    BulliedProtectedMinor,
    SidedWithProtectedMinor,
    SpiedOnUs,
    StoleOurAlly,

    // Positive
    YearsOfPeace,
    SharedEnemy,
    LiberatedCity,
    DeclarationOfFriendship,
    DeclaredFriendshipWithOurAllies,
    DefensivePact,
    SignedDefensivePactWithOurAllies,
    DenouncedOurEnemies,
    OpenBorders,
    FulfilledPromiseToNotSettleCitiesNearUs,
    FulfilledPromiseToNotSpreadReligion,
    GaveUsUnits,
    GaveUsGifts,
    ReturnedCapturedUnits,
    BelieveSameReligion,
}

impl DiplomaticModifiers {
    pub fn text(&self) -> &'static str {
        match self {
            // Negative
            Self::DeclaredWarOnUs => "You declared war on us!",
            Self::WarMongerer => "Your warmongering ways are unacceptable to us.",
            Self::CapturedOurCities => "You have captured our cities!",
            Self::DeclaredFriendshipWithOurEnemies => "You have declared friendship with our enemies!",
            Self::BetrayedDeclarationOfFriendship => "Your so-called 'friendship' is worth nothing.",
            Self::SignedDefensivePactWithOurEnemies => "You have declared a defensive pact with our enemies!",
            Self::BetrayedDefensivePact => "Your so-called 'defensive pact' is worth nothing.",
            Self::Denunciation => "You have publicly denounced us!",
            Self::DenouncedOurAllies => "You have denounced our allies",
            Self::RefusedToNotSettleCitiesNearUs => "You refused to stop settling cities near us",
            Self::RefusedToNotSpreadReligionToUs => "You refused to stop spreading religion to us",
            Self::BetrayedPromiseToNotSettleCitiesNearUs => "You betrayed your promise to not settle cities near us",
            Self::BetrayedPromiseToNotSpreadReligionToUs => "You betrayed your promise to not spread your religion to us",
            Self::UnacceptableDemands => "Your arrogant demands are in bad taste",
            Self::UsedNuclearWeapons => "Your use of nuclear weapons is disgusting!",
            Self::StealingTerritory => "You have stolen our lands!",
            Self::DestroyedProtectedMinor => "You destroyed City-States that were under our protection!",
            Self::AttackedProtectedMinor => "You attacked City-States that were under our protection!",
            Self::AttackedAlliedMinor => "You attacked our allied City-States!",
            Self::BulliedProtectedMinor => "You demanded tribute from City-States that were under our protection!",
            Self::SidedWithProtectedMinor => "You sided with a City-State over us",
            Self::SpiedOnUs => "You spied on us!",
            Self::StoleOurAlly => "You took the alliance we had with a City-State",

            // Positive
            Self::YearsOfPeace => "Years of peace have strengthened our relations.",
            Self::SharedEnemy => "Our mutual military struggle brings us closer together.",
            Self::LiberatedCity => "We applaud your liberation of conquered cities!",
            Self::DeclarationOfFriendship => "We have signed a public declaration of friendship",
            Self::DeclaredFriendshipWithOurAllies => "You have declared friendship with our allies",
            Self::DefensivePact => "We have signed a promise to protect each other.",
            Self::SignedDefensivePactWithOurAllies => "You have declared a defensive pact with our allies",
            Self::DenouncedOurEnemies => "You have denounced our enemies",
            Self::OpenBorders => "Our open borders have brought us closer together.",
            Self::FulfilledPromiseToNotSettleCitiesNearUs => "You fulfilled your promise to stop settling cities near us!",
            Self::FulfilledPromiseToNotSpreadReligion => "You fulfilled your promise to stop spreading religion to us!",
            Self::GaveUsUnits => "You gave us units!",
            Self::GaveUsGifts => "We appreciate your gifts",
            Self::ReturnedCapturedUnits => "You returned captured units to us",
            Self::BelieveSameReligion => "We believe in the same religion",
        }
    }
}

pub struct DiplomacyManager {
    pub civ_info: Civilization,
    pub other_civ_name: String,
    pub trades: Vec<Trade>,
    pub diplomatic_status: DiplomaticStatus,
    pub flags_countdown: HashMap<String, i32>,
    pub diplomatic_modifiers: HashMap<String, f32>,
    pub influence: f32,
    pub total_of_science_during_ra: i32,
    pub has_open_borders: bool,
}

impl DiplomacyManager {
    pub const MINIMUM_INFLUENCE: f32 = -60.0;

    pub fn new(civilization: &Civilization, other_civ_name: String) -> Self {
        let mut dm = Self {
            civ_info: civilization.clone(),
            other_civ_name,
            trades: Vec::new(),
            diplomatic_status: DiplomaticStatus::War,
            flags_countdown: HashMap::new(),
            diplomatic_modifiers: HashMap::new(),
            influence: 0.0,
            total_of_science_during_ra: 0,
            has_open_borders: false,
        };
        dm.update_has_open_borders();
        dm
    }

    pub fn clone(&self) -> Self {
        Self {
            civ_info: self.civ_info.clone(),
            other_civ_name: self.other_civ_name.clone(),
            trades: self.trades.iter().map(|t| t.clone()).collect(),
            diplomatic_status: self.diplomatic_status,
            flags_countdown: self.flags_countdown.clone(),
            diplomatic_modifiers: self.diplomatic_modifiers.clone(),
            influence: self.influence,
            total_of_science_during_ra: self.total_of_science_during_ra,
            has_open_borders: self.has_open_borders,
        }
    }

    pub fn other_civ(&self) -> &Civilization {
        self.civ_info.game_info.get_civilization(&self.other_civ_name)
    }

    pub fn other_civ_diplomacy(&self) -> &DiplomacyManager {
        self.other_civ().get_diplomacy_manager(&self.civ_info).unwrap()
    }

    pub fn turns_to_peace_treaty(&self) -> i32 {
        for trade in &self.trades {
            for offer in &trade.our_offers {
                if offer.name == "Peace Treaty" && offer.duration > 0 {
                    return offer.duration;
                }
            }
        }
        0
    }

    pub fn opinion_of_other_civ(&self) -> f32 {
        let mut modifier_sum: f32 = self.diplomatic_modifiers.values().sum();

        // Angry about attacked CS and destroyed CS do not stack
        if self.has_modifier(DiplomaticModifiers::DestroyedProtectedMinor)
            && self.has_modifier(DiplomaticModifiers::AttackedProtectedMinor) {
            modifier_sum -= self.get_modifier(DiplomaticModifiers::AttackedProtectedMinor);
        }

        modifier_sum
    }

    pub fn relationship_level(&self) -> RelationshipLevel {
        let level = self.relationship_ignore_afraid();

        if level != RelationshipLevel::Neutral || !self.civ_info.is_city_state {
            return level;
        }

        if self.civ_info.city_state_functions.get_tribute_willingness(self.other_civ()) > 0 {
            RelationshipLevel::Afraid
        } else {
            RelationshipLevel::Neutral
        }
    }

    pub fn relationship_ignore_afraid(&self) -> RelationshipLevel {
        if self.civ_info.is_human() && self.other_civ().is_human() {
            return RelationshipLevel::Neutral; // People make their own choices.
        }

        if self.civ_info.is_human() {
            return self.other_civ_diplomacy().relationship_level();
        }

        if self.civ_info.is_city_state {
            return match self.get_influence() {
                i if i <= -30.0 => RelationshipLevel::Unforgivable,  // get_influence tests is_at_war_with
                i if i < 0.0 => RelationshipLevel::Enemy,
                i if i >= 60.0 && self.civ_info.get_ally_civ() == Some(self.other_civ_name.clone()) => RelationshipLevel::Ally,
                i if i >= 30.0 => RelationshipLevel::Friend,
                _ => RelationshipLevel::Neutral,
            };
        }

        // not entirely sure what to do between AI civs, because they probably have different views of each other,
        // maybe we need to average their views of each other? That makes sense to me.
        let opinion = self.opinion_of_other_civ();
        match opinion {
            o if o <= -80.0 => RelationshipLevel::Unforgivable,
            o if o <= -40.0 || self.civ_info.is_at_war_with(self.other_civ()) => RelationshipLevel::Enemy,
            o if o <= -15.0 => RelationshipLevel::Competitor,
            o if o >= 80.0 => RelationshipLevel::Ally,
            o if o >= 40.0 => RelationshipLevel::Friend,
            o if o >= 15.0 => RelationshipLevel::Favorable,
            _ => RelationshipLevel::Neutral,
        }
    }

    pub fn believes_same_religion(&self) -> bool {
        let civ_majority_religion = match self.civ_info.religion_manager.get_majority_religion() {
            Some(religion) => religion,
            None => return false,
        };

        self.other_civ().religion_manager.is_majority_religion_for_civ(&civ_majority_religion)
    }

    pub fn get_turns_to_relationship_change(&self) -> i32 {
        if self.other_civ().is_city_state {
            return self.other_civ_diplomacy().get_turns_to_relationship_change();
        }

        if self.civ_info.is_city_state && !self.other_civ().is_city_state {
            let drop_per_turn = self.get_city_state_influence_degrade();
            match drop_per_turn {
                0.0 => 0,
                _ => {
                    match self.relationship_level() {
                        RelationshipLevel::Ally => {
                            ((self.get_influence() - 60.0) / drop_per_turn).ceil() as i32 + 1
                        }
                        RelationshipLevel::Friend => {
                            ((self.get_influence() - 30.0) / drop_per_turn).ceil() as i32 + 1
                        }
                        _ => 0,
                    }
                }
            }
        } else {
            0
        }
    }

    pub fn add_influence(&mut self, amount: f32) {
        self.set_influence(self.influence + amount);
    }

    pub fn reduce_influence(&mut self, amount: f32) {
        if self.influence <= 0.0 {
            return;
        }
        self.influence = max(0.0, self.influence - amount);
    }

    pub fn set_influence(&mut self, amount: f32) {
        self.influence = max(amount, Self::MINIMUM_INFLUENCE);
        self.civ_info.city_state_functions.update_ally_civ_for_city_state();
    }

    pub fn get_influence(&self) -> f32 {
        if self.civ_info.is_at_war_with(self.other_civ()) {
            Self::MINIMUM_INFLUENCE
        } else {
            self.influence
        }
    }

    pub fn get_city_state_influence_resting_point(&self) -> f32 {
        let mut resting_point = 0.0;

        for unique in self.other_civ().get_matching_uniques(UniqueType::CityStateRestingPoint) {
            resting_point += unique.params[0].parse::<f32>().unwrap();
        }

        if !self.civ_info.cities.is_empty() && self.civ_info.get_capital().is_some() {
            for unique in self.other_civ().get_matching_uniques(UniqueType::RestingPointOfCityStatesFollowingReligionChange) {
                if let Some(religion) = &self.other_civ().religion_manager.religion {
                    if let Some(capital) = self.civ_info.get_capital() {
                        if religion.name == capital.religion.get_majority_religion_name() {
                            resting_point += unique.params[0].parse::<f32>().unwrap();
                        }
                    }
                }
            }
        }

        if self.diplomatic_status == DiplomaticStatus::Protector {
            resting_point += 10.0;
        }

        if self.has_flag(DiplomacyFlags::WaryOf) {
            resting_point -= 20.0;
        }

        resting_point
    }

    pub fn get_city_state_influence_degrade(&self) -> f32 {
        if self.get_influence() <= this.get_city_state_influence_resting_point() {
            return 0.0;
        }

        let decrement = match self.civ_info.city_state_personality {
            CityStatePersonality::Hostile => 1.5,
            _ if self.other_civ().is_minor_civ_aggressor() => 2.0,
            _ => 1.0,
        };

        let mut modifier_percent = 0.0;
        for unique in self.other_civ().get_matching_uniques(UniqueType::CityStateInfluenceDegradation) {
            modifier_percent += unique.params[0].parse::<f32>().unwrap();
        }

        let religion = if self.civ_info.cities.is_empty() || self.civ_info.get_capital().is_none() {
            None
        } else {
            self.civ_info.get_capital().map(|cap| cap.religion.get_majority_religion_name())
        };

        if let Some(religion) = religion {
            if let Some(other_religion) = &self.other_civ().religion_manager.religion {
                if religion == other_religion.name {
                    modifier_percent -= 25.0;  // 25% slower degrade when sharing a religion
                }
            }
        }

        for civ in self.civ_info.game_info.civilizations.iter()
            .filter(|c| c.is_major_civ() && c != self.other_civ()) {
            for unique in civ.get_matching_uniques(UniqueType::OtherCivsCityStateRelationsDegradeFaster) {
                modifier_percent += unique.params[0].parse::<f32>().unwrap();
            }
        }

        max(0.0, decrement) * max(-100.0, modifier_percent).to_percent()
    }

    pub fn can_declare_war(&self) -> bool {
        !self.civ_info.is_defeated()
            && !self.other_civ().is_defeated()
            && self.turns_to_peace_treaty() == 0
            && self.diplomatic_status != DiplomaticStatus::War
    }

    pub fn declare_war(&mut self, declare_war_reason: DeclareWarReason) {
        DeclareWar::declare_war(self, declare_war_reason);
    }

    pub fn can_attack(&self) -> bool {
        self.turns_to_peace_treaty() == 0
    }

    pub fn gold_per_turn(&self) -> i32 {
        let mut gold_per_turn_for_us = 0;
        for trade in &self.trades {
            for offer in trade.our_offers.iter()
                .filter(|o| o.trade_type == TradeOfferType::GoldPerTurn) {
                gold_per_turn_for_us -= offer.amount;
            }
            for offer in trade.their_offers.iter()
                .filter(|o| o.trade_type == TradeOfferType::GoldPerTurn) {
                gold_per_turn_for_us += offer.amount;
            }
        }
        gold_per_turn_for_us
    }

    pub fn resources_from_trade(&self) -> ResourceSupplyList {
        let mut new_resource_supply_list = ResourceSupplyList::new();
        let resources_map = &self.civ_info.game_info.ruleset.tile_resources;

        let is_resource_filter = |offer: &TradeOffer| {
            (offer.trade_type == TradeOfferType::StrategicResource
                || offer.trade_type == TradeOfferType::LuxuryResource)
                && resources_map.contains_key(&offer.name)
                && !resources_map[&offer.name].is_stockpiled
        };

        for trade in &self.trades {
            for offer in trade.our_offers.iter().filter(|o| is_resource_filter(o)) {
                new_resource_supply_list.add(
                    resources_map[&offer.name].clone(),
                    "Trade",
                    -offer.amount
                );
            }
            for offer in trade.their_offers.iter().filter(|o| is_resource_filter(o)) {
                new_resource_supply_list.add(
                    resources_map[&offer.name].clone(),
                    "Trade",
                    offer.amount
                );
            }
        }

        new_resource_supply_list
    }

    pub fn get_common_known_civs(&self) -> Vec<&Civilization> {
        self.civ_info.get_known_civs()
            .iter()
            .filter(|c| self.other_civ().knows(c))
            .collect()
    }

    pub fn get_common_known_civs_with_spectators(&self) -> Vec<&Civilization> {
        self.civ_info.get_known_civs_with_spectators()
            .iter()
            .filter(|c| self.other_civ().knows_with_spectators(c))
            .collect()
    }

    pub fn is_considered_friendly_territory(&self) -> bool {
        if self.civ_info.is_city_state
            && (self.is_relationship_level_ge(RelationshipLevel::Friend)
                || self.other_civ().has_unique(UniqueType::CityStateTerritoryAlwaysFriendly)) {
            return true;
        }

        self.other_civ_diplomacy().has_open_borders
    }

    pub fn update_has_open_borders(&mut self) {
        let new_has_open_borders = self.civ_info.get_ally_civ() == Some(self.other_civ_name.clone())
            || self.trades.iter()
                .flat_map(|t| t.their_offers.iter())
                .any(|o| o.name == "Open Borders" && o.duration > 0);

        let borders_were_closed = self.has_open_borders && !new_has_open_borders;
        self.has_open_borders = new_has_open_borders;

        if borders_were_closed {
            // borders were closed, get out!
            for unit in self.civ_info.units.get_civ_units()
                .iter()
                .filter(|u| u.current_tile.get_owner().map_or(false, |o| o.civ_name == self.other_civ_name))
                .cloned()
                .collect::<Vec<_>>() {
                unit.movement.teleport_to_closest_moveable_tile();
            }
        }
    }

    pub fn make_peace_one_side(&mut self) {
        self.diplomatic_status = DiplomaticStatus::Peace;
        let other_civ = self.other_civ();

        // Get out of others' territory
        for unit in self.civ_info.units.get_civ_units()
            .iter()
            .filter(|u| u.get_tile().get_owner().map_or(false, |o| o == other_civ))
            .cloned()
            .collect::<Vec<_>>() {
            unit.movement.teleport_to_closest_moveable_tile();
        }

        for third_civ in self.civ_info.get_known_civs() {
            // Our ally city states make peace with us
            if third_civ.get_ally_civ() == Some(self.civ_info.civ_name.clone())
                && third_civ.is_at_war_with(other_civ) {
                let mut third_civ_diplo = third_civ.get_diplomacy_manager(other_civ).unwrap();
                third_civ_diplo.make_peace();

                // Make the peace treaty so that the civ can't declare war immediately
                let mut trade_logic = TradeLogic::new(third_civ, other_civ);
                trade_logic.current_trade.our_offers.push(
                    TradeOffer::new("Peace Treaty", TradeOfferType::Treaty, self.civ_info.game_info.speed)
                );
                trade_logic.current_trade.their_offers.push(
                    TradeOffer::new("Peace Treaty", TradeOfferType::Treaty, self.civ_info.game_info.speed)
                );
                third_civ_diplo.trades.push(trade_logic.current_trade.clone());
                third_civ_diplo.other_civ_diplomacy().trades.push(trade_logic.current_trade.reverse());
            }

            // Other City-States that are not our ally don't like the fact that we made peace with their enemy
            if third_civ.get_ally_civ() != Some(self.civ_info.civ_name.clone())
                && third_civ.is_at_war_with(other_civ) {
                third_civ.get_diplomacy_manager(&self.civ_info).unwrap().add_influence(-10.0);
            }
        }
    }

    pub fn make_peace(&mut self) {
        self.make_peace_one_side();
        self.other_civ_diplomacy().make_peace_one_side();

        for civ in self.get_common_known_civs_with_spectators() {
            civ.add_notification(
                format!("[{}] and [{}] have signed a Peace Treaty!",
                    self.civ_info.civ_name, self.other_civ_name),
                NotificationCategory::Diplomacy,
                &self.civ_info.civ_name,
                NotificationIcon::Diplomacy,
                &self.other_civ_name
            );
        }
    }

    pub fn has_flag(&self, flag: DiplomacyFlags) -> bool {
        self.flags_countdown.contains_key(&format!("{:?}", flag))
    }

    pub fn set_flag(&mut self, flag: DiplomacyFlags, amount: i32) {
        self.flags_countdown.insert(format!("{:?}", flag), amount);
    }

    pub fn get_flag(&self, flag: DiplomacyFlags) -> i32 {
        *self.flags_countdown.get(&format!("{:?}", flag)).unwrap()
    }

    pub fn remove_flag(&mut self, flag: DiplomacyFlags) {
        self.flags_countdown.remove(&format!("{:?}", flag));
    }

    pub fn add_modifier(&mut self, modifier: DiplomaticModifiers, amount: f32) {
        let modifier_string = format!("{:?}", modifier);
        if !self.has_modifier(modifier) {
            self.set_modifier(modifier, 0.0);
        }
        let current = self.diplomatic_modifiers.get_mut(&modifier_string).unwrap();
        *current += amount;
        if *current == 0.0 {
            self.diplomatic_modifiers.remove(&modifier_string);
        }
    }

    pub fn set_modifier(&mut self, modifier: DiplomaticModifiers, amount: f32) {
        self.diplomatic_modifiers.insert(format!("{:?}", modifier), amount);
    }

    pub fn get_modifier(&self, modifier: DiplomaticModifiers) -> f32 {
        if !self.has_modifier(modifier) {
            return 0.0;
        }
        *self.diplomatic_modifiers.get(&format!("{:?}", modifier)).unwrap()
    }

    pub fn remove_modifier(&mut self, modifier: DiplomaticModifiers) {
        self.diplomatic_modifiers.remove(&format!("{:?}", modifier));
    }

    pub fn has_modifier(&self, modifier: DiplomaticModifiers) -> bool {
        self.diplomatic_modifiers.contains_key(&format!("{:?}", modifier))
    }

    pub fn sign_declaration_of_friendship(&mut self) {
        self.set_modifier(DiplomaticModifiers::DeclarationOfFriendship, 35.0);
        self.other_civ_diplomacy().set_modifier(DiplomaticModifiers::DeclarationOfFriendship, 35.0);
        self.set_flag(DiplomacyFlags::DeclarationOfFriendship, 30);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::DeclarationOfFriendship, 30);

        for third_civ in self.get_common_known_civs_with_spectators() {
            third_civ.add_notification(
                format!("[{}] and [{}] have signed the Declaration of Friendship!",
                    self.civ_info.civ_name, self.other_civ_name),
                NotificationCategory::Diplomacy,
                &self.civ_info.civ_name,
                NotificationIcon::Diplomacy,
                &self.other_civ_name
            );
            third_civ.get_diplomacy_manager(&self.civ_info).unwrap().set_friendship_based_modifier();
            if !third_civ.is_spectator() {
                third_civ.get_diplomacy_manager(&self.civ_info).unwrap().set_friendship_based_modifier();
            }
        }

        // Ignore conditionals as triggerUnique will check again, and that would break
        // UniqueType.ConditionalChance - 25% declared chance would work as 6% actual chance
        for unique in self.civ_info.get_triggered_uniques(
            UniqueType::TriggerUponDeclaringFriendship,
            StateForConditionals::IgnoreConditionals
        ) {
            UniqueTriggerActivation::trigger_unique(unique, &self.civ_info);
        }
        for unique in self.other_civ().get_triggered_uniques(
            UniqueType::TriggerUponDeclaringFriendship,
            StateForConditionals::IgnoreConditionals
        ) {
            UniqueTriggerActivation::trigger_unique(unique, self.other_civ());
        }
    }

    pub fn set_friendship_based_modifier(&mut self) {
        self.remove_modifier(DiplomaticModifiers::DeclaredFriendshipWithOurAllies);
        self.remove_modifier(DiplomaticModifiers::DeclaredFriendshipWithOurEnemies);

        let civs_other_civ_has_declared_friendship_with = self.get_common_known_civs()
            .into_iter()
            .filter(|c| c.get_diplomacy_manager(self.other_civ()).unwrap()
                .has_flag(DiplomacyFlags::DeclarationOfFriendship))
            .collect::<Vec<_>>();

        for third_civ in civs_other_civ_has_declared_friendship_with {
            // What do we (A) think about the otherCiv() (B) being friends with the third Civ (C)?
            let our_relationship_with_third_civ = self.civ_info.get_diplomacy_manager(third_civ).unwrap()
                .relationship_ignore_afraid();

            let (modifier_type, modifier_value) = match our_relationship_with_third_civ {
                RelationshipLevel::Unforgivable | RelationshipLevel::Enemy => {
                    (DiplomaticModifiers::DeclaredFriendshipWithOurEnemies, -15.0)
                }
                RelationshipLevel::Friend => {
                    (DiplomaticModifiers::DeclaredFriendshipWithOurAllies, 5.0)
                }
                RelationshipLevel::Ally => {
                    (DiplomaticModifiers::DeclaredFriendshipWithOurAllies, 15.0)
                }
                _ => continue,
            };

            self.add_modifier(modifier_type, modifier_value);
        }
    }

    pub fn sign_defensive_pact(&mut self, duration: i32) {
        //Note: These modifiers are additive to the friendship modifiers
        self.set_modifier(DiplomaticModifiers::DefensivePact, 10.0);
        self.other_civ_diplomacy().set_modifier(DiplomaticModifiers::DefensivePact, 10.0);
        self.set_flag(DiplomacyFlags::DefensivePact, duration);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::DefensivePact, duration);
        self.diplomatic_status = DiplomaticStatus::DefensivePact;
        self.other_civ_diplomacy().diplomatic_status = DiplomaticStatus::DefensivePact;

        for third_civ in self.get_common_known_civs_with_spectators() {
            third_civ.add_notification(
                format!("[{}] and [{}] have signed a Defensive Pact!",
                    self.civ_info.civ_name, self.other_civ_name),
                NotificationCategory::Diplomacy,
                &self.civ_info.civ_name,
                NotificationIcon::Diplomacy,
                &self.other_civ_name
            );
            if !third_civ.is_spectator() {
                third_civ.get_diplomacy_manager(&self.civ_info).unwrap().set_defensive_pact_based_modifier();
            }
        }

        // Ignore conditionals as triggerUnique will check again, and that would break
        // UniqueType.ConditionalChance - 25% declared chance would work as 6% actual chance
        for unique in self.civ_info.get_triggered_uniques(
            UniqueType::TriggerUponSigningDefensivePact,
            StateForConditionals::IgnoreConditionals
        ) {
            UniqueTriggerActivation::trigger_unique(unique, &self.civ_info);
        }
        for unique in self.other_civ().get_triggered_uniques(
            UniqueType::TriggerUponSigningDefensivePact,
            StateForConditionals::IgnoreConditionals
        ) {
            UniqueTriggerActivation::trigger_unique(unique, self.other_civ());
        }
    }

    pub fn set_defensive_pact_based_modifier(&mut self) {
        self.remove_modifier(DiplomaticModifiers::SignedDefensivePactWithOurAllies);
        self.remove_modifier(DiplomaticModifiers::SignedDefensivePactWithOurEnemies);

        for third_civ in self.get_common_known_civs()
            .into_iter()
            .filter(|c| c.get_diplomacy_manager(&self.civ_info).unwrap()
                .has_flag(DiplomacyFlags::DefensivePact)) {
            //Note: These modifiers are additive to the friendship modifiers
            let relationship_level = self.other_civ_diplomacy().relationship_ignore_afraid();

            let (modifier_type, modifier_value) = match relationship_level {
                RelationshipLevel::Unforgivable | RelationshipLevel::Enemy => {
                    (DiplomaticModifiers::SignedDefensivePactWithOurEnemies, -15.0)
                }
                RelationshipLevel::Friend => {
                    (DiplomaticModifiers::SignedDefensivePactWithOurAllies, 2.0)
                }
                RelationshipLevel::Ally => {
                    (DiplomaticModifiers::SignedDefensivePactWithOurAllies, 5.0)
                }
                _ => continue,
            };

            self.add_modifier(modifier_type, modifier_value);
        }
    }

    pub fn set_religion_based_modifier(&mut self) {
        if self.other_civ_diplomacy().believes_same_religion() {
            // they share same majority religion
            self.set_modifier(DiplomaticModifiers::BelieveSameReligion, 5.0);
        } else {
            // their majority religions differ or one or both don't have a majority religion at all
            self.remove_modifier(DiplomaticModifiers::BelieveSameReligion);
        }
    }

    pub fn denounce(&mut self) {
        self.set_modifier(DiplomaticModifiers::Denunciation, -35.0);
        self.other_civ_diplomacy().set_modifier(DiplomaticModifiers::Denunciation, -35.0);
        self.set_flag(DiplomacyFlags::Denunciation, 30);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::Denunciation, 30);

        self.other_civ().add_notification(
            format!("[{}] has denounced us!", self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            NotificationIcon::Diplomacy,
            &self.civ_info.civ_name
        );

        // We, A, are denouncing B. What do other major civs (C,D, etc) think of this?
        for third_civ in self.get_common_known_civs_with_spectators() {
            third_civ.add_notification(
                format!("[{}] has denounced [{}]!", self.civ_info.civ_name, self.other_civ_name),
                NotificationCategory::Diplomacy,
                &self.civ_info.civ_name,
                NotificationIcon::Diplomacy,
                &self.other_civ_name
            );

            if third_civ.is_spectator() {
                continue;
            }

            let third_civ_relationship_with_other_civ = third_civ.get_diplomacy_manager(self.other_civ()).unwrap()
                .relationship_ignore_afraid();
            let third_civ_diplomacy_manager = third_civ.get_diplomacy_manager(&self.civ_info).unwrap();

            match third_civ_relationship_with_other_civ {
                RelationshipLevel::Unforgivable => {
                    third_civ_diplomacy_manager.add_modifier(DiplomaticModifiers::DenouncedOurEnemies, 15.0);
                }
                RelationshipLevel::Enemy => {
                    third_civ_diplomacy_manager.add_modifier(DiplomaticModifiers::DenouncedOurEnemies, 5.0);
                }
                RelationshipLevel::Friend => {
                    third_civ_diplomacy_manager.add_modifier(DiplomaticModifiers::DenouncedOurAllies, -5.0);
                }
                RelationshipLevel::Ally => {
                    third_civ_diplomacy_manager.add_modifier(DiplomaticModifiers::DenouncedOurAllies, -15.0);
                }
                _ => {}
            }
        }
    }

    pub fn agree_not_to_settle_near(&mut self) {
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::AgreedToNotSettleNearUs, 100);
        self.add_modifier(DiplomaticModifiers::UnacceptableDemands, -10.0);
        self.other_civ().add_notification(
            format!("[{}] agreed to stop settling cities near us!", self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            NotificationIcon::Diplomacy,
            &self.civ_info.civ_name
        );
    }

    pub fn refuse_demand_not_to_settle_near(&mut self) {
        self.add_modifier(DiplomaticModifiers::UnacceptableDemands, -20.0);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::IgnoreThemSettlingNearUs, 100);
        self.other_civ_diplomacy().add_modifier(DiplomaticModifiers::RefusedToNotSettleCitiesNearUs, -15.0);
        self.other_civ().add_notification(
            format!("[{}] refused to stop settling cities near us!", self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            NotificationIcon::Diplomacy,
            &self.civ_info.civ_name
        );
    }

    pub fn agree_not_to_spread_religion_to(&mut self) {
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::AgreedToNotSpreadReligion, 100);
        self.add_modifier(DiplomaticModifiers::UnacceptableDemands, -10.0);
        self.other_civ().add_notification(
            format!("[{}] agreed to stop spreading religion to us!", self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            NotificationIcon::Diplomacy,
            &self.civ_info.civ_name
        );
    }

    pub fn refuse_not_to_spread_religion_to(&mut self) {
        self.add_modifier(DiplomaticModifiers::UnacceptableDemands, -20.0);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::IgnoreThemSpreadingReligion, 100);
        self.other_civ_diplomacy().add_modifier(DiplomaticModifiers::RefusedToNotSpreadReligionToUs, -15.0);
        self.other_civ().add_notification(
            format!("[{}] refused to stop spreading religion to us!", self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            NotificationIcon::Diplomacy,
            &self.civ_info.civ_name
        );
    }

    pub fn side_with_city_state(&mut self) {
        self.other_civ_diplomacy().set_modifier(DiplomaticModifiers::SidedWithProtectedMinor, -5.0);
        self.other_civ_diplomacy().set_flag(DiplomacyFlags::RememberSidedWithProtectedMinor, 25);
    }

    pub fn become_wary(&mut self) {
        if self.has_flag(DiplomacyFlags::WaryOf) {
            return; // once is enough
        }
        self.set_flag(DiplomacyFlags::WaryOf, -1); // Never expires
        self.other_civ().add_notification(
            format!("City-States grow wary of your aggression. The resting point for Influence has decreased by [20] for [{}].",
                self.civ_info.civ_name),
            NotificationCategory::Diplomacy,
            &self.civ_info.civ_name
        );
    }

    pub fn gift_gold(&mut self, gold: i32, is_pure_gift: bool) {
        let current_gold = if is_pure_gift {
            (gold as f32 * self.civ_info.game_info.ruleset.mod_options.constants.gold_gift_multiplier) as i32
        } else {
            (gold as f32 * self.civ_info.game_info.ruleset.mod_options.constants.gold_gift_trade_multiplier) as i32
        };

        let other_gold = self.other_civ_diplomacy().get_gold_gifts();
        if other_gold > current_gold {
            self.other_civ_diplomacy().recieve_gold_gifts(-current_gold);
        } else {
            self.other_civ_diplomacy().remove_modifier(DiplomaticModifiers::GaveUsGifts);
            self.recieve_gold_gifts(current_gold - other_gold);
        }
    }

    pub fn recieve_gold_gifts(&mut self, gold: i32) {
        let diplomatic_value_of_trade = (gold as f32 * TradeEvaluation::new().get_gold_inflation(&self.civ_info))
            / (self.civ_info.game_info.speed.gold_gift_modifier * 100.0);
        self.add_modifier(DiplomaticModifiers::GaveUsGifts, diplomatic_value_of_trade);
    }

    pub fn get_gold_gifts(&self) -> i32 {
        // The inverse of how we calculate GaveUsGifts in TradeLogic.acceptTrade gives us how much gold it is worth
        let gift_amount = self.get_modifier(DiplomaticModifiers::GaveUsGifts);
        ((gift_amount * self.civ_info.game_info.speed.gold_gift_modifier * 100.0)
            / TradeEvaluation::new().get_gold_inflation(&self.civ_info)) as i32
    }
}