use crate::automation::automation::Automation;
use crate::city::City;
use crate::civilization::location_action::LocationAction;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::map::tile::Tile;
use crate::models::ruleset::unique::local_unique_cache::LocalUniqueCache;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::utils::to_percent;
use nalgebra::Vector2;
use std::f32::consts::E;
use std::sync::Arc;

/// Manages city expansion mechanics, including culture-based border growth and tile purchasing
pub struct CityExpansionManager {
    /// The city this manager belongs to
    pub city: Option<Arc<City>>,
    /// Culture stored for border expansion
    pub culture_stored: i32,
}

impl CityExpansionManager {
    /// Creates a new CityExpansionManager
    pub fn new() -> Self {
        CityExpansionManager {
            city: None,
            culture_stored: 0,
        }
    }

    /// Gets the number of tiles claimed by the city
    pub fn tiles_claimed(&self) -> i32 {
        if let Some(city) = &self.city {
            let tiles_around_city: Vec<_> = city
                .get_center_tile()
                .neighbors
                .iter()
                .map(|tile| tile.position)
                .collect();

            city.tiles
                .iter()
                .filter(|tile| *tile != city.location && !tiles_around_city.contains(&tile))
                .count() as i32
        } else {
            0
        }
    }

    /// Gets the culture required to expand to the next tile
    pub fn get_culture_to_next_tile(&self) -> i32 {
        if let Some(city) = &self.city {
            let mut culture_to_next_tile =
                6.0 * (0.0f32.max(self.tiles_claimed() as f32) + 1.4813).powf(1.3);

            culture_to_next_tile *= city.civ.game_info.speed.culture_cost_modifier;

            if city.civ.is_city_state {
                culture_to_next_tile *= 1.5; // City states grow slower, perhaps 150% cost?
            }

            for unique in city.get_matching_uniques(UniqueType::BorderGrowthPercentage) {
                if city.matches_filter(&unique.params[1]) {
                    culture_to_next_tile *= to_percent(&unique.params[0]);
                }
            }

            culture_to_next_tile.round() as i32
        } else {
            0
        }
    }

    /// Checks if a tile can be bought
    pub fn can_buy_tile(&self, tile: &Tile) -> bool {
        if let Some(city) = &self.city {
            if city.is_puppet || city.is_being_razed {
                return false;
            }
            if tile.get_owner().is_some() {
                return false;
            }
            if city.is_in_resistance() {
                return false;
            }
            if !city.tiles_in_range.contains(&tile.position) {
                return false;
            }
            tile.neighbors
                .iter()
                .any(|neighbor| neighbor.get_city().map_or(false, |c| c == city))
        } else {
            false
        }
    }

    /// Buys a tile
    pub fn buy_tile(&mut self, tile: &Tile) -> Result<(), String> {
        if let Some(city) = &self.city {
            let gold_cost = self.get_gold_cost_of_tile(tile);

            // Check if the tile is contiguous with the city
            if !tile
                .neighbors
                .iter()
                .any(|neighbor| neighbor.get_city().map_or(false, |c| c == city))
            {
                return Err(format!(
                    "{} tried to buy {}, but it owns none of the neighbors",
                    city.name, tile.position
                ));
            }

            // Check if the city has enough gold
            if city.civ.gold < gold_cost && !city.civ.game_info.game_parameters.god_mode {
                return Err(format!(
                    "{} tried to buy {}, but lacks gold (cost {}, has {})",
                    city.name, tile.position, gold_cost, city.civ.gold
                ));
            }

            city.civ.add_gold(-gold_cost);
            self.take_ownership(tile);

            // Reapply worked tiles optimization (aka CityFocus) - doing it here means AI profits too
            city.reassign_population_deferred();
            Ok(())
        } else {
            Err("City not set".to_string())
        }
    }

    /// Gets the gold cost of a tile
    pub fn get_gold_cost_of_tile(&self, tile: &Tile) -> i32 {
        if let Some(city) = &self.city {
            let base_cost = 50;
            let distance_from_center = tile.aerial_distance_to(&city.get_center_tile());
            let mut cost = base_cost as f32 * (distance_from_center - 1) as f32
                + self.tiles_claimed() as f32 * 5.0;

            cost *= city.civ.game_info.speed.gold_cost_modifier;

            for unique in city.get_matching_uniques(UniqueType::TileCostPercentage) {
                if city.matches_filter(&unique.params[1]) {
                    cost *= to_percent(&unique.params[0]);
                }
            }

            cost.round() as i32
        } else {
            0
        }
    }

