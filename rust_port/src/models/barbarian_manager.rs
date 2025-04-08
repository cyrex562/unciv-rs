use std::collections::{HashMap, HashSet, VecDeque};
use rand::Rng;
use crate::models::game_info::{GameInfo, Position};
use crate::models::tile::Tile;
use crate::models::constants::Constants;
use crate::models::ruleset::{BaseUnit, UniqueType};
use crate::models::civilization::{Civilization, NotificationCategory, NotificationIcon};

/// Manages barbarian encampments and their spawning behavior.
pub struct BarbarianManager {
    pub encampments: VecDeque<Encampment>,
    game_info: Option<GameInfo>,
    tile_map: Option<HashMap<Position, Tile>>,
}

impl BarbarianManager {
    /// Creates a new BarbarianManager instance.
    pub fn new() -> Self {
        BarbarianManager {
            encampments: VecDeque::new(),
            game_info: None,
            tile_map: None,
        }
    }

    /// Sets the transient game info and tile map references.
    pub fn set_transients(&mut self, game_info: GameInfo) {
        self.game_info = Some(game_info.clone());
        self.tile_map = Some(game_info.tile_map.clone());

        // Add any preexisting camps as Encampment objects
        let existing_encampment_locations: HashSet<Position> = self.encampments
            .iter()
            .map(|e| e.position.clone())
            .collect();

        if let Some(tile_map) = &self.tile_map {
            for (position, tile) in tile_map {
                if tile.improvement.as_ref().map_or(false, |imp| imp == Constants::BARBARIAN_ENCAMPMENT)
                    && !existing_encampment_locations.contains(position) {
                    let mut encampment = Encampment::new(position.clone());
                    encampment.set_game_info(game_info.clone());
                    self.encampments.push_back(encampment);
                }
            }
        }

        for camp in &mut self.encampments {
            camp.set_game_info(game_info.clone());
        }
    }

    /// Updates all encampments, checking for destroyed camps and spawning new ones.
    pub fn update_encampments(&mut self) {
        // Check if camps were destroyed
        let encampments_to_check: Vec<_> = self.encampments.iter().cloned().collect();
        for encampment in encampments_to_check {
            if let Some(tile_map) = &self.tile_map {
                if let Some(tile) = tile_map.get(&encampment.position) {
                    if tile.improvement.as_ref().map_or(true, |imp| imp != Constants::BARBARIAN_ENCAMPMENT) {
                        if let Some(camp) = self.encampments.iter_mut().find(|e| e.position == encampment.position) {
                            camp.was_destroyed();
                        }
                    }
                }
            }

            // Check if the ghosts are ready to depart
            if let Some(camp) = self.encampments.iter().position(|e| e.position == encampment.position && e.destroyed && e.countdown == 0) {
                self.encampments.remove(camp);
            }
        }

        // Possibly place a new encampment
        self.place_barbarian_encampment(false);

        for encampment in &mut self.encampments {
            encampment.update();
        }
    }

    /// Called when an encampment was attacked, will speed up time to next spawn.
    pub fn camp_attacked(&mut self, position: Position) {
        if let Some(camp) = self.encampments.iter_mut().find(|e| e.position == position) {
            camp.was_attacked();
        }
    }

