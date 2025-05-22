use crate::city::City;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::constants::Constants;
use crate::models::counter::Counter;
use crate::models::religion::Religion;
use crate::models::ruleset::unique::unique::Unique;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::utils::to_percent;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

/// Manages religion-related functionality for cities, including religious pressure, followers, and spread
pub struct CityReligionManager {
    /// The city this manager belongs to
    pub city: Option<Arc<City>>,
    /// Religions that have been adopted at some point by this city
    religions_at_some_point_adopted: HashSet<String>,
    /// Religious pressures by religion name
    pressures: Counter<String>,
    /// Cached followers by religion name
    followers: Counter<String>,
    /// Religion this city is the holy city of
    pub religion_this_is_the_holy_city_of: Option<String>,
    /// Whether this city is blocked from being a holy city
    pub is_blocked_holy_city: bool,
}

impl CityReligionManager {
    /// Creates a new CityReligionManager
    pub fn new() -> Self {
        let mut manager = CityReligionManager {
            city: None,
            religions_at_some_point_adopted: HashSet::new(),
            pressures: Counter::new(),
            followers: Counter::new(),
            religion_this_is_the_holy_city_of: None,
            is_blocked_holy_city: false,
        };
        manager.clear_all_pressures();
        manager
    }

    /// Sets transient references
    pub fn set_transients(&mut self, city: Arc<City>) {
        self.city = Some(city);
        // We don't need to check for changes in the majority religion, and as this
        // loads in the religion, _of course_ the religion changes, but it shouldn't
        // have any effect
        self.update_number_of_followers(false);
    }

    /// Processes the end of turn for religion
    pub fn end_turn(&mut self) {
        self.get_affected_by_surrounding_cities();
    }

    /// Gets uniques of a specific type from the majority religion
    pub fn get_uniques(&self, unique_type: UniqueType) -> Vec<&Unique> {
        if let Some(majority_religion) = self.get_majority_religion() {
            majority_religion.follower_belief_unique_map.get_uniques(unique_type)
        } else {
            Vec::new()
        }
    }

    /// Gets a clone of the pressures
    pub fn get_pressures(&self) -> Counter<String> {
        self.pressures.clone()
    }

    /// Clears all pressures and adds a default pressure for no religion
    fn clear_all_pressures(&mut self) {
        self.pressures.clear();
        // We add pressure for following no religion
        // Basically used as a failsafe so that there is always some religion,
        // and we don't suddenly divide by 0 somewhere
        // Should be removed when updating the followers so it never becomes the majority religion,
        // `null` is used for that instead.
        self.pressures.add(Constants::no_religion_name(), 100);
    }

    /// Adds pressure for a religion
    pub fn add_pressure(&mut self, religion_name: &str, amount: i32, should_update_followers: bool) {
        if let Some(city) = &self.city {
            if !city.civ.game_info.is_religion_enabled() {
                return; // No religion, no pressures
            }
            self.pressures.add(religion_name, amount);

            if should_update_followers {
                self.update_number_of_followers(true);
            }
        }
    }

    /// Removes all pressures except for a specific religion
    pub fn remove_all_pressures_except_for(&mut self, religion: &str) {
        let pressure_from_this_religion = self.pressures.get(religion).unwrap_or(0);
        // Atheism is never removed
        let pressure_from_atheism = self.pressures.get(Constants::no_religion_name()).unwrap_or(0);
        self.clear_all_pressures();
        self.pressures.add(religion, pressure_from_this_religion);
        if pressure_from_atheism != 0 {
            self.pressures.set(Constants::no_religion_name(), pressure_from_atheism);
        }
        self.update_number_of_followers(true);
    }

    /// Updates pressure on population change
    pub fn update_pressure_on_population_change(&mut self, population_change_amount: i32) {
        let majority_religion = if let Some(majority_religion_name) = self.get_majority_religion_name() {
            majority_religion_name
        } else {
            Constants::no_religion_name()
        };

        if population_change_amount > 0 {
            self.add_pressure(&majority_religion, 100 * population_change_amount, true);
        } else {
            self.update_number_of_followers(true);
        }
    }

