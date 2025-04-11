use crate::models::civilization::Civilization;
use crate::models::civilization::diplomacy::RelationshipLevel;
use crate::models::Spy;
use crate::models::SpyAction;
use rand::Rng;
use std::collections::HashMap;

/// Contains logic for automating spy actions
pub struct EspionageAutomation<'a> {
    civ_info: &'a Civilization,
    civs_to_steal_from: Vec<&'a Civilization>,
    get_civs_to_steal_from_sorted: Vec<&'a Civilization>,
    city_states_to_rig: Vec<&'a Civilization>,
}

impl<'a> EspionageAutomation<'a> {
    /// Creates a new EspionageAutomation instance
    pub fn new(civ_info: &'a Civilization) -> Self {
        let civs_to_steal_from = Self::get_civs_to_steal_from(civ_info);
        let get_civs_to_steal_from_sorted = Self::get_civs_to_steal_from_sorted(civ_info, &civs_to_steal_from);
        let city_states_to_rig = Self::get_city_states_to_rig(civ_info);

        Self {
            civ_info,
            civs_to_steal_from,
            get_civs_to_steal_from_sorted,
            city_states_to_rig,
        }
    }

    /// Gets civilizations that can be stolen from
    fn get_civs_to_steal_from(civ_info: &Civilization) -> Vec<&Civilization> {
        civ_info.get_known_civs()
            .iter()
            .filter(|other_civ| {
                other_civ.is_major_civ() &&
                other_civ.cities.iter().any(|city| {
                    city.get_center_tile().is_explored(civ_info) &&
                    civ_info.espionage_manager.get_spy_assigned_to_city(city).is_none()
                }) &&
                !civ_info.espionage_manager.get_techs_to_steal(other_civ).is_empty()
            })
            .collect()
    }

    /// Gets civilizations to steal from sorted by priority
    fn get_civs_to_steal_from_sorted(civ_info: &Civilization, civs_to_steal_from: &[&Civilization]) -> Vec<&Civilization> {
        let mut sorted_civs = civs_to_steal_from.to_vec();

        sorted_civs.sort_by(|a, b| {
            let a_count = civ_info.espionage_manager.spy_list
                .iter()
                .filter(|spy| spy.is_doing_work() && spy.get_city_or_null().map_or(false, |city| city.civ == **a))
                .count();

            let b_count = civ_info.espionage_manager.spy_list
                .iter()
                .filter(|spy| spy.is_doing_work() && spy.get_city_or_null().map_or(false, |city| city.civ == **b))
                .count();

            a_count.cmp(&b_count)
        });

        sorted_civs
    }

    /// Gets city-states that can be rigged
    fn get_city_states_to_rig(civ_info: &Civilization) -> Vec<&Civilization> {
        civ_info.get_known_civs()
            .iter()
            .filter(|other_civ| {
                other_civ.is_minor_civ() &&
                other_civ.knows(civ_info) &&
                !civ_info.is_at_war_with(other_civ)
            })
            .collect()
    }

    /// Automates all spies
    pub fn automate_spies(&self) {
        let spies = &self.civ_info.espionage_manager.spy_list;
        let spies_to_move: Vec<_> = spies.iter()
            .filter(|spy| spy.is_alive() && !spy.is_doing_work())
            .collect();

        for spy in spies_to_move {
            let random_seed = spies.len() + spies.iter().position(|s| s == *spy).unwrap_or(0) + self.civ_info.game_info.turns;
            let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed as u64);
            let random_action = rng.gen_range(0..10);

            // Try each operation based on the random value and the success rate
            // If an operation was not successful try the next one
            if random_action <= 7 && self.automate_spy_steal_tech(spy) {
                continue;
            } else if random_action <= 9 && self.automate_spy_rig_election(spy) {
                continue;
            } else if self.automate_spy_counter_intelligence(spy) {
                continue;
            } else if spy.is_doing_work() {
                continue; // We might have been doing counter intelligence and wanted to look for something better
            } else {
                // Retry all of the operations one more time
                if self.automate_spy_steal_tech(spy) { continue; }
                if self.automate_spy_rig_election(spy) { continue; }
                if self.automate_spy_counter_intelligence(spy) { continue; }
            }

            // There is nothing for our spy to do, put it in a random city
            let random_city = self.civ_info.game_info.get_cities()
                .iter()
                .filter(|city| spy.can_move_to(city))
                .collect::<Vec<_>>()
                .choose(&mut rng);

            if let Some(city) = random_city {
                spy.move_to(city);
            }
        }

