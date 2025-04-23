use std::collections::HashMap;
use ggez::mint::Vector2;
use crate::city::city::City;
use crate::civilization::civilization::Civilization;
use crate::game_info::GameInfo;
use crate::models::game_info::Position;
use crate::tile::tile::Tile;

/// Represents a city and its distance from a tile
#[derive(Debug, Clone)]
pub struct CityDistance {
    /// The city
    pub city: City,
    /// The distance from the tile to the city
    pub distance: i32,
}

impl CityDistance {
    /// Compares two CityDistance objects and returns the one with the lower distance.
    /// If distances are equal, prioritizes major civilizations over minor ones.
    ///
    /// # Arguments
    /// * `a` - The first CityDistance to compare
    /// * `b` - The second CityDistance to compare
    ///
    /// # Returns
    /// The CityDistance with the lower distance, or the one from a major civilization if distances are equal
    pub fn compare(a: Option<&CityDistance>, b: Option<&CityDistance>) -> Option<CityDistance> {
        match (a, b) {
            (None, Some(b)) => Some(b.clone()),
            (Some(a), None) => Some(a.clone()),
            (None, None) => None,
            (Some(a), Some(b)) => {
                if a.distance < b.distance {
                    Some(a.clone())
                } else if a.distance > b.distance {
                    Some(b.clone())
                } else {
                    // If distances are equal, prioritize major civilizations
                    if a.city.civ.is_major_civ() && b.city.civ.is_minor_civ() {
                        Some(a.clone())
                    } else if b.city.civ.is_major_civ() && a.city.civ.is_minor_civ() {
                        Some(b.clone())
                    } else {
                        Some(a.clone())
                    }
                }
            }
        }
    }
}

/// This struct holds information about distance from every tile to the nearest city
pub struct CityDistanceData {
    /// The game instance
    #[serde(skip)]
    pub game: Option<GameInfo>,
    /// Flag indicating if the data needs to be updated
    should_update: bool,
    /// Identifier -> Map (Tile position -> Distance)
    /// Identifier is either: Civ name, ALL_CIVS or MAJOR_CIVS
    data: HashMap<String, HashMap<Vector2, Option<CityDistance>>>,
}

impl CityDistanceData {
    /// Identifier for all civilizations
    pub const IDENTIFIER_ALL_CIVS: &'static str = "ALL_CIVS";
    /// Identifier for major civilizations only
    pub const IDENTIFIER_MAJOR_CIVS: &'static str = "MAJOR_CIVS";

    /// Creates a new CityDistanceData instance
    pub fn new() -> Self {
        let mut data = HashMap::new();
        data.insert(Self::IDENTIFIER_ALL_CIVS.to_string(), HashMap::new());
        data.insert(Self::IDENTIFIER_MAJOR_CIVS.to_string(), HashMap::new());

        Self {
            game: None,
            should_update: true,
            data,
        }
    }

    /// Resets the data for all identifiers
    fn reset(&mut self) {
        self.data = HashMap::new();
        self.data.insert(Self::IDENTIFIER_ALL_CIVS.to_string(), HashMap::new());
        self.data.insert(Self::IDENTIFIER_MAJOR_CIVS.to_string(), HashMap::new());
    }

    /// Resets the data for a specific player
    fn reset_player(&mut self, identifier: &str) {
        self.data.insert(identifier.to_string(), HashMap::new());
    }

    /// Updates the distance if the new distance is lower than the current one
    fn update_distance_if_lower(&mut self, identifier: &str, position: Position, city: &City, distance: f32) {
        let entry = self.data.entry(identifier.to_string()).or_insert_with(HashMap::new);
        let current_distance = entry.get(&position).cloned().flatten();
        let new_distance = CityDistance {
            city: city.clone(),
            distance,
        };
        entry.insert(position, CityDistance::compare(current_distance.as_ref(), Some(&new_distance)));
    }

    /// Updates the distances for a tile with respect to a city
    fn update_distances(&mut self, this_tile: &Tile, city: &City, owner: &Civilization, is_major: bool) {
        let city_tile = city.get_center_tile();
        let distance = this_tile.aerial_distance_to(&city_tile);
        let position = this_tile.position;

        self.update_distance_if_lower(Self::IDENTIFIER_ALL_CIVS, position, city, distance);

        if is_major {
            self.update_distance_if_lower(Self::IDENTIFIER_MAJOR_CIVS, position, city, distance);
            self.update_distance_if_lower(&owner.civ_name, position, city, distance);
        }
    }

    /// Updates all distances
    fn update(&mut self) {
        // Clear previous info
        self.reset();

        if let Some(game) = &self.game {
            for player in &game.civilizations {
                // Not interested in defeated players
                if player.is_defeated() {
                    continue;
                }

                let is_major = player.is_major_civ();
                if is_major {
                    self.reset_player(&player.civ_name);
                }

                // Update distances for each tile inside radius 4 around each city
                for city in player.cities.iter() {
                    for other_tile in city.get_center_tile().get_tiles_in_distance(4) {
                        self.update_distances(&other_tile, city, player, is_major);
                    }
                }
            }
        }

        self.should_update = false;
    }

    /// Gets the closest city distance for a tile
    ///
    /// # Arguments
    /// * `tile` - The tile to get the closest city distance for
    /// * `player` - Optional player to filter by
    /// * `majors_only` - Whether to only consider major civilizations
    ///
    /// # Returns
    /// The closest city distance, or None if no city is found
    pub fn get_closest_city_distance(&mut self, tile: &Tile, player: Option<&Civilization>, majors_only: bool) -> Option<CityDistance> {
        if self.should_update {
            self.update();
        }

        let identifier = match (player, majors_only) {
            (Some(p), _) if p.is_major_civ() => p.civ_name.clone(),
            (_, true) => Self::IDENTIFIER_MAJOR_CIVS.to_string(),
            _ => Self::IDENTIFIER_ALL_CIVS.to_string(),
        };

        self.data.get(&identifier)
            .and_then(|map| map.get(&tile.position))
            .cloned()
            .flatten()
    }

    /// Marks the data as dirty, indicating it needs to be updated
    pub fn set_dirty(&mut self) {
        self.should_update = true;
    }
}