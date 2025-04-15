use crate::city::City;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::map::tile::Tile;
use crate::models::counter::Counter;
use crate::models::ruleset::unique::local_unique_cache::LocalUniqueCache;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::utils::to_percent;
use crate::automation::automation::Automation;
use std::sync::Arc;
use nalgebra::Vector2;
use std::f32::consts::E;

/// Manages city population, including growth, food consumption, and specialist allocation
pub struct CityPopulationManager {
    /// The city this manager belongs to
    pub city: Option<Arc<City>>,
    /// Current population count
    pub population: i32,
    /// Food stored for growth
    pub food_stored: i32,
    /// Specialist allocations by type
    pub specialist_allocations: Counter<String>,
}

impl CityPopulationManager {
    /// Creates a new CityPopulationManager
    pub fn new() -> Self {
        CityPopulationManager {
            city: None,
            population: 1,
            food_stored: 0,
            specialist_allocations: Counter::new(),
        }
    }

    /// Gets the current specialist allocations
    pub fn get_new_specialists(&self) -> &Counter<String> {
        &self.specialist_allocations
    }

    /// Gets the total number of specialists
    pub fn get_number_of_specialists(&self) -> i32 {
        self.get_new_specialists().values().sum()
    }

    /// Gets the number of free (unemployed) population
    pub fn get_free_population(&self) -> i32 {
        if let Some(city) = &self.city {
            let working_population = city.worked_tiles.len() as i32;
            self.population - working_population - self.get_number_of_specialists()
        } else {
            0
        }
    }

    /// Gets the food required to grow to the next population
    pub fn get_food_to_next_population(&self) -> i32 {
        if let Some(city) = &self.city {
            // civ v math, civilization.wikia
            let mut food_required = 15.0 + 6.0 * (self.population - 1) as f32 +
                ((self.population - 1) as f32).powf(1.8).floor();

            food_required *= city.civ.game_info.speed.modifier;

            if city.civ.is_city_state {
                food_required *= 1.5;
            }
            if !city.civ.is_human() {
                food_required *= city.civ.game_info.get_difficulty().ai_city_growth_modifier;
            }
            food_required as i32
        } else {
            0
        }
    }

    /// Gets the number of turns until starvation (None if not starving)
    pub fn get_num_turns_to_starvation(&self) -> Option<i32> {
        if let Some(city) = &self.city {
            if !city.is_starving() {
                return None;
            }
            Some(self.food_stored / -city.food_for_next_turn() + 1)
        } else {
            None
        }
    }

    /// Gets the number of turns until new population (None if not growing)
    pub fn get_num_turns_to_new_population(&self) -> Option<i32> {
        if let Some(city) = &self.city {
            if !city.is_growing() {
                return None;
            }
            let rounded_food_per_turn = city.food_for_next_turn() as f32;
            let remaining_food = self.get_food_to_next_population() - self.food_stored;
            let mut turns_to_growth = (remaining_food as f32 / rounded_food_per_turn).ceil() as i32;
            if turns_to_growth < 1 {
                turns_to_growth = 1;
            }
            Some(turns_to_growth)
        } else {
            None
        }
    }

    /// Gets the population filter amount for a given filter
    pub fn get_population_filter_amount(&self, filter: &str) -> i32 {
        match filter {
            "Specialists" => self.get_number_of_specialists(),
            "Population" => self.population,
            "Followers of the Majority Religion" | "Followers of this Religion" => {
                if let Some(city) = &self.city {
                    city.religion.get_followers_of_majority_religion()
                } else {
                    0
                }
            },
            "Unemployed" => self.get_free_population(),
            _ => self.specialist_allocations.get(filter).unwrap_or(0),
        }
    }

    /// Processes the next turn's food for population growth
    pub fn next_turn(&mut self, food: i32) {
        if let Some(city) = &self.city {
            self.food_stored += food;
            if food < 0 {
                city.civ.add_notification(
                    format!("[{}] is starving!", city.name),
                    city.location,
                    NotificationCategory::Cities,
                    NotificationIcon::Growth,
                    Some(NotificationIcon::Death)
                );
            }
            if self.food_stored < 0 {        // starvation!
                if self.population > 1 {
                    self.add_population(-1);
                }
                self.food_stored = 0;
            }
            let food_needed_to_grow = self.get_food_to_next_population();
            if self.food_stored < food_needed_to_grow {
                return;
            }

            // What if the stores are already over foodNeededToGrow but NullifiesGrowth is in effect?
            // We could simply test food==0 - but this way NullifiesStat(food) will still allow growth:
            if city.get_matching_uniques(UniqueType::NullifiesGrowth).any() {
                return;
            }

            // Hard block growth when using Avoid Growth, cap stored food
            if city.avoid_growth {
                self.food_stored = food_needed_to_grow;
                return;
            }

            // growth!
            self.food_stored -= food_needed_to_grow;
            let percent_of_food_carried_over = city.get_matching_uniques(UniqueType::CarryOverFood)
                .filter(|unique| city.matches_filter(&unique.params[1]))
                .map(|unique| unique.params[0].parse::<i32>().unwrap_or(0))
                .sum::<i32>()
                .min(95);  // Try to avoid runaway food gain in mods, just in case
            self.food_stored += (food_needed_to_grow as f32 * percent_of_food_carried_over as f32 / 100.0) as i32;
            self.add_population(1);
            city.should_reassign_population = true;
            city.civ.add_notification(
                format!("[{}] has grown!", city.name),
                city.location,
                NotificationCategory::Cities,
                NotificationIcon::Growth,
                None
            );
        }
    }