    /// Gets tiles that can be chosen for expansion
    pub fn get_choosable_tiles(&self) -> Vec<Arc<Tile>> {
        if let Some(city) = &self.city {
            city.get_center_tile()
                .get_tiles_in_distance(city.get_expand_range())
                .into_iter()
                .filter(|tile| tile.get_owner().is_none())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Chooses a new tile to own based on automation ranking
    pub fn choose_new_tile_to_own(&self) -> Option<Arc<Tile>> {
        if let Some(city) = &self.city {
            // Technically, in the original a random tile with the lowest score was selected
            // However, doing this requires either caching it, which is way more work,
            // or selecting all possible tiles and only choosing one when the border expands.
            // But since the order in which tiles are selected in distance is kinda random anyways,
            // this is fine.
            let local_unique_cache = LocalUniqueCache::new();
            self.get_choosable_tiles().into_iter().min_by_key(|tile| {
                Automation::rank_tile_for_expansion(tile, city, &local_unique_cache)
            })
        } else {
            None
        }
    }

    /// Resets the city's tiles to just the center tile and immediate neighbors
    pub fn reset(&mut self) {
        if let Some(city) = &self.city {
            for tile in city.get_tiles() {
                self.relinquish_ownership(&tile);
            }

            // The only way to create a city inside an owned tile is if it's in your territory
            // In this case, if you don't assign control of the central tile to the city,
            // It becomes an invisible city and weird shit starts happening
            self.take_ownership(&city.get_center_tile());

            for tile in city
                .get_center_tile()
                .get_tiles_in_distance(1)
                .into_iter()
                .filter(|tile| tile.get_city().is_none())
            {
                // can't take ownership of owned tiles (by other cities)
                self.take_ownership(&tile);
            }
        }
    }

    /// Adds a new tile with culture and returns its position if successful
    fn add_new_tile_with_culture(&mut self) -> Option<Vector2<i32>> {
        if let Some(city) = &self.city {
            if let Some(chosen_tile) = self.choose_new_tile_to_own() {
                let culture_cost = self.get_culture_to_next_tile();
                self.culture_stored -= culture_cost;
                self.take_ownership(&chosen_tile);
                Some(chosen_tile.position)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Relinquishes ownership of a tile
    pub fn relinquish_ownership(&mut self, tile: &Tile) {
        if let Some(city) = &self.city {
            city.tiles.retain(|pos| pos != &tile.position);

            for city in city.civ.cities.iter() {
                if city.is_worked(tile) {
                    city.population.stop_working_tile(&tile.position);
                    city.population.auto_assign_population();
                }
            }

            tile.improvement_functions
                .remove_creates_one_improvement_marker();

            tile.set_owning_city(None);

            city.civ.cache.update_our_tiles();
            city.city_stats.update();

            tile.history.record_relinquish_ownership(tile);
        }
    }

    /// Takes ownership of a tile
    pub fn take_ownership(&mut self, tile: &Tile) {
        if let Some(city) = &self.city {
            assert!(
                !tile.is_city_center(),
                "Trying to found a city in a tile that already has one"
            );

            if let Some(owning_city) = tile.get_city() {
                owning_city.expansion.relinquish_ownership(tile);
            }

            if !city.tiles.contains(&tile.position) {
                city.tiles.push(tile.position);
            }

            tile.set_owning_city(Some(city.clone()));
            city.population.auto_assign_population();
            city.civ.cache.update_our_tiles();
            city.city_stats.update();

            for unit in tile.get_units().iter().cloned().collect::<Vec<_>>() {
                // cloned because we're modifying
                if !unit
                    .civ
                    .diplomacy_functions
                    .can_pass_through_tiles(&city.civ)
                {
                    unit.movement.teleport_to_closest_moveable_tile();
                } else if unit.civ == city.civ && unit.is_sleeping() {
                    // If the unit is sleeping and is a worker, it might want to build on this tile
                    // So lets try to wake it up for the player to notice it
                    if unit.cache.has_unique_to_build_improvements
                        || unit.cache.has_unique_to_create_water_improvements
                    {
                        unit.due = true;
                        unit.action = None;
                    }
                }
            }

            tile.history.record_take_ownership(tile);
        }
    }

    /// Processes the next turn's culture for border expansion
    pub fn next_turn(&mut self, culture: f32) {
        if let Some(city) = &self.city {
            self.culture_stored += culture as i32;
            if self.culture_stored >= self.get_culture_to_next_tile() {
                if let Some(location) = self.add_new_tile_with_culture() {
                    let locations = LocationAction::new(location, city.location);
                    city.civ.add_notification(
                        format!("[{}] has expanded its borders!", city.name),
                        locations,
                        NotificationCategory::Cities,
                        NotificationIcon::Culture,
                    );
                }
            }
        }
    }

    /// Sets transient references
    pub fn set_transients(&mut self, city: Arc<City>) {
        self.city = Some(city.clone());

        let tiles = city.get_tiles();
        for tile in tiles {
            tile.set_owning_city(Some(city.clone()));
        }
    }
}

impl Clone for CityExpansionManager {
    fn clone(&self) -> Self {
        CityExpansionManager {
            city: None, // Transient field, will be set later
            culture_stored: self.culture_stored,
        }
    }
}
