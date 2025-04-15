use crate::civilization::Civilization;
use crate::city::City;
use crate::models::spy::Spy;
use crate::models::ruleset::unique::UniqueType;
use crate::map::tile::Tile;
use std::collections::{HashSet, LinkedHashSet};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use rand::seq::SliceRandom;

/// Manages espionage activities for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct EspionageManager {
    /// List of spies belonging to this civilization
    pub spy_list: Vec<Spy>,

    /// Eras for which spies have been earned
    pub eras_spy_earned_for: LinkedHashSet<String>,

    /// Reference to the civilization this manager belongs to
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// Whether the player has dismissed the "should move spies" notification
    #[serde(skip)]
    pub dismissed_should_move_spies: bool,
}

impl EspionageManager {
    /// Creates a new EspionageManager
    pub fn new() -> Self {
        Self {
            spy_list: Vec::new(),
            eras_spy_earned_for: LinkedHashSet::new(),
            civ_info: None,
            dismissed_should_move_spies: false,
        }
    }

    /// Sets the transient references to the civilization
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info.clone());
        for spy in &mut self.spy_list {
            spy.set_transients(civ_info.clone());
        }
    }

    /// Processes end-of-turn actions for all spies
    pub fn end_turn(&mut self) {
        for spy in self.spy_list.iter_mut() {
            spy.end_turn();
        }
    }

    /// Generates a unique name for a new spy
    pub fn get_spy_name(&self) -> String {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let used_spy_names: HashSet<String> = self.spy_list.iter()
            .map(|s| s.name.clone())
            .collect();

        let valid_spy_names: Vec<String> = civ_info.nation.spy_names.iter()
            .filter(|name| !used_spy_names.contains(*name))
            .cloned()
            .collect();

        if let Some(name) = valid_spy_names.choose(&mut rand::thread_rng()) {
            name.clone()
        } else {
            format!("Spy {}", self.spy_list.len() + 1) // +1 as non-programmers count from 1
        }
    }

    /// Adds a new spy to the civilization
    pub fn add_spy(&mut self) -> Spy {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let spy_name = self.get_spy_name();
        let new_spy = Spy::new(spy_name, self.get_starting_spy_rank());
        let mut spy = new_spy;
        spy.set_transients(civ_info.clone());
        self.spy_list.push(spy.clone());
        spy
    }

    /// Gets all tiles visible through spy operations
    pub fn get_tiles_visible_via_spies(&self) -> Vec<Tile> {
        self.spy_list.iter()
            .filter(|spy| spy.is_set_up())
            .filter_map(|spy| spy.get_city_or_null())
            .flat_map(|city| city.get_center_tile().get_tiles_in_distance(1))
            .collect()
    }

    /// Gets technologies that can be stolen from another civilization
    pub fn get_techs_to_steal(&self, other_civ: &Civilization) -> HashSet<String> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let mut techs_to_steal = HashSet::new();

        for tech in &other_civ.tech.techs_researched {
            if civ_info.tech.is_researched(tech) {
                continue;
            }
            if !civ_info.tech.can_be_researched(tech) {
                continue;
            }
            techs_to_steal.insert(tech.clone());
        }

        techs_to_steal
    }

    /// Gets all spies assigned to a specific city
    pub fn get_spies_in_city(&self, city: &City) -> Vec<Spy> {
        self.spy_list.iter()
            .filter(|spy| spy.get_city_or_null().map_or(false, |c| c == city))
            .cloned()
            .collect()
    }

    /// Gets the starting rank for a new spy
    pub fn get_starting_spy_rank(&self) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        1 + civ_info.get_matching_uniques(UniqueType::SpyStartingLevel)
            .iter()
            .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
            .sum::<i32>()
    }

    /// Gets all cities that have spies assigned to them
    pub fn get_cities_with_our_spies(&self) -> Vec<City> {
        self.spy_list.iter()
            .filter(|spy| spy.is_set_up())
            .filter_map(|spy| spy.get_city_or_null())
            .collect()
    }

    /// Gets the spy assigned to a specific city
    pub fn get_spy_assigned_to_city(&self, city: &City) -> Option<Spy> {
        self.spy_list.iter()
            .find(|spy| spy.get_city_or_null().map_or(false, |c| c == city))
            .cloned()
    }

    /// Determines whether the "Move Spies" action should be shown
    pub fn should_show_move_spies(&self) -> bool {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        !self.dismissed_should_move_spies &&
        self.spy_list.iter().any(|spy| spy.is_idle()) &&
        civ_info.game_info.get_cities().iter().any(|city|
            civ_info.has_explored(city.get_center_tile()) &&
            self.get_spy_assigned_to_city(city).is_none()
        )
    }

    /// Gets all idle spies
    pub fn get_idle_spies(&self) -> Vec<Spy> {
        self.spy_list.iter()
            .filter(|spy| spy.is_idle())
            .cloned()
            .collect()
    }

    /// Removes all spies from their assigned cities
    pub fn remove_all_spies(&mut self) {
        for spy in &mut self.spy_list {
            spy.move_to(None);
        }
    }
}