    /// Places a new barbarian encampment on the map.
    pub fn place_barbarian_encampment(&mut self, for_testing: bool) {
        let game_info = match &self.game_info {
            Some(info) => info,
            None => return,
        };

        let tile_map = match &self.tile_map {
            Some(map) => map,
            None => return,
        };

        // Before we do the expensive stuff, do a roll to see if we will place a camp at all
        if !for_testing && game_info.turns > 1 && !rand::thread_rng().gen_bool(0.5) {
            return;
        }

        // Barbarians will only spawn in places that no one can see
        let all_viewable_tiles: HashSet<Position> = game_info.civilizations
            .iter()
            .filter(|civ| !civ.is_barbarian && !civ.is_spectator())
            .flat_map(|civ| civ.viewable_tiles.iter().cloned())
            .collect();

        let fog_tiles: Vec<&Tile> = tile_map.values()
            .filter(|tile| tile.is_land && !all_viewable_tiles.contains(&tile.position))
            .collect();

        let fog_tiles_per_camp = (tile_map.len() as f32).powf(0.4) as i32; // Approximately

        // Check if we have more room
        let mut camps_to_add = (fog_tiles.len() as i32 / fog_tiles_per_camp) - self.encampments.iter().filter(|e| !e.destroyed).count() as i32;

        // First turn of the game add 1/3 of all possible camps
        if game_info.turns == 1 {
            camps_to_add /= 3;
            camps_to_add = std::cmp::max(camps_to_add, 1); // At least 1 on first turn
        } else if camps_to_add > 0 {
            camps_to_add = 1;
        }

        if camps_to_add <= 0 {
            return;
        }

        // Camps can't spawn within 7 tiles of each other or within 4 tiles of major civ capitals
        let too_close_to_capitals: HashSet<Position> = game_info.civilizations
            .iter()
            .filter(|civ| !civ.is_barbarian && !civ.is_spectator() && !civ.cities.is_empty() && !civ.is_city_state)
            .filter_map(|civ| civ.get_capital())
            .flat_map(|city| city.get_center_tile().get_tiles_in_distance(4))
            .map(|tile| tile.position)
            .collect();

        let too_close_to_camps: HashSet<Position> = self.encampments
            .iter()
            .flat_map(|camp| {
                if let Some(tile) = tile_map.get(&camp.position) {
                    let distance = if camp.destroyed { 4 } else { 7 };
                    tile.get_tiles_in_distance(distance).into_iter().map(|t| t.position).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            })
            .collect();

        let mut viable_tiles: Vec<&Tile> = fog_tiles
            .into_iter()
            .filter(|tile| {
                !tile.is_impassible() &&
                tile.resource.is_none() &&
                !tile.terrain_feature_objects.iter().any(|feature| feature.has_unique(UniqueType::RestrictedBuildableImprovements)) &&
                tile.neighbors.iter().any(|neighbor| neighbor.is_land) &&
                !too_close_to_capitals.contains(&tile.position) &&
                !too_close_to_camps.contains(&tile.position)
            })
            .collect();

        let mut added_camps = 0;
        let mut bias_coast = rand::thread_rng().gen_range(0..6) == 0;

        // Add the camps
        while added_camps < camps_to_add {
            if viable_tiles.is_empty() {
                break;
            }

            // If we're biasing for coast, get a coast tile if possible
            let tile_index = if bias_coast {
                let coastal_tiles: Vec<usize> = viable_tiles.iter()
                    .enumerate()
                    .filter(|(_, tile)| tile.is_coastal_tile())
                    .map(|(i, _)| i)
                    .collect();

                if !coastal_tiles.is_empty() {
                    coastal_tiles[rand::thread_rng().gen_range(0..coastal_tiles.len())]
                } else {
                    rand::thread_rng().gen_range(0..viable_tiles.len())
                }
            } else {
                rand::thread_rng().gen_range(0..viable_tiles.len())
            };

            let tile = viable_tiles[tile_index];
            let position = tile.position.clone();

            // Set the improvement on the tile
            if let Some(tile_map) = &mut self.tile_map {
                if let Some(tile) = tile_map.get_mut(&position) {
                    tile.improvement = Some(Constants::BARBARIAN_ENCAMPMENT.to_string());
                }
            }

            // Create and add the new encampment
            let mut new_camp = Encampment::new(position);
            new_camp.set_game_info(game_info.clone());
            self.encampments.push_back(new_camp);

            // Notify civilizations
            self.notify_civs_of_barbarian_encampment(tile);
            added_camps += 1;

            // Still more camps to add?
            if added_camps < camps_to_add {
                // Remove some newly non-viable tiles
                let tiles_to_remove: HashSet<Position> = tile.get_tiles_in_distance(7)
                    .into_iter()
                    .map(|t| t.position)
                    .collect();

                viable_tiles.retain(|t| !tiles_to_remove.contains(&t.position));

                // Reroll bias
                bias_coast = rand::thread_rng().gen_range(0..6) == 0;
            }
        }
    }

    /// Notifies civilizations that have adopted Honor policy about a new barbarian encampment.
    fn notify_civs_of_barbarian_encampment(&self, tile: &Tile) {
        if let Some(game_info) = &self.game_info {
            for civ in game_info.civilizations.iter() {
                if civ.has_unique(UniqueType::NotifiedOfBarbarianEncampments) && civ.has_explored(tile) {
                    civ.add_notification(
                        "A new barbarian encampment has spawned!".to_string(),
                        tile.position.clone(),
                        NotificationCategory::War,
                        NotificationIcon::War
                    );
                    civ.set_last_seen_improvement(tile.position.clone(), Constants::BARBARIAN_ENCAMPMENT.to_string());
                }
            }
        }
    }
}

/// Represents a barbarian encampment.
pub struct Encampment {
    pub position: Position,
    pub countdown: i32,
    pub spawned_units: i32,
    pub destroyed: bool, // destroyed encampments haunt the vicinity for 15 turns preventing new spawns
    game_info: Option<GameInfo>,
}

impl Encampment {
    /// Creates a new Encampment instance.
    pub fn new(position: Position) -> Self {
        Encampment {
            position,
            countdown: 0,
            spawned_units: -1,
            destroyed: false,
            game_info: None,
        }
    }

