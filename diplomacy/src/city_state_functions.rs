use rand::Rng;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use core::unique::unique_type::UniqueType;

/// Class containing city-state-specific functions
// pub struct CityStateFunctions<'a> {
//     civ_info: &'a Civilization,
// }

// impl<'a> CityStateFunctions<'a> {
//     pub fn new(civ_info: &'a Civilization) -> Self {
//         Self { civ_info }
//     }
// }

/// Attempts to initialize the city state, returning true if successful.
pub fn init_city_state(

    ruleset: &Ruleset,
    starting_era: &str,
    used_major_civs: &[String],
) -> bool {
    let all_mercantile_resources: Vec<String> = ruleset
        .tile_resources
        .values()
        .filter(|it| it.has_unique(UniqueType::CityStateOnlyResource))
        .map(|it| it.name.clone())
        .collect();

    let mut unique_types = HashSet::new(); // We look through these to determine what kinds of city states we have

    let nation = &ruleset.nations[&self.civ_info.civ_name];
    let city_state_type = &ruleset.city_state_types[&nation.city_state_type];
    unique_types.extend(
        city_state_type
            .friend_bonus_unique_map
            .get_all_uniques()
            .filter_map(|it| it.type_),
    );
    unique_types.extend(
        city_state_type
            .ally_bonus_unique_map
            .get_all_uniques()
            .filter_map(|it| it.type_),
    );

    // CS Personality
    self.civ_info.city_state_personality = CityStatePersonality::entries()
        .choose(&mut rand::thread_rng())
        .cloned()
        .unwrap_or(CityStatePersonality::Neutral);

    // Mercantile bonus resources
    if unique_types.contains(&UniqueType::CityStateUniqueLuxury) {
        self.civ_info.city_state_resource = all_mercantile_resources
            .choose(&mut rand::thread_rng())
            .cloned();
    }

    // Unique unit for militaristic city-states
    if unique_types.contains(&UniqueType::CityStateMilitaryUnits) {
        let possible_units: Vec<&BaseUnit> = ruleset
            .units
            .values()
            .filter(|it| {
                !it.available_in_era(ruleset, starting_era) // Not from the start era or before
                        && it.unique_to.as_ref().map_or(false, |unique_to| !used_major_civs.contains(unique_to)) // Must be from a major civ not in the game
                        && ruleset.nations.get(it.unique_to.as_deref().unwrap_or(""))
                            .map_or(false, |n| n.is_major_civ) // don't take unique units from other city states / barbs
                        && ruleset.unit_types[&it.unit_type].is_land_unit()
                        && (it.strength > 0 || it.ranged_strength > 0) // Must be a land military unit
            })
            .collect();

        if !possible_units.is_empty() {
            self.civ_info.city_state_unique_unit = Some(
                possible_units
                    .choose(&mut rand::thread_rng())
                    .unwrap()
                    .name
                    .clone(),
            );
        }
    }

    // TODO: Return false if attempting to put a religious city-state in a game without religion

    true
}

pub fn hold_elections(&self) {
    self.civ_info.add_flag(
        CivFlags::TurnsTillCityStateElection.name(),
        self.civ_info
            .game_info
            .ruleset
            .mod_options
            .constants
            .city_state_election_turns,
    );

    let capital = match self.civ_info.get_capital() {
        Some(cap) => cap,
        None => return,
    };

    let spies: Vec<&Spy> = capital
        .espionage
        .get_all_stationed_spies()
        .filter(|it| it.action == SpyAction::RiggingElections)
        .collect();

    if spies.is_empty() {
        return;
    }

    fn get_votes_from_spy(spy: Option<&Spy>, civ_info: &Civilization) -> f32 {
        match spy {
            None => 20.0,
            Some(spy) => {
                let votes = (civ_info
                    .get_diplomacy_manager_or_meet(&spy.civ_info)
                    .influence
                    / 2.0) as f32;
                votes + (spy.get_skill_modifier_percent() * spy.get_efficiency_modifier()) as f32
            }
        }
    }

    let mut parties: Vec<Option<&Spy>> = spies.iter().map(Some).collect();
    parties.push(None); // Null spy is a neutral party in the election

    let random_seed = (capital.location.x * capital.location.y as f32
        + 123.0 * self.civ_info.game_info.turns as f32) as u64;
    let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed);

    let winner = random_weighted(&mut rng, &parties, |spy| {
        get_votes_from_spy(*spy, self.civ_info)
    })
    .and_then(|spy| spy.map(|s| s.civ_info.clone()));

    // There may be no winner, in that case all spies will lose 5 influence
    if let Some(winner) = winner {
        let ally_civ = self
            .civ_info
            .get_ally_civ()
            .and_then(|name| self.civ_info.game_info.get_civilization(name));

        // Winning civ gets influence and all others lose influence
        for civ in self.civ_info.get_known_civs() {
            let influence = if civ == &winner { 20.0 } else { -5.0 };
            self.civ_info
                .get_diplomacy_manager(civ)
                .unwrap()
                .add_influence(influence);

            if civ == &winner {
                civ.add_notification(
                    &format!(
                        "Your spy successfully rigged the election in [{}]!",
                        self.civ_info.civ_name
                    ),
                    capital.location,
                    NotificationCategory::Espionage,
                    NotificationIcon::Spy,
                );
            } else if spies.iter().any(|it| it.civ_info == *civ) {
                civ.add_notification(
                    &format!(
                        "Your spy lost the election in [{}] to [{}]!",
                        self.civ_info.civ_name, winner.civ_name
                    ),
                    capital.location,
                    NotificationCategory::Espionage,
                    NotificationIcon::Spy,
                );
            } else if ally_civ.as_ref().map_or(false, |ac| ac == civ) {
                // If the previous ally has no spy in the city then we should notify them
                civ.add_notification(
                    &format!(
                        "The election in [{}] were rigged by [{}]!",
                        self.civ_info.civ_name, winner.civ_name
                    ),
                    capital.location,
                    NotificationCategory::Espionage,
                    NotificationIcon::Spy,
                );
            }
        }
    } else {
        // No spy won the election, the civs that tried to rig the election lose influence
        for spy in spies {
            self.civ_info
                .get_diplomacy_manager(&spy.civ_info)
                .unwrap()
                .add_influence(-5.0);
            spy.civ_info.add_notification(
                &format!("Your spy lost the election in [{}]!", capital.name),
                capital.location,
                NotificationCategory::Espionage,
                NotificationIcon::Spy,
            );
        }
    }
}