    /// Adds or removes population
    pub fn add_population(&mut self, count: i32) {
        if let Some(city) = &self.city {
            let changed_amount = count.max(1 - self.population);
            self.population += changed_amount;
            let free_population = self.get_free_population();
            if free_population < 0 {
                self.unassign_extra_population();
                city.city_stats.update();
            } else {
                self.auto_assign_population();
            }

            if city.civ.game_info.is_religion_enabled() {
                city.religion.update_pressure_on_population_change(changed_amount);
            }
        }
    }

    /// Sets the population to a specific value
    pub fn set_population(&mut self, count: i32) {
        self.add_population(-self.population + count);
    }

    /// Automatically assigns free population to tiles and specialists
    pub fn auto_assign_population(&mut self) {
        if let Some(city) = &self.city {
            city.city_stats.update();  // calculate current stats with current assignments
            let free_population = self.get_free_population();
            if free_population <= 0 {
                return;
            }

            let city_stats = &city.city_stats.current_city_stats;
            city.current_gpp_bonus = city.get_great_person_percentage_bonus();  // pre-calculate for use in Automation.rankSpecialist
            let mut specialist_food_bonus = 2.0;  // See CityStats.calcFoodEaten()
            for unique in city.get_matching_uniques(UniqueType::FoodConsumptionBySpecialists) {
                if city.matches_filter(&unique.params[1]) {
                    specialist_food_bonus *= to_percent(&unique.params[0]);
                }
            }
            specialist_food_bonus = 2.0 - specialist_food_bonus;

            let tiles_to_evaluate: Vec<_> = city.get_workable_tiles()
                .into_iter()
                .filter(|tile| !tile.is_blockaded())
                .collect();

            let local_unique_cache = LocalUniqueCache::new();
            // Calculate stats once - but the *ranking of those stats* is dynamic and depends on what the city needs
            let tile_stats: HashMap<_, _> = tiles_to_evaluate.iter()
                .filter(|tile| !tile.provides_yield())
                .map(|tile| (tile, tile.stats.get_tile_stats(city, &city.civ, &local_unique_cache)))
                .collect();

            let max_specialists = self.get_max_specialists();

            for _ in 0..free_population {
                // evaluate tiles
                let best_tile_and_rank = tiles_to_evaluate.iter()
                    .filter(|tile| !tile.provides_yield()) // Changes with every tile assigned
                    .map(|tile| (tile, Automation::rank_stats_for_city_work(&tile_stats[tile], city, false, &local_unique_cache)))
                    .max_by(|a, b| {
                        // We need to make sure that we work the same tiles as last turn on a tile
                        // so that our workers know to prioritize this tile and don't move to the other tile
                        // This was just the easiest way I could think of.
                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.0.longitude.cmp(&b.0.longitude))
                            .then_with(|| a.0.latitude.cmp(&b.0.latitude))
                    });

                let best_tile = best_tile_and_rank.map(|(tile, _)| tile);
                let value_best_tile = best_tile_and_rank.map(|(_, value)| value).unwrap_or(0.0);

                // evaluate specialists
                let best_job_and_rank = if city.manual_specialists {
                    None
                } else {
                    max_specialists.iter()
                        .filter(|(specialist_name, max_amount)| self.specialist_allocations.get(specialist_name).unwrap_or(0) < *max_amount)
                        .map(|(specialist_name, _)| (specialist_name, Automation::rank_specialist(specialist_name, city, &local_unique_cache)))
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                };

                let best_job = best_job_and_rank.map(|(job, _)| job);
                let value_best_specialist = best_job_and_rank.map(|(_, value)| value).unwrap_or(0.0);

                // assign population
                if value_best_tile > value_best_specialist {
                    if let Some(best_tile) = best_tile {
                        if !city.worked_tiles.contains(&best_tile.position) {
                            city.worked_tiles.push(best_tile.position);
                            city_stats.food += tile_stats[best_tile].food;
                        }
                    }
                } else if let Some(best_job) = best_job {
                    self.specialist_allocations.add(best_job, 1);
                    city_stats.food += specialist_food_bonus;
                }
            }
            city.city_stats.update();
        }
    }

    /// Stops working a tile
    pub fn stop_working_tile(&mut self, position: &Vector2<i32>) {
        if let Some(city) = &self.city {
            city.worked_tiles.retain(|pos| pos != position);
            city.locked_tiles.remove(position);
        }
    }

    /// Unassigns extra population when there's not enough work
    pub fn unassign_extra_population(&mut self) {
        if let Some(city) = &self.city {
            for tile in city.worked_tiles.iter()
                .filter_map(|pos| city.tile_map.get(pos))
                .cloned()
                .collect::<Vec<_>>() {
                if tile.get_owner().map_or(true, |owner| owner != city.civ) ||
                   tile.get_working_city().map_or(true, |working_city| working_city != city) ||
                   tile.aerial_distance_to(&city.get_center_tile()) > city.get_work_range() {
                    self.stop_working_tile(&tile.position);
                }
            }

            // unassign specialists that cannot be (e.g. the city was captured and one of the specialist buildings was destroyed)
            for (specialist_name, max_amount) in self.get_max_specialists().iter() {
                if self.specialist_allocations.get(specialist_name).unwrap_or(0) > *max_amount {
                    self.specialist_allocations.set(specialist_name, *max_amount);
                }
            }

            let local_unique_cache = LocalUniqueCache::new();

            while self.get_free_population() < 0 {
                // evaluate tiles
                let worst_worked_tile: Option<&Tile> = if city.worked_tiles.is_empty() {
                    None
                } else {
                    city.worked_tiles.iter()
                        .filter_map(|pos| city.tile_map.get(pos))
                        .min_by_key(|tile| {
                            Automation::rank_tile_for_city_work(tile, city, &local_unique_cache) +
                            if tile.is_locked() { 10.0 } else { 0.0 }
                        })
                };

                let value_worst_tile = worst_worked_tile
                    .map(|tile| Automation::rank_tile_for_city_work(tile, city, &local_unique_cache))
                    .unwrap_or(0.0);

                // evaluate specialists
                let worst_auto_job: Option<&str> = if city.manual_specialists {
                    None
                } else {
                    self.specialist_allocations.keys()
                        .min_by_key(|specialist| Automation::rank_specialist(specialist, city, &local_unique_cache))
                };

                let value_worst_specialist = worst_auto_job
                    .map(|job| Automation::rank_specialist(job, city, &local_unique_cache))
                    .unwrap_or(0.0);

                // un-assign population
                match (worst_auto_job, worst_worked_tile) {
                    (Some(worst_auto_job), Some(worst_worked_tile)) => {
                        // choose between removing a specialist and removing a tile
                        if value_worst_tile < value_worst_specialist {
                            self.stop_working_tile(&worst_worked_tile.position);
                        } else {
                            self.specialist_allocations.add(worst_auto_job, -1);
                        }
                    },
                    (Some(worst_auto_job), None) => {
                        self.specialist_allocations.add(worst_auto_job, -1);
                    },
                    (None, Some(worst_worked_tile)) => {
                        self.stop_working_tile(&worst_worked_tile.position);
                    },
                    (None, None) => {
                        // It happens when "city.manualSpecialists == true"
                        //  and population goes below the number of specialists, e.g. city is razing.
                        // Let's give a chance to do the work automatically at least.
                        if let Some(worst_job) = self.specialist_allocations.keys()
                            .min_by_key(|specialist| Automation::rank_specialist(specialist, city, &local_unique_cache)) {
                            self.specialist_allocations.add(worst_job, -1);
                        } else {
                            break; // sorry, we can do nothing about that
                        }
                    }
                }
            }
        }
    }

    /// Gets the maximum number of specialists by type
    pub fn get_max_specialists(&self) -> Counter<String> {
        if let Some(city) = &self.city {
            let mut counter = Counter::new();
            for building in city.city_constructions.get_built_buildings() {
                counter.add(building.new_specialists());
            }
            counter
        } else {
            Counter::new()
        }
    }

    /// Sets transient references
    pub fn set_transients(&mut self, city: Arc<City>) {
        self.city = Some(city);
    }
}

impl Clone for CityPopulationManager {
    fn clone(&self) -> Self {
        CityPopulationManager {
            city: None, // Transient field, will be set later
            population: self.population,
            food_stored: self.food_stored,
            specialist_allocations: self.specialist_allocations.clone(),
        }
    }
}