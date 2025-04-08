use crate::automation::Automation;
use crate::models::city::City;
use crate::models::civilization::Civilization;
use crate::models::civilization::diplomacy::DiplomacyFlags;
use crate::models::map::hex_math::HexMath;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::models::ruleset::unique::StateForConditionals;
use crate::models::ruleset::unique::UniqueType;
use std::collections::{HashMap, HashSet};

/// Contains logic for ranking and selecting optimal city locations
pub struct CityLocationTileRanker;

/// Represents the best tiles to found a city on
pub struct BestTilesToFoundCity {
    pub tile_rank_map: HashMap<Tile, f32>,
    pub best_tile: Option<Tile>,
    pub best_tile_rank: f32,
}

impl BestTilesToFoundCity {
    /// Creates a new BestTilesToFoundCity instance
    pub fn new() -> Self {
        Self {
            tile_rank_map: HashMap::new(),
            best_tile: None,
            best_tile_rank: 0.0,
        }
    }
}

impl CityLocationTileRanker {
    /// Returns a hashmap of tiles to their ranking plus the highest value tile and its value
    pub fn get_best_tiles_to_found_city(unit: &MapUnit, distance_to_search: Option<i32>, minimum_value: f32) -> BestTilesToFoundCity {
        let distance_modifier = 3.0; // percentage penalty per aerial distance from unit (Settler)

        let range = if let Some(distance) = distance_to_search {
            distance
        } else {
            let distance_from_home = if unit.civ.cities.is_empty() {
                0
            } else {
                unit.civ.cities.iter()
                    .map(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()))
                    .min()
                    .unwrap_or(0)
            };

            (8 - distance_from_home).clamp(1, 5) // Restrict vision when far from home to avoid death marches
        };

        let nearby_cities: Vec<_> = unit.civ.game_info.get_cities()
            .iter()
            .filter(|city| city.get_center_tile().aerial_distance_to(unit.get_tile()) <= 7 + range)
            .cloned()
            .collect();

        let uniques = unit.get_matching_uniques(UniqueType::FoundCity);
        let possible_city_locations: Vec<_> = unit.get_tile().get_tiles_in_distance(range)
            .iter()
            // Filter out tiles that we can't actually found on
            .filter(|tile| {
                uniques.iter().any(|unique|
                    unique.conditionals_apply(StateForConditionals::new(Some(unit.clone()), Some(tile.clone())))
                )
            })
            .filter(|tile| {
                Self::can_settle_tile(tile, &unit.civ, &nearby_cities) &&
                (unit.get_tile() == *tile || unit.movement.can_move_to(tile))
            })
            .cloned()
            .collect();

        let unique_cache = LocalUniqueCache::new();
        let mut best_tiles_to_found_city = BestTilesToFoundCity::new();
        let mut base_tile_map = HashMap::new();

        let possible_tile_locations_with_rank: Vec<_> = possible_city_locations.iter()
            .map(|tile| {
                let mut tile_value = Self::rank_tile_to_settle(tile, &unit.civ, &nearby_cities, &mut base_tile_map, &unique_cache);
                let distance_score = (unit.current_tile.aerial_distance_to(tile) as f32 * distance_modifier).clamp(0.0, 99.0);
                tile_value *= (100.0 - distance_score) / 100.0;

                if tile_value >= minimum_value {
                    best_tiles_to_found_city.tile_rank_map.insert(tile.clone(), tile_value);
                }

                (tile.clone(), tile_value)
            })
            .filter(|(_, value)| *value >= minimum_value)
            .collect();

        // Sort by descending value
        let mut sorted_tiles = possible_tile_locations_with_rank;
        sorted_tiles.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        // Find the best reachable tile
        if let Some((best_tile, best_value)) = sorted_tiles.iter()
            .find(|(tile, _)| unit.movement.can_reach(tile))
        {
            best_tiles_to_found_city.best_tile = Some(best_tile.clone());
            best_tiles_to_found_city.best_tile_rank = *best_value;
        }