pub fn turns_for_great_person_from_city_state(&self) -> i32 {
    ((37 + rand::thread_rng().gen_range(0..7)) as f32 * self.civ_info.game_info.speed.modifier)
        as i32
}

/// Gain a random great person from the city state
pub fn give_great_person_to_patron(&self, receiving_civ: &mut Civilization) {
    // Great Prophets can't be gotten from CS
    let giftable_units: Vec<&BaseUnit> = self
        .civ_info
        .game_info
        .ruleset
        .units
        .values()
        .filter(|it| it.is_great_person && !it.has_unique(UniqueType::MayFoundReligion))
        .collect();

    if giftable_units.is_empty() {
        // For badly defined mods that don't have great people but do have the policy that makes city states grant them
        return;
    }

    let gifted_unit = giftable_units.choose(&mut rand::thread_rng()).unwrap();
    let cities = match NextTurnAutomation::get_closest_cities(receiving_civ, self.civ_info) {
        Some(cities) => cities,
        None => return,
    };

    let placed_unit = match receiving_civ
        .units
        .place_unit_near_tile(cities.city1.location, gifted_unit)
    {
        Some(unit) => unit,
        None => return,
    };

    let locations = vec![
        LocationAction::new(placed_unit.get_tile().position),
        LocationAction::new(cities.city2.location),
    ];

    receiving_civ.add_notification(
        &format!(
            "[{}] gave us a [{}] as a gift!",
            self.civ_info.civ_name, gifted_unit.name
        ),
        locations,
        NotificationCategory::Units,
        &[&self.civ_info.civ_name, &gifted_unit.name],
    );
}

pub fn give_military_unit_to_patron(&self, receiving_civ: &mut Civilization) {
    let cities = match NextTurnAutomation::get_closest_cities(receiving_civ, self.civ_info) {
        Some(cities) => cities,
        None => return,
    };

    let city = &cities.city1;

    fn giftable_unique_unit(
        civ_info: &Civilization,
        receiving_civ: &Civilization,
    ) -> Option<&BaseUnit> {
        let unique_unit = civ_info
            .game_info
            .ruleset
            .units
            .get(&civ_info.city_state_unique_unit?)?;
        if !receiving_civ.tech.is_researched(unique_unit) {
            return None;
        }
        if receiving_civ.tech.is_obsolete(unique_unit) {
            return None;
        }
        Some(unique_unit)
    }

    fn random_giftable_unit(city: &City, receiving_civ: &Civilization) -> Option<&BaseUnit> {
        city.city_constructions
            .get_constructable_units()
            .filter(|it| !it.is_civilian() && it.is_land_unit && it.unique_to.is_none())
            .filter(|it| {
                it.get_resource_requirements_per_turn(&receiving_civ.state)
                    .all(|req| {
                        req.value <= 0 || receiving_civ.get_resource_amount(&req.key) >= req.value
                    })
            })
            .collect::<Vec<_>>()
            .choose(&mut rand::thread_rng())
            .copied()
    }

    let military_unit = giftable_unique_unit(self.civ_info, receiving_civ)
        .or_else(|| random_giftable_unit(city, receiving_civ))
        .unwrap_or_else(|| return); // That filter _can_ result in no candidates, if so, quit silently

    // placing the unit may fail - in that case stay quiet
    let placed_unit = match receiving_civ
        .units
        .place_unit_near_tile(city.location, &military_unit.name)
    {
        Some(unit) => unit,
        None => return,
    };

    // The unit should have bonuses from Barracks, Alhambra etc as if it was built in the CS capital
    military_unit.add_construction_bonuses(
        &placed_unit,
        &self.civ_info.get_capital().unwrap().city_constructions,
    );

    // Siam gets +10 XP for all CS units
    for unique in receiving_civ.get_matching_uniques(UniqueType::CityStateGiftedUnitsStartWithXp) {
        placed_unit.promotions.xp += unique.params[0].parse::<i32>().unwrap_or(0);
    }

    // Point to the gifted unit, then to the other places mentioned in the message
    let unit_action = vec![MapUnitAction::new(placed_unit)];
    let notification_actions = unit_action
        .into_iter()
        .chain(std::iter::once(LocationAction::new(cities.city2.location)))
        .chain(std::iter::once(LocationAction::new(city.location)))
        .collect::<Vec<_>>();

    receiving_civ.add_notification(
        &format!(
            "[{}] gave us a [{}] as gift near [{}]!",
            self.civ_info.civ_name, military_unit.name, city.name
        ),
        notification_actions,
        NotificationCategory::Units,
        &[&self.civ_info.civ_name, &military_unit.name],
    );
}

pub fn influence_gained_by_gift(&self, donor_civ: &Civilization, gift_amount: i32) -> i32 {
    // https://github.com/Gedemon/Civ5-DLL/blob/aa29e80751f541ae04858b6d2a2c7dcca454201e/CvGameCoreDLL_Expansion1/CvMinorCivAI.cpp
    // line 8681 and below
    let mut influence_gained = (gift_amount as f32).powf(1.01) / 9.8;
    let speed = &self.civ_info.game_info.speed;
    let game_progress_approximate = min(
        self.civ_info.game_info.turns as f32 / (400.0 * speed.modifier),
        1.0,
    );
    influence_gained *= 1.0 - (2.0 / 3.0) * game_progress_approximate;
    influence_gained *= speed.gold_gift_modifier;

    for unique in donor_civ.get_matching_uniques(UniqueType::CityStateGoldGiftsProvideMoreInfluence)
    {
        influence_gained *= 1.0 + unique.params[0].parse::<f32>().unwrap_or(0.0) / 100.0;
    }

    // Bonus due to "Invest" quests
    influence_gained *= self
        .civ_info
        .quest_manager
        .get_investment_multiplier(&donor_civ.civ_name);

    influence_gained -= influence_gained % 5.0;
    if influence_gained < 5.0 {
        influence_gained = 5.0;
    }
    influence_gained as i32
}

