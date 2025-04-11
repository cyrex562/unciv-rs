use crate::battle::battle::Battle;
use crate::city::City;
use crate::city::city_flags::CityFlags;
use crate::city::city_focus::CityFocus;
use crate::civilization::Civilization;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::civilization::diplomacy::diplomatic_modifiers::DiplomaticModifiers;
use crate::civilization::diplomacy::diplomatic_status::DiplomaticStatus;
use crate::constants::Constants;
use crate::map::map_unit::unit_promotions::UnitPromotions;
use crate::models::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::trade::trade_logic::TradeLogic;
use crate::trade::trade_offer::TradeOffer;
use crate::trade::trade_offer_type::TradeOfferType;
use crate::espionage::spy_flee_reason::SpyFleeReason;
use std::collections::HashMap;
use std::sync::Arc;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Helper struct for containing logic for moving cities between civilizations
pub struct CityConquestFunctions {
    /// The city being conquered
    pub city: Arc<City>,
    /// Random number generator seeded with the city's position
    tile_based_random: StdRng,
}

impl CityConquestFunctions {
    /// Creates a new CityConquestFunctions instance
    pub fn new(city: Arc<City>) -> Self {
        let seed = city.get_center_tile().position.to_string().as_bytes();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash_slice(seed, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher) as u64;

        CityConquestFunctions {
            city,
            tile_based_random: StdRng::seed_from_u64(hash),
        }
    }

    /// Gets the gold amount for capturing a city
    fn get_gold_for_capturing_city(&mut self, conquering_civ: &Civilization) -> i32 {
        let base_gold = 20 + 10 * self.city.population.population + self.tile_based_random.gen_range(0..40);
        let turn_modifier = (self.city.civ.game_info.turns - self.city.turn_acquired).max(0).min(50) as f32 / 50.0;
        let city_modifier = if self.city.contains_building_unique(UniqueType::DoublesGoldFromCapturingCity) { 2.0 } else { 1.0 };
        let conquering_civ_modifier = if conquering_civ.has_unique(UniqueType::TripleGoldFromEncampmentsAndCities) { 3.0 } else { 1.0 };

        let gold_plundered = base_gold as f32 * turn_modifier * city_modifier * conquering_civ_modifier;
        gold_plundered as i32
    }

    /// Destroys buildings on capture based on various conditions
    fn destroy_buildings_on_capture(&mut self) {
        // Possibly remove other buildings
        for building in self.city.city_constructions.get_built_buildings() {
            if building.has_unique(UniqueType::NotDestroyedWhenCityCaptured) || building.is_wonder {
                continue;
            }
            if building.has_unique(UniqueType::IndicatesCapital, Some(&self.city.state)) {
                continue; // Palace needs to stay a just a bit longer so moveToCiv isn't confused
            }
            if building.has_unique(UniqueType::DestroyedWhenCityCaptured) {
                self.city.city_constructions.remove_building(&building);
                continue;
            }
            // Regular buildings have a 34% chance of removal
            if self.tile_based_random.gen_range(0..100) < 34 {
                self.city.city_constructions.remove_building(&building);
            }
        }
    }

    /// Removes auto promotion from city
    fn remove_auto_promotion(&mut self) {
        self.city.unit_should_use_saved_promotion = HashMap::new();
        self.city.unit_to_promotions = HashMap::new();
    }

    /// Removes buildings when moving a city to a new civilization
    fn remove_buildings_on_move_to_civ(&mut self) {
        // Remove all buildings provided for free to this city
        // At this point, the city has *not* yet moved to the new civ
        for building in self.city.civ.civ_constructions.get_free_building_names(&self.city) {
            self.city.city_constructions.remove_building(&building);
        }
        self.city.city_constructions.free_buildings_provided_from_this_city.clear();

        for building in self.city.city_constructions.get_built_buildings() {
            // Remove national wonders
            if building.is_national_wonder && !building.has_unique(UniqueType::NotDestroyedWhenCityCaptured) {
                self.city.city_constructions.remove_building(&building);
            }

            // Check if we exceed MaxNumberBuildable for any buildings
            for unique in building.get_matching_uniques(UniqueType::MaxNumberBuildable) {
                if self.city.civ.cities.iter()
                        .filter(|city| {
                            city.city_constructions.contains_building_or_equivalent(&building.name) ||
                            city.city_constructions.is_being_constructed_or_enqueued(&building.name)
                        })
                        .count() >= unique.params[0].parse::<usize>().unwrap_or(0) {
                    // For now, just destroy in new city. Even if constructing in own cities
                    self.city.city_constructions.remove_building(&building);
                }
            }
        }
    }