    /// Sets the game info reference.
    pub fn set_game_info(&mut self, game_info: GameInfo) {
        self.game_info = Some(game_info);
    }

    /// Updates the encampment state.
    pub fn update(&mut self) {
        if self.countdown > 0 {
            // Not yet
            self.countdown -= 1;
        } else if !self.destroyed && self.spawn_barbarian() {
            // Countdown at 0, try to spawn a barbarian
            // Successful
            self.spawned_units += 1;
            self.reset_countdown();
        }
    }

    /// Called when the encampment was attacked.
    pub fn was_attacked(&mut self) {
        if !self.destroyed {
            self.countdown /= 2;
        }
    }

    /// Called when the encampment was destroyed.
    pub fn was_destroyed(&mut self) {
        if !self.destroyed {
            self.countdown = 15;
            self.destroyed = true;
        }
    }

    /// Attempts to spawn a Barbarian from this encampment. Returns true if a unit was spawned.
    fn spawn_barbarian(&self) -> bool {
        let game_info = match &self.game_info {
            Some(info) => info,
            None => return false,
        };

        let tile = match game_info.tile_map.get(&self.position) {
            Some(t) => t,
            None => return false,
        };

        // Empty camp - spawn a defender
        if tile.military_unit.is_none() {
            return self.spawn_unit(false); // Try spawning a unit on this tile, return false if unsuccessful
        }

        // Don't spawn wandering barbs too early
        if game_info.turns < 10 {
            return false;
        }

        // Too many barbarians around already?
        let barbarian_civ = game_info.get_barbarian_civilization();
        if tile.get_tiles_in_distance(4).iter().filter(|t| t.military_unit.as_ref().map_or(false, |u| u.civ == barbarian_civ)).count() > 2 {
            return false;
        }

        let can_spawn_boats = game_info.turns > 30;
        let valid_tiles: Vec<&Tile> = tile.neighbors.iter()
            .filter(|t| {
                !t.is_impassible() &&
                !t.is_city_center() &&
                t.get_first_unit().is_none() &&
                !(t.is_water && !can_spawn_boats) &&
                !(t.terrain_has_unique(UniqueType::FreshWater) && t.is_water) // No Lakes
            })
            .collect();

        if valid_tiles.is_empty() {
            return false;
        }

        let random_tile = valid_tiles[rand::thread_rng().gen_range(0..valid_tiles.len())];
        self.spawn_unit(random_tile.is_water) // Attempt to spawn a barbarian on a valid tile
    }

