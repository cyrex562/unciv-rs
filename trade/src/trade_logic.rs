use std::collections::HashMap;

use crate::constants::Constants;
use crate::civilization::Civilization;
use crate::city::City;
use crate::models::ruleset::unique::UniqueType;
use crate::models::ruleset::tile::ResourceType;
use crate::diplomacy::{DiplomacyFlags, DeclareWarReason, WarType};
use crate::civilization::AlertType;
use crate::city::managers::SpyFleeReason;

/// Handles the logic for trading between civilizations
pub struct TradeLogic<'a> {
    /// Our civilization
    pub our_civilization: &'a Civilization,
    /// The other civilization we're trading with
    pub other_civilization: &'a Civilization,
    /// Contains everything we could offer the other player, whether we've actually offered it or not
    pub our_available_offers: TradeOffersList,
    /// Contains everything the other player could offer us, whether they've actually offered it or not
    pub their_available_offers: TradeOffersList,
    /// The current trade being negotiated
    pub current_trade: Trade,
}

impl<'a> TradeLogic<'a> {
    /// Create a new TradeLogic instance
    pub fn new(our_civilization: &'a Civilization, other_civilization: &'a Civilization) -> Self {
        let our_available_offers = Self::get_available_offers(our_civilization, other_civilization);
        let their_available_offers = Self::get_available_offers(other_civilization, our_civilization);

        Self {
            our_civilization,
            other_civilization,
            our_available_offers,
            their_available_offers,
            current_trade: Trade::new(),
        }
    }