    /// Function for stuff that should happen on any capture, be it puppet, annex or liberate.
    /// Stuff that should happen any time a city is moved between civs, so also when trading,
    /// should go in `this.moveToCiv()`, which is called by `this.conquerCity()`.
    fn conquer_city(&mut self, conquering_civ: &Civilization, conquered_civ: &Civilization, receiving_civ: &Civilization) {
        self.city.espionage.remove_all_present_spies(SpyFleeReason::CityCaptured);

        // Gain gold for plundering city
        let gold_plundered = self.get_gold_for_capturing_city(conquering_civ);
        conquering_civ.add_gold(gold_plundered);
        conquering_civ.add_notification(
            format!("Received [{}] Gold for capturing [{}]", gold_plundered, self.city.name),
            self.city.get_center_tile().position,
            NotificationCategory::General,
            NotificationIcon::Gold
        );

        let reconquered_city_while_still_in_resistance = self.city.previous_owner == receiving_civ.civ_name && self.city.is_in_resistance();

        self.destroy_buildings_on_capture();

        self.move_to_civ(receiving_civ);

        Battle::destroy_if_defeated(conquered_civ, conquering_civ, self.city.location);

        self.city.health = self.city.get_max_health() / 2; // I think that cities recover to half health when conquered?
        self.city.avoid_growth = false; // reset settings
        self.city.set_city_focus(CityFocus::NoFocus); // reset settings
        if self.city.population.population > 1 {
            self.city.population.add_population(-1 - self.city.population.population / 4); // so from 2-4 population, remove 1, from 5-8, remove 2, etc.
        }
        self.city.reassign_all_population();

        if !reconquered_city_while_still_in_resistance && self.city.founding_civ != receiving_civ.civ_name {
            // add resistance
            // I checked, and even if you puppet there's resistance for conquering
            self.city.set_flag(CityFlags::Resistance, self.city.population.population);
        } else {
            // reconquering or liberating city in resistance so eliminate it
            self.city.remove_flag(CityFlags::Resistance);
        }
    }

    /// This happens when we either puppet OR annex, basically whenever we conquer a city and don't liberate it
    pub fn puppet_city(&mut self, conquering_civ: &Civilization) {
        let old_civ = self.city.civ.clone();

        // must be before moving the city to the conquering civ,
        // so the repercussions are properly checked
        self.diplomatic_repercussions_for_conquering_city(&old_civ, conquering_civ);

        self.conquer_city(conquering_civ, &old_civ, conquering_civ);

        self.city.is_puppet = true;
        self.city.city_stats.update();
        // The city could be producing something that puppets shouldn't, like units
        self.city.city_constructions.current_construction_is_user_set = false;
        self.city.city_constructions.in_progress_constructions.clear(); // undo all progress of the previous civ on units etc.
        self.city.city_constructions.construction_queue.clear();
        self.city.city_constructions.choose_next_construction();
    }

    /// Annexes a city
    pub fn annex_city(&mut self) {
        self.city.is_puppet = false;
        if !self.city.is_in_resistance() {
            self.city.should_reassign_population = true;
        }
        self.city.avoid_growth = false;
        self.city.set_city_focus(CityFocus::NoFocus);
        self.city.city_stats.update();
        // GUI.set_update_world_on_next_render();
    }

    /// Handles diplomatic repercussions for conquering a city
    fn diplomatic_repercussions_for_conquering_city(&self, old_civ: &Civilization, conquering_civ: &Civilization) {
        let current_population = self.city.population.population;
        let percentage_of_civ_population_in_that_city = current_population as f32 * 100.0 /
                old_civ.cities.iter().map(|city| city.population.population as f32).sum::<f32>();
        let aggro_generated = 10.0 + percentage_of_civ_population_in_that_city.round() as f32;

        // How can you conquer a city but not know the civ you conquered it from?!
        // I don't know either, but some of our players have managed this, and crashed their game!
        if !conquering_civ.knows(old_civ) {
            conquering_civ.diplomacy_functions.make_civilizations_meet(old_civ);
        }

        old_civ.get_diplomacy_manager(conquering_civ)
                .add_modifier(DiplomaticModifiers::CapturedOurCities, -aggro_generated);

        for third_party_civ in conquering_civ.get_known_civs().iter().filter(|civ| civ.is_major_civ()) {
            let aggro_generated_for_other_civs = (aggro_generated / 10.0).round() as f32;
            if third_party_civ.is_at_war_with(old_civ) {
                // Shared Enemies should like us more
                third_party_civ.get_diplomacy_manager(conquering_civ)
                        .add_modifier(DiplomaticModifiers::SharedEnemy, aggro_generated_for_other_civs); // Cool, keep at it! =D
            } else {
                third_party_civ.get_diplomacy_manager(conquering_civ)
                        .add_modifier(DiplomaticModifiers::WarMongerer, -aggro_generated_for_other_civs); // Uncool bro.
            }
        }
    }

