use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use log::debug;
use uuid::Uuid;
use crate::barbarians::barbarian_manager::BarbarianManager;
use crate::city::city::City;
use crate::city_distance::CityDistanceData;
use crate::civilization::managers::turn_manager::TurnManager;
use crate::civilization::notification::{Notification, NotificationCategory};
use crate::civilization::notification_actions::MapUnitAction;
use crate::civilization::notification_icons::NotificationIcon;
use crate::game_info_preview::GameInfoPreview;
use crate::map_parameters::MapShape;
use crate::tile::tile::Tile;
use crate::progress_bar::ProgressBar;
use crate::unique::unique::LocalUniqueCache;
use crate::unique::UniqueType;

/// A trait for classes that are part of GameInfo serialization, i.e. save files.
///
/// Take care with lateinit and lazy fields - both are **never** serialized.
///
/// When you change the structure of any class with this trait in a way which makes it impossible
/// to load the new saves from an older game version, increment `CURRENT_COMPATIBILITY_NUMBER`! And don't forget
/// to add backwards compatibility for the previous format.
///
/// Reminder: In all subclasses, do use only actual Collection types, not abstractions like
/// `= mutableSetOf<Something>()`. That would make the reflection type of the field an interface, which
/// hides the actual implementation from Gdx Json, so it will not try to call a no-args constructor but
/// will instead deserialize a List in the jsonData.isArray() -> isAssignableFrom(Collection) branch of readValue.
pub trait IsPartOfGameInfoSerialization {}

/// A trait for classes that have a game info serialization version
pub trait HasGameInfoSerializationVersion {
    fn version(&self) -> &CompatibilityVersion;
    fn set_version(&mut self, version: CompatibilityVersion);
}

/// Contains the current serialization version of GameInfo, i.e. when this number is not equal to `CURRENT_COMPATIBILITY_NUMBER`, it means
/// this instance has been loaded from a save file json that was made with another version of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompatibilityVersion {
    /// The version number
    pub number: i32,
    /// The version of the game that created this save
    pub created_with: Version,
}

impl CompatibilityVersion {
    /// Create a new CompatibilityVersion
    pub fn new(number: i32, created_with: Version) -> Self {
        Self {
            number,
            created_with,
        }
    }

    /// Default constructor for serialization
    pub fn default() -> Self {
        Self {
            number: -1,
            created_with: Version::default(),
        }
    }