pub fn receive_gold_gift(&self, donor_civ: &mut Civilization, gift_amount: i32) {
    if !self.civ_info.is_city_state {
        panic!("You can only gain influence with City-States!");
    }
    donor_civ.add_gold(-gift_amount);
    self.civ_info.add_gold(gift_amount);
    self.civ_info
        .get_diplomacy_manager(donor_civ)
        .unwrap()
        .add_influence(self.influence_gained_by_gift(donor_civ, gift_amount) as f32);
    self.civ_info.quest_manager.received_gold_gift(donor_civ);
}

pub fn get_protector_civs(&self) -> Vec<&Civilization> {
    if self.civ_info.is_major_civ() {
        return Vec::new();
    }
    self.civ_info
        .diplomacy
        .values()
        .filter(|it| {
            !it.other_civ().is_defeated() && it.diplomatic_status == DiplomaticStatus::Protector
        })
        .map(|it| it.other_civ())
        .collect()
}

pub fn add_protector_civ(&self, other_civ: &Civilization) {
    if !self.other_civ_can_pledge_protection(other_civ) {
        return;
    }

    let diplomacy = self
        .civ_info
        .get_diplomacy_manager(&other_civ.civ_name)
        .unwrap();
    diplomacy.diplomatic_status = DiplomaticStatus::Protector;
    diplomacy.set_flag(DiplomacyFlags::RecentlyPledgedProtection, 10); // Can't break for 10 turns
}

pub fn remove_protector_civ(&self, other_civ: &Civilization, forced: bool) {
    if !forced && !self.other_civ_can_withdraw_protection(other_civ) {
        return;
    }

    let diplomacy = self.civ_info.get_diplomacy_manager(other_civ).unwrap();
    diplomacy.diplomatic_status = DiplomaticStatus::Peace;
    diplomacy.set_flag(DiplomacyFlags::RecentlyWithdrewProtection, 20); // Can't re-pledge for 20 turns
    diplomacy.add_influence(-20.0);
}

pub fn other_civ_can_pledge_protection(&self, other_civ: &Civilization) -> bool {
    // Must be a known city state
    if !self.civ_info.is_city_state
        || !other_civ.is_major_civ()
        || other_civ.is_defeated()
        || !self.civ_info.knows(other_civ)
    {
        return false;
    }
    let diplomacy = self.civ_info.get_diplomacy_manager(other_civ).unwrap();
    // Can't pledge too soon after withdrawing
    if diplomacy.has_flag(DiplomacyFlags::RecentlyWithdrewProtection) {
        return false;
    }
    // Must have at least 0 influence
    if diplomacy.get_influence() < 0.0 {
        return false;
    }
    // can't be at war
    if self.civ_info.is_at_war_with(other_civ) {
        return false;
    }
    // Must not be protected already
    if diplomacy.diplomatic_status == DiplomaticStatus::Protector {
        return false;
    }
    true
}

pub fn other_civ_can_withdraw_protection(&self, other_civ: &Civilization) -> bool {
    // Must be a known city state
    if !self.civ_info.is_city_state
        || !other_civ.is_major_civ()
        || other_civ.is_defeated()
        || !self.civ_info.knows(other_civ)
    {
        return false;
    }
    let diplomacy = self.civ_info.get_diplomacy_manager(other_civ).unwrap();
    // Can't withdraw too soon after pledging
    if diplomacy.has_flag(DiplomacyFlags::RecentlyPledgedProtection) {
        return false;
    }
    // Must be protected
    if diplomacy.diplomatic_status != DiplomaticStatus::Protector {
        return false;
    }
    true
}

pub fn update_ally_civ_for_city_state(&self) {
    if !self.civ_info.is_city_state {
        return;
    }

    let new_ally_name = self
        .civ_info
        .diplomacy
        .iter()
        .filter(|(_, it)| it.other_civ().is_major_civ() && !it.other_civ().is_defeated())
        .max_by_key(|(_, it)| it.get_influence() as i32)
        .filter(|(_, it)| it.get_influence() >= 60.0)
        .map(|(name, _)| name.clone());

    if self.civ_info.get_ally_civ() == new_ally_name {
        return;
    }

    let old_ally_name = self.civ_info.get_ally_civ();
    self.civ_info.set_ally_civ(new_ally_name.clone());

    if let Some(new_ally_name) = new_ally_name {
        let new_ally_civ = self.civ_info.game_info.get_civilization(&new_ally_name);
        let text = format!("We have allied with [{}].", self.civ_info.civ_name);
        new_ally_civ.add_notification(
            &text,
            self.get_notification_actions(),
            NotificationCategory::Diplomacy,
            &[&self.civ_info.civ_name],
            NotificationIcon::Diplomacy,
        );
        new_ally_civ.cache.update_viewable_tiles();
        new_ally_civ.cache.update_civ_resources();

        for unique in new_ally_civ
            .get_matching_uniques(UniqueType::CityStateCanBeBoughtForGold)
            .chain(new_ally_civ.get_matching_uniques(UniqueType::CityStateCanBeBoughtForGoldOld))
        {
            new_ally_civ
                .get_diplomacy_manager(self.civ_info)
                .unwrap()
                .set_flag(
                    DiplomacyFlags::MarriageCooldown,
                    unique.params[0].parse::<i32>().unwrap_or(0),
                );
        }

        // Join the wars of our new ally - loop through all civs they are at war with
        for new_enemy in self
            .civ_info
            .game_info
            .civilizations
            .iter()
            .filter(|it| it.is_at_war_with(&new_ally_civ) && it.is_alive())
        {
            if !self.civ_info.is_at_war_with(new_enemy) {
                if !self.civ_info.knows(new_enemy) {
                    // We have to meet first (meet interesting people - and kill them!)
                    self.civ_info
                        .diplomacy_functions
                        .make_civilizations_meet(new_enemy, true);
                }
                self.civ_info
                    .get_diplomacy_manager(new_enemy)
                    .unwrap()
                    .declare_war(DeclareWarReason::new(
                        WarType::CityStateAllianceWar,
                        &new_ally_civ,
                    ));
            }
        }
    }

    if let Some(old_ally_name) = old_ally_name {
        let old_ally_civ = self.civ_info.game_info.get_civilization(&old_ally_name);
        let text = format!("We have lost alliance with [{}].", self.civ_info.civ_name);
        old_ally_civ.add_notification(
            &text,
            self.get_notification_actions(),
            NotificationCategory::Diplomacy,
            &[&self.civ_info.civ_name],
            NotificationIcon::Diplomacy,
        );

        if let Some(new_ally_name) = &new_ally_name {
            if old_ally_civ.knows_by_name(new_ally_name) {
                let diplomacy_manager = old_ally_civ
                    .get_diplomacy_manager_by_name(new_ally_name)
                    .unwrap();
                diplomacy_manager.add_modifier(DiplomaticModifiers::StoleOurAlly, -10.0);
            }
        }

        old_ally_civ.cache.update_viewable_tiles();
        old_ally_civ.cache.update_civ_resources();
    }
}