    /// Liberates a city
    pub fn liberate_city(&mut self, conquering_civ: &Civilization) {
        if self.city.founding_civ.is_empty() { // this should never happen but just in case...
            self.puppet_city(conquering_civ);
            self.annex_city();
            return;
        }

        let founding_civ = self.city.civ.game_info.get_civilization(&self.city.founding_civ);
        if founding_civ.is_defeated() { // resurrected civ
            for diplo_manager in founding_civ.diplomacy.values() {
                if diplo_manager.diplomatic_status == DiplomaticStatus::War {
                    diplo_manager.make_peace();
                }
            }
        }

        let old_civ = self.city.civ.clone();

        self.diplomatic_repercussions_for_liberating_city(conquering_civ, &old_civ);

        self.conquer_city(conquering_civ, &old_civ, &founding_civ);

        if founding_civ.cities.len() == 1 {
            // Resurrection!
            if let Some(capital_city_indicator) = conquering_civ.capital_city_indicator(&self.city) {
                self.city.city_constructions.add_building(capital_city_indicator);
            }
            for civ in self.city.civ.game_info.civilizations.iter() {
                if civ == &founding_civ || civ == conquering_civ {
                    continue; // don't need to notify these civs
                }
                if civ.knows(conquering_civ) && civ.knows(&founding_civ) {
                    civ.add_notification(
                        format!("[{}] has liberated [{}]", conquering_civ.civ_name, founding_civ.civ_name),
                        NotificationCategory::Diplomacy,
                        founding_civ.civ_name.clone(),
                        NotificationIcon::Diplomacy,
                        conquering_civ.civ_name.clone()
                    );
                } else if civ.knows(conquering_civ) && !civ.knows(&founding_civ) {
                    civ.add_notification(
                        format!("[{}] has liberated an unknown civilization", conquering_civ.civ_name),
                        NotificationCategory::Diplomacy,
                        NotificationIcon::Diplomacy,
                        conquering_civ.civ_name.clone()
                    );
                } else if !civ.knows(conquering_civ) && civ.knows(&founding_civ) {
                    civ.add_notification(
                        format!("An unknown civilization has liberated [{}]", founding_civ.civ_name),
                        NotificationCategory::Diplomacy,
                        NotificationIcon::Diplomacy,
                        founding_civ.civ_name.clone()
                    );
                }
            }
        }
        self.city.is_puppet = false;
        self.city.city_stats.update();

        // Move units out of the city when liberated
        for unit in self.city.get_center_tile().get_units().iter().cloned().collect::<Vec<_>>() {
            unit.movement.teleport_to_closest_moveable_tile();
        }
        for unit in self.city.get_tiles().iter().flat_map(|tile| tile.get_units()).collect::<Vec<_>>() {
            if !unit.movement.can_pass_through(unit.current_tile) {
                unit.movement.teleport_to_closest_moveable_tile();
            }
        }
    }