    /// Attempts to spawn a barbarian on position, returns true if successful and false if unsuccessful.
    fn spawn_unit(&self, naval: bool) -> bool {
        let game_info = match &self.game_info {
            Some(info) => info,
            None => return false,
        };

        let unit_to_spawn = match self.choose_barbarian_unit(naval) {
            Some(unit) => unit,
            None => return false, // return false if we didn't find a unit
        };

        let barbarian_civ = game_info.get_barbarian_civilization();
        let spawned_unit = game_info.tile_map.place_unit_near_tile(&self.position, &unit_to_spawn, barbarian_civ);

        spawned_unit.is_some()
    }

    /// Chooses a barbarian unit to spawn based on available technologies.
    fn choose_barbarian_unit(&self, naval: bool) -> Option<BaseUnit> {
        let game_info = match &self.game_info {
            Some(info) => info,
            None => return None,
        };

        // Get all researched technologies from non-barbarian, non-defeated civilizations
        let all_researched_techs: HashSet<String> = game_info.ruleset.technologies.keys
            .iter()
            .filter(|tech| {
                game_info.civilizations
                    .iter()
                    .filter(|civ| !civ.is_barbarian && !civ.is_defeated())
                    .all(|civ| civ.tech.techs_researched.contains(*tech))
            })
            .cloned()
            .collect();

        let barbarian_civ = game_info.get_barbarian_civilization();
        barbarian_civ.tech.techs_researched = all_researched_techs;

        let unit_list: Vec<&BaseUnit> = game_info.ruleset.units.values
            .iter()
            .filter(|unit| {
                unit.is_military &&
                !unit.has_unique(UniqueType::CannotAttack) &&
                !unit.has_unique(UniqueType::CannotBeBarbarian) &&
                (if naval { unit.is_water_unit } else { unit.is_land_unit }) &&
                unit.is_buildable(&barbarian_civ)
            })
            .collect();

        if unit_list.is_empty() {
            return None; // No naval tech yet? Mad modders?
        }

        // Civ V weights its list by FAST_ATTACK or ATTACK_SEA AI types, we'll do it a bit differently
        // get_force_evaluation is already conveniently biased towards fast units and against ranged naval
        let weightings: Vec<f32> = unit_list.iter()
            .map(|unit| unit.get_force_evaluation() as f32)
            .collect();

        // Choose a unit based on the weightings
        let total_weight: f32 = weightings.iter().sum();
        let mut random_value = rand::thread_rng().gen_range(0.0..total_weight);

        for (i, weight) in weightings.iter().enumerate() {
            random_value -= weight;
            if random_value <= 0.0 {
                return Some(unit_list[i].clone());
            }
        }

        // Fallback to the last unit if something went wrong
        unit_list.last().cloned()
    }

    /// When a barbarian is spawned, seed the counter for next spawn.
    fn reset_countdown(&mut self) {
        let game_info = match &self.game_info {
            Some(info) => info,
            None => return,
        };

        // Base 8-12 turns
        self.countdown = 8 + rand::thread_rng().gen_range(0..5);

        // Quicker on Raging Barbarians
        if game_info.game_parameters.raging_barbarians {
            self.countdown /= 2;
        }

        // Higher on low difficulties
        self.countdown += game_info.ruleset.difficulties[game_info.game_parameters.difficulty].barbarian_spawn_delay;

        // Quicker if this camp has already spawned units
        self.countdown -= std::cmp::min(3, self.spawned_units);

        self.countdown = (self.countdown as f32 * game_info.speed.barbarian_modifier) as i32;
    }
}