use std::collections::HashMap;
use crate::map::mapgenerator::mapregions::ImpactType;
use crate::map::mapgenerator::mapregions::TileDataMap;
use crate::map::mapgenerator::mapregions::anonymize_unique;
use crate::map::mapgenerator::mapregions::get_terrain_rule;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::tile::Terrain;
use crate::models::ruleset::tile::TileResource;
use crate::models::ruleset::unique::StateForConditionals;
use crate::models::ruleset::unique::UniqueType;
use crate::utils::random_weighted;

/// This struct deals with the internals of *how* to place resources in tiles
/// It does not contain the logic of *when* to do so
pub struct MapRegionResources;

impl MapRegionResources {
    /// Given a tile_list and possible resource_options, will place a resource on every frequency tiles.
    /// Tries to avoid impacts, but falls back to lowest impact otherwise.
    /// Goes through the list in order, so pre-shuffle it!
    /// Assumes all tiles in the list are of the same terrain type when generating weightings, irrelevant if only one option.
    /// Returns a map of the resources in the options list to number placed.
    pub fn place_resources_in_tiles(
        tile_data: &mut TileDataMap,
        frequency: i32,
        tile_list: &[&Tile],
        resource_options: &[&TileResource],
        base_impact: i32,
        random_impact: i32,
        major_deposit: bool
    ) -> HashMap<&TileResource, i32> {
        if tile_list.is_empty() || resource_options.is_empty() {
            return HashMap::new();
        }

        if frequency == 0 {
            return HashMap::new();
        }

        let impact_type = match resource_options[0].resource_type {
            ResourceType::Strategic => ImpactType::Strategic,
            ResourceType::Bonus => ImpactType::Bonus,
            ResourceType::Luxury => ImpactType::Luxury
        };

        let conditional_terrain = StateForConditionals::new(attacked_tile: tile_list.first().cloned());

        let mut weightings: HashMap<&TileResource, f32> = HashMap::new();
        for resource in resource_options {
            let unique = resource.get_matching_uniques(UniqueType::ResourceWeighting, &conditional_terrain)
                .first()
                .cloned();

            let weight = if let Some(unique) = unique {
                unique.params[0].parse::<f32>().unwrap_or(1.0)
            } else {
                1.0
            };

            weightings.insert(resource, weight);
        }

        let amount_to_place = (tile_list.len() / frequency as usize) + 1;
        let mut amount_placed = 0;
        let mut detailed_placed: HashMap<&TileResource, i32> = HashMap::new();

        for resource in resource_options {
            detailed_placed.insert(resource, 0);
        }

        let mut fallback_tiles: Vec<&Tile> = Vec::new();

        // First pass - avoid impacts entirely
        for tile in tile_list {
            if tile.resource.is_some() {
                continue;
            }

            let possible_resources_for_tile: Vec<&TileResource> = resource_options.iter()
                .filter(|r| r.generates_naturally_on(tile))
                .cloned()
                .collect();

            if possible_resources_for_tile.is_empty() {
                continue;
            }

            if let Some(impacts) = tile_data.get(&tile.position) {
                if impacts.impacts.contains_key(&impact_type) {
                    fallback_tiles.push(tile); // Taken but might be a viable fallback tile
                } else {
                    // Add a resource to the tile
                    let resource_to_place = random_weighted(&possible_resources_for_tile, |r| {
                        weightings.get(r).cloned().unwrap_or(0.0)
                    });

                    tile.set_tile_resource(resource_to_place, major_deposit);
                    tile_data.place_impact(impact_type, tile, base_impact + rand::random::<i32>() % (random_impact + 1));
                    amount_placed += 1;
                    *detailed_placed.get_mut(resource_to_place).unwrap() += 1;

                    if amount_placed >= amount_to_place {
                        return detailed_placed;
                    }
                }
            }
        }

        // Second pass - place on least impacted tiles
        while amount_placed < amount_to_place && !fallback_tiles.is_empty() {
            // Sorry, we do need to re-sort the list for every pass since new impacts are made with every placement
            let best_tile = fallback_tiles.iter()
                .min_by_key(|tile| {
                    tile_data.get(&tile.position)
                        .and_then(|impacts| impacts.impacts.get(&impact_type))
                        .cloned()
                        .unwrap_or(0)
                })
                .unwrap();

            fallback_tiles.retain(|t| t != best_tile);

            let possible_resources_for_tile: Vec<&TileResource> = resource_options.iter()
                .filter(|r| r.generates_naturally_on(best_tile))
                .cloned()
                .collect();

            let resource_to_place = random_weighted(&possible_resources_for_tile, |r| {
                weightings.get(r).cloned().unwrap_or(0.0)
            });

            best_tile.set_tile_resource(resource_to_place, major_deposit);
            tile_data.place_impact(impact_type, best_tile, base_impact + rand::random::<i32>() % (random_impact + 1));
            amount_placed += 1;
            *detailed_placed.get_mut(resource_to_place).unwrap() += 1;
        }

        detailed_placed
    }