    /// Get all available offers that a civilization can make to another civilization
    fn get_available_offers(civ_info: &Civilization, other_civilization: &Civilization) -> TradeOffersList {
        let mut offers = TradeOffersList::new();

        // City states can't trade with each other
        if civ_info.is_city_state() && other_civilization.is_city_state() {
            return offers;
        }

        // Peace treaty if at war
        if civ_info.is_at_war_with(other_civilization) {
            offers.add(TradeOffer::new(
                Constants::peace_treaty.to_string(),
                TradeOfferType::Treaty,
                0,
                civ_info.game_info.speed,
            ));
        }

        // Open borders if both have the unique
        if !other_civilization.get_diplomacy_manager(civ_info).unwrap().has_open_borders
            && !other_civilization.is_city_state()
            && civ_info.has_unique(UniqueType::EnablesOpenBorders)
            && other_civilization.has_unique(UniqueType::EnablesOpenBorders) {
            offers.add(TradeOffer::new(
                Constants::open_borders.to_string(),
                TradeOfferType::Agreement,
                0,
                civ_info.game_info.speed,
            ));
        }

        // Research agreement if possible
        if civ_info.diplomacy_functions.can_sign_research_agreement_no_cost_with(other_civilization) {
            offers.add(TradeOffer::new(
                Constants::research_agreement.to_string(),
                TradeOfferType::Treaty,
                civ_info.diplomacy_functions.get_research_agreement_cost(other_civilization),
                civ_info.game_info.speed,
            ));
        }

        // Defensive pact if possible
        if civ_info.diplomacy_functions.can_sign_defensive_pact_with(other_civilization) {
            offers.add(TradeOffer::new(
                Constants::defensive_pact.to_string(),
                TradeOfferType::Treaty,
                0,
                civ_info.game_info.speed,
            ));
        }

        // Per turn resources
        for entry in civ_info.get_per_turn_resources_with_origins_for_trade()
            .iter()
            .filter(|it| it.resource.resource_type != ResourceType::Bonus)
            .filter(|it| it.origin == Constants::tradable) {
            let resource_trade_offer_type = if entry.resource.resource_type == ResourceType::Luxury {
                TradeOfferType::Luxury_Resource
            } else {
                TradeOfferType::Strategic_Resource
            };

            offers.add(TradeOffer::new(
                entry.resource.name.clone(),
                resource_trade_offer_type,
                entry.amount,
                civ_info.game_info.speed,
            ));
        }

        // Stockpiled resources
        for entry in civ_info.get_stockpiled_resources_for_trade() {
            offers.add(TradeOffer::new(
                entry.resource.name.clone(),
                TradeOfferType::Stockpiled_Resource,
                entry.amount,
                civ_info.game_info.speed,
            ));
        }

        // Gold
        offers.add(TradeOffer::new(
            "Gold".to_string(),
            TradeOfferType::Gold,
            civ_info.gold,
            civ_info.game_info.speed,
        ));

        // Gold per turn
        offers.add(TradeOffer::new(
            "Gold per turn".to_string(),
            TradeOfferType::Gold_Per_Turn,
            civ_info.stats.stats_for_next_turn.gold as i32,
            civ_info.game_info.speed,
        ));

        // Cities (not capital or in resistance)
        if !civ_info.is_one_city_challenger() && !other_civilization.is_one_city_challenger()
            && !civ_info.is_city_state() && !other_civilization.is_city_state() {
            for city in civ_info.cities.iter().filter(|it| !it.is_capital() && !it.is_in_resistance()) {
                offers.add(TradeOffer::new(
                    city.id.clone(),
                    TradeOfferType::City,
                    0,
                    civ_info.game_info.speed,
                ));
            }
        }

        // Civilizations we know that they don't
        let other_civs_we_know = civ_info.get_known_civs()
            .iter()
            .filter(|it| it.civ_name != other_civilization.civ_name && it.is_major_civ() && !it.is_defeated())
            .collect::<Vec<_>>();

        if civ_info.game_info.ruleset.mod_options.has_unique(UniqueType::TradeCivIntroductions) {
            let civs_we_know_and_they_dont = other_civs_we_know
                .iter()
                .filter(|it| !other_civilization.diplomacy.contains_key(&it.civ_name) && !it.is_defeated());

            for third_civ in civs_we_know_and_they_dont {
                offers.add(TradeOffer::new(
                    third_civ.civ_name.clone(),
                    TradeOfferType::Introduction,
                    0,
                    civ_info.game_info.speed,
                ));
            }
        }

        // War declarations
        if !civ_info.is_city_state() && !other_civilization.is_city_state()
            && !civ_info.game_info.ruleset.mod_options.has_unique(UniqueType::DiplomaticRelationshipsCannotChange) {
            let civs_we_both_know = other_civs_we_know
                .iter()
                .filter(|it| other_civilization.diplomacy.contains_key(&it.civ_name));

            let civs_we_arent_at_war_with = civs_we_both_know
                .filter(|it| civ_info.get_diplomacy_manager(it).unwrap().can_declare_war());

            for third_civ in civs_we_arent_at_war_with {
                offers.add(TradeOffer::new(
                    third_civ.civ_name.clone(),
                    TradeOfferType::WarDeclaration,
                    0,
                    civ_info.game_info.speed,
                ));
            }
        }

        offers
    }