/// Returns a Vec of NotificationActions for use in add_notification, showing Capital on map if any, then opening diplomacy
pub fn get_notification_actions(&self) -> Vec<Box<dyn NotificationAction>> {
    let mut actions = Vec::new();

    // Notification click will first point to CS location, if any, then open diplomacy.
    // That's fine for the influence notifications and for afraid too.
    //
    // If the city-state is captured by a civ, it stops being the ally of the civ it was previously an ally of.
    //  This means that it will NOT HAVE a capital at that time, so if we run get_capital()!! we'll get a crash!
    // Or, City States can get stuck with only their Settler and no cities until late into a game if city placements are rare
    if let Some(capital) = self.civ_info.get_capital() {
        actions.push(Box::new(LocationAction::new(capital.location)));
    }
    actions.push(Box::new(DiplomacyAction::new(
        self.civ_info.civ_name.clone(),
    )));

    actions
}

pub fn get_diplomatic_marriage_cost(&self) -> i32 {
    // https://github.com/Gedemon/Civ5-DLL/blob/master/CvGameCoreDLL_Expansion1/CvMinorCivAI.cpp, line 7812
    let mut cost = (500.0 * self.civ_info.game_info.speed.gold_cost_modifier) as i32;
    // Plus disband value of all units
    for unit in self.civ_info.units.get_civ_units() {
        cost += unit.base_unit.get_disband_gold(self.civ_info);
    }
    // Round to lower multiple of 5
    cost /= 5;
    cost *= 5;

    cost
}

pub fn can_be_married_by(&self, other_civ: &Civilization) -> bool {
    !self.civ_info.is_defeated()
        && self.civ_info.is_city_state
        && !self.civ_info.cities.is_empty()
        && self
            .civ_info
            .get_diplomacy_manager(other_civ)
            .unwrap()
            .is_relationship_level_eq(RelationshipLevel::Ally)
        && !other_civ
            .get_diplomacy_manager(self.civ_info)
            .unwrap()
            .has_flag(DiplomacyFlags::MarriageCooldown)
        && (other_civ
            .get_matching_uniques(UniqueType::CityStateCanBeBoughtForGold)
            .next()
            .is_some()
            || other_civ
                .get_matching_uniques(UniqueType::CityStateCanBeBoughtForGoldOld)
                .next()
                .is_some())
        && other_civ.gold >= self.get_diplomatic_marriage_cost()
}

pub fn diplomatic_marriage(&self, other_civ: &mut Civilization) {
    if !self.can_be_married_by(other_civ) {
        // Just in case
        return;
    }

    other_civ.add_gold(-self.get_diplomatic_marriage_cost());

    let notification_location = self.civ_info.get_capital().unwrap().location;
    other_civ.add_notification(
        &format!(
            "We have married into the ruling family of [{}], bringing them under our control.",
            self.civ_info.civ_name
        ),
        notification_location,
        NotificationCategory::Diplomacy,
        &[&self.civ_info.civ_name],
        NotificationIcon::Diplomacy,
        Some(&other_civ.civ_name),
    );

    for civ in self
        .civ_info
        .game_info
        .civilizations
        .iter()
        .filter(|it| it != other_civ)
    {
        civ.add_notification(
                &format!("[{}] has married into the ruling family of [{}], bringing them under their control.",
                    other_civ.civ_name, self.civ_info.civ_name),
                notification_location,
                NotificationCategory::Diplomacy,
                &[&self.civ_info.civ_name],
                NotificationIcon::Diplomacy,
                Some(&other_civ.civ_name)
            );
    }

    for unit in self.civ_info.units.get_civ_units() {
        unit.gift(other_civ);
    }

    // Make sure this CS can never be liberated
    for city in self
        .civ_info
        .game_info
        .get_cities()
        .filter(|it| it.founding_civ == self.civ_info.civ_name)
    {
        city.founding_civ = String::new();
        city.is_original_capital = false;
    }

    for city in &self.civ_info.cities {
        city.espionage
            .remove_all_present_spies(SpyFleeReason::CityTakenOverByMarriage);
        city.move_to_civ(other_civ);
        city.is_puppet = true; // Human players get a popup that allows them to annex instead
    }
    self.civ_info.destroy(notification_location);
}

pub fn get_tribute_willingness(&self, demanding_civ: &Civilization, demanding_worker: bool) -> i32 {
    self.get_tribute_modifiers(demanding_civ, demanding_worker)
        .values()
        .sum()
}

