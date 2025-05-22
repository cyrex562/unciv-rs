use std::collections::HashMap;
use std::f32;
use crate::map::tile_map::TileMap;
use crate::map::mapgenerator::mapregions::*;
use crate::map::mapgenerator::mapregions::MapRegions::base_minor_deposit_frequency;
use crate::map::mapgenerator::mapregions::ImpactType;
use crate::map::mapgenerator::mapregions::anonymize_unique;
use crate::map::mapgenerator::mapregions::get_terrain_rule;
use crate::map::mapgenerator::mapregions::is_water_only_resource;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::tile::TileResource;
use crate::models::ruleset::unique::StateForConditionals;
use crate::models::ruleset::unique::Unique;
use crate::models::ruleset::unique::UniqueType;
use crate::unique::state_for_conditionals::StateForConditionals;
use crate::utils::random_weighted;
use crate::map::mapgenerator::resourceplacement::map_region_resources::MapRegionResources;

/// Logic for placing strategic and bonus resources on the map
pub struct StrategicBonusResourcePlacementLogic;

impl StrategicBonusResourcePlacementLogic {
    /// Places strategic and bonus resources on the map
    ///
    /// There are a couple competing/complementary distribution systems at work here. First, major
    /// deposits are placed according to a frequency defined in the terrains themselves, for each
    /// tile that is eligible to get a major deposit, there is a weighted random choice between
    /// resource types.
    /// Minor deposits are placed by randomly picking a number of land tiles from anywhere on the
    /// map (so not stratified by terrain type) and assigning a weighted randomly picked resource.
    /// Bonuses are placed according to a frequency for a rule like "every 8 jungle hills", here
    /// implemented as a conditional.
    ///
    /// We need to build lists of all tiles following a given rule to place these, which is BY FAR
    /// the most expensive calculation in this entire class. To save some time we anonymize the
    /// uniques so we only have to make one list for each set of conditionals, so eg Wheat and
    /// Horses can share a list since they are both interested in Featureless Plains.
    /// We also save a list of all land tiles for minor deposit generation.
    pub fn place_strategic_and_bonuses(
        tile_map: &TileMap,
        regions: &mut Vec<Region>,
        tile_data: &mut MapGenTileData
    ) {
        let ruleset = &tile_map.ruleset;
        let strategic_resources: Vec<&TileResource> = ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Strategic)
            .collect();

        // As usual, if there are any relevant json definitions, assume they are complete
        let fallback_strategic = ruleset.tile_resources.values()
            .filter(|r| {
                r.resource_type == ResourceType::Strategic &&
                (r.has_unique(UniqueType::ResourceWeighting) ||
                r.has_unique(UniqueType::MinorDepositWeighting))
            })
            .count() == 0;

        // Determines number tiles per resource
        let bonus_multiplier = tile_map.map_parameters.get_map_resources().bonus_frequency_multiplier;
        let mut land_list: Vec<&Tile> = Vec::new(); // For minor deposits

        // Maps resource uniques for determining frequency/weighting/size to relevant tiles
        let mut rule_lists = Self::build_rule_lists(ruleset, tile_map, regions, fallback_strategic, &strategic_resources); // For rule-based generation

        // Now go through the entire map to build lists
        let mut shuffled_tiles: Vec<&Tile> = tile_map.values().collect();
        shuffled_tiles.shuffle(&mut rand::thread_rng());

        for tile in shuffled_tiles {
            let terrain_conditional = StateForConditionals::new { 
                attacked_tile: Some(tile.clone()),
                region: regions.iter().find(|r| r.tiles.contains(tile)).cloned()
             };

            if tile.get_base_terrain().has_unique(UniqueType::BlocksResources, &terrain_conditional) {
                continue; // Don't count snow hills
            }

            if tile.is_land {
                land_list.push(tile);
            }

            for (rule, list) in &mut rule_lists {
                if rule.conditionals_apply(&terrain_conditional) {
                    list.push(tile);
                }
            }
        }