    /// Accept the current trade
    pub fn accept_trade(&mut self, apply_gifts: bool) {
        let our_diplo_manager = self.our_civilization.get_diplomacy_manager(self.other_civilization).unwrap();
        let their_diplo_manager = self.other_civilization.get_diplomacy_manager(self.our_civilization).unwrap();

        // Add the trade to both civilizations' diplomacy managers
        our_diplo_manager.apply(|manager| {
            manager.trades.push(self.current_trade.clone());
            manager.update_has_open_borders();
        });

        their_diplo_manager.apply(|manager| {
            manager.trades.push(self.current_trade.reverse());
            manager.update_has_open_borders();
        });

        // Transfer resources, cities, etc.
        for offer in self.current_trade.their_offers.iter().filter(|offer| offer.trade_offer_type != TradeOfferType::Treaty) {
            self.transfer_trade(self.other_civilization, self.our_civilization, offer);
        }

        for offer in self.current_trade.our_offers.iter().filter(|offer| offer.trade_offer_type != TradeOfferType::Treaty) {
            self.transfer_trade(self.our_civilization, self.other_civilization, offer);
        }

        // Transfer treaties (only from one side to avoid double signing)
        for offer in self.current_trade.their_offers.iter().filter(|offer| offer.trade_offer_type == TradeOfferType::Treaty) {
            self.transfer_trade(self.other_civilization, self.our_civilization, offer);
        }

        // Evaluate gifts if needed
        if apply_gifts && !self.current_trade.our_offers.iter().any(|offer| offer.name == Constants::peace_treaty) {
            // Must evaluate before moving, or else cities have already moved and we get an exception
            let our_gold_value_of_trade = TradeEvaluation::get_trade_acceptability(
                &self.current_trade,
                self.our_civilization,
                self.other_civilization,
                false
            );

            let their_gold_value_of_trade = TradeEvaluation::get_trade_acceptability(
                &self.current_trade.reverse(),
                self.other_civilization,
                self.our_civilization,
                false
            );

            if our_gold_value_of_trade > their_gold_value_of_trade {
                let is_pure_gift = self.current_trade.our_offers.is_empty();
                our_diplo_manager.gift_gold(our_gold_value_of_trade - their_gold_value_of_trade.max(0), is_pure_gift);
            } else if their_gold_value_of_trade > our_gold_value_of_trade {
                let is_pure_gift = self.current_trade.their_offers.is_empty();
                their_diplo_manager.gift_gold(their_gold_value_of_trade - our_gold_value_of_trade.max(0), is_pure_gift);
            }
        }

        // Update resources and stats
        self.our_civilization.cache.update_civ_resources();
        self.our_civilization.update_stats_for_next_turn();

        self.other_civilization.cache.update_civ_resources();
        self.other_civilization.update_stats_for_next_turn();
    }

    /// Transfer a trade offer from one civilization to another
    fn transfer_trade(&self, from: &Civilization, to: &Civilization, offer: &TradeOffer) {
        match offer.trade_offer_type {
            TradeOfferType::Gold => {
                to.add_gold(offer.amount);
                from.add_gold(-offer.amount);
            },
            TradeOfferType::Technology => {
                to.tech.add_technology(&offer.name);
            },
            TradeOfferType::City => {
                let city = from.cities.iter().find(|it| it.id == offer.name).unwrap();

                city.espionage.remove_all_present_spies(SpyFleeReason::CityBought);
                city.move_to_civ(to);

                // Teleport units in the city
                for unit in city.get_center_tile().get_units().to_vec() {
                    unit.movement.teleport_to_closest_moveable_tile();
                }

                // Teleport units in the city's tiles
                for tile in city.get_tiles() {
                    for unit in tile.get_units().to_vec() {
                        if !unit.civ.diplomacy_functions.can_pass_through_tiles(to) && !unit.cache.can_enter_foreign_terrain {
                            unit.movement.teleport_to_closest_moveable_tile();
                        }
                    }
                }

                to.cache.update_our_tiles();
                from.cache.update_our_tiles();

                // Suggest an option to liberate the city
                if to.is_human()
                    && !city.founding_civ.is_empty()
                    && from.civ_name != city.founding_civ // can't liberate if the city actually belongs to those guys
                    && to.civ_name != city.founding_civ // can't liberate if it's our city
                {
                    to.popup_alerts.push(AlertType::CityTraded(city.id.clone()));
                }
            },
            TradeOfferType::Treaty => {
                // Note: Treaties are not transferred from both sides due to notifications and double signing
                if offer.name == Constants::peace_treaty {
                    to.get_diplomacy_manager(from).unwrap().make_peace();
                }

                if offer.name == Constants::research_agreement {
                    to.add_gold(-offer.amount);
                    from.add_gold(-offer.amount);

                    to.get_diplomacy_manager(from).unwrap()
                        .set_flag(DiplomacyFlags::ResearchAgreement, offer.duration);

                    from.get_diplomacy_manager(to).unwrap()
                        .set_flag(DiplomacyFlags::ResearchAgreement, offer.duration);
                }

                if offer.name == Constants::defensive_pact {
                    to.get_diplomacy_manager(from).unwrap().sign_defensive_pact(offer.duration);
                }
            },
            TradeOfferType::Introduction => {
                to.diplomacy_functions.make_civilizations_meet(to.game_info.get_civilization(&offer.name));
            },
            TradeOfferType::WarDeclaration => {
                let name_of_civ_to_declare_war_on = &offer.name;
                let war_type = if self.current_trade.their_offers.iter().any(|it| it.trade_offer_type == TradeOfferType::WarDeclaration && it.name == *name_of_civ_to_declare_war_on)
                        && self.current_trade.our_offers.iter().any(|it| it.trade_offer_type == TradeOfferType::WarDeclaration && it.name == *name_of_civ_to_declare_war_on) {
                    WarType::TeamWar
                } else {
                    WarType::JoinWar
                };

                from.get_diplomacy_manager(name_of_civ_to_declare_war_on).unwrap()
                    .declare_war(DeclareWarReason::new(war_type, to));
            },
            _ => {
                // Handle other trade types (resources, gold per turn, etc.)
                if offer.trade_offer_type == TradeOfferType::Gold_Per_Turn {
                    // Gold per turn is handled by the diplomacy manager
                } else if offer.trade_offer_type == TradeOfferType::Luxury_Resource
                    || offer.trade_offer_type == TradeOfferType::Strategic_Resource
                    || offer.trade_offer_type == TradeOfferType::Stockpiled_Resource {
                    // Resources are handled by the diplomacy manager
                } else if offer.trade_offer_type == TradeOfferType::Agreement {
                    // Agreements are handled by the diplomacy manager
                }
            }
        }
    }
}

