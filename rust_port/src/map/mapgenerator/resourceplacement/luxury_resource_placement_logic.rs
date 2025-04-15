use std::collections::HashMap;
use std::f32;
use crate::map::mapgenerator::MapResourceSetting;
use crate::map::tile_map::TileMap;
use crate::map::mapgenerator::mapregions::*;
use crate::map::mapgenerator::mapregions::is_water_only_resource;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::tile::TileResource;
use crate::models::ruleset::unique::StateForConditionals;
use crate::models::ruleset::unique::UniqueType;
use crate::utils::random_weighted;
use crate::map::mapgenerator::resourceplacement::map_region_resources::MapRegionResources;

/// Logic for placing luxury resources on the map
pub struct LuxuryResourcePlacementLogic;

impl LuxuryResourcePlacementLogic {
    /// Assigns a luxury to each region. No luxury can be assigned to too many regions.
    /// Some luxuries are earmarked for city states. The rest are randomly distributed or
    /// don't occur at all in the map
    pub fn assign_luxuries(
        regions: &mut Vec<Region>,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset
    ) -> (Vec<String>, Vec<String>) {
        // If there are any weightings defined in json, assume they are complete. If there are none, use flat weightings instead
        let fallback_weightings = ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Luxury &&
                (r.has_unique(UniqueType::ResourceWeighting) || r.has_unique(UniqueType::LuxuryWeightingForCityStates)))
            .count() == 0;

        let max_regions_with_luxury = match regions.len() {
            size if size > 12 => 3,
            size if size > 8 => 2,
            _ => 1
        };