        // Keep track of total placed strategic resources in case we need to top them up later
        let mut total_placed: HashMap<&TileResource, i32> = HashMap::new();
        for resource in &strategic_resources {
            total_placed.insert(resource, 0);
        }

        Self::place_major_deposits_on_land(ruleset, &rule_lists, &mut total_placed, tile_data, fallback_strategic);
        Self::place_small_deposits_of_modern_strategic_resources_on_city_states(ruleset, &strategic_resources, tile_map, &mut total_placed, tile_data);
        Self::place_minor_deposits_on_land(bonus_multiplier, &land_list, tile_data, &strategic_resources, fallback_strategic, &mut total_placed);
        Self::place_major_deposits_on_water(ruleset, &rule_lists, &mut total_placed, tile_data, fallback_strategic);
        Self::ensure_minimum_resources_per_civ(&strategic_resources, regions, &mut total_placed, ruleset, &land_list, tile_map, tile_data);
        Self::place_bonus_resources(ruleset, &rule_lists, tile_data, bonus_multiplier, tile_map);
        Self::place_bonus_in_third_ring_of_start(regions, ruleset, tile_map, tile_data);
    }

    fn place_bonus_in_third_ring_of_start(
        regions: &[Region],
        ruleset: &Ruleset,
        tile_map: &TileMap,
        tile_data: &mut MapGenTileData
    ) {
        for region in regions {
            let terrain = if region.type_name == "Hybrid" {
                region.terrain_counts.iter()
                    .filter(|(k, _)| *k != "Coastal")
                    .max_by_key(|(_, v)| *v)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default()
            } else {
                region.type_name.clone()
            };

            let resource_unique = ruleset.terrains.get(&terrain)
                .and_then(|t| t.get_matching_uniques(UniqueType::RegionExtraResource).first().cloned());

            // If this region has an explicit "this is the bonus" unique go with that, else random appropriate
            let resource = if let Some(unique) = resource_unique {
                if let Some(resource_name) = unique.params.get(0) {
                    ruleset.tile_resources.get(resource_name).cloned()
                } else {
                    None
                }
            } else {
                let possible_resources: Vec<&TileResource> = ruleset.tile_resources.values()
                    .filter(|r| r.resource_type == ResourceType::Bonus && r.terrains_can_be_found_on.contains(&terrain))
                    .collect();

                if possible_resources.is_empty() {
                    continue;
                }

                Some(possible_resources.choose(&mut rand::thread_rng()).unwrap())
            };

            if let Some(resource) = resource {
                if let Some(start_position) = &region.start_position {
                    let mut candidate_tiles: Vec<&Tile> = tile_map[&start_position].get_tiles_at_distance(3).collect();
                    candidate_tiles.shuffle(&mut rand::thread_rng());

                    let amount = if resource_unique.is_some() { 2 } else { 1 }; // Place an extra if the region type requests it

                    if MapRegionResources::try_adding_resource_to_tiles(
                        tile_data,
                        resource,
                        amount,
                        &candidate_tiles,
                        1.0,
                        false,
                        -1,
                        0,
                        false
                    ) == 0 {
                        // We couldn't place any, try adding a fish instead
                        let fishy_bonus: Option<&TileResource> = ruleset.tile_resources.values()
                            .filter(|r| {
                                r.resource_type == ResourceType::Bonus &&
                                r.terrains_can_be_found_on.iter().any(|terrain_name| {
                                    ruleset.terrains.get(terrain_name)
                                        .map_or(false, |t| t.terrain_type == TerrainType::Water)
                                })
                            })
                            .choose(&mut rand::thread_rng());

                        if let Some(fish) = fishy_bonus {
                            MapRegionResources::try_adding_resource_to_tiles(
                                tile_data,
                                fish,
                                1,
                                &candidate_tiles,
                                1.0,
                                false,
                                -1,
                                0,
                                false
                            );
                        }
                    }
                }
            }
        }
    }

    fn place_bonus_resources(
        ruleset: &Ruleset,
        rule_lists: &HashMap<&Unique, Vec<&Tile>>,
        tile_data: &mut MapGenTileData,
        bonus_multiplier: f32,
        tile_map: &TileMap
    ) {
        // Figure out if bonus generation rates are defined in json. Assume that if there are any, the definitions are complete.
        let use_fallback_bonuses = ruleset.tile_resources.values()
            .filter(|r| r.has_unique(UniqueType::ResourceFrequency))
            .count() == 0;

        // Water-based bonuses go last and have extra impact, because coasts are very common and we don't want too much clustering
        let mut sorted_resource_list: Vec<&TileResource> = ruleset.tile_resources.values().collect();
        sorted_resource_list.sort_by(|a, b| {
            let a_is_water = is_water_only_resource(a, ruleset);
            let b_is_water = is_water_only_resource(b, ruleset);
            a_is_water.partial_cmp(&b_is_water).unwrap()
        });

        for resource in sorted_resource_list {
            let extra_impact = if is_water_only_resource(resource, ruleset) { 1 } else { 0 };

            for rule in resource.unique_objects.iter().filter(|u| u.type_name == UniqueType::ResourceFrequency) {
                // Figure out which list applies, if any
                let simple_rule = anonymize_unique(rule);
                let list = rule_lists.iter()
                    .find(|(k, _)| k.text == simple_rule.text)
                    .map(|(_, v)| v);

                // If there is no matching list, it is because the rule was determined to be impossible and so can be safely skipped
                if let Some(list) = list {
                    // Place the resources
                    MapRegionResources::place_resources_in_tiles(
                        tile_data,
                        (rule.params[0].parse::<f32>().unwrap_or(1.0) * bonus_multiplier) as i32,
                        list,
                        &[resource],
                        0 + extra_impact,
                        2 + extra_impact,
                        false
                    );
                }
            }

            if use_fallback_bonuses && resource.resource_type == ResourceType::Bonus {
                // Since we haven't been able to generate any rule-based lists, just generate new ones on the fly
                // Increase impact to avoid clustering since there is no terrain type stratification.
                let mut fallback_list: Vec<&Tile> = tile_map.values()
                    .filter(|t| resource.terrains_can_be_found_on.contains(&t.last_terrain.name))
                    .collect();

                fallback_list.shuffle(&mut rand::thread_rng());

                MapRegionResources::place_resources_in_tiles(
                    tile_data,
                    (20.0 * bonus_multiplier) as i32,
                    &fallback_list,
                    &[resource],
                    2 + extra_impact,
                    2 + extra_impact,
                    false
                );
            }
        }
    }

    /// Place up to 2 extra deposits of each resource type if there is < 1 per civ
    fn ensure_minimum_resources_per_civ(
        strategic_resources: &[&TileResource],
        regions: &[Region],
        total_placed: &mut HashMap<&TileResource, i32>,
        ruleset: &Ruleset,
        land_list: &[&Tile],
        tile_map: &TileMap,
        tile_data: &mut MapGenTileData
    ) {
        for resource in strategic_resources {
            let extra_needed = 2.min(regions.len() as i32 - total_placed[resource]);

            if extra_needed > 0 {
                let tiles_to_add_to = if !is_water_only_resource(resource, ruleset) {
                    land_list.iter().cloned().collect::<Vec<&Tile>>()
                } else {
                    tile_map.values()
                        .filter(|t| t.is_water)
                        .collect::<Vec<&Tile>>()
                };

                let mut shuffled_tiles = tiles_to_add_to;
                shuffled_tiles.shuffle(&mut rand::thread_rng());

                let placed = MapRegionResources::try_adding_resource_to_tiles(
                    tile_data,
                    resource,
                    extra_needed,
                    &shuffled_tiles,
                    1.0,
                    true,
                    -1,
                    0,
                    false
                );

                *total_placed.get_mut(resource).unwrap() += placed;
            }
        }
    }

    // Extra impact because we don't want them too clustered and there is usually lots to go around
    fn place_major_deposits_on_water(
        ruleset: &Ruleset,
        rule_lists: &HashMap<&Unique, Vec<&Tile>>,
        total_placed: &mut HashMap<&TileResource, i32>,
        tile_data: &mut MapGenTileData,
        fallback_strategic: bool
    ) {
        for terrain in ruleset.terrains.values().filter(|t| t.terrain_type == TerrainType::Water) {
            // Figure out if we generated a list for this terrain
            let terrain_rule = get_terrain_rule(terrain, ruleset);
            let list = rule_lists.iter()
                .find(|(k, _)| k.text == terrain_rule.text)
                .map(|(_, v)| v);

            if let Some(list) = list {
                let placed = MapRegionResources::place_major_deposits(
                    tile_data,
                    ruleset,
                    list,
                    terrain,
                    fallback_strategic,
                    4,
                    3
                );

                for (resource, count) in placed {
                    *total_placed.get_mut(resource).unwrap() += count;
                }
            }
        }
    }

    fn place_minor_deposits_on_land(
        bonus_multiplier: f32,
        land_list: &[&Tile],
        tile_data: &mut MapGenTileData,
        strategic_resources: &[&TileResource],
        fallback_strategic: bool,
        total_placed: &mut HashMap<&TileResource, i32>
    ) {
        let frequency = (base_minor_deposit_frequency * bonus_multiplier) as i32;
        let minor_deposits_to_add = (land_list.len() / frequency as usize) + 1; // I sometimes have division by zero errors on this line
        let mut minor_deposits_added = 0;

        for tile in land_list {
            if tile.resource.is_some() || tile_data.get(&tile.position)
                .map_or(false, |impacts| impacts.impacts.contains_key(&ImpactType::Strategic)) {
                continue;
            }

            let conditional_terrain = StateForConditionals::new { attacked_tile: Some(tile.clone()) };

            if tile.get_base_terrain().has_unique(UniqueType::BlocksResources, &conditional_terrain) {
                continue;
            }

            let weightings: Vec<f32> = strategic_resources.iter()
                .map(|r| {
                    if fallback_strategic {
                        if r.generates_naturally_on(tile) { 1.0 } else { 0.0 }
                    } else {
                        let uniques = r.get_matching_uniques(UniqueType::MinorDepositWeighting, &conditional_terrain);
                        uniques.iter()
                            .map(|unique| unique.params[0].parse::<i32>().unwrap_or(0) as f32)
                            .sum()
                    }
                })
                .collect();

            if weightings.iter().sum::<f32>() <= 0.0 {
                continue;
            }

            let resource_to_place = random_weighted(strategic_resources, |r| {
                weightings[strategic_resources.iter().position(|x| x == r).unwrap()]
            });

            tile.set_tile_resource(resource_to_place, false);
            tile_data.place_impact(ImpactType::Strategic, tile, rand::random::<i32>() % 2 + rand::random::<i32>() % 2);
            *total_placed.get_mut(resource_to_place).unwrap() += 1;
            minor_deposits_added += 1;

            if minor_deposits_added >= minor_deposits_to_add {
                break;
            }
        }
    }

    fn place_small_deposits_of_modern_strategic_resources_on_city_states(
        ruleset: &Ruleset,
        strategic_resources: &[&TileResource],
        tile_map: &TileMap,
        total_placed: &mut HashMap<&TileResource, i32>,
        tile_data: &mut MapGenTileData
    ) {
        let last_era = ruleset.eras.values()
            .map(|e| e.era_number)
            .max()
            .unwrap_or(0);

        let modern_options: Vec<&TileResource> = strategic_resources.iter()
            .filter(|r| {
                if let Some(revealed_by) = &r.revealed_by {
                    if let Some(tech) = ruleset.technologies.get(revealed_by) {
                        if let Some(era) = ruleset.eras.get(&tech.era()) {
                            era.era_number >= last_era / 2
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        if !modern_options.is_empty() {
            for (nation_id, start_locations) in &tile_map.starting_locations_by_nation {
                if let Some(nation) = ruleset.nations.get(nation_id) {
                    if nation.is_city_state {
                        if let Some(city_state_location) = start_locations.first() {
                            let resource_to_place = modern_options.choose(&mut rand::thread_rng()).unwrap();

                            let tiles = city_state_location.get_tiles_in_distance_range(1..=3);

                            let placed = MapRegionResources::try_adding_resource_to_tiles(
                                tile_data,
                                resource_to_place,
                                1,
                                &tiles,
                                1.0,
                                false,
                                -1,
                                0,
                                false
                            );

                            *total_placed.get_mut(resource_to_place).unwrap() += placed;
                        }
                    }
                }
            }
        }
    }

    fn place_major_deposits_on_land(
        ruleset: &Ruleset,
        rule_lists: &HashMap<&Unique, Vec<&Tile>>,
        total_placed: &mut HashMap<&TileResource, i32>,
        tile_data: &mut MapGenTileData,
        fallback_strategic: bool
    ) {
        for terrain in ruleset.terrains.values().filter(|t| t.terrain_type != TerrainType::Water) {
            // Figure out if we generated a list for this terrain
            let terrain_rule = get_terrain_rule(terrain, ruleset);
            let list = rule_lists.iter()
                .find(|(k, _)| k.text == terrain_rule.text)
                .map(|(_, v)| v);

            if let Some(list) = list {
                let placed = MapRegionResources::place_major_deposits(
                    tile_data,
                    ruleset,
                    list,
                    terrain,
                    fallback_strategic,
                    2,
                    2
                );

                for (resource, count) in placed {
                    *total_placed.get_mut(resource).unwrap() += count;
                }
            }
        }
    }

    fn build_rule_lists(
        ruleset: &Ruleset,
        tile_map: &TileMap,
        regions: &[Region],
        fallback_strategic: bool,
        strategic_resources: &[&TileResource]
    ) -> HashMap<&Unique, Vec<&Tile>> {
        let mut rule_lists: HashMap<&Unique, Vec<&Tile>> = HashMap::new(); // For rule-based generation

        // Figure out which rules (sets of conditionals) need lists built
        for resource in ruleset.tile_resources.values().filter(|r| {
            r.resource_type == ResourceType::Strategic ||
            r.resource_type == ResourceType::Bonus
        }) {
            for rule in resource.unique_objects.iter().filter(|unique| {
                unique.type_name == UniqueType::ResourceFrequency ||
                unique.type_name == UniqueType::ResourceWeighting ||
                unique.type_name == UniqueType::MinorDepositWeighting
            }) {
                // Weed out some clearly impossible rules straight away to save time later
                if rule.modifiers.iter().any(|conditional| {
                    (conditional.type_name == UniqueType::ConditionalOnWaterMaps && !tile_map.using_archipelago_regions()) ||
                    (conditional.type_name == UniqueType::ConditionalInRegionOfType &&
                        regions.iter().none(|r| r.type_name == conditional.params[0])) ||
                    (conditional.type_name == UniqueType::ConditionalInRegionExceptOfType &&
                        regions.iter().all(|r| r.type_name == conditional.params[0]))
                }) {
                    continue;
                }

                let simple_rule = anonymize_unique(rule);
                if !rule_lists.keys().any(|k| k.text == simple_rule.text) { // Need to do text comparison since the uniques will not be equal otherwise
                    rule_lists.insert(simple_rule, Vec::new());
                }
            }
        }

        // Make up some rules for placing strategics in a fallback situation
        if fallback_strategic {
            let interesting_terrains: Vec<&Terrain> = strategic_resources.iter()
                .flat_map(|r| r.terrains_can_be_found_on.iter())
                .filter_map(|name| ruleset.terrains.get(name))
                .collect();

            for terrain in interesting_terrains {
                let fallback_rule = if terrain.terrain_type == TerrainType::TerrainFeature {
                    Unique::new("RULE <in [${terrain.name}] tiles>")
                } else {
                    Unique::new("RULE <in [Featureless] [${terrain.name}] tiles>")
                };

                if !rule_lists.keys().any(|k| k.text == fallback_rule.text) { // Need to do text comparison since the uniques will not be equal otherwise
                    rule_lists.insert(fallback_rule, Vec::new());
                }
            }
        }

        rule_lists
    }
}