pub fn get_tribute_modifiers(
    &self,
    demanding_civ: &Civilization,
    demanding_worker: bool,
    require_whole_list: bool,
) -> HashMap<String, i32> {
    let mut modifiers = HashMap::new(); // Linked to preserve order when presenting the modifiers table
                                        // Can't bully major civs or unsettled CS's
    if !self.civ_info.is_city_state {
        modifiers.insert("Major Civ".to_string(), -999);
        return modifiers;
    }
    if self.civ_info.cities.is_empty() || self.civ_info.get_capital().is_none() {
        modifiers.insert("No Cities".to_string(), -999);
        return modifiers;
    }

    modifiers.insert("Base value".to_string(), -110);

    if self.civ_info.city_state_personality == CityStatePersonality::Hostile {
        modifiers.insert("Hostile".to_string(), -10);
    }
    if let Some(ally_civ) = self.civ_info.get_ally_civ() {
        if ally_civ != demanding_civ.civ_name {
            modifiers.insert("Has Ally".to_string(), -10);
        }
    }
    if self
        .get_protector_civs()
        .iter()
        .any(|it| it != demanding_civ)
    {
        modifiers.insert("Has Protector".to_string(), -20);
    }
    if demanding_worker {
        modifiers.insert("Demanding a Worker".to_string(), -30);
    }
    if demanding_worker && self.civ_info.get_capital().unwrap().population.population < 4 {
        modifiers.insert("Demanding a Worker from small City-State".to_string(), -300);
    }
    if let Some(recent_bullying) = self.civ_info.get_recent_bullying_countdown() {
        if recent_bullying > 10 {
            modifiers.insert("Very recently paid tribute".to_string(), -300);
        } else if recent_bullying > 0 {
            modifiers.insert("Recently paid tribute".to_string(), -40);
        }
    }
    if self
        .civ_info
        .get_diplomacy_manager(demanding_civ)
        .unwrap()
        .get_influence()
        < -30.0
    {
        modifiers.insert("Influence below -30".to_string(), -300);
    }

    // Slight optimization, we don't do the expensive stuff if we have no chance of getting a >= 0 result
    if !require_whole_list && modifiers.values().sum::<i32>() < -200 {
        return modifiers;
    }

    let force_rank = self
        .civ_info
        .game_info
        .get_alive_major_civs()
        .iter()
        .enumerate()
        .find(|(_, civ)| *civ == demanding_civ)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let global_modifier = self
        .civ_info
        .game_info
        .ruleset
        .mod_options
        .constants
        .tribute_global_modifier;
    modifiers.insert(
        "Military Rank".to_string(),
        global_modifier
            - ((global_modifier / self.civ_info.game_info.game_parameters.players.len() as i32)
                * force_rank as i32),
    );

    if !require_whole_list && modifiers.values().sum::<i32>() < -100 {
        return modifiers;
    }

    let bully_range = (self.civ_info.game_info.tile_map.tile_matrix.size / 10).clamp(5, 10); // Longer range for larger maps
    let in_range_tiles = self
        .civ_info
        .get_capital()
        .unwrap()
        .get_center_tile()
        .get_tiles_in_distance_range(1..=bully_range);
    let force_near_city = in_range_tiles
        .iter()
        .map(|it| {
            if let Some(military_unit) = it.military_unit.as_ref() {
                if military_unit.civ == *demanding_civ {
                    military_unit.get_force_evaluation()
                } else {
                    0
                }
            } else {
                0
            }
        })
        .sum::<i32>();
    let cs_force = (CityCombatant::new(self.civ_info.get_capital().unwrap())
        .get_defending_strength() as f32)
        .powf(1.5) as i32
        + in_range_tiles
            .iter()
            .map(|it| {
                if let Some(military_unit) = it.military_unit.as_ref() {
                    if military_unit.civ == *self.civ_info {
                        military_unit.get_force_evaluation()
                    } else {
                        0
                    }
                } else {
                    0
                }
            })
            .sum::<i32>();
    let force_ratio = force_near_city as f32 / cs_force as f32;
    let local_modifier = self
        .civ_info
        .game_info
        .ruleset
        .mod_options
        .constants
        .tribute_local_modifier;

    modifiers.insert(
        "Military near City-State".to_string(),
        match force_ratio {
            r if r > 3.0 => local_modifier,
            r if r > 2.0 => local_modifier * 4 / 5,
            r if r > 1.5 => local_modifier * 3 / 5,
            r if r > 1.0 => local_modifier * 2 / 5,
            r if r > 0.5 => local_modifier / 5,
            _ => 0,
        },
    );

    modifiers
}

pub fn gold_gained_by_tribute(&self) -> i32 {
    // These values are close enough, linear increase throughout the game
    let mut gold = (10.0 * self.civ_info.game_info.speed.gold_gift_modifier) as i32 * 5; // rounding down to nearest 5
    let turns_to_increment = self
        .civ_info
        .game_info
        .speed
        .city_state_tribute_scaling_interval;
    gold += 5 * (self.civ_info.game_info.turns / turns_to_increment) as i32;

    gold
}

pub fn tribute_gold(&self, demanding_civ: &mut Civilization) {
    if !self.civ_info.is_city_state {
        panic!("You can only demand gold from City-States!");
    }
    let gold_amount = self.gold_gained_by_tribute();
    demanding_civ.add_gold(gold_amount);
    self.civ_info
        .get_diplomacy_manager(demanding_civ)
        .unwrap()
        .add_influence(-15.0);
    self.city_state_bullied(demanding_civ);
    self.civ_info.add_flag(CivFlags::RecentlyBullied.name(), 20);
}

