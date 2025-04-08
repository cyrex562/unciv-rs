use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::diplomacy::{DiplomacyFlags, RelationshipLevel};
use crate::models::unit::MapUnit;
use crate::automation::unit::UnitAutomation;
use crate::automation::unit::EspionageAutomation;
use crate::automation::civilization::DiplomacyAutomation;
use crate::automation::civilization::TradeAutomation;
use crate::automation::civilization::ReligionAutomation;
use crate::automation::civilization::UseGoldAutomation;
use crate::automation::civilization::BarbarianAutomation;
use crate::models::battle::{Battle, MapUnitCombatant, CityCombatant, TargetHelper};

/// Contains logic for automating a civilization's turn.
pub struct NextTurnAutomation;

impl NextTurnAutomation {
    /// Top-level AI turn task list
    pub fn automate_civ_moves(civ_info: &mut Civilization, trade_and_change_state: bool) {
        if civ_info.is_barbarian {
            BarbarianAutomation::new(civ_info).automate();
            return;
        }
        if civ_info.is_spectator() {
            return; // When there's a spectator in multiplayer games, it's processed automatically, but shouldn't be able to actually do anything
        }

        Self::respond_to_popup_alerts(civ_info);
        TradeAutomation::respond_to_trade_requests(civ_info, trade_and_change_state);

        if trade_and_change_state && civ_info.is_major_civ() {
            if !civ_info.game_info.ruleset.mod_options.has_unique(UniqueType::DiplomaticRelationshipsCannotChange) {
                DiplomacyAutomation::declare_war(civ_info);
                DiplomacyAutomation::offer_peace_treaty(civ_info);
                DiplomacyAutomation::ask_for_help(civ_info);
                DiplomacyAutomation::offer_declaration_of_friendship(civ_info);
            }
            if civ_info.game_info.is_religion_enabled() {
                ReligionAutomation::spend_faith_on_religion(civ_info);
            }

            DiplomacyAutomation::offer_open_borders(civ_info);
            DiplomacyAutomation::offer_research_agreement(civ_info);
            DiplomacyAutomation::offer_defensive_pact(civ_info);
            TradeAutomation::exchange_luxuries(civ_info);

            Self::issue_requests(civ_info);
            Self::adopt_policy(civ_info);
            Self::free_up_space_resources(civ_info);
        } else if civ_info.is_city_state {
            civ_info.city_state_functions.get_free_tech_for_city_state();
            civ_info.city_state_functions.update_diplomatic_relationship_for_city_state();
        }

        Self::choose_tech_to_research(civ_info);
        Self::automate_city_bombardment(civ_info);
        if trade_and_change_state {
            UseGoldAutomation::use_gold(civ_info);
        }
        if trade_and_change_state && !civ_info.is_city_state {
            Self::protect_city_states(civ_info);
            Self::bully_city_states(civ_info);
        }
        Self::automate_units(civ_info); // this is the most expensive part

        if trade_and_change_state && civ_info.is_major_civ() {
            if civ_info.game_info.is_religion_enabled() {
                // Can only be done now, as the prophet first has to decide to found/enhance a religion
                ReligionAutomation::choose_religious_beliefs(civ_info);
            }
            if civ_info.game_info.is_espionage_enabled() {
                // Do after cities are conquered
                EspionageAutomation::new(civ_info).automate_spies();
            }
        }

        Self::automate_cities(civ_info); // second most expensive
        if trade_and_change_state {
            Self::train_settler(civ_info);
        }
        // I'm not sure what will happen if we *don't* vote when we can, so automate vote even when forced automation
        Self::try_vote_for_diplomatic_victory(civ_info);
    }

    /// Protects city states by pledging protection to them if possible
    fn protect_city_states(civ_info: &mut Civilization) {
        for state in civ_info.get_known_civs().iter()
            .filter(|civ| !civ.is_defeated() && civ.is_city_state) {
            if state.city_state_functions.other_civ_can_pledge_protection(civ_info) {
                state.city_state_functions.add_protector_civ(civ_info);
            }
            // Always pledge to protect, as it makes it harder for others to demand tribute, and grants +10 resting Influence
        }
    }

    /// Bullies city states by demanding tribute from them if conditions are met
    fn bully_city_states(civ_info: &mut Civilization) {
        for state in civ_info.get_known_civs().iter()
            .filter(|civ| !civ.is_defeated() && civ.is_city_state) {
            let diplomacy_manager = state.get_diplomacy_manager(civ_info).unwrap();
            if diplomacy_manager.is_relationship_level_lt(RelationshipLevel::Friend)
                && diplomacy_manager.diplomatic_status == DiplomaticStatus::Peace
                && Self::value_city_state_alliance(civ_info, state) <= 0
                && state.city_state_functions.get_tribute_willingness(civ_info) >= 0 {
                if state.city_state_functions.get_tribute_willingness(civ_info, true) > 0 {
                    state.city_state_functions.tribute_worker(civ_info);
                } else {
                    state.city_state_functions.tribute_gold(civ_info);
                }
            }
        }
    }