    /// Triggers religion adoption
    fn trigger_religion_adoption(&mut self, new_majority_religion: &str) {
        if let Some(city) = &self.city {
            let new_majority_religion_object = &city.civ.game_info.religions[new_majority_religion];
            city.civ.add_notification(
                format!("Your city [{}] was converted to [{}]!",
                    city.name,
                    new_majority_religion_object.get_religion_display_name()),
                city.location,
                NotificationCategory::Religion,
                NotificationIcon::Faith,
                None
            );

            if self.religions_at_some_point_adopted.contains(new_majority_religion) {
                return;
            }

            let religion_owning_civ = new_majority_religion_object.get_founder();
            if religion_owning_civ.has_unique(UniqueType::StatsWhenAdoptingReligion) {
                let stats_granted = religion_owning_civ.get_matching_uniques(UniqueType::StatsWhenAdoptingReligion)
                    .iter()
                    .map(|unique| {
                        let mut stats = unique.stats.clone();
                        if !unique.is_modified_by_game_speed() {
                            stats.multiply(1.0);
                        } else {
                            stats.multiply(city.civ.game_info.speed.modifier);
                        }
                        stats
                    })
                    .fold(HashMap::new(), |mut acc, stats| {
                        for (key, value) in stats {
                            *acc.entry(key).or_insert(0) += value;
                        }
                        acc
                    });

                for (key, value) in &stats_granted {
                    religion_owning_civ.add_stat(key, *value);
                }

                if religion_owning_civ.has_explored(&city.get_center_tile()) {
                    religion_owning_civ.add_notification(
                        format!("You gained [{:?}] as your religion was spread to [{}]",
                            stats_granted,
                            city.name),
                        city.location,
                        NotificationCategory::Religion,
                        NotificationIcon::Faith,
                        None
                    );
                } else {
                    religion_owning_civ.add_notification(
                        format!("You gained [{:?}] as your religion was spread to an unknown city",
                            stats_granted),
                        NotificationCategory::Religion,
                        NotificationIcon::Faith,
                        None
                    );
                }
            }
            self.religions_at_some_point_adopted.insert(new_majority_religion.to_string());
        }
    }