        best_tiles_to_found_city
    }

    /// Checks if a tile can be settled on
    fn can_settle_tile(tile: &Tile, civ: &Civilization, nearby_cities: &[City]) -> bool {
        let mod_constants = &civ.game_info.ruleset.mod_options.constants;

        if !tile.is_land || tile.is_impassible() {
            return false;
        }

        if let Some(owner) = tile.get_owner() {
            if owner != *civ {
                return false;
            }
        }

        for city in nearby_cities {
            let distance = city.get_center_tile().aerial_distance_to(tile);

            // Check if we agreed not to settle near this city
            if distance <= 6 && civ.knows(&city.civ)
                && !civ.is_at_war_with(&city.civ)
                // If the CITY OWNER knows that the UNIT OWNER agreed not to settle near them
                && city.civ.get_diplomacy_manager(civ)
                    .map_or(false, |dm| dm.has_flag(DiplomacyFlags::AgreedToNotSettleNearUs))
            {
                return false;
            }

            if tile.get_continent() == city.get_center_tile().get_continent() {
                if distance <= mod_constants.minimal_city_distance {
                    return false;
                }
            } else {
                if distance <= mod_constants.minimal_city_distance_on_different_continents {
                    return false;
                }
            }
        }

        true
    }

    /// Ranks a tile for settling a new city
    fn rank_tile_to_settle(
        new_city_tile: &Tile,
        civ: &Civilization,
        nearby_cities: &[City],
        base_tile_map: &mut HashMap<Tile, f32>,
        unique_cache: &LocalUniqueCache
    ) -> f32 {
        let mut tile_value = 0.0;
        tile_value += Self::get_distance_to_city_modifier(new_city_tile, nearby_cities, civ);

        let on_coast = new_city_tile.is_coastal_tile();
        let on_hill = new_city_tile.is_hill();
        let is_next_to_mountain = new_city_tile.is_adjacent_to("Mountain");
        // Only count a luxury resource that we don't have yet as unique once
        let mut new_unique_luxury_resources = HashSet::new();

        if on_coast {
            tile_value += 3.0;
        }
        // Hills are free production and defence
        if on_hill {
            tile_value += 7.0;
        }
        // Observatories are good, but current implementation not mod-friendly
        if is_next_to_mountain {
            tile_value += 5.0;
        }
        // This bonus for settling on river is a bit outsized for the importance, but otherwise they have a habit of settling 1 tile away
        if new_city_tile.is_adjacent_to_river() {
            tile_value += 20.0;
        }
        // We want to found the city on an oasis because it can't be improved otherwise
        if new_city_tile.terrain_has_unique(UniqueType::Unbuildable) {
            tile_value += 3.0;
        }
        // If we build the city on a resource tile, then we can't build any special improvements on it
        if new_city_tile.has_viewable_resource(civ) {
            tile_value -= 4.0;
        }
        if new_city_tile.has_viewable_resource(civ) && new_city_tile.tile_resource.resource_type == ResourceType::Bonus {
            tile_value -= 8.0;
        }
        // Settling on bonus resources tends to waste a food
        // Settling on luxuries generally speeds up our game, and settling on strategics as well, as the AI cheats and can see them.

        let mut tiles = 0;
        for i in 0..=3 {
            // Ideally, we shouldn't really count the center tile, as it's converted into 1 production 2 food anyways with special cases treated above, but doing so can lead to AI moving settler back and forth until forever
            for nearby_tile in new_city_tile.get_tiles_at_distance(i) {
                tiles += 1;
                tile_value += Self::rank_tile(
                    &nearby_tile,
                    civ,
                    on_coast,
                    &mut new_unique_luxury_resources,
                    base_tile_map,
                    unique_cache
                ) * (3.0 / (i as f32 + 1.0));
                // Tiles close to the city can be worked more quickly, and thus should gain higher weight.
            }
        }

        // Placing cities on the edge of the map is bad, we can't even build improvements on them!
        tile_value -= (HexMath::get_number_of_tiles_in_hexagon(3) - tiles) as f32 * 2.4;

        tile_value
    }

    /// Calculates a modifier based on distance to nearby cities
    fn get_distance_to_city_modifier(new_city_tile: &Tile, nearby_cities: &[City], civ: &Civilization) -> f32 {
        let mut modifier = 0.0;

        for city in nearby_cities {
            let distance_to_city = new_city_tile.aerial_distance_to(city.get_center_tile());

            let distance_to_city_modifier = match distance_to_city {
                // NOTE: the line it.getCenterTile().aerialDistanceTo(unit.getTile()) <= X + range
                // above MUST have the constant X that is added to the range be higher or equal to the highest distance here + 1
                // If it is not higher the settler may get stuck when it ranks the same tile differently
                // as it moves away from the city and doesn't include it in the calculation
                // and values it higher than when it moves closer to the city
                7 => 2.0,
                6 => 4.0,
                5 => 8.0, // Settling further away sacrifices tempo
                4 => 6.0,
                3 => -25.0,
                d if d < 3 => -30.0, // Even if it is a mod that lets us settle closer, lets still not do it
                _ => 0.0,
            };

            // We want a defensive ring around our capital
            let adjusted_modifier = if city.civ == *civ {
                if city.is_capital() {
                    distance_to_city_modifier * 2.0
                } else {
                    distance_to_city_modifier
                }
            } else {
                distance_to_city_modifier
            };

            modifier += adjusted_modifier;
        }

        modifier
    }

    /// Ranks a tile based on its characteristics
    fn rank_tile(
        rank_tile: &Tile,
        civ: &Civilization,
        on_coast: bool,
        new_unique_luxury_resources: &mut HashSet<String>,
        base_tile_map: &mut HashMap<Tile, f32>,
        unique_cache: &LocalUniqueCache
    ) -> f32 {
        if rank_tile.get_city().is_some() {
            return -1.0;
        }

        let mut location_specific_tile_value = 0.0;

        // Don't settle near but not on the coast
        if rank_tile.is_coastal_tile() && !on_coast {
            location_specific_tile_value -= 2.0;
        }

        // Check if there are any new unique luxury resources
        if rank_tile.has_viewable_resource(civ) && rank_tile.tile_resource.resource_type == ResourceType::Luxury {
            if let Some(resource) = &rank_tile.resource {
                if !civ.has_resource(resource) && !new_unique_luxury_resources.contains(resource) {
                    location_specific_tile_value += 10.0;
                    new_unique_luxury_resources.insert(resource.clone());
                }
            }
        }

        // Check if everything else has been calculated, if so return it
        if let Some(base_value) = base_tile_map.get(rank_tile) {
            return location_specific_tile_value + base_value;
        }

        if let Some(owner) = rank_tile.get_owner() {
            if owner != *civ {
                return 0.0;
            }
        }

        let mut rank_tile_value = Automation::rank_stats_value(
            rank_tile.stats.get_tile_stats(None, civ, unique_cache),
            civ
        );

        if rank_tile.has_viewable_resource(civ) {
            if let Some(resource) = &rank_tile.resource {
                rank_tile_value += match rank_tile.tile_resource.resource_type {
                    ResourceType::Bonus => 2.0,
                    ResourceType::Strategic => 1.2 * rank_tile.resource_amount as f32,
                    ResourceType::Luxury => 10.0 * rank_tile.resource_amount as f32, // very important for humans who might want to conquer the AI
                };
            }
        }

        if rank_tile.terrain_has_unique(UniqueType::FreshWater) {
            rank_tile_value += 0.5; // Taking into account freshwater farm food, maybe less important in baseruleset mods
        }

        if !rank_tile.terrain_features.is_empty() && rank_tile.last_terrain.has_unique(UniqueType::ProductionBonusWhenRemoved) {
            rank_tile_value += 0.5; // Taking into account yields from forest chopping
        }

        if rank_tile.is_natural_wonder() {
            rank_tile_value += 10.0;
        }

        base_tile_map.insert(rank_tile.clone(), rank_tile_value);

        rank_tile_value + location_specific_tile_value
    }
}