/// A list of trade offers
#[derive(Clone, Default)]
pub struct TradeOffersList {
    /// The list of offers
    pub offers: Vec<TradeOffer>,
}

impl TradeOffersList {
    /// Create a new empty trade offers list
    pub fn new() -> Self {
        Self {
            offers: Vec::new(),
        }
    }

    /// Add an offer to the list
    pub fn add(&mut self, offer: TradeOffer) {
        self.offers.push(offer);
    }
}

/// A trade offer
#[derive(Clone)]
pub struct TradeOffer {
    /// The name of the offer (resource name, city ID, etc.)
    pub name: String,
    /// The type of offer
    pub trade_offer_type: TradeOfferType,
    /// The amount of the offer (for resources, gold, etc.)
    pub amount: i32,
    /// The duration of the offer (for per-turn offers)
    pub duration: i32,
    /// The game speed
    pub speed: i32,
}

impl TradeOffer {
    /// Create a new trade offer
    pub fn new(name: String, trade_offer_type: TradeOfferType, amount: i32, speed: i32) -> Self {
        Self {
            name,
            trade_offer_type,
            amount,
            duration: 30, // Default duration
            speed,
        }
    }
}

/// A trade between two civilizations
#[derive(Clone, Default)]
pub struct Trade {
    /// Our offers to the other civilization
    pub our_offers: Vec<TradeOffer>,
    /// Their offers to us
    pub their_offers: Vec<TradeOffer>,
}

impl Trade {
    /// Create a new empty trade
    pub fn new() -> Self {
        Self {
            our_offers: Vec::new(),
            their_offers: Vec::new(),
        }
    }

    /// Reverse the trade (swap our offers and their offers)
    pub fn reverse(&self) -> Self {
        Self {
            our_offers: self.their_offers.clone(),
            their_offers: self.our_offers.clone(),
        }
    }
}

/// The type of trade offer
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TradeOfferType {
    /// Gold
    Gold,
    /// Gold per turn
    Gold_Per_Turn,
    /// Technology
    Technology,
    /// City
    City,
    /// Treaty (peace, research agreement, defensive pact)
    Treaty,
    /// Agreement (open borders)
    Agreement,
    /// Luxury resource
    Luxury_Resource,
    /// Strategic resource
    Strategic_Resource,
    /// Stockpiled resource
    Stockpiled_Resource,
    /// Introduction to another civilization
    Introduction,
    /// Declaration of war
    WarDeclaration,
}