pub fn tribute_worker(&self, demanding_civ: &mut Civilization) {
    if !self.civ_info.is_city_state {
        panic!("You can only demand workers from City-States!");
    }

    let buildable_worker_like_units: Vec<&BaseUnit> = self
        .civ_info
        .game_info
        .ruleset
        .units
        .values()
        .filter(|it| {
            it.value.has_unique(UniqueType::BuildImprovements)
                && it.value.is_civilian()
                && it.value.is_buildable(&self.civ_info)
        })
        .collect();

    if buildable_worker_like_units.is_empty() {
        return; // Bad luck?
    }

    demanding_civ.units.place_unit_near_tile(
        self.civ_info.get_capital().unwrap().location,
        buildable_worker_like_units
            .choose(&mut rand::thread_rng())
            .unwrap(),
    );

    self.civ_info
        .get_diplomacy_manager(demanding_civ)
        .unwrap()
        .add_influence(-50.0);
    self.city_state_bullied(demanding_civ);
    self.civ_info.add_flag(CivFlags::RecentlyBullied.name(), 20);
}

pub fn can_provide_stat(&self, stat_type: Stat) -> bool {
    if !self.civ_info.is_city_state {
        return false;
    }
    for bonus in
        self.get_city_state_bonuses(&self.civ_info.city_state_type, RelationshipLevel::Ally)
    {
        if bonus.stats[stat_type] > 0 {
            return true;
        }
    }
    false
}

pub fn update_diplomatic_relationship_for_city_state(&self) {
    // Check if city-state invaded by other civs
    if self.get_num_threatening_barbarians() > 0 {
        return; // Assume any players are there to fight barbarians
    }

    for other_civ in self
        .civ_info
        .get_known_civs()
        .filter(|it| it.is_major_civ())
        .collect::<Vec<_>>()
    {
        if self.civ_info.is_at_war_with(other_civ) {
            continue;
        }
        if other_civ.has_unique(UniqueType::CityStateTerritoryAlwaysFriendly) {
            continue;
        }
        let diplomacy = self.civ_info.get_diplomacy_manager(other_civ).unwrap();
        if diplomacy.has_flag(DiplomacyFlags::AngerFreeIntrusion) {
            continue; // They recently helped us
        }

        let units_in_border = other_civ
            .units
            .get_civ_units()
            .filter(|it| !it.is_civilian() && it.get_tile().get_owner() == Some(self.civ_info))
            .count();
        if units_in_border > 0 && diplomacy.is_relationship_level_lt(RelationshipLevel::Friend) {
            diplomacy.add_influence(-10.0);
            if !diplomacy.has_flag(DiplomacyFlags::BorderConflict) {
                other_civ.popup_alerts.push(PopupAlert::new(
                    AlertType::BorderConflict,
                    self.civ_info.civ_name.clone(),
                ));
                diplomacy.set_flag(DiplomacyFlags::BorderConflict, 10);
            }
        }
    }
}

pub fn get_free_tech_for_city_state(&self) {
    // City-States automatically get all techs that at least half of the major civs know
    let researchable_techs: Vec<String> = self
        .civ_info
        .game_info
        .ruleset
        .technologies
        .keys()
        .filter(|it| self.civ_info.tech.can_be_researched(it))
        .cloned()
        .collect();

    for tech in researchable_techs {
        let alive_major_civs: Vec<&Civilization> = self.civ_info.game_info.get_alive_major_civs();
        if alive_major_civs
            .iter()
            .filter(|it| it.tech.is_researched(&tech))
            .count()
            >= alive_major_civs.len() / 2
        {
            self.civ_info.tech.add_technology(&tech);
        }
    }
}

pub fn get_num_threatening_barbarians(&self) -> i32 {
    if self.civ_info.game_info.game_parameters.no_barbarians {
        return 0;
    }
    let barbarian_civ = self
        .civ_info
        .game_info
        .civilizations
        .iter()
        .find(|it| it.is_barbarian)
        .unwrap_or_else(|| return 0);
    barbarian_civ
        .units
        .get_civ_units()
        .filter(|it| it.threatens_civ(self.civ_info))
        .count() as i32
}

pub fn threatening_barbarian_killed_by(&self, other_civ: &mut Civilization) {
    let diplomacy = self.civ_info.get_diplomacy_manager(other_civ).unwrap();
    if diplomacy.diplomatic_status == DiplomaticStatus::War {
        return; // No reward for enemies
    }

    diplomacy.add_influence(12.0);

    if diplomacy.has_flag(DiplomacyFlags::AngerFreeIntrusion) {
        diplomacy.set_flag(
            DiplomacyFlags::AngerFreeIntrusion,
            diplomacy.get_flag(DiplomacyFlags::AngerFreeIntrusion) + 5,
        );
    } else {
        diplomacy.set_flag(DiplomacyFlags::AngerFreeIntrusion, 5);
    }

    other_civ.add_notification(
        &format!(
            "[{}] is grateful that you killed a Barbarian that was threatening them!",
            self.civ_info.civ_name
        ),
        vec![Box::new(DiplomacyAction::new(
            self.civ_info.civ_name.clone(),
        ))],
        NotificationCategory::Diplomacy,
        &[&self.civ_info.civ_name],
    );
}

/// A city state was bullied. What are its protectors going to do about it???
fn city_state_bullied(&self, bully: &Civilization) {
    if !self.civ_info.is_city_state {
        return; // What are we doing here?
    }

    for protector in self.get_protector_civs() {
        if !protector.knows(bully) {
            // Who?
            continue;
        }
        let protector_diplomacy = protector.get_diplomacy_manager(bully).unwrap();
        if protector_diplomacy.has_modifier(DiplomaticModifiers::BulliedProtectedMinor)
            && protector_diplomacy.get_flag(DiplomacyFlags::RememberBulliedProtectedMinor) > 50
        {
            protector_diplomacy.add_modifier(DiplomaticModifiers::BulliedProtectedMinor, -10.0);
        // Penalty less severe for second offence
        } else {
            protector_diplomacy.add_modifier(DiplomaticModifiers::BulliedProtectedMinor, -15.0);
        }
        protector_diplomacy.set_flag(DiplomacyFlags::RememberBulliedProtectedMinor, 75); // Reset their memory

        if protector.player_type != PlayerType::Human {
            // Humans can have their own emotions
            bully.add_notification(
                    &format!("[{}] is upset that you demanded tribute from [{}], whom they have pledged to protect!",
                        protector.civ_name, self.civ_info.civ_name),
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Diplomacy,
                    &[&protector.civ_name]
                );
        } else {
            // Let humans choose who to side with
            protector.popup_alerts.push(PopupAlert::new(
                AlertType::BulliedProtectedMinor,
                format!("{}@{}", bully.civ_name, self.civ_info.civ_name),
            )); // we need to pass both civs as argument, hence the horrible chimera
        }
    }

    // Set a diplomatic flag so we remember for future quests (and not to give them any)
    self.civ_info
        .get_diplomacy_manager(bully)
        .unwrap()
        .set_flag(DiplomacyFlags::Bullied, 20);

    // Notify all City-States that we were bullied (for quests)
    for city_state in self.civ_info.game_info.get_alive_city_states() {
        city_state
            .quest_manager
            .city_state_bullied(self.civ_info, bully);
    }
}