    /// Automates city bombardment for all cities that can attack
    pub fn automate_city_bombardment(civ_info: &mut Civilization) {
        for city in &civ_info.cities {
            UnitAutomation::try_bombard_enemy(city);
        }
    }

    /// Automates all units belonging to the civilization
    fn automate_units(civ_info: &mut Civilization) {
        let is_at_war = civ_info.is_at_war();
        let sorted_units = civ_info.units.get_civ_units()
            .sorted_by(|unit| Self::get_unit_priority(unit, is_at_war));

        let cities_requiring_manual_placement = civ_info.get_known_civs().iter()
            .filter(|civ| civ.is_at_war_with(civ_info))
            .flat_map(|civ| &civ.cities)
            .filter(|city| city.get_center_tile().get_tiles_in_distance(4)
                .filter(|tile| tile.military_unit.as_ref().map_or(false, |unit| unit.civ == civ_info))
                .count() > 4)
            .collect::<Vec<_>>();

        for city in cities_requiring_manual_placement {
            Self::automate_city_conquer(civ_info, city);
        }

        for unit in sorted_units {
            UnitAutomation::automate_unit_moves(unit);
        }
    }

    /// Automates the conquest of a specific city
    pub fn automate_city_conquer(civ_info: &mut Civilization, city: &City) {
        fn our_units_in_range(range: i32, city: &City, civ_info: &Civilization) -> Vec<&MapUnit> {
            city.get_center_tile().get_tiles_in_distance(range)
                .filter_map(|tile| tile.military_unit.as_ref())
                .filter(|unit| unit.civ == civ_info)
                .collect()
        }

        fn attack_if_possible(unit: &mut MapUnit, tile: &Tile) {
            let attackable_tile = TargetHelper::get_attackable_enemies(unit,
                unit.movement.get_distance_to_tiles(), vec![tile]).first();
            if let Some(target_tile) = attackable_tile {
                Battle::move_and_attack(MapUnitCombatant::new(unit), target_tile);
            }
        }

        // Air units should do their thing before any of this
        for unit in our_units_in_range(7, city, civ_info).iter()
            .filter(|unit| unit.base_unit.is_air_unit()) {
            UnitAutomation::automate_unit_moves(unit);
        }

        // First off, any siege unit that can attack the city, should
        let siege_units = our_units_in_range(4, city, civ_info).iter()
            .filter(|unit| unit.base_unit.is_probably_siege_unit());
        for unit in siege_units {
            if !unit.has_unique(UniqueType::MustSetUp) || unit.is_set_up_for_siege() {
                attack_if_possible(unit, city.get_center_tile());
            }
        }

        // Melee units should focus on getting rid of enemy units that threaten the siege units
        // If there are no units, this means attacking the city
        let melee_units = our_units_in_range(5, city, civ_info).iter()
            .filter(|unit| unit.base_unit.is_melee());
        for unit in melee_units {
            UnitAutomation::automate_unit_moves(unit);
        }
    }

    /// Gets the priority for a unit in the automation queue
    pub fn get_unit_priority(unit: &MapUnit, is_at_war: bool) -> i32 {
        if unit.is_exploring() {
            return 9;
        }
        if unit.is_automated() {
            return 8;
        }
        if unit.is_moving() {
            return 7;
        }
        if unit.is_fortified() {
            return 6;
        }
        if unit.base_unit.is_ranged() && is_at_war {
            return 5;
        }
        if unit.base_unit.is_melee() && is_at_war {
            return 4;
        }
        if unit.base_unit.is_civilian() {
            return 3;
        }
        if unit.base_unit.is_ranged() {
            return 2;
        }
        if unit.base_unit.is_melee() {
            return 1;
        }
        0
    }

    /// Gets the closest cities between two civilizations
    pub fn get_closest_cities(civ1: &Civilization, civ2: &Civilization) -> Option<CityDistance> {
        let mut min_distance = None;
        for civ1_city in &civ1.cities {
            for civ2_city in &civ2.cities {
                let current_distance = civ1_city.get_center_tile().aerial_distance_to(civ2_city.get_center_tile());
                if min_distance.is_none() || current_distance < min_distance.unwrap().aerial_distance {
                    min_distance = Some(CityDistance {
                        city1: civ1_city,
                        city2: civ2_city,
                        aerial_distance: current_distance,
                    });
                }
            }
        }
        min_distance
    }
}

/// Represents the distance between two cities
pub struct CityDistance<'a> {
    pub city1: &'a City,
    pub city2: &'a City,
    pub aerial_distance: i32,
}