        let assignable_luxuries: Vec<&TileResource> = ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Luxury &&
                !r.has_unique(UniqueType::LuxurySpecialPlacement) &&
                !r.has_unique(UniqueType::CityStateOnlyResource))
            .collect();

        let mut amount_regions_with_luxury: HashMap<String, i32> = HashMap::new();
        // Init map
        for resource in ruleset.tile_resources.values() {
            amount_regions_with_luxury.insert(resource.name.clone(), 0);
        }

        // Sort regions by priority
        regions.sort_by(|a, b| {
            let a_priority = Self::get_region_priority(&ruleset.terrains[&a.region_type]);
            let b_priority = Self::get_region_priority(&ruleset.terrains[&b.region_type]);
            a_priority.partial_cmp(&b_priority).unwrap()
        });

        for region in regions.iter_mut() {
            let candidate_luxuries = Self::get_candidate_luxuries(
                &assignable_luxuries,
                &amount_regions_with_luxury,
                max_regions_with_luxury,
                fallback_weightings,
                region,
                ruleset
            );

            // If there are no candidates (mad modders???) just skip this region
            if candidate_luxuries.is_empty() {
                continue;
            }

            // Pick a luxury at random. Weight is reduced if the luxury has been picked before
            let region_conditional = StateForConditionals::new(region: Some(region.clone()));

            let luxury = random_weighted(&candidate_luxuries, |luxury| {
                let weighting_unique = luxury.get_matching_uniques(UniqueType::ResourceWeighting, &region_conditional)
                    .first()
                    .cloned();

                let relative_weight = if weighting_unique.is_none() {
                    1.0
                } else {
                    weighting_unique.unwrap().params[0].parse::<f32>().unwrap_or(1.0)
                };

                relative_weight / (1.0 + amount_regions_with_luxury[&luxury.name] as f32)
            });

            region.luxury = Some(luxury.name.clone());
            *amount_regions_with_luxury.get_mut(&luxury.name).unwrap() += 1;
        }

        let city_state_luxuries = Self::assign_city_state_luxuries(
            3, // was probably intended to be "if (tileData.size > 5000) 4 else 3",
            &assignable_luxuries,
            &amount_regions_with_luxury,
            fallback_weightings
        );

        let random_luxuries = Self::get_luxuries_for_random_placement(
            &assignable_luxuries,
            &amount_regions_with_luxury,
            tile_data,
            ruleset
        );

        (city_state_luxuries, random_luxuries)
    }

    fn get_luxuries_for_random_placement(
        assignable_luxuries: &[&TileResource],
        amount_regions_with_luxury: &HashMap<String, i32>,
        tile_data: &MapGenTileData,
        ruleset: &Ruleset
    ) -> Vec<String> {
        let mut remaining_luxuries: Vec<String> = assignable_luxuries.iter()
            .filter(|r| amount_regions_with_luxury[&r.name] == 0)
            .map(|r| r.name.clone())
            .collect();

        remaining_luxuries.shuffle(&mut rand::thread_rng());

        let disabled_percent = 100 - (tile_data.size as f32).powf(0.2) * 16.0.min(100.0) as f32;
        let target_disabled_luxuries = (ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Luxury)
            .count() as f32 * disabled_percent / 100.0) as i32;

        remaining_luxuries.into_iter().skip(target_disabled_luxuries as usize).collect()
    }

    fn get_candidate_luxuries(
        assignable_luxuries: &[&TileResource],
        amount_regions_with_luxury: &HashMap<String, i32>,
        max_regions_with_luxury: i32,
        fallback_weightings: bool,
        region: &Region,
        ruleset: &Ruleset
    ) -> Vec<&TileResource> {
        let region_conditional = StateForConditionals::new(region: Some(region.clone()));

        let mut candidate_luxuries: Vec<&TileResource> = assignable_luxuries.iter()
            .filter(|r| {
                amount_regions_with_luxury[&r.name] < max_regions_with_luxury &&
                // Check that it has a weight for this region type
                (fallback_weightings ||
                    r.has_unique(UniqueType::ResourceWeighting, &region_conditional)) &&
                // Check that there is enough coast if it is a water based resource
                (region.terrain_counts.get("Coastal").unwrap_or(&0) >= &12 ||
                    r.terrains_can_be_found_on.iter().any(|terrain|
                        ruleset.terrains[terrain].terrain_type != TerrainType::Water))
            })
            .cloned()
            .collect();

        // If we couldn't find any options, pick from all luxuries. First try to not pick water luxuries on land regions
        if candidate_luxuries.is_empty() {
            candidate_luxuries = assignable_luxuries.iter()
                .filter(|r| {
                    amount_regions_with_luxury[&r.name] < max_regions_with_luxury &&
                    // Ignore weightings for this pass
                    // Check that there is enough coast if it is a water based resource
                    (region.terrain_counts.get("Coastal").unwrap_or(&0) >= &12 ||
                        r.terrains_can_be_found_on.iter().any(|terrain|
                            ruleset.terrains[terrain].terrain_type != TerrainType::Water))
                })
                .cloned()
                .collect();
        }

        // If there are still no candidates, ignore water restrictions
        if candidate_luxuries.is_empty() {
            candidate_luxuries = assignable_luxuries.iter()
                .filter(|r| amount_regions_with_luxury[&r.name] < max_regions_with_luxury)
                .cloned()
                .collect();
        }

        candidate_luxuries
    }

    fn assign_city_state_luxuries(
        target_city_state_luxuries: i32,
        assignable_luxuries: &[&TileResource],
        amount_regions_with_luxury: &mut HashMap<String, i32>,
        fallback_weightings: bool
    ) -> Vec<String> {
        let mut city_state_luxuries = Vec::new();

        for _ in 0..target_city_state_luxuries {
            let candidate_luxuries: Vec<&TileResource> = assignable_luxuries.iter()
                .filter(|r| {
                    amount_regions_with_luxury[&r.name] == 0 &&
                    (fallback_weightings || r.has_unique(UniqueType::LuxuryWeightingForCityStates))
                })
                .cloned()
                .collect();

            if candidate_luxuries.is_empty() {
                continue;
            }

            let luxury = random_weighted(&candidate_luxuries, |r| {
                let weighting_unique = r.get_matching_uniques(UniqueType::LuxuryWeightingForCityStates)
                    .first()
                    .cloned();

                if weighting_unique.is_none() {
                    1.0
                } else {
                    weighting_unique.unwrap().params[0].parse::<f32>().unwrap_or(1.0)
                }
            });

            city_state_luxuries.push(luxury.name.clone());
            *amount_regions_with_luxury.get_mut(&luxury.name).unwrap() = 1;
        }

        city_state_luxuries
    }

    /// Places all Luxuries onto tile_map. Assumes that assign_luxuries and place_minor_civs have been called.
    pub fn place_luxuries(
        regions: &[Region],
        tile_map: &TileMap,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset,
        city_state_luxuries: &[String],
        random_luxuries: &[String]
    ) {
        Self::place_luxuries_at_major_civ_start_locations(regions, tile_map, ruleset, tile_data, random_luxuries);
        Self::place_luxuries_at_minor_civ_start_locations(tile_map, ruleset, regions, random_luxuries, city_state_luxuries, tile_data);
        Self::add_regional_luxuries(tile_data, regions, tile_map, ruleset);
        Self::add_random_luxuries(random_luxuries, tile_data, tile_map, regions, ruleset);

        let special_luxuries: Vec<&TileResource> = ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Luxury &&
                r.has_unique(UniqueType::LuxurySpecialPlacement))
            .collect();

        let mut placed_specials: HashMap<String, i32> = HashMap::new();
        for luxury in &special_luxuries {
            placed_specials.insert(luxury.name.clone(), 0);
        }

        Self::add_extra_luxury_to_starts(
            tile_map,
            regions,
            random_luxuries,
            &special_luxuries,
            city_state_luxuries,
            tile_data,
            ruleset,
            &mut placed_specials
        );

        Self::fill_special_luxuries(&special_luxuries, tile_map, regions, &mut placed_specials, tile_data);
    }

    /// Top up marble-type specials if needed
    fn fill_special_luxuries(
        special_luxuries: &[&TileResource],
        tile_map: &TileMap,
        regions: &[Region],
        placed_specials: &mut HashMap<String, i32>,
        tile_data: &mut MapGenTileData
    ) {
        for special in special_luxuries {
            let target_number = (regions.len() as f32 * tile_map.map_parameters.get_map_resources().special_luxuries_target_factor) as i32;
            let number_to_place = 2.max(target_number - placed_specials[&special.name]);

            let mut world_tiles: Vec<&Tile> = tile_map.values().collect();
            world_tiles.shuffle(&mut rand::thread_rng());

            MapRegionResources::try_adding_resource_to_tiles(
                tile_data,
                special,
                number_to_place,
                &world_tiles,
                1.0,
                true,
                6,
                0
            );
        }
    }

    fn add_extra_luxury_to_starts(
        tile_map: &TileMap,
        regions: &[Region],
        random_luxuries: &[String],
        special_luxuries: &[&TileResource],
        city_state_luxuries: &[String],
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset,
        placed_specials: &mut HashMap<String, i32>
    ) {
        if tile_map.map_parameters.map_resources == MapResourceSetting::Sparse {
            return;
        }

        for region in regions {
            if let Some(start_position) = region.start_position {
                let tiles_to_check: Vec<&Tile> = tile_map[&start_position].get_tiles_in_distance_range(1..=2).collect();

                let mut candidate_luxuries: Vec<String> = random_luxuries.iter().cloned().collect();
                candidate_luxuries.shuffle(&mut rand::thread_rng());

                if !tile_map.map_parameters.get_strategic_balance() {
                    let mut special_names: Vec<String> = special_luxuries.iter().map(|r| r.name.clone()).collect();
                    special_names.shuffle(&mut rand::thread_rng());
                    candidate_luxuries.extend(special_names);
                }

                let mut city_state_names: Vec<String> = city_state_luxuries.iter().cloned().collect();
                city_state_names.shuffle(&mut rand::thread_rng());
                candidate_luxuries.extend(city_state_names);

                let mut region_luxuries: Vec<String> = regions.iter()
                    .filter_map(|r| r.luxury.clone())
                    .collect();
                region_luxuries.shuffle(&mut rand::thread_rng());
                candidate_luxuries.extend(region_luxuries);

                for luxury_name in candidate_luxuries {
                    if let Some(luxury) = ruleset.tile_resources.get(&luxury_name) {
                        if MapRegionResources::try_adding_resource_to_tiles(
                            tile_data,
                            luxury,
                            1,
                            &tiles_to_check
                        ) > 0 {
                            if placed_specials.contains_key(&luxury_name) {
                                *placed_specials.get_mut(&luxury_name).unwrap() += 1;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    fn add_random_luxuries(
        random_luxuries: &[String],
        tile_data: &mut MapGenTileData,
        tile_map: &TileMap,
        regions: &[Region],
        ruleset: &Ruleset
    ) {
        if random_luxuries.is_empty() {
            return;
        }

        let mut target_random_luxuries = (tile_data.size as f32).powf(0.45) as i32; // Approximately
        target_random_luxuries *= tile_map.map_parameters.get_map_resources().random_luxuries_percent;
        target_random_luxuries /= 100;
        target_random_luxuries += rand::random::<i32>() % regions.len() as i32; // Add random number based on number of civs

        let minimum_random_luxuries = (tile_data.size as f32).powf(0.2) as i32; // Approximately

        let mut world_tiles: Vec<&Tile> = tile_map.values().collect();
        world_tiles.shuffle(&mut rand::thread_rng());

        let mut shuffled_luxuries: Vec<String> = random_luxuries.iter().cloned().collect();
        shuffled_luxuries.shuffle(&mut rand::thread_rng());

        for (index, luxury_name) in shuffled_luxuries.iter().enumerate() {
            let target_for_this_luxury = if random_luxuries.len() > 8 {
                target_random_luxuries / 10
            } else {
                let minimum = 3.max(minimum_random_luxuries - index as i32);
                minimum.max(
                    (target_random_luxuries as f32 * MapRegions::random_luxury_ratios[random_luxuries.len()][index] + 0.5) as i32
                )
            };

            if let Some(luxury) = ruleset.tile_resources.get(luxury_name) {
                MapRegionResources::try_adding_resource_to_tiles(
                    tile_data,
                    luxury,
                    target_for_this_luxury,
                    &world_tiles,
                    0.25,
                    true,
                    4,
                    2
                );
            }
        }
    }

    fn add_regional_luxuries(
        tile_data: &mut MapGenTileData,
        regions: &[Region],
        tile_map: &TileMap,
        ruleset: &Ruleset
    ) {
        let ideal_civs_for_map_size = 2.max(tile_data.size / 500);
        let mut region_target_number = (tile_data.size / 600) as i32 -
            (0.3 * (regions.len() as i32 - ideal_civs_for_map_size).abs()) as i32;

        region_target_number += tile_map.map_parameters.get_map_resources().regional_luxuries_delta;
        region_target_number = 1.max(region_target_number);

        for region in regions {
            if let Some(luxury_name) = &region.luxury {
                if let Some(resource) = ruleset.tile_resources.get(luxury_name) {
                    let candidates = if is_water_only_resource(resource, ruleset) {
                        tile_map.get_tiles_in_rectangle(&region.rect)
                            .filter(|t| t.is_water && t.neighbors().iter().any(|n| n.get_continent() == region.continent_id))
                            .collect()
                    } else {
                        region.tiles.iter().collect()
                    };

                    let mut shuffled_candidates: Vec<&Tile> = candidates;
                    shuffled_candidates.shuffle(&mut rand::thread_rng());

                    MapRegionResources::try_adding_resource_to_tiles(
                        tile_data,
                        resource,
                        region_target_number,
                        &shuffled_candidates,
                        0.4,
                        true,
                        4,
                        2
                    );
                }
            }
        }
    }

    fn place_luxuries_at_minor_civ_start_locations(
        tile_map: &TileMap,
        ruleset: &Ruleset,
        regions: &[Region],
        random_luxuries: &[String],
        city_state_luxuries: &[String],
        tile_data: &mut MapGenTileData
    ) {
        for (nation_id, start_locations) in &tile_map.starting_locations_by_nation {
            if let Some(nation) = ruleset.nations.get(nation_id) {
                if nation.is_city_state {
                    if let Some(start_location) = start_locations.first() {
                        let region = regions.iter().find(|r| r.tiles.contains(start_location));

                        let tiles_to_check: Vec<&Tile> = start_location.get_tiles_in_distance_range(1..=2).collect();

                        // 75% probability that we first attempt to place a "city state" luxury, then a random or regional one
                        // 25% probability of going the other way around
                        let mut global_luxuries: Vec<String> = if let Some(region) = region {
                            if let Some(region_luxury) = &region.luxury {
                                let mut luxuries: Vec<String> = random_luxuries.iter().cloned().collect();
                                luxuries.push(region_luxury.clone());
                                luxuries
                            } else {
                                random_luxuries.iter().cloned().collect()
                            }
                        } else {
                            random_luxuries.iter().cloned().collect()
                        };

                        let mut city_state_names: Vec<String> = city_state_luxuries.iter().cloned().collect();

                        let candidate_luxuries = if rand::random::<i32>() % 100 >= 25 {
                            city_state_names.shuffle(&mut rand::thread_rng());
                            global_luxuries.shuffle(&mut rand::thread_rng());
                            city_state_names.iter().chain(global_luxuries.iter()).cloned().collect::<Vec<String>>()
                        } else {
                            global_luxuries.shuffle(&mut rand::thread_rng());
                            city_state_names.shuffle(&mut rand::thread_rng());
                            global_luxuries.iter().chain(city_state_names.iter()).cloned().collect::<Vec<String>>()
                        };

                        // Now try adding one until we are successful
                        for luxury_name in candidate_luxuries {
                            if let Some(luxury) = ruleset.tile_resources.get(&luxury_name) {
                                if MapRegionResources::try_adding_resource_to_tiles(
                                    tile_data,
                                    luxury,
                                    1,
                                    &tiles_to_check
                                ) > 0 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn place_luxuries_at_major_civ_start_locations(
        regions: &[Region],
        tile_map: &TileMap,
        ruleset: &Ruleset,
        tile_data: &mut MapGenTileData,
        random_luxuries: &[String]
    ) {
        let average_fertility_density = regions.iter()
            .map(|r| r.total_fertility)
            .sum::<f32>() / regions.iter()
            .map(|r| r.tiles.len() as f32)
            .sum::<f32>();

        for region in regions {
            let mut target_luxuries = 2;
            if tile_map.map_parameters.get_legendary_start() {
                target_luxuries += 1;
            }

            if region.total_fertility / region.tiles.len() as f32 < average_fertility_density {
                target_luxuries += 1;
            }

            if let Some(luxury_name) = &region.luxury {
                if let Some(luxury_to_place) = ruleset.tile_resources.get(luxury_name) {
                    // First check 2 inner rings
                    let mut first_pass: Vec<&Tile> = tile_map[&region.start_position.unwrap()]
                        .get_tiles_in_distance_range(1..=2)
                        .collect();

                    first_pass.shuffle(&mut rand::thread_rng());
                    first_pass.sort_by(|a, b| {
                        a.get_tile_fertility(false).partial_cmp(&b.get_tile_fertility(false)).unwrap()
                    });

                    target_luxuries -= MapRegionResources::try_adding_resource_to_tiles(
                        tile_data,
                        luxury_to_place,
                        target_luxuries,
                        &first_pass,
                        0.5
                    );

                    if target_luxuries > 0 {
                        let mut second_pass: Vec<&Tile> = first_pass;
                        let mut third_ring: Vec<&Tile> = tile_map[&region.start_position.unwrap()]
                            .get_tiles_at_distance(3)
                            .collect();

                        third_ring.shuffle(&mut rand::thread_rng());
                        third_ring.sort_by(|a, b| {
                            a.get_tile_fertility(false).partial_cmp(&b.get_tile_fertility(false)).unwrap()
                        });

                        second_pass.extend(third_ring);

                        target_luxuries -= MapRegionResources::try_adding_resource_to_tiles(
                            tile_data,
                            luxury_to_place,
                            target_luxuries,
                            &second_pass,
                            1.0
                        );
                    }

                    if target_luxuries > 0 {
                        // Try adding in 1 luxury from the random rotation as compensation
                        for luxury_name in random_luxuries {
                            if let Some(luxury) = ruleset.tile_resources.get(luxury_name) {
                                if MapRegionResources::try_adding_resource_to_tiles(
                                    tile_data,
                                    luxury,
                                    1,
                                    &first_pass
                                ) > 0 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_region_priority(terrain: &Terrain) -> f32 {
        // Higher values mean higher priority
        match terrain.terrain_type {
            TerrainType::Land => 1.0,
            TerrainType::Water => 0.0,
            TerrainType::TerrainFeature => 0.5,
            _ => 0.0
        }
    }
}