/// A city state was attacked. What are its protectors going to do about it??? Also checks for Wary
pub fn city_state_attacked(&self, attacker: &Civilization) {
    if !self.civ_info.is_city_state {
        return; // What are we doing here?
    }
    if attacker.is_city_state {
        return; // City states can't be upset with each other
    }

    // We might become wary!
    if attacker.is_minor_civ_warmonger() {
        // They've attacked a lot of city-states
        self.civ_info
            .get_diplomacy_manager(attacker)
            .unwrap()
            .become_wary();
    } else if attacker.is_minor_civ_aggressor() {
        // They've attacked a few
        if rand::thread_rng().gen_bool(0.5) {
            // 50% chance
            self.civ_info
                .get_diplomacy_manager(attacker)
                .unwrap()
                .become_wary();
        }
    }
    // Others might become wary!
    if attacker.is_minor_civ_aggressor() {
        self.make_other_city_states_wary_of_attacker(attacker);
    }

    self.trigger_protector_civs(attacker);

    // Even if we aren't *technically* protectors, we *can* still be pissed you attacked our allies
    self.trigger_ally_civs(attacker);

    // Set up war with major pseudo-quest
    self.civ_info.quest_manager.was_attacked_by(attacker);
    self.civ_info
        .get_diplomacy_manager(attacker)
        .unwrap()
        .set_flag(DiplomacyFlags::RecentlyAttacked, 2); // Reminder to ask for unit gifts in 2 turns
}

fn make_other_city_states_wary_of_attacker(&self, attacker: &Civilization) {
    for city_state in self.civ_info.game_info.get_alive_city_states() {
        if city_state == self.civ_info {
            // Must be a different minor
            continue;
        }
        if city_state
            .get_ally_civ()
            .as_ref()
            .map_or(false, |name| name == &attacker.civ_name)
        {
            // Must not be allied to the attacker
            continue;
        }
        if !city_state.knows(attacker) {
            // Must have met
            continue;
        }
        if city_state.quest_manager.wants_dead(&self.civ_info.civ_name) {
            // Must not want us dead
            continue;
        }

        let mut probability = if attacker.is_minor_civ_warmonger() {
            // High probability if very aggressive
            match city_state.get_proximity(attacker) {
                Proximity::Neighbors => 100,
                Proximity::Close => 75,
                Proximity::Far => 50,
                Proximity::Distant => 25,
                _ => 0,
            }
        } else {
            // Lower probability if only somewhat aggressive
            match city_state.get_proximity(attacker) {
                Proximity::Neighbors => 50,
                Proximity::Close => 20,
                _ => 0,
            }
        };

        // Higher probability if already at war
        if city_state.is_at_war_with(attacker) {
            probability += 50;
        }

        if rand::thread_rng().gen_range(0..100) <= probability {
            city_state
                .get_diplomacy_manager(attacker)
                .unwrap()
                .become_wary();
        }
    }
}

fn trigger_protector_civs(&self, attacker: &Civilization) {
    for protector in self.get_protector_civs() {
        let protector_diplomacy = match protector.get_diplomacy_manager(attacker) {
            Some(dm) => dm,
            None => continue, // Who?
        };
        if protector_diplomacy.has_modifier(DiplomaticModifiers::AttackedProtectedMinor)
            && protector_diplomacy.get_flag(DiplomacyFlags::RememberAttackedProtectedMinor) > 50
        {
            protector_diplomacy.add_modifier(DiplomaticModifiers::AttackedProtectedMinor, -15.0);
        // Penalty less severe for second offence
        } else {
            protector_diplomacy.add_modifier(DiplomaticModifiers::AttackedProtectedMinor, -20.0);
        }
        protector_diplomacy.set_flag(DiplomacyFlags::RememberAttackedProtectedMinor, 75); // Reset their memory

        if protector.player_type != PlayerType::Human {
            // Humans can have their own emotions
            attacker.add_notification(
                &format!(
                    "[{}] is upset that you attacked [{}], whom they have pledged to protect!",
                    protector.civ_name, self.civ_info.civ_name
                ),
                NotificationCategory::Diplomacy,
                NotificationIcon::Diplomacy,
                &[&protector.civ_name],
            );
        } else {
            // Let humans choose who to side with
            protector.popup_alerts.push(PopupAlert::new(
                AlertType::AttackedProtectedMinor,
                format!("{}@{}", attacker.civ_name, self.civ_info.civ_name),
            )); // we need to pass both civs as argument, hence the horrible chimera
        }
    }
}