    /// Updates the number of followers
    fn update_number_of_followers(&mut self, check_for_religion_adoption: bool) {
        let old_majority_religion = if check_for_religion_adoption {
            self.get_majority_religion_name()
        } else {
            None
        };

        let previous_followers = self.followers.clone();
        self.followers.clear();

        if let Some(city) = &self.city {
            if city.population.population <= 0 {
                return;
            }

            let mut remainders = HashMap::new();
            let pressure_per_follower = self.pressures.values().sum::<i32>() as f32 / city.population.population as f32;

            // First give each religion an approximate share based on pressure
            for (religion, pressure) in &self.pressures {
                let followers_of_this_religion = (pressure as f32 / pressure_per_follower) as i32;
                self.followers.add(religion, followers_of_this_religion);
                remainders.insert(religion.clone(), pressure as f32 - followers_of_this_religion as f32 * pressure_per_follower);
            }

            let mut unallocated_population = city.population.population - self.followers.values().sum::<i32>();

            // Divide up the remaining population
            while unallocated_population > 0 {
                let largest_remainder = remainders.iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(k, _)| k.clone());

                if let Some(largest_remainder) = largest_remainder {
                    self.followers.add(&largest_remainder, 1);
                    remainders.insert(largest_remainder, 0.0);
                    unallocated_population -= 1;
                } else {
                    self.followers.add(Constants::no_religion_name(), unallocated_population);
                    break;
                }
            }

            self.followers.remove(Constants::no_religion_name());

            if check_for_religion_adoption {
                let new_majority_religion = self.get_majority_religion_name();
                if old_majority_religion != new_majority_religion && new_majority_religion.is_some() {
                    self.trigger_religion_adoption(new_majority_religion.unwrap());
                }
                if old_majority_religion != new_majority_religion {
                    city.civ.cache.update_civ_resources(); // follower uniques can provide resources
                }
                if self.followers != previous_followers {
                    city.city_stats.update();
                }
            }
        }
    }

    /// Gets the number of followers
    pub fn get_number_of_followers(&self) -> Counter<String> {
        self.followers.clone()
    }

    /// Gets the number of followers of a specific religion
    pub fn get_followers_of(&self, religion: &str) -> i32 {
        self.followers.get(religion).unwrap_or(0)
    }

    /// Gets the number of followers of the majority religion
    pub fn get_followers_of_majority_religion(&self) -> i32 {
        if let Some(majority_religion) = self.get_majority_religion_name() {
            self.followers.get(&majority_religion).unwrap_or(0)
        } else {
            0
        }
    }

    /// Gets the number of followers of our religion
    pub fn get_followers_of_our_religion(&self) -> i32 {
        if let Some(city) = &self.city {
            if let Some(our_religion) = &city.civ.religion_manager.religion {
                self.followers.get(&our_religion.name).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Gets the number of followers of religions other than the specified one
    pub fn get_followers_of_other_religions_than(&self, religion: &str) -> i32 {
        self.followers.iter()
            .filter(|(key, _)| key != religion)
            .map(|(_, value)| value)
            .sum()
    }

    /// Removes all pantheons except for the one founded by the current owner of the city
    /// Should be called whenever a city changes hands, e.g. conquering and trading
    pub fn remove_unknown_pantheons(&mut self) {
        if let Some(city) = &self.city {
            let pressure_keys: Vec<String> = self.pressures.keys().cloned().collect();
            for pressure in pressure_keys {
                if pressure == Constants::no_religion_name() {
                    continue;
                }
                if let Some(corresponding_religion) = city.civ.game_info.religions.get(&pressure) {
                    if corresponding_religion.is_pantheon() &&
                       corresponding_religion.founding_civ_name != city.civ.civ_name {
                        self.pressures.remove(&pressure);
                    }
                }
            }
            self.update_number_of_followers(true);
        }
    }

    /// Gets the name of the majority religion
    pub fn get_majority_religion_name(&self) -> Option<String> {
        if self.followers.is_empty() {
            return None;
        }

        let religion_with_max_pressure = self.followers.iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(key, _)| key.clone());

        if let Some(religion_with_max_pressure) = religion_with_max_pressure {
            if religion_with_max_pressure == Constants::no_religion_name() {
                None
            } else if let Some(city) = &self.city {
                if self.followers.get(&religion_with_max_pressure).unwrap_or(0) >= city.population.population / 2 {
                    Some(religion_with_max_pressure)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets the majority religion
    pub fn get_majority_religion(&self) -> Option<&Religion> {
        if let Some(city) = &self.city {
            if let Some(majority_religion_name) = self.get_majority_religion_name() {
                city.civ.game_info.religions.get(&majority_religion_name)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets affected by surrounding cities
    fn get_affected_by_surrounding_cities(&mut self) {
        if let Some(city) = &self.city {
            if !city.civ.game_info.is_religion_enabled() {
                return; // No religion, no spreading
            }
            // We don't update the amount of followers yet, as only the end result should matter
            // If multiple religions would become the majority religion due to pressure,
            // this will make it so we only receive a notification for the last one.
            // Also, doing it like this increases performance :D
            if city.is_holy_city() {
                if let Some(holy_city_religion) = &self.religion_this_is_the_holy_city_of {
                    self.add_pressure(holy_city_religion, 5 * self.pressure_from_adjacent_cities(), false);
                }
            }

            for other_city in city.civ.game_info.get_cities() {
                if other_city == city {
                    continue;
                }
                if let Some(majority_religion_of_city) = other_city.religion.get_majority_religion_name() {
                    if let Some(religion) = city.civ.game_info.religions.get(&majority_religion_of_city) {
                        if !religion.is_major_religion() {
                            continue;
                        }
                        if other_city.get_center_tile().aerial_distance_to(&city.get_center_tile()) >
                           other_city.religion.get_spread_range() {
                            continue;
                        }
                        self.add_pressure(&majority_religion_of_city,
                            other_city.religion.pressure_amount_to_adjacent_cities(city), false);
                    }
                }
            }

            self.update_number_of_followers(true);
        }
    }

    /// Gets the spread range
    fn get_spread_range(&self) -> i32 {
        if let Some(city) = &self.city {
            let mut spread_range = 10;

            for unique in city.get_matching_uniques(UniqueType::ReligionSpreadDistance) {
                spread_range += unique.params[0].parse::<i32>().unwrap_or(0);
            }

            if let Some(majority_religion) = self.get_majority_religion() {
                for unique in majority_religion.get_founder().get_matching_uniques(UniqueType::ReligionSpreadDistance) {
                    spread_range += unique.params[0].parse::<i32>().unwrap_or(0);
                }
            }

            spread_range
        } else {
            10
        }
    }

    /// Gets the pressure from adjacent cities
    fn pressure_from_adjacent_cities(&self) -> i32 {
        if let Some(city) = &self.city {
            city.civ.game_info.speed.religious_pressure_adjacent_city
        } else {
            0
        }
    }

    /// Gets the pressures from surrounding cities
    pub fn get_pressures_from_surrounding_cities(&self) -> Counter<String> {
        let mut added_pressure = Counter::new();

        if let Some(city) = &self.city {
            if city.is_holy_city() {
                if let Some(holy_city_religion) = &self.religion_this_is_the_holy_city_of {
                    added_pressure.add(holy_city_religion, 5 * self.pressure_from_adjacent_cities());
                }
            }

            let all_cities_within_range = city.civ.game_info.get_cities()
                .iter()
                .filter(|other_city| {
                    other_city != city &&
                    other_city.get_center_tile().aerial_distance_to(&city.get_center_tile()) <=
                    other_city.religion.get_spread_range()
                })
                .collect::<Vec<_>>();

            for other_city in all_cities_within_range {
                if let Some(majority_religion) = other_city.religion.get_majority_religion() {
                    if !majority_religion.is_major_religion() {
                        continue;
                    }
                    added_pressure.add(&majority_religion.name,
                        other_city.religion.pressure_amount_to_adjacent_cities(city));
                }
            }
        }

        added_pressure
    }

    /// Checks if the city is protected by an inquisitor
    pub fn is_protected_by_inquisitor(&self, from_religion: Option<&str>) -> bool {
        if let Some(city) = &self.city {
            for tile in city.get_center_tile().get_tiles_in_distance(1) {
                for unit in [tile.civilian_unit.as_ref(), tile.military_unit.as_ref()].iter().flatten() {
                    if let Some(unit_religion) = &unit.religion {
                        if from_religion.map_or(true, |r| unit_religion != r) &&
                           unit.has_unique(UniqueType::PreventSpreadingReligion) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Gets the pressure amount to adjacent cities
    fn pressure_amount_to_adjacent_cities(&self, pressured_city: &City) -> i32 {
        if let Some(city) = &self.city {
            let mut pressure = self.pressure_from_adjacent_cities() as f32;

            // Follower beliefs of this religion
            for unique in city.get_matching_uniques(UniqueType::NaturalReligionSpreadStrength) {
                if pressured_city.matches_filter(&unique.params[1]) {
                    pressure *= to_percent(&unique.params[0]);
                }
            }

            // Founder beliefs of this religion
            if let Some(majority_religion) = self.get_majority_religion() {
                for unique in majority_religion.get_founder().get_matching_uniques(UniqueType::NaturalReligionSpreadStrength) {
                    if pressured_city.matches_filter(&unique.params[1]) {
                        pressure *= to_percent(&unique.params[0]);
                    }
                }
            }

            pressure as i32
        } else {
            0
        }
    }

    /// Gets the pressure deficit between the majority religion and another religion
    pub fn get_pressure_deficit(&self, other_religion: Option<&str>) -> i32 {
        let majority_religion = self.get_majority_religion_name();
        let majority_pressure = majority_religion.as_ref()
            .and_then(|r| self.get_pressures().get(r))
            .unwrap_or(0);
        let other_pressure = other_religion
            .and_then(|r| self.get_pressures().get(r))
            .unwrap_or(0);

        majority_pressure - other_pressure
    }
}

impl Clone for CityReligionManager {
    fn clone(&self) -> Self {
        CityReligionManager {
            city: None, // Transient field, will be set later
            religions_at_some_point_adopted: self.religions_at_some_point_adopted.clone(),
            pressures: self.pressures.clone(),
            followers: self.followers.clone(),
            religion_this_is_the_holy_city_of: self.religion_this_is_the_holy_city_of.clone(),
            is_blocked_holy_city: self.is_blocked_holy_city,
        }
    }
}