    /// Compare this version with another
    pub fn compare_to(&self, other: &CompatibilityVersion) -> std::cmp::Ordering {
        self.number.cmp(&other.number)
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::barbarians::barbarians::Barbarians;
use crate::tile::tile::Tile;
use crate::models::barbarians::Barbarians;
use crate::models::civilization::Civilization;
use crate::models::ruleset::Ruleset;
use crate::models::map::Map;
use crate::version::Version;

/// Contains game state information.
/// The virtual world the users play in.
pub struct GameInfo {
    /// The compatibility version of this game
    pub version: Version,
    
    /// The civilizations in the game
    pub civilizations: Vec<Civilization>,
    
    /// The barbarian manager
    pub barbarians: Barbarians,
    
    /// The tile map of the game
    pub tile_map: HashMap<Position, Tile>,
    
    /// The game map
    pub map: Map,
    
    /// The religions in the game
    pub religions: HashMap<String, Religion>,
    
    /// The difficulty level of the game
    pub difficulty: String,
    
    /// The game parameters
    pub game_parameters: GameParameters,
    
    /// The current turn number
    pub turns: i32,
    
    /// The speed of the game
    pub speed: GameSpeed,
    
    /// Whether the game is in one more turn mode
    pub one_more_turn_mode: bool,
    
    /// The name of the current player
    pub current_player: String,
    
    /// The time when the current turn started
    pub current_turn_start_time: i64,
    
    /// The unique ID of the game
    pub game_id: String,
    
    /// The checksum of the game
    pub checksum: String,
    
    /// The ID of the last unit created
    pub last_unit_id: i32,
    
    /// Data about the victory in the game
    pub victory_data: Option<VictoryData>,
    
    /// Maps a civ to the civ they voted for - None on the value side means they abstained
    pub diplomatic_victory_votes_cast: HashMap<String, Option<String>>,
    
    /// Set to false whenever the results still need to be processed
    pub diplomatic_victory_votes_processed: bool,
    
    /// The turn the replay history started recording.
    ///
    /// * `-1` means the game was serialized with an older version without replay
    /// * `0` would be the normal value in any newer game
    ///   (remember game_parameters.starting_era is not implemented through turns starting > 0)
    /// * `>0` would be set by compatibility migration, handled in `BackwardCompatibility::migrate_to_tile_history`
    pub history_start_turn: i32,
    
    /// Keep track of a custom location this game was saved to _or_ loaded from, using it as the default custom location for any further save/load attempts.
    pub custom_save_location: Option<String>,
    
    // Transient fields (not serialized)
    
    /// The difficulty object
    pub difficulty_object: Option<Difficulty>,
    
    /// The current player's civilization
    pub current_player_civ: Option<Civilization>,
    
    /// Whether the game is up to date
    pub is_up_to_date: bool,
    
    /// The ruleset of the game
    pub ruleset: Arc<Ruleset>,
    
    /// The maximum number of turns to simulate
    pub simulate_max_turns: i32,
    
    /// Whether to simulate until a player wins
    pub simulate_until_win: bool,
    
    /// The space resources
    pub space_resources: HashSet<String>,
    
    /// The city distances
    pub city_distances: CityDistanceData,
}

/// Represents a position on the map.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Game speed settings
#[derive(Clone, Debug)]
pub enum GameSpeed {
    Quick,
    Standard,
    Epic,
    Marathon,
    Custom(f32),
}

impl GameInfo {
    /// Creates a new GameInfo instance
    pub fn new() -> Self {
        Self {
            version: Version::default(),
            civilizations: Vec::new(),
            barbarians: Barbarians::new(),
            tile_map: HashMap::new(),
            map: Map::default(),
            religions: HashMap::new(),
            difficulty: "Prince".to_string(),
            game_parameters: GameParameters::default(),
            turns: 0,
            speed: GameSpeed::Standard,
            one_more_turn_mode: false,
            current_player: String::new(),
            current_turn_start_time: 0,
            game_id: String::new(),
            checksum: String::new(),
            last_unit_id: 0,
            victory_data: None,
            diplomatic_victory_votes_cast: HashMap::new(),
            diplomatic_victory_votes_processed: true,
            history_start_turn: 0,
            custom_save_location: None,
            difficulty_object: None,
            current_player_civ: None,
            is_up_to_date: true,
            ruleset: Arc::new(Ruleset::default()),
            simulate_max_turns: 0,
            simulate_until_win: false,
            space_resources: HashSet::new(),
            city_distances: CityDistanceData::default(),
        }
    }
    
    /// Gets a tile at the specified position.
    pub fn get_tile(&self, position: Position) -> Option<&Tile> {
        self.tile_map.get(&position)
    }
    
    /// Gets the player to view as
    pub fn get_player_to_view_as(&self) -> String {
        // Implementation would depend on game logic
        self.current_player.clone()
    }
}

// Placeholder struct definitions for compilation
#[derive(Default)]
pub struct Religion;

#[derive(Default)]
pub struct GameParameters;

#[derive(Default)]
pub struct VictoryData;

#[derive(Default)]
pub struct Difficulty;

#[derive(Default)]
pub struct CityDistanceData;

impl GameInfo {
    /// The current compatibility version of GameInfo
    pub const CURRENT_COMPATIBILITY_NUMBER: i32 = 4;

    /// The first version without compatibility version
    pub const FIRST_WITHOUT: CompatibilityVersion = CompatibilityVersion {
        number: 1,
        created_with: Version {
            version: "4.1.14",
            build: 731,
        },
    };

    /// Create a new GameInfo
    pub fn new() -> Self {
        Self {
            version: Self::FIRST_WITHOUT,
            civilizations: Vec::new(),
            barbarians: BarbarianManager::new(),
            religions: HashMap::new(),
            difficulty: "Chieftain".to_string(),
            tile_map: TileMap::new(),
            game_parameters: GameParameters::new(),
            turns: 0,
            one_more_turn_mode: false,
            current_player: String::new(),
            current_turn_start_time: 0,
            game_id: Uuid::new_v4().to_string(),
            checksum: String::new(),
            last_unit_id: 0,
            victory_data: None,
            diplomatic_victory_votes_cast: HashMap::new(),
            diplomatic_victory_votes_processed: false,
            history_start_turn: -1,
            custom_save_location: None,
            difficulty_object: None,
            speed: None,
            current_player_civ: None,
            is_up_to_date: false,
            ruleset: None,
            simulate_max_turns: 1000,
            simulate_until_win: false,
            space_resources: HashSet::new(),
            city_distances: CityDistanceData::new(),
        }
    }

    /// Clone this GameInfo
    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.tile_map = self.tile_map.clone();
        to_return.civilizations = self.civilizations.iter().map(|c| c.clone()).collect();
        to_return.barbarians = self.barbarians.clone();
        to_return.religions = self.religions.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        to_return.current_player = self.current_player.clone();
        to_return.current_turn_start_time = self.current_turn_start_time;
        to_return.turns = self.turns;
        to_return.difficulty = self.difficulty.clone();
        to_return.game_parameters = self.game_parameters.clone();
        to_return.game_id = self.game_id.clone();
        to_return.diplomatic_victory_votes_cast = self.diplomatic_victory_votes_cast.clone();
        to_return.one_more_turn_mode = self.one_more_turn_mode;
        to_return.custom_save_location = self.custom_save_location.clone();
        to_return.victory_data = self.victory_data.clone();
        to_return.history_start_turn = self.history_start_turn;
        to_return.last_unit_id = self.last_unit_id;

        to_return
    }

    /// Get the player to view as
    pub fn get_player_to_view_as(&self) -> &Civilization {
        if !self.game_parameters.is_online_multiplayer {
            return self.get_current_player_civilization();
        }

        let user_id = "current_user_id"; // This would come from settings in the actual implementation

        // Iterating on all civs, starting from the current player, gives us the one that will have the next turn
        // This allows multiple civs from the same UserID
        if self.civilizations.iter().any(|c| c.player_id == user_id) {
            let mut civ_index = self.civilizations.iter().position(|c| c.civ_name == self.current_player).unwrap_or(0);

            loop {
                let civ_to_check = &self.civilizations[civ_index % self.civilizations.len()];
                if civ_to_check.player_id == user_id {
                    return civ_to_check;
                }
                civ_index += 1;
            }
        } else {
            // you aren't anyone. How did you even get this game? Can you spectate?
            return self.get_spectator(user_id);
        }
    }

    /// Get a map of civilizations by name
    pub fn get_civ_map(&self) -> HashMap<String, &Civilization> {
        self.civilizations.iter().map(|c| (c.civ_name.clone(), c)).collect()
    }

    /// Get a civilization by name
    pub fn get_civilization(&self, civ_name: &str) -> &Civilization {
        self.get_civ_map().get(civ_name)
            .unwrap_or_else(|| self.civilizations.iter().find(|c| c.civ_name == civ_name)
                .expect("No civilization found with that name"))
    }

    /// Get the current player's civilization
    pub fn get_current_player_civilization(&self) -> &Civilization {
        self.current_player_civ.as_ref().expect("Current player civilization not set")
    }

    /// Get civilizations as previews
    pub fn get_civilizations_as_previews(&self) -> Vec<CivilizationInfoPreview> {
        self.civilizations.iter().map(|c| c.as_preview()).collect()
    }

    /// Get the barbarian civilization
    pub fn get_barbarian_civilization(&self) -> &Civilization {
        self.get_civilization(Constants::BARBARIANS)
    }

    /// Get the difficulty
    pub fn get_difficulty(&self) -> &Difficulty {
        self.difficulty_object.as_ref().expect("Difficulty not set")
    }

    /// Get all cities
    pub fn get_cities(&self) -> Vec<&City> {
        self.civilizations.iter().flat_map(|c| &c.cities).collect()
    }

    /// Get alive city states
    pub fn get_alive_city_states(&self) -> Vec<&Civilization> {
        self.civilizations.iter().filter(|c| c.is_alive() && c.is_city_state).collect()
    }

    /// Get alive major civilizations
    pub fn get_alive_major_civs(&self) -> Vec<&Civilization> {
        self.civilizations.iter().filter(|c| c.is_alive() && c.is_major_civ()).collect()
    }

    /// Get civilizations sorted
    pub fn get_civs_sorted(
        &self,
        include_city_states: bool,
        include_defeated: bool,
        civ_to_sort_first: Option<&Civilization>,
        additional_filter: Option<&dyn Fn(&Civilization) -> bool>
    ) -> Vec<&Civilization> {
        let mut civs: Vec<&Civilization> = self.civilizations.iter()
            .filter(|c| {
                !c.is_barbarian &&
                !c.is_spectator() &&
                (include_defeated || !c.is_defeated()) &&
                (include_city_states || !c.is_city_state) &&
                additional_filter.map_or(true, |f| f(c))
            })
            .collect();

        // Sort by major civ first, then by name
        civs.sort_by(|a, b| {
            if let Some(first) = civ_to_sort_first {
                if a.civ_name == first.civ_name {
                    return std::cmp::Ordering::Less;
                }
                if b.civ_name == first.civ_name {
                    return std::cmp::Ordering::Greater;
                }
            }

            match b.is_major_civ().cmp(&a.is_major_civ()) {
                std::cmp::Ordering::Equal => a.civ_name.cmp(&b.civ_name),
                other => other,
            }
        });

        civs
    }

    /// Get a spectator for a player ID
    pub fn get_spectator(&self, player_id: &str) -> &Civilization {
        self.civilizations.iter()
            .find(|c| c.is_spectator() && c.player_id == player_id)
            .unwrap_or_else(|| self.create_temporary_spectator_civ(player_id))
    }

    /// Create a temporary spectator civilization
    fn create_temporary_spectator_civ(&self, player_id: &str) -> &Civilization {
        // This would create a new spectator civilization and add it to the list
        // In Rust, we can't modify self in a method that returns a reference to self
        // So this is a simplified version that would need to be implemented differently
        unimplemented!("create_temporary_spectator_civ not implemented")
    }

    /// Check if religion is enabled
    pub fn is_religion_enabled(&self) -> bool {
        if let Some(ruleset) = &self.ruleset {
            if let Some(era) = ruleset.eras.get(&self.game_parameters.starting_era) {
                if era.has_unique(UniqueType::DisablesReligion) {
                    return false;
                }
            }

            if ruleset.mod_options.has_unique(UniqueType::DisableReligion) {
                return false;
            }
        }

        true
    }

    /// Check if espionage is enabled
    pub fn is_espionage_enabled(&self) -> bool {
        self.game_parameters.espionage_enabled
    }

    /// Get the equivalent turn
    fn get_equivalent_turn(&self) -> i32 {
        if let Some(speed) = &self.speed {
            if let Some(ruleset) = &self.ruleset {
                if let Some(era) = ruleset.eras.get(&self.game_parameters.starting_era) {
                    let total_turns = speed.num_total_turns();
                    let start_percent = era.start_percent;
                    return self.turns + (total_turns * start_percent / 100);
                }
            }
        }

        self.turns
    }

    /// Get the year for a turn offset
    pub fn get_year(&self, turn_offset: i32) -> i32 {
        let turn = self.get_equivalent_turn() + turn_offset;

        if let Some(speed) = &self.speed {
            let years_to_turn = &speed.years_per_turn;
            let mut year = speed.start_year;
            let mut i = 0;

            while i < turn {
                let years_per_turn = years_to_turn.iter()
                    .find(|yt| i < yt.until_turn)
                    .map(|yt| yt.year_interval)
                    .unwrap_or_else(|| years_to_turn.last().map(|yt| yt.year_interval).unwrap_or(1.0));

                year += years_per_turn as i32;
                i += 1;
            }

            year
        } else {
            0
        }
    }

    /// Calculate the checksum
    pub fn calculate_checksum(&mut self) -> String {
        let old_checksum = self.checksum.clone();
        self.checksum = String::new(); // Checksum calculation cannot include old checksum, obvs

        // In Rust, we would use a proper hashing library
        // This is a simplified version
        let checksum = format!("checksum_{}", self.game_id);

        self.checksum = old_checksum;
        checksum
    }

    /// Check if the game is in simulation mode
    pub fn is_simulation(&self) -> bool {
        self.turns < 1000 || (self.turns < self.simulate_max_turns && self.simulate_until_win)
    }

    /// Get the enabled victories
    pub fn get_enabled_victories(&self) -> HashMap<String, &VictoryType> {
        if let Some(ruleset) = &self.ruleset {
            ruleset.victories.iter()
                .filter(|(_, v)| !v.hidden_in_victory_screen && self.game_parameters.victory_types.contains(&v.name))
                .map(|(k, v)| (k.clone(), v))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Process diplomatic victory
    pub fn process_diplomatic_victory(&mut self) {
        if self.diplomatic_victory_votes_processed {
            return;
        }

        for civ in &mut self.civilizations {
            if civ.victory_manager.has_enough_votes_for_diplomatic_victory() {
                civ.victory_manager.has_ever_won_diplomatic_vote = true;
            }
        }

        self.diplomatic_victory_votes_processed = true;
    }

    /// Check for victory
    pub fn check_for_victory(&mut self) -> bool {
        if self.victory_data.is_some() {
            return true;
        }

        for civ in &mut self.civilizations {
            TurnManager::new(civ).update_winning_civ();
            if self.victory_data.is_some() {
                return true;
            }
        }

        false
    }

    /// Add a notification about enemy units
    fn add_enemy_unit_notification(&self, this_player: &mut Civilization, tiles: &[&Tile], in_or_near: &str) {
        // don't flood the player with similar messages. instead cycle through units by clicking the message multiple times.
        if tiles.len() < 3 {
            for tile in tiles {
                if let Some(unit) = &tile.military_unit {
                    let unit_name = unit.name.clone();
                    this_player.add_notification(
                        &format!("An enemy [{}] was spotted {} our territory", unit_name, in_or_near),
                        &tile.position,
                        NotificationCategory::War,
                        NotificationIcon::War,
                        Some(unit_name),
                    );
                }
            }
        } else {
            let positions: Vec<_> = tiles.iter().map(|t| t.position.clone()).collect();
            this_player.add_notification(
                &format!("[{}] enemy units were spotted {} our territory", tiles.len(), in_or_near),
                &LocationAction::new(positions),
                NotificationCategory::War,
                NotificationIcon::War,
                None,
            );
        }
    }

    /// Add a notification about bombardment
    fn add_bombard_notification(&self, this_player: &mut Civilization, cities: &[&City]) {
        if cities.len() < 3 {
            for city in cities {
                this_player.add_notification(
                    &format!("Your city [{}] can bombard the enemy!", city.name),
                    &MapUnitAction::new(city.location.clone()),
                    NotificationCategory::War,
                    NotificationIcon::City,
                    Some(NotificationIcon::Crosshair),
                );
            }
        } else {
            let notification_actions: Vec<_> = cities.iter()
                .map(|c| MapUnitAction::new(c.location.clone()))
                .collect();

            this_player.add_notification(
                &format!("[{}] of your cities can bombard the enemy!", cities.len()),
                &notification_actions,
                NotificationCategory::War,
                NotificationIcon::City,
                Some(NotificationIcon::Crosshair),
            );
        }
    }

    /// Notify about explored resources
    pub fn notify_explored_resources(
        &self,
        civ_info: &mut Civilization,
        resource_name: &str,
        max_distance: i32,
        filter: &dyn Fn(&Tile) -> bool,
    ) -> bool {
        if let Some(notification) = self.get_explored_resources_notification(civ_info, resource_name, max_distance, filter) {
            civ_info.notifications.push(notification);
            true
        } else {
            false
        }
    }

    /// Get a notification about explored resources
    pub fn get_explored_resources_notification(
        &self,
        civ: &Civilization,
        resource_name: &str,
        max_distance: i32,
        filter: &dyn Fn(&Tile) -> bool,
    ) -> Option<Notification> {
        if let Some(ruleset) = &self.ruleset {
            if let Some(resource) = ruleset.tile_resources.get(resource_name) {
                if !civ.tech.is_revealed(resource) {
                    return None;
                }

                // Include your city-state allies' cities with your own for the purpose of showing the closest city
                let relevant_cities: Vec<&City> = civ.cities.iter()
                    .chain(
                        civ.get_known_civs()
                            .iter()
                            .filter(|c| c.is_city_state && c.get_ally_civ() == Some(civ.civ_name.clone()))
                            .flat_map(|c| &c.cities)
                    )
                    .collect();

                // All sources of the resource on the map, using a city-state's capital center tile for the CityStateOnlyResource types
                let explored_reveal_tiles: Vec<&Tile> = if resource.has_unique(UniqueType::CityStateOnlyResource) {
                    // Look for matching mercantile CS centers
                    self.get_alive_city_states()
                        .iter()
                        .filter(|cs| cs.city_state_resource == Some(resource_name.to_string()))
                        .filter_map(|cs| cs.get_capital().map(|c| c.get_center_tile()))
                        .collect()
                } else {
                    self.tile_map.values()
                        .iter()
                        .filter(|t| t.resource == Some(resource_name.to_string()))
                        .collect()
                };

                // Apply all filters to the above collection and sort them by distance to closest city
                let mut explored_reveal_info: Vec<(i32, &City, &Tile)> = Vec::new();

                for tile in explored_reveal_tiles {
                    if !civ.has_explored(tile) {
                        continue;
                    }

                    for city in &relevant_cities {
                        let distance = tile.aerial_distance_to(city.get_center_tile());
                        if distance <= max_distance && filter(tile) {
                            explored_reveal_info.push((distance, city, tile));
                        }
                    }
                }

                explored_reveal_info.sort_by_key(|(distance, _, _)| *distance);

                // Remove duplicates by tile
                let mut seen_tiles = HashSet::new();
                explored_reveal_info.retain(|(_, _, tile)| {
                    let position = tile.position.clone();
                    seen_tiles.insert(position)
                });

                if let Some((_, chosen_city, _)) = explored_reveal_info.first() {
                    // Re-sort to a more pleasant display order
                    explored_reveal_info.sort_by_key(|(_, _, tile)| tile.aerial_distance_to(chosen_city.get_center_tile()));

                    let positions: Vec<_> = explored_reveal_info.iter().map(|(_, _, tile)| tile.position.clone()).collect();

                    let positions_count = positions.len();
                    let text = if positions_count == 1 {
                        format!("[{}] revealed near [{}]", resource_name, chosen_city.name)
                    } else {
                        format!("[{}] sources of [{}] revealed, e.g. near [{}]", positions_count, resource_name, chosen_city.name)
                    };

                    return Some(Notification::new(
                        &text,
                        &[format!("ResourceIcons/{}", resource_name)],
                        LocationAction::new(positions),
                        NotificationCategory::General,
                    ));
                }
            }
        }

        None
    }

    /// Convert the game to a preview
    pub fn as_preview(&self) -> GameInfoPreview {
        GameInfoPreview::from_game_info(this)
    }

    /// Process the next turn
    pub fn next_turn(&mut self, progress_bar: Option<&mut dyn ProgressBar>) {
        let mut player = self.current_player_civ.as_ref().expect("Current player civilization not set").clone();
        let mut player_index = self.civilizations.iter().position(|c| c.civ_name == player.civ_name).unwrap();

        // We rotate Players in cycle: 1,2...N,1,2...
        let mut set_next_player = || {
            player_index = (player_index + 1) % self.civilizations.len();
            if player_index == 0 {
                self.turns += 1;
                debug!("Starting simulation of turn {}", self.turns);
            }
            player = self.civilizations[player_index].clone();
        };

        // Ending current player's turn
        // (Check is important or else switchTurn
        // would skip a turn if an AI civ calls nextTurn
        // this happens when resigning a multiplayer game)
        if player.is_human() {
            TurnManager::new(&mut player).end_turn(progress_bar);
            set_next_player();
        }

        let is_online = self.game_parameters.is_online_multiplayer;

        // Skip the player if we are playing hotseat
        // If all hotseat players are defeated then skip all but the first one
        let should_auto_process_hotseat_player = || {
            !is_online &&
            player.is_defeated() && (
                self.civilizations.iter().any(|c| c.is_human() && c.is_alive()) ||
                self.civilizations.iter().find(|c| c.is_human()).map_or(false, |c| c.civ_name != player.civ_name)
            )
        };

        // Skip all spectators and defeated players
        // If all players are defeated then let the first player control next turn
        let should_auto_process_online_player = || {
            is_online && (
                player.is_spectator() ||
                (player.is_defeated() && (
                    self.civilizations.iter().any(|c| c.is_human() && c.is_alive()) ||
                    self.civilizations.iter().find(|c| c.is_human()).map_or(false, |c| c.civ_name != player.civ_name)
                ))
            )
        };

        // We process player automatically if:
        while self.is_simulation() ||                    // simulation is active
              player.is_ai() ||                          // or player is AI
              should_auto_process_hotseat_player() ||    // or a player is defeated in hotseat
              should_auto_process_online_player()        // or player is online spectator
        {
            // Starting preparations
            TurnManager::new(&mut player).start_turn(progress_bar);

            // Automation done here
            TurnManager::new(&mut player).automate_turn();

            // Do we need to break if player won?
            if self.simulate_until_win && player.victory_manager.has_won() {
                self.simulate_until_win = false;
                // world_screen?.auto_play?.stop_auto_play();
                break;
            }

            // Do we need to stop AutoPlay?
            // if world_screen != null && world_screen.auto_play.is_auto_playing() && player.victory_manager.has_won() && !one_more_turn_mode
            //     world_screen.auto_play.stop_auto_play()

            // Clean up
            TurnManager::new(&mut player).end_turn(progress_bar);

            // To the next player
            set_next_player();
        }

        // We found a human player, so we are making them current
        self.current_turn_start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.current_player = player.civ_name.clone();
        self.current_player_civ = Some(player.clone());

        // Starting their turn
        TurnManager::new(&mut self.current_player_civ.as_mut().unwrap()).start_turn(progress_bar);

        // No popups for spectators
        if self.current_player_civ.as_ref().unwrap().is_spectator() {
            self.current_player_civ.as_mut().unwrap().popup_alerts.clear();
        }

        // Play some nice music
        if self.turns % 10 == 0 {
            // In Rust, we would use a proper audio system
            // This is a simplified version
        }

        // Start our turn immediately before the player can make decisions - affects
        // whether our units can commit automated actions and then be attacked immediately etc.
        self.notify_of_close_enemy_units(&mut self.current_player_civ.as_mut().unwrap());
    }

    /// Notify of close enemy units
    fn notify_of_close_enemy_units(&self, this_player: &mut Civilization) {
        let viewable_invisible_tiles: Vec<_> = this_player.viewable_invisible_units_tiles.iter()
            .map(|t| t.position.clone())
            .collect();

        let enemy_units_close_to_territory: Vec<&Tile> = this_player.viewable_tiles.iter()
            .filter(|t| {
                if let Some(unit) = &t.military_unit {
                    unit.civ.civ_name != this_player.civ_name &&
                    this_player.is_at_war_with(&unit.civ) &&
                    (t.get_owner() == Some(this_player.civ_name.clone()) ||
                    t.neighbors.iter().any(|n| n.get_owner() == Some(this_player.civ_name.clone())) &&
                    (!unit.is_invisible(this_player) || viewable_invisible_tiles.contains(&t.position)))
                } else {
                    false
                }
            })
            .collect();

        // enemy units IN our territory
        let enemy_units_in_territory: Vec<&Tile> = enemy_units_close_to_territory.iter()
            .filter(|t| t.get_owner() == Some(this_player.civ_name.clone()))
            .copied()
            .collect();

        self.add_enemy_unit_notification(this_player, &enemy_units_in_territory, "in");

        // enemy units NEAR our territory
        let enemy_units_near_territory: Vec<&Tile> = enemy_units_close_to_territory.iter()
            .filter(|t| t.get_owner() != Some(this_player.civ_name.clone()))
            .copied()
            .collect();

        self.add_enemy_unit_notification(this_player, &enemy_units_near_territory, "near");

        // Add bombard notification
        let cities_that_can_bombard: Vec<&City> = this_player.cities.iter()
            .filter(|city| {
                city.can_bombard() &&
                enemy_units_close_to_territory.iter().any(|tile| {
                    tile.aerial_distance_to(city.get_center_tile()) <= city.get_bombard_range()
                })
            })
            .collect();

        self.add_bombard_notification(this_player, &cities_that_can_bombard);
    }

    /// Set transients for the game
    pub fn set_transients(&mut self) {
        self.tile_map.game_info = Some(self.clone());

        // Convert old saves to newer ones by moving base rulesets from the mod list to the base ruleset field
        self.convert_old_saves_to_new_saves();

        // Cater for the mad modder using trailing '-' in their repo name - convert the mods list so
        // it requires our new, Windows-safe local name (no trailing blanks)
        let mods_to_rename: Vec<(String, String)> = self.game_parameters.mods.iter()
            .map(|m| (m.clone(), m.repo_name_to_folder_name(true)))
            .filter(|(old, new)| old != new)
            .collect();

        for (old_name, new_name) in mods_to_rename {
            self.game_parameters.mods.remove(&old_name);
            self.game_parameters.mods.insert(new_name);
        }

        self.ruleset = Some(RulesetCache::get_complex_ruleset(&self.game_parameters));

        // any mod the saved game lists that is currently not installed causes null pointer
        // exceptions in this routine unless it contained no new objects or was very simple.
        // Player's fault, so better complain early:
        let missing_mods: Vec<String> = vec![self.game_parameters.base_ruleset.clone()]
            .into_iter()
            .chain(self.game_parameters.mods.iter().cloned())
            .filter(|m| !self.ruleset.as_ref().unwrap().mods.contains_key(m))
            .collect();

        if !missing_mods.is_empty() {
            panic!("Missing mods: {:?}", missing_mods);
        }

        // Remove missing mod references
        crate::backward_compatibility::BackwardCompatibility::remove_missing_mod_references(self);

        // Set ruleset for base units
        if let Some(ruleset) = &self.ruleset {
            for base_unit in ruleset.units.values() {
                base_unit.set_ruleset(ruleset);
            }

            for building in ruleset.buildings.values() {
                building.ruleset = Some(ruleset.clone());
            }
        }

        // This needs to go before tileMap.setTransients, as units need to access
        // the nation of their civilization when setting transients
        for civ in &mut self.civilizations {
            civ.game_info = Some(self.clone());
        }

        for civ in &mut self.civilizations {
            civ.set_nation_transient();
            civ.cache.update_state();
        }

        // must be done before updating tileMap, since unit uniques depend on civ uniques depend on allied city-state uniques depend on diplomacy
        for civ in &mut self.civilizations {
            for diplomacy_manager in civ.diplomacy.values_mut() {
                diplomacy_manager.civ_info = Some(civ.clone());
                diplomacy_manager.update_has_open_borders();
            }
        }

        if let Some(ruleset) = &self.ruleset {
            self.tile_map.set_transients(ruleset);
        }

        // Temporary - All games saved in 4.12.15 turned into 'hexagonal non world wrapped'
        // Here we attempt to fix that

        // How do we recognize a rectangle? By having many tiles at the lowest edge
        let tiles_with_lowest_row: Vec<&Tile> = self.tile_map.tile_list.iter()
            .filter(|t| t.get_row() == self.tile_map.tile_list.iter().map(|t| t.get_row()).min().unwrap_or(0))
            .collect();

        if tiles_with_lowest_row.len() > 2 {
            self.tile_map.map_parameters.shape = MapShape::Rectangular;
        }

        if self.current_player.is_empty() {
            self.current_player = if self.game_parameters.is_online_multiplayer {
                self.civilizations.iter()
                    .find(|c| c.is_human() && !c.is_spectator())
                    .map(|c| c.civ_name.clone())
                    .unwrap_or_default()
            } else {
                self.civilizations.iter()
                    .find(|c| c.is_human())
                    .map(|c| c.civ_name.clone())
                    .unwrap_or_default()
            };
        }

        self.current_player_civ = Some(self.get_civilization(&self.current_player).clone());

        if let Some(ruleset) = &self.ruleset {
            self.difficulty_object = ruleset.difficulties.get(&self.difficulty).cloned();
            self.speed = ruleset.speeds.get(&self.game_parameters.speed).cloned();
        }

        for religion in self.religions.values_mut() {
            religion.set_transients(self);
        }

        for civ in &mut self.civilizations {
            civ.set_transients();
        }

        self.tile_map.set_neutral_transients(); // has to happen after civInfo.setTransients() sets owningCity

        for civ in &mut self.civilizations {
            // Due to religion victory, has to happen after civInfo.religionManager is set for all civs
            civ.things_to_focus_on_for_victory = civ.get_preferred_victory_type_objects()
                .iter()
                .flat_map(|v| v.get_things_to_focus(civ))
                .collect();
        }

        // Apply backward compatibility fixes
        crate::backward_compatibility::BackwardCompatibility::convert_fortify(self);
        self.update_civilization_state();

        self.space_resources.clear();

        if let Some(ruleset) = &self.ruleset {
            for building in ruleset.buildings.values() {
                if building.has_unique(UniqueType::SpaceshipPart) {
                    for resource in building.get_resource_requirements_per_turn().keys() {
                        self.space_resources.insert(resource.clone());
                    }
                }
            }

            for victory in ruleset.victories.values() {
                for part in &victory.required_spaceship_parts {
                    self.space_resources.insert(part.clone());
                }
            }
        }

        self.barbarians.set_transients(self);
        self.city_distances.game = Some(self.clone());

        // Apply more backward compatibility fixes
        crate::backward_compatibility::BackwardCompatibility::guarantee_unit_promotions(self);
        crate::backward_compatibility::BackwardCompatibility::migrate_to_tile_history(self);
        crate::backward_compatibility::BackwardCompatibility::migrate_great_general_pools(self);
        crate::backward_compatibility::BackwardCompatibility::ensure_unit_ids(self);
    }

    /// Update civilization state
    fn update_civilization_state(&mut self) {
        // Update city-state resource first since the happiness of major civ depends on it.
        let mut civs_to_update: Vec<&mut Civilization> = self.civilizations.iter_mut().collect();
        civs_to_update.sort_by(|a, b| b.is_city_state.cmp(&a.is_city_state));

        for civ in civs_to_update {
            for unit in civ.units.get_civ_units_mut() {
                unit.update_visible_tiles(false); // this needs to be done after all the units are assigned to their civs and all other transients are set
            }

            if civ.player_type == PlayerType::Human {
                civ.explored_region.set_map_parameters(&self.tile_map.map_parameters); // Required for the correct calculation of the explored region on world wrap maps
            }

            civ.cache.update_our_tiles();
            civ.cache.update_sight_and_resources(); // only run ONCE and not for each unit - this is a huge performance saver!

            // Since this depends on the cities of ALL civilizations,
            // we need to wait until we've set the transients of all the cities before we can run this.
            // Hence why it's not in CivInfo.setTransients().
            civ.cache.update_cities_connected_to_capital(true);

            // We need to determine the GLOBAL happiness state in order to determine the city stats
            let local_unique_cache = LocalUniqueCache::new();

            for city in &mut civ.cities {
                city.city_stats.update_tile_stats(&local_unique_cache); // Some nat wonders can give happiness!
                city.city_stats.update_city_happiness(
                    city.city_constructions.get_stats(&local_unique_cache)
                );
            }

            for city in &mut civ.cities {
                // We remove constructions from the queue that aren't defined in the ruleset.
                // This can lead to situations where the city is puppeted and had its construction removed, and there's no way to user-set it
                // So for cities like those, we'll auto-set the construction
                // Also set construction for human players who have automate production turned on
                if city.city_constructions.construction_queue.is_empty() {
                    city.city_constructions.choose_next_construction();
                }

                // We also remove resources that the city may be demanding but are no longer in the ruleset
                if let Some(ruleset) = &self.ruleset {
                    if !ruleset.tile_resources.contains_key(&city.demanded_resource) {
                        city.demanded_resource = String::new();
                    }
                }

                // No uniques have changed since the cache was created, so we can still use it
                city.city_stats.update(&local_unique_cache);
            }
        }
    }

    /// Convert old saves to new saves
    fn convert_old_saves_to_new_saves(&mut self) {
        if let Some(base_ruleset_in_mods) = self.game_parameters.mods.iter()
            .find(|m| RulesetCache::get(m).map_or(false, |r| r.mod_options.is_base_ruleset))
            .cloned()
        {
            self.game_parameters.base_ruleset = base_ruleset_in_mods.clone();
            self.game_parameters.mods.retain(|m| m != &base_ruleset_in_mods);
        }
    }
}

/// Class to use when parsing jsons if you only want the serialization version.
pub struct GameInfoSerializationVersion {
    /// The version of the game
    pub version: CompatibilityVersion,
}

impl GameInfoSerializationVersion {
    /// Create a new GameInfoSerializationVersion
    pub fn new() -> Self {
        Self {
            version: GameInfo::FIRST_WITHOUT,
        }
    }
}

impl HasGameInfoSerializationVersion for GameInfoSerializationVersion {
    fn version(&self) -> &CompatibilityVersion {
        &self.version
    }

    fn set_version(&mut self, version: CompatibilityVersion) {
        self.version = version;
    }
}

impl HasGameInfoSerializationVersion for GameInfo {
    fn version(&self) -> &CompatibilityVersion {
        &self.version
    }

    fn set_version(&mut self, version: CompatibilityVersion) {
        self.version = version;
    }
}