    /// Attempts to place amount resource on tiles, checking tiles in order. A ratio below 1 means skipping
    /// some tiles, ie ratio = 0.25 will put a resource on every 4th eligible tile. Can optionally respect impact flags,
    /// and places impact if base_impact >= 0. Returns number of placed resources.
    pub fn try_adding_resource_to_tiles(
        tile_data: &mut TileDataMap,
        resource: &TileResource,
        amount: i32,
        tiles: &[&Tile],
        ratio: f32,
        respect_impacts: bool,
        base_impact: i32,
        random_impact: i32,
        major_deposit: bool
    ) -> i32 {
        if amount <= 0 {
            return 0;
        }

        let mut amount_added = 0;
        let mut ratio_progress = 1.0;

        let impact_type = match resource.resource_type {
            ResourceType::Luxury => ImpactType::Luxury,
            ResourceType::Strategic => ImpactType::Strategic,
            ResourceType::Bonus => ImpactType::Bonus
        };

        for tile in tiles {
            if tile.resource.is_none() && resource.generates_naturally_on(tile) {
                if ratio_progress >= 1.0 &&
                    !(respect_impacts && tile_data.get(&tile.position)
                        .map_or(false, |impacts| impacts.impacts.contains_key(&impact_type))) {

                    tile.set_tile_resource(resource, major_deposit);
                    ratio_progress -= 1.0;
                    amount_added += 1;

                    if base_impact + random_impact >= 0 {
                        tile_data.place_impact(
                            impact_type,
                            tile,
                            base_impact + rand::random::<i32>() % (random_impact + 1)
                        );
                    }

                    if amount_added >= amount {
                        break;
                    }
                }

                ratio_progress += ratio;
            }
        }

        amount_added
    }

    /// Attempts to place major deposits in a tile_list consisting exclusively of terrain tiles.
    /// Lifted out of the main function to allow postponing water resources.
    /// Returns a map of resource types to placed deposits.
    pub fn place_major_deposits(
        tile_data: &mut TileDataMap,
        ruleset: &Ruleset,
        tile_list: &[&Tile],
        terrain: &Terrain,
        fallback_weightings: bool,
        base_impact: i32,
        random_impact: i32
    ) -> HashMap<&TileResource, i32> {
        if tile_list.is_empty() {
            return HashMap::new();
        }

        let frequency = if terrain.has_unique(UniqueType::MajorStrategicFrequency) {
            terrain.get_matching_uniques(UniqueType::MajorStrategicFrequency)
                .first()
                .map(|unique| unique.params[0].parse::<i32>().unwrap_or(25))
                .unwrap_or(25)
        } else {
            25
        };

        let terrain_rule = get_terrain_rule(terrain, ruleset);

        let resource_options: Vec<&TileResource> = ruleset.tile_resources.values()
            .filter(|r| {
                r.resource_type == ResourceType::Strategic &&
                ((fallback_weightings && r.terrains_can_be_found_on.contains(&terrain.name)) ||
                    r.unique_objects.iter().any(|unique| {
                        anonymize_unique(unique).text == terrain_rule.text
                    }))
            })
            .collect();

        if !resource_options.is_empty() {
            Self::place_resources_in_tiles(
                tile_data,
                frequency,
                tile_list,
                &resource_options,
                base_impact,
                random_impact,
                true
            )
        } else {
            HashMap::new()
        }
    }
}