fn trigger_ally_civs(&self, attacker: &Civilization) {
    if let Some(ally_civ_name) = self.civ_info.get_ally_civ() {
        let ally_civ = self.civ_info.game_info.get_civilization(&ally_civ_name);
        if !self.get_protector_civs().contains(&ally_civ) && ally_civ.knows(attacker) {
            let ally_diplomacy = ally_civ.get_diplomacy_manager(attacker).unwrap();
            // Less than if we were protectors
            ally_diplomacy.add_modifier(DiplomaticModifiers::AttackedAlliedMinor, -10.0);

            if ally_civ.player_type != PlayerType::Human {
                // Humans can have their own emotions
                attacker.add_notification(
                    &format!(
                        "[{}] is upset that you attacked [{}], whom they are allied with!",
                        ally_civ.civ_name, self.civ_info.civ_name
                    ),
                    NotificationCategory::Diplomacy,
                    NotificationIcon::Diplomacy,
                    &[&ally_civ.civ_name],
                );
            } else {
                // Let humans choose who to side with
                ally_civ.popup_alerts.push(PopupAlert::new(
                    AlertType::AttackedAllyMinor,
                    format!("{}@{}", attacker.civ_name, self.civ_info.civ_name),
                ));
            }
        }
    }
}

/// A city state was destroyed. Its protectors are going to be upset!
pub fn city_state_destroyed(&self, attacker: &Civilization) {
    if !self.civ_info.is_city_state {
        return; // What are we doing here?
    }

    for protector in self.get_protector_civs() {
        if !protector.knows(attacker) {
            // Who?
            continue;
        }
        let protector_diplomacy = protector.get_diplomacy_manager(attacker).unwrap();
        if protector_diplomacy.has_modifier(DiplomaticModifiers::DestroyedProtectedMinor) {
            protector_diplomacy.add_modifier(DiplomaticModifiers::DestroyedProtectedMinor, -10.0);
        // Penalty less severe for second offence
        } else {
            protector_diplomacy.add_modifier(DiplomaticModifiers::DestroyedProtectedMinor, -40.0);
            // Oof
        }
        protector_diplomacy.set_flag(DiplomacyFlags::RememberDestroyedProtectedMinor, 125); // Reset their memory

        if protector.player_type != PlayerType::Human {
            // Humans can have their own emotions
            attacker.add_notification(
                &format!(
                    "[{}] is outraged that you destroyed [{}], whom they had pledged to protect!",
                    protector.civ_name, self.civ_info.civ_name
                ),
                NotificationCategory::Diplomacy,
                NotificationIcon::Diplomacy,
                &[&protector.civ_name],
            );
        }
        protector.add_notification(
            &format!(
                "[{}] has destroyed [{}], whom you had pledged to protect!",
                attacker.civ_name, self.civ_info.civ_name
            ),
            NotificationCategory::Diplomacy,
            &[&attacker.civ_name],
            NotificationIcon::Death,
            Some(&self.civ_info.civ_name),
        );
    }

    // Notify all City-States that we were killed (for quest completion)
    for city_state in self.civ_info.game_info.get_alive_city_states() {
        city_state
            .quest_manager
            .city_state_conquered(self.civ_info, attacker);
    }
}

/// Asks all met majors that haven't yet declared war on [attacker] to at least give some units
pub fn ask_for_unit_gifts(&self, attacker: &Civilization) {
    if attacker.is_defeated() || self.civ_info.is_defeated() {
        // never mind, someone died
        return;
    }
    if self.civ_info.cities.is_empty() {
        // Can't receive units with no cities
        return;
    }

    for third_civ in self.civ_info.get_known_civs().filter(|it| {
        it != attacker
            && it.is_alive()
            && it.knows(attacker)
            && !it.is_at_war_with(attacker)
            && it.is_major_civ()
    }) {
        third_civ.add_notification(
                &format!("[{}] is being attacked by [{}] and asks all major civilizations to help them out by gifting them military units.",
                    self.civ_info.civ_name, attacker.civ_name),
                self.civ_info.get_capital().unwrap().location,
                NotificationCategory::Diplomacy,
                &[&self.civ_info.civ_name],
                "OtherIcons/Present"
            );
    }
}

pub fn get_city_state_resources_for_ally(&self) -> ResourceSupplyList {
    let mut resource_list = ResourceSupplyList::new();
    // TODO: City-states don't give allies resources from civ-wide uniques!
    let civ_resource_modifiers = self.civ_info.get_resource_modifiers();
    for city in &self.civ_info.cities {
        // IGNORE the fact that they consume their own resources - #4769
        resource_list.add_positive_by_resource(
            city.get_resources_generated_by_city(&civ_resource_modifiers),
            Constants::city_states,
        );
    }
    resource_list
}

// TODO: Optimize, update whenever status changes, otherwise retain the same list
pub fn get_uniques_provided_by_city_states(
    &self,
    unique_type: UniqueType,
    state_for_conditionals: &StateForConditionals,
) -> impl Iterator<Item = &Unique> {
    if self.civ_info.is_city_state {
        return std::iter::empty();
    }

    self.civ_info
        .get_known_civs()
        .filter(|it| it.is_city_state)
        .flat_map(|city_state| {
            // We don't use DiplomacyManager.get_relationship_level for performance reasons - it tries to calculate get_tribute_willingness which is heavy
            let relationship_level = if city_state
                .get_ally_civ()
                .as_ref()
                .map_or(false, |name| name == &self.civ_info.civ_name)
            {
                RelationshipLevel::Ally
            } else if city_state
                .get_diplomacy_manager(self.civ_info)
                .unwrap()
                .get_influence()
                >= 30.0
            {
                RelationshipLevel::Friend
            } else {
                RelationshipLevel::Neutral
            };
            self.get_city_state_bonuses(
                &city_state.city_state_type,
                relationship_level,
                Some(unique_type),
            )
        })
        .filter(|it| it.conditionals_apply(state_for_conditionals))
}

pub fn get_city_state_bonuses(
    &self,
    city_state_type: &CityStateType,
    relationship_level: RelationshipLevel,
    unique_type: Option<UniqueType>,
) -> impl Iterator<Item = &Unique> {
    let city_state_unique_map = match relationship_level {
        RelationshipLevel::Ally => &city_state_type.ally_bonus_unique_map,
        RelationshipLevel::Friend => &city_state_type.friend_bonus_unique_map,
        _ => return std::iter::empty(),
    };
    match unique_type {
        None => city_state_unique_map.get_all_uniques(),
        Some(unique_type) => city_state_unique_map.get_uniques(unique_type),
    }
}