    /// Handles diplomatic repercussions for liberating a city
    fn diplomatic_repercussions_for_liberating_city(&self, conquering_civ: &Civilization, conquered_civ: &Civilization) {
        let founding_civ = conquered_civ.game_info.civilizations.iter().find(|civ| civ.civ_name == self.city.founding_civ).unwrap();
        let percentage_of_civ_population_in_that_city = self.city.population.population as f32 *
                100.0 / (founding_civ.cities.iter().map(|city| city.population.population as f32).sum::<f32>() + self.city.population.population as f32);
        let respect_for_liberating_our_city = 10.0 + percentage_of_civ_population_in_that_city.round() as f32;

        if founding_civ.is_major_civ() {
            // In order to get "plus points" in Diplomacy, you have to establish diplomatic relations if you haven't yet
            founding_civ.get_diplomacy_manager_or_meet(conquering_civ)
                    .add_modifier(DiplomaticModifiers::CapturedOurCities, respect_for_liberating_our_city);
            let mut open_borders_trade = TradeLogic::new(founding_civ.clone(), conquering_civ.clone());
            open_borders_trade.current_trade.our_offers.push(TradeOffer::new(Constants::open_borders, TradeOfferType::Agreement, speed: conquering_civ.game_info.speed));
            open_borders_trade.accept_trade(false);
        } else {
            //Liberating a city state gives a large amount of influence, and peace
            founding_civ.get_diplomacy_manager_or_meet(conquering_civ).set_influence(90.0);
            if founding_civ.is_at_war_with(conquering_civ) {
                let mut trade_logic = TradeLogic::new(founding_civ.clone(), conquering_civ.clone());
                trade_logic.current_trade.our_offers.push(TradeOffer::new(Constants::peace_treaty, TradeOfferType::Treaty, speed: conquering_civ.game_info.speed));
                trade_logic.current_trade.their_offers.push(TradeOffer::new(Constants::peace_treaty, TradeOfferType::Treaty, speed: conquering_civ.game_info.speed));
                trade_logic.accept_trade(false);
            }
        }

        let other_civs_respect_for_liberating = (respect_for_liberating_our_city / 10.0).round() as f32;
        for third_party_civ in conquering_civ.get_known_civs().iter().filter(|civ| civ.is_major_civ() && civ != conquered_civ) {
            third_party_civ.get_diplomacy_manager(conquering_civ)
                    .add_modifier(DiplomaticModifiers::LiberatedCity, other_civs_respect_for_liberating); // Cool, keep at at! =D
        }
    }

    /// Moves a city to a new civilization
    pub fn move_to_civ(&mut self, new_civ: &Civilization) {
        let old_civ = self.city.civ.clone();

        // Remove/relocate palace for old Civ - need to do this BEFORE we move the cities between
        //  civs so the capitalCityIndicator recognizes the unique buildings of the conquered civ
        if self.city.is_capital() {
            old_civ.move_capital_to_next_largest(&self.city);
        }

        old_civ.cities = old_civ.cities.iter().filter(|c| c != &self.city).cloned().collect();
        new_civ.cities.push(self.city.clone());
        self.city.civ = new_civ.clone();
        self.city.state = StateForConditionals::new(Some(self.city.clone()));
        self.city.has_just_been_conquered = false;
        self.city.turn_acquired = self.city.civ.game_info.turns;
        self.city.previous_owner = old_civ.civ_name.clone();

        // now that the tiles have changed, we need to reassign population
        for worked_tile in self.city.worked_tiles.iter().filter(|tile| !self.city.tiles.contains(tile)).cloned().collect::<Vec<_>>() {
            self.city.population.stop_working_tile(&worked_tile);
            self.city.population.auto_assign_population();
        }

        // Stop WLTKD if it's still going
        self.city.reset_wltkd();

        // Remove their free buildings from this city and remove free buildings provided by the city from their cities
        self.remove_buildings_on_move_to_civ();

        // Remove auto promotion from city that is being moved
        self.remove_auto_promotion();

        // catch-all - should ideally not happen as we catch the individual cases with an appropriate notification
        self.city.espionage.remove_all_present_spies(SpyFleeReason::Other);

        // Place palace for newCiv if this is the only city they have.
        if new_civ.cities.len() == 1 {
            new_civ.move_capital_to(&this.city, None);
        }

        // Add our free buildings to this city and add free buildings provided by the city to other cities
        self.city.civ.civ_constructions.try_add_free_buildings();

        self.city.is_being_razed = false;

        // Transfer unique buildings
        for building in self.city.city_constructions.get_built_buildings() {
            let civ_equivalent_building = new_civ.get_equivalent_building(&building);
            if building != civ_equivalent_building {
                self.city.city_constructions.remove_building(&building);
                self.city.city_constructions.add_building(civ_equivalent_building);
            }
        }

        if self.city.civ.game_info.is_religion_enabled() {
            self.city.religion.remove_unknown_pantheons();
        }

        if new_civ.has_unique(UniqueType::MayNotAnnexCities) {
            self.city.is_puppet = true;
            self.city.city_constructions.current_construction_is_user_set = false;
            self.city.city_constructions.construction_queue.clear();
            self.city.city_constructions.choose_next_construction();
        }

        self.city.try_update_road_status();
        self.city.city_stats.update();

        // Update proximity rankings
        self.city.civ.update_proximity(&old_civ, old_civ.update_proximity(&self.city.civ));

        // Update history
        for tile in self.city.get_tiles() {
            tile.history.record_take_ownership(&tile);
        }

        new_civ.cache.update_our_tiles();
        old_civ.cache.update_our_tiles();
    }
}