        for spy in spies {
            self.check_if_should_stage_coup(spy);
        }
    }

    /// Moves the spy to a city that we can steal a tech from
    pub fn automate_spy_steal_tech(&self, spy: &Spy) -> bool {
        if self.civs_to_steal_from.is_empty() {
            return false;
        }

        // We want to move the spy to the city with the highest science generation
        // Players can't usually figure this out so lets do highest population instead
        let city_to_move_to = self.get_civs_to_steal_from_sorted.first()
            .and_then(|civ| {
                civ.cities.iter()
                    .filter(|city| spy.can_move_to(city))
                    .max_by(|city1, city2| {
                        city1.population.population.cmp(&city2.population.population)
                    })
            });

        if let Some(city) = city_to_move_to {
            spy.move_to(city);
            return true;
        }

        false
    }

    /// Moves the spy to a random city-state
    fn automate_spy_rig_election(&self, spy: &Spy) -> bool {
        let city_to_move_to = self.city_states_to_rig.iter()
            .flat_map(|civ| civ.cities.iter())
            .filter(|city| {
                !city.is_being_razed &&
                spy.can_move_to(city) &&
                (city.civ.get_diplomacy_manager(self.civ_info)
                    .map_or(false, |dm| dm.get_influence() < 150) ||
                 city.civ.get_ally_civ() != self.civ_info.civ_name)
            })
            .max_by(|city1, city2| {
                let influence1 = city1.civ.get_diplomacy_manager(self.civ_info)
                    .map_or(0, |dm| dm.get_influence());
                let influence2 = city2.civ.get_diplomacy_manager(self.civ_info)
                    .map_or(0, |dm| dm.get_influence());
                influence1.cmp(&influence2)
            });

        if let Some(city) = city_to_move_to {
            spy.move_to(city);
            return true;
        }

        false
    }

    /// Moves the spy to a random city of ours
    fn automate_spy_counter_intelligence(&self, spy: &Spy) -> bool {
        let cities: Vec<_> = self.civ_info.cities.iter()
            .filter(|city| spy.can_move_to(city))
            .collect();

        if let Some(city) = cities.choose(&mut rand::thread_rng()) {
            spy.move_to(city);
        }

        spy.action == SpyAction::CounterIntelligence
    }

    /// Checks if a spy should stage a coup
    fn check_if_should_stage_coup(&self, spy: &Spy) {
        if !spy.can_do_coup() {
            return;
        }

        if spy.get_coup_chance_of_success(false) < 0.7 {
            return;
        }

        let ally_civ = spy.get_city()
            .map(|city| city.civ.get_ally_civ())
            .flatten()
            .and_then(|ally_name| self.civ_info.game_info.get_civilization(ally_name));

        // Don't coup city-states whose allies are our friends
        if let Some(ally) = ally_civ {
            if self.civ_info.get_diplomacy_manager(ally)
                .map_or(false, |dm| dm.is_relationship_level_ge(RelationshipLevel::Friend))
            {
                return;
            }
        }

        let spies = &self.civ_info.espionage_manager.spy_list;
        let random_seed = spies.len() + spies.iter().position(|s| s == spy).unwrap_or(0) + self.civ_info.game_info.turns;
        let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed as u64);
        let random_action = rng.gen_range(0..100);

        if random_action < 20 {
            spy.set_action(SpyAction::Coup, 1);
        }
    }
}