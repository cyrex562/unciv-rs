use std::collections::{HashMap, HashSet};
use std::f32;
use rand::Rng;
use crate::map::tile::Tile;
use crate::map::tile_map::TileMap;
use crate::map::map_shape::MapShape;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::{Unique, UniqueType};
use crate::models::metadata::GameParameters;
use crate::civilization::Civilization;
use crate::utils::log::{Log, Tag};
use crate::map::mapgenerator::mapregions::map_gen_tile_data::MapGenTileData;
use crate::map::mapgenerator::resourceplacement::luxury_resource_placement_logic::LuxuryResourcePlacementLogic;
use crate::map::mapgenerator::resourceplacement::strategic_bonus_resource_placement_logic::StrategicBonusResourcePlacementLogic;
use crate::constants::Constants;

/// Impact types for tile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpactType {
    Strategic,
    Luxury,
    Bonus,
    MinorCiv,
}

/// Bias types for region assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiasTypes {
    Coastal,
    Positive,
    Negative,
    Random,
    PositiveFallback,
}

/// A map of tile data indexed by position
pub struct TileDataMap {
    data: HashMap<(i32, i32), MapGenTileData>,
}

impl TileDataMap {
    /// Create a new empty TileDataMap
    pub fn new() -> Self {
        TileDataMap {
            data: HashMap::new(),
        }
    }

    /// Get tile data for a position
    pub fn get(&self, position: (i32, i32)) -> Option<&MapGenTileData> {
        self.data.get(&position)
    }

    /// Get mutable tile data for a position
    pub fn get_mut(&mut self, position: (i32, i32)) -> Option<&mut MapGenTileData> {
        self.data.get_mut(&position)
    }

    /// Insert tile data for a position
    pub fn insert(&mut self, position: (i32, i32), tile_data: MapGenTileData) {
        self.data.insert(position, tile_data);
    }

    /// Add numbers to tileData in a similar way to closeStartPenalty, but for different types
    pub fn place_impact(&mut self, impact_type: ImpactType, tile: &Tile, radius: i32) {
        // Epicenter
        if let Some(tile_data) = self.get_mut(tile.position) {
            tile_data.impacts.insert(impact_type, 99);
        }

        if radius <= 0 {
            return;
        }

        for ring in 1..=radius {
            let ring_value = radius - ring + 1;
            for outer_tile in tile.get_tiles_at_distance(ring) {
                if let Some(tile_data) = self.get_mut(outer_tile.position) {
                    if tile_data.impacts.contains_key(&impact_type) {
                        let current_value = tile_data.impacts.get(&impact_type).unwrap();
                        let new_value = (ring_value + 2).min(50).max(*current_value);
                        tile_data.impacts.insert(impact_type, new_value);
                    } else {
                        tile_data.impacts.insert(impact_type, ring_value);
                    }
                }
            }
        }
    }
}

/// Main class for managing map regions
pub struct MapRegions<'a> {
    ruleset: &'a Ruleset,
    regions: Vec<Region>,
    tile_data: TileDataMap,
}

impl<'a> MapRegions<'a> {
    /// Create a new MapRegions instance
    pub fn new(ruleset: &'a Ruleset) -> Self {
        MapRegions {
            ruleset,
            regions: Vec::new(),
            tile_data: TileDataMap::new(),
        }
    }

    /// Generate regions for the map
    pub fn generate_regions(&mut self, tile_map: &TileMap, num_regions: i32) {
        if num_regions <= 0 {
            return; // Don't bother about regions, probably map editor
        }

        if tile_map.continent_sizes.is_empty() {
            panic!("No Continents on this map!");
        }

        let radius = if tile_map.map_parameters.shape == MapShape::Hexagonal ||
                       tile_map.map_parameters.shape == MapShape::FlatEarth {
            tile_map.map_parameters.map_size.radius as f32
        } else {
            (tile_map.map_parameters.map_size.width / 2)
                .max(tile_map.map_parameters.map_size.height / 2) as f32
        };

        // A huge box including the entire map.
        let map_rect = Rectangle {
            x: -radius,
            y: -radius,
            width: radius * 2.0 + 1.0,
            height: radius * 2.0 + 1.0,
        };

        // Lots of small islands - just split up the map in rectangles while ignoring Continents
        // 25% is chosen as limit so Four Corners maps don't fall in this category
        if tile_map.using_archipelago_regions() {
            // Make a huge rectangle covering the entire map
            let mut huge_rect = Region::new(tile_map, map_rect, -1); // -1 meaning ignore continent data
            huge_rect.affected_by_world_wrap = false; // Might as well start at the seam
            huge_rect.update_tiles();
            self.divide_region(huge_rect, num_regions);
            return;
        }

        // Continents type - distribute civs according to total fertility, then split as needed
        let continents: Vec<i32> = tile_map.continent_sizes.keys().cloned().collect();
        let mut civs_added_to_continent: HashMap<i32, i32> = HashMap::new(); // Continent ID, civs added
        let mut continent_fertility: HashMap<i32, i32> = HashMap::new(); // Continent ID, total fertility
        // Keep track of the even-q columns each continent is at, to figure out if they wrap
        let mut continent_to_columns_its_in: HashMap<i32, HashSet<i32>> = HashMap::new();

        // Calculate continent fertilities and columns
        for tile in tile_map.values() {
            let continent = tile.get_continent();
            if continent != -1 {
                *continent_fertility.entry(continent).or_insert(0) +=
                    tile.get_tile_fertility(true);

                continent_to_columns_its_in
                    .entry(continent)
                    .or_insert_with(HashSet::new)
                    .insert(tile.get_column());
            }
        }

        // Assign regions to the best continents, giving half value for region #2 etc
        for _ in 0..num_regions {
            let best_continent = continents.iter()
                .max_by_key(|&continent| {
                    continent_fertility.get(continent).unwrap_or(&0) /
                    (1 + civs_added_to_continent.get(continent).unwrap_or(&0))
                })
                .unwrap();

            *civs_added_to_continent.entry(*best_continent).or_insert(0) += 1;
        }

        // Split up the continents
        for &continent in civs_added_to_continent.keys() {
            let mut continent_region = Region::new(tile_map, map_rect, continent);
            let cols = continent_to_columns_its_in.get(&continent).unwrap();

            // Set origin at the rightmost column which does not have a neighbor on the left
            continent_region.rect.x = cols.iter()
                .filter(|&&col| !cols.contains(&(col - 1)))
                .max()
                .unwrap_or(&0) as f32;

            continent_region.rect.width = cols.len() as f32;

            if tile_map.map_parameters.world_wrap {
                // Check if the continent is wrapping - if the leftmost col is not the one we set origin by
                if cols.iter().min().unwrap() < &(continent_region.rect.x as i32) {
                    continent_region.affected_by_world_wrap = true;
                }
            }

            continent_region.update_tiles();
            self.divide_region(continent_region, *civs_added_to_continent.get(&continent).unwrap());
        }
    }

    /// Recursive function, divides a region into num_divisions parts of equal-ish fertility
    fn divide_region(&mut self, region: Region, num_divisions: i32) {
        if num_divisions <= 1 {
            // We're all set, save the region and return
            self.regions.push(region);
            return;
        }

        let first_divisions = num_divisions / 2; // Since int division rounds down, works for all numbers
        let (first_region, second_region) = self.split_region(region, (100 * first_divisions) / num_divisions);
        self.divide_region(first_region, first_divisions);
        self.divide_region(second_region, num_divisions - first_divisions);
    }

    /// Splits a region in 2, with the first having first_percent of total fertility
    fn split_region(&self, region_to_split: Region, first_percent: i32) -> (Region, Region) {
        let target_fertility = (region_to_split.total_fertility * first_percent) / 100;

        let mut split_off_region = Region::new(
            region_to_split.tile_map,
            region_to_split.rect,
            region_to_split.continent_id
        );

        let wider_than_tall = region_to_split.rect.width > region_to_split.rect.height;

        let mut best_split_point = 1; // will be the size of the split-off region
        let mut closest_fertility = 0;
        let mut cumulative_fertility = 0;

        let highest_point_to_try = if wider_than_tall {
            region_to_split.rect.width as i32
        } else {
            region_to_split.rect.height as i32
        };

        let points_to_try = 1..=highest_point_to_try;
        let halfway_point = highest_point_to_try / 2;

        for split_point in points_to_try {
            let next_rect = if wider_than_tall {
                split_off_region.tile_map.get_tiles_in_rectangle(Rectangle {
                    x: split_off_region.rect.x + split_point as f32 - 1.0,
                    y: split_off_region.rect.y,
                    width: 1.0,
                    height: split_off_region.rect.height,
                })
            } else {
                split_off_region.tile_map.get_tiles_in_rectangle(Rectangle {
                    x: split_off_region.rect.x,
                    y: split_off_region.rect.y + split_point as f32 - 1.0,
                    width: split_off_region.rect.width,
                    height: 1.0,
                })
            };

            cumulative_fertility += if split_off_region.continent_id == -1 {
                next_rect.iter().map(|t| t.get_tile_fertility(false)).sum()
            } else {
                next_rect.iter()
                    .map(|t| if t.get_continent() == split_off_region.continent_id {
                        t.get_tile_fertility(true)
                    } else {
                        0
                    })
                    .sum()
            };

            // Better than last try?
            let best_split_point_fertility_delta_from_target = (closest_fertility - target_fertility).abs();
            let current_split_point_fertility_delta_from_target = (cumulative_fertility - target_fertility).abs();

            if current_split_point_fertility_delta_from_target < best_split_point_fertility_delta_from_target
                || (current_split_point_fertility_delta_from_target == best_split_point_fertility_delta_from_target // same fertility split but better 'amount of tiles' split
                    && (halfway_point - split_point).abs() < (halfway_point - best_split_point).abs()) { // current split point is closer to the halfway point
                best_split_point = split_point;
                closest_fertility = cumulative_fertility;
            }
        }

        if wider_than_tall {
            split_off_region.rect.width = best_split_point as f32;
            let mut region_to_split = region_to_split;
            region_to_split.rect.x = split_off_region.rect.x + split_off_region.rect.width;
            region_to_split.rect.width = region_to_split.rect.width - best_split_point as f32;
            split_off_region.update_tiles();
            region_to_split.update_tiles();
            (split_off_region, region_to_split)
        } else {
            split_off_region.rect.height = best_split_point as f32;
            let mut region_to_split = region_to_split;
            region_to_split.rect.y = split_off_region.rect.y + split_off_region.rect.height;
            region_to_split.rect.height = region_to_split.rect.height - best_split_point as f32;
            split_off_region.update_tiles();
            region_to_split.update_tiles();
            (split_off_region, region_to_split)
        }
    }

    /// Assign regions to civilizations
    pub fn assign_regions(&mut self, tile_map: &TileMap, civilizations: &[Civilization], game_parameters: &GameParameters) {
        if civilizations.is_empty() {
            return;
        }

        self.assign_region_types();

        // Generate tile data for all tiles
        for tile in tile_map.values() {
            let new_data = MapGenTileData::new(
                tile,
                self.regions.iter().find(|r| r.tiles.contains(tile)),
                self.ruleset
            );
            self.tile_data.insert(tile.position, new_data);
        }

        // Sort regions by fertility so the worse regions get to pick first
        let sorted_regions: Vec<_> = self.regions.iter()
            .sorted_by_key(|r| r.total_fertility)
            .collect();

        for region in &sorted_regions {
            RegionStartFinder::find_start(region, &mut self.tile_data);
        }

        for region in &self.regions {
            if let Some(start_pos) = region.start_position {
                StartNormalizer::normalize_start(
                    &tile_map.get_tile(start_pos),
                    tile_map,
                    &mut self.tile_data,
                    self.ruleset,
                    false
                );
            }
        }

        let civ_biases: HashMap<_, _> = civilizations.iter()
            .map(|civ| (civ, self.ruleset.nations.get(&civ.civ_name).unwrap().start_bias.clone()))
            .collect();

        // This ensures each civ can only be in one of the buckets
        let mut civs_by_bias_type: HashMap<BiasTypes, Vec<&Civilization>> = HashMap::new();

        for (civ, start_bias) in &civ_biases {
            let bias_type = if game_parameters.no_start_bias {
                BiasTypes::Random
            } else if start_bias.iter().any(|bias| bias.equals_placeholder_text("Avoid []")) {
                BiasTypes::Negative
            } else if start_bias.iter().any(|bias| bias == "Coast") {
                BiasTypes::Coastal
            } else if !start_bias.is_empty() {
                BiasTypes::Positive
            } else {
                BiasTypes::Random
            };

            civs_by_bias_type.entry(bias_type).or_insert_with(Vec::new).push(civ);
        }

        let coast_bias_civs = civs_by_bias_type.get(&BiasTypes::Coastal).unwrap_or(&Vec::new());
        let mut positive_bias_civs = civs_by_bias_type.get(&BiasTypes::Positive)
            .map(|civs| {
                let mut sorted = civs.clone();
                sorted.sort_by_key(|civ| civ_biases.get(civ).map(|b| b.len()).unwrap_or(0));
                sorted
            })
            .unwrap_or_else(Vec::new);

        let mut negative_bias_civs = civs_by_bias_type.get(&BiasTypes::Negative)
            .map(|civs| {
                let mut sorted = civs.clone();
                sorted.sort_by_key(|civ| -(civ_biases.get(civ).map(|b| b.len()).unwrap_or(0) as i32));
                sorted
            })
            .unwrap_or_else(Vec::new);

        let mut random_civs = civs_by_bias_type.get(&BiasTypes::Random)
            .map(|civs| civs.clone())
            .unwrap_or_else(Vec::new);

        let mut positive_bias_fallback_civs = Vec::new(); // Civs who couldn't get their desired region at first pass
        let mut unpicked_regions = self.regions.clone();

        // First assign coast bias civs
        for &civ in coast_bias_civs {
            // Try to find a coastal start, preferably a really coastal one
            let start_region = unpicked_regions.iter()
                .filter(|r| {
                    if let Some(start_pos) = r.start_position {
                        tile_map.get_tile(start_pos).is_coastal_tile()
                    } else {
                        false
                    }
                })
                .max_by_key(|r| r.terrain_counts.get("Coastal").unwrap_or(&0));

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Coastal, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            }

            // Else adjacent to a lake
            let start_region = unpicked_regions.iter()
                .filter(|r| {
                    if let Some(start_pos) = r.start_position {
                        let tile = tile_map.get_tile(start_pos);
                        tile.neighbors.iter().any(|neighbor|
                            neighbor.get_base_terrain().has_unique(UniqueType::FreshWater))
                    } else {
                        false
                    }
                })
                .max_by_key(|r| r.terrain_counts.get("Coastal").unwrap_or(&0));

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Coastal, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            }

            // Else adjacent to a river
            let start_region = unpicked_regions.iter()
                .filter(|r| {
                    if let Some(start_pos) = r.start_position {
                        tile_map.get_tile(start_pos).is_adjacent_to_river()
                    } else {
                        false
                    }
                })
                .max_by_key(|r| r.terrain_counts.get("Coastal").unwrap_or(&0));

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Coastal, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            }

            // Else at least close to a river
            let start_region = unpicked_regions.iter()
                .filter(|r| {
                    if let Some(start_pos) = r.start_position {
                        let tile = tile_map.get_tile(start_pos);
                        tile.neighbors.iter().any(|neighbor| neighbor.is_adjacent_to_river())
                    } else {
                        false
                    }
                })
                .max_by_key(|r| r.terrain_counts.get("Coastal").unwrap_or(&0));

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Coastal, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            }

            // Else pick a random region at the end
            self.log_assign_region(false, BiasTypes::Coastal, civ, None);
            random_civs.push(civ);
        }

        // Next do positive bias civs
        for &civ in &positive_bias_civs {
            // Try to find a start that matches any of the desired regions, ideally with lots of desired terrain
            let preferred = civ_biases.get(civ).unwrap();
            let start_region = unpicked_regions.iter()
                .filter(|r| preferred.contains(&r.r#type))
                .max_by_key(|r| {
                    r.terrain_counts.iter()
                        .filter(|(terrain, _)| preferred.contains(terrain.as_str()))
                        .map(|(_, count)| count)
                        .sum::<i32>()
                });

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Positive, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            } else if preferred.len() == 1 { // Civs with a single bias (only) get to look for a fallback region
                positive_bias_fallback_civs.push(civ);
            } else { // Others get random starts
                self.log_assign_region(false, BiasTypes::Positive, civ, None);
                random_civs.push(civ);
            }
        }

        // Do a second pass for fallback civs, choosing the region most similar to the desired type
        for &civ in &positive_bias_fallback_civs {
            let preferred_type = civ_biases.get(civ).unwrap().first().unwrap();
            let start_region = self.get_fallback_region(preferred_type, &unpicked_regions);
            self.log_assign_region(true, BiasTypes::PositiveFallback, civ, Some(&start_region));
            self.assign_civ_to_region(civ, &start_region);
            unpicked_regions.retain(|r| r != &start_region);
        }

        // Next do negative bias ones (ie "Avoid []")
        for &civ in &negative_bias_civs {
            let start_biases = civ_biases.get(civ).unwrap();
            let (avoid_bias, preferred): (Vec<_>, Vec<_>) = start_biases.iter()
                .partition(|bias| bias.equals_placeholder_text("Avoid []"));

            let avoided: Vec<_> = avoid_bias.iter()
                .map(|bias| bias.get_placeholder_parameters()[0].clone())
                .collect();

            // Try to find a region not of the avoided types, secondary sort by
            // least number of undesired terrains (weighed double) / most number of desired terrains
            let start_region = unpicked_regions.iter()
                .filter(|r| !avoided.contains(&r.r#type))
                .min_by_key(|r| {
                    2 * r.terrain_counts.iter()
                        .filter(|(terrain, _)| avoided.contains(terrain.as_str()))
                        .map(|(_, count)| count)
                        .sum::<i32>()
                    - r.terrain_counts.iter()
                        .filter(|(terrain, _)| preferred.contains(terrain.as_str()))
                        .map(|(_, count)| count)
                        .sum::<i32>()
                });

            if let Some(start_region) = start_region {
                self.log_assign_region(true, BiasTypes::Negative, civ, Some(start_region));
                self.assign_civ_to_region(civ, start_region);
                unpicked_regions.retain(|r| r != start_region);
                continue;
            } else {
                self.log_assign_region(false, BiasTypes::Negative, civ, None);
                random_civs.push(civ); // else pick a random region at the end
            }
        }

        // Finally assign the remaining civs randomly
        for &civ in &random_civs {
            // throws if regions.size < civilizations.size or if the assigning mismatched - leads to popup on newgame screen
            let mut rng = rand::thread_rng();
            let start_region = unpicked_regions.choose(&mut rng).unwrap();
            self.log_assign_region(true, BiasTypes::Random, civ, Some(start_region));
            self.assign_civ_to_region(civ, start_region);
            unpicked_regions.retain(|r| r != start_region);
        }
    }

    /// Sets region.type
    fn assign_region_types(&mut self) {
        let region_types: Vec<_> = self.ruleset.terrains.values()
            .filter(|terrain| get_region_priority(terrain).is_some())
            .sorted_by_key(|terrain| get_region_priority(terrain))
            .collect();

        for region in &mut self.regions {
            region.count_terrains();

            for terrain in &region_types {
                // Test exclusion criteria first
                if terrain.get_matching_uniques(UniqueType::RegionRequireFirstLessThanSecond).iter().any(|unique| {
                    region.get_terrain_amount(&unique.params[0]) >= region.get_terrain_amount(&unique.params[1])
                }) {
                    continue;
                }

                // Test inclusion criteria
                if terrain.get_matching_uniques(UniqueType::RegionRequirePercentSingleType).iter().any(|unique| {
                    region.get_terrain_amount(&unique.params[1]) >= (unique.params[0].parse::<i32>().unwrap() * region.tiles.len() as i32) / 100
                }) || terrain.get_matching_uniques(UniqueType::RegionRequirePercentTwoTypes).iter().any(|unique| {
                    region.get_terrain_amount(&unique.params[1]) + region.get_terrain_amount(&unique.params[2]) >=
                        (unique.params[0].parse::<i32>().unwrap() * region.tiles.len() as i32) / 100
                }) {
                    region.r#type = terrain.name.clone();
                    break;
                }
            }
        }
    }

    /// Log region assignment
    fn log_assign_region(&self, success: bool, start_bias_type: BiasTypes, civ: &Civilization, region: Option<&Region>) {
        if Log::backend().is_release() {
            return;
        }

        let log_civ = format!("{} ({})",
            civ.civ_name,
            self.ruleset.nations.get(&civ.civ_name).unwrap().start_bias.join(", ")
        );

        let msg = if success {
            format!("({:?}): {} to {:?}", start_bias_type, log_civ, region)
        } else {
            format!("no region ({:?}) found for {}", start_bias_type, log_civ)
        };

        Log::debug(Tag::new("assignRegions"), &msg);
    }

    /// Assign a civilization to a region
    fn assign_civ_to_region(&mut self, civ: &Civilization, region: &Region) {
        if let Some(start_pos) = region.start_position {
            let tile = region.tile_map.get_tile(start_pos);
            region.tile_map.add_starting_location(&civ.civ_name, tile);

            // Place impacts to keep city states etc at appropriate distance
            self.tile_data.place_impact(ImpactType::MinorCiv, tile, 6);
            self.tile_data.place_impact(ImpactType::Luxury, tile, 3);
            self.tile_data.place_impact(ImpactType::Strategic, tile, 0);
            self.tile_data.place_impact(ImpactType::Bonus, tile, 3);
        }
    }

    /// Get the region most similar to a region of type
    fn get_fallback_region(&self, r#type: &str, candidates: &[Region]) -> Region {
        candidates.iter()
            .max_by_key(|r| r.terrain_counts.get(r#type).unwrap_or(&0))
            .unwrap()
            .clone()
    }

    /// Place resources and minor civs
    pub fn place_resources_and_minor_civs(&mut self, tile_map: &TileMap, minor_civs: &[Civilization]) {
        self.place_natural_wonder_impacts(tile_map);

        let (city_state_luxuries, random_luxuries) = LuxuryResourcePlacementLogic::assign_luxuries(
            &self.regions,
            &mut self.tile_data,
            self.ruleset
        );

        MinorCivPlacer::place_minor_civs(
            &self.regions,
            tile_map,
            minor_civs,
            &mut self.tile_data,
            self.ruleset
        );

        LuxuryResourcePlacementLogic::place_luxuries(
            &self.regions,
            tile_map,
            &mut self.tile_data,
            self.ruleset,
            city_state_luxuries,
            random_luxuries
        );

        StrategicBonusResourcePlacementLogic::place_strategic_and_bonuses(
            tile_map,
            &self.regions,
            &mut self.tile_data
        );
    }

    /// Places impacts from NWs that have been generated just prior to this step.
    fn place_natural_wonder_impacts(&mut self, tile_map: &TileMap) {
        for tile in tile_map.values().filter(|t| t.is_natural_wonder()) {
            self.tile_data.place_impact(ImpactType::Bonus, tile, 1);
            self.tile_data.place_impact(ImpactType::Strategic, tile, 1);
            self.tile_data.place_impact(ImpactType::Luxury, tile, 1);
            self.tile_data.place_impact(ImpactType::MinorCiv, tile, 1);
        }
    }
}

/// Rectangle structure for region boundaries
#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Region structure for map generation
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub tile_map: TileMap,
    pub rect: Rectangle,
    pub continent_id: i32,
    pub affected_by_world_wrap: bool,
    pub tiles: Vec<Tile>,
    pub r#type: String,
    pub terrain_counts: HashMap<String, i32>,
    pub total_fertility: i32,
    pub start_position: Option<(i32, i32)>,
}

impl Region {
    /// Create a new Region
    pub fn new(tile_map: &TileMap, rect: Rectangle, continent_id: i32) -> Self {
        Region {
            tile_map: tile_map.clone(),
            rect,
            continent_id,
            affected_by_world_wrap: false,
            tiles: Vec::new(),
            r#type: String::new(),
            terrain_counts: HashMap::new(),
            total_fertility: 0,
            start_position: None,
        }
    }

    /// Update tiles in the region
    pub fn update_tiles(&mut self) {
        self.tiles = self.tile_map.get_tiles_in_rectangle(self.rect);
        self.count_terrains();
        self.calculate_total_fertility();
    }

    /// Count terrains in the region
    pub fn count_terrains(&mut self) {
        self.terrain_counts.clear();

        for tile in &self.tiles {
            for terrain in tile.all_terrains() {
                *self.terrain_counts.entry(terrain.name.clone()).or_insert(0) += 1;
            }
        }
    }

    /// Calculate total fertility of the region
    fn calculate_total_fertility(&mut self) {
        self.total_fertility = self.tiles.iter()
            .map(|tile| {
                if self.continent_id == -1 {
                    tile.get_tile_fertility(false)
                } else {
                    if tile.get_continent() == self.continent_id {
                        tile.get_tile_fertility(true)
                    } else {
                        0
                    }
                }
            })
            .sum();
    }

    /// Get terrain amount in the region
    pub fn get_terrain_amount(&self, terrain_name: &str) -> i32 {
        *self.terrain_counts.get(terrain_name).unwrap_or(&0)
    }
}

/// Extension trait for Tile to get tile fertility
pub trait TileFertilityExt {
    fn get_tile_fertility(&self, check_coasts: bool) -> i32;
}

impl TileFertilityExt for Tile {
    fn get_tile_fertility(&self, check_coasts: bool) -> i32 {
        let mut fertility = 0;

        for terrain in self.all_terrains() {
            if terrain.has_unique(UniqueType::OverrideFertility) {
                return terrain.get_matching_uniques(UniqueType::OverrideFertility)
                    .first()
                    .unwrap()
                    .params[0]
                    .parse::<i32>()
                    .unwrap();
            } else {
                fertility += terrain.get_matching_uniques(UniqueType::AddFertility)
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .sum::<i32>();
            }
        }

        if self.is_adjacent_to_river() {
            fertility += 1;
        }

        if self.is_adjacent_to(Constants::fresh_water) {
            fertility += 1; // meaning total +2 for river
        }

        if check_coasts && self.is_coastal_tile() {
            fertility += 2;
        }

        fertility
    }
}

/// Get region priority for a terrain
pub fn get_region_priority(terrain: &Terrain) -> Option<i32> {
    if terrain.is_none() { // ie "hybrid"
        return Some(99999); // a big number
    }

    if !terrain.has_unique(UniqueType::RegionRequirePercentSingleType)
        && !terrain.has_unique(UniqueType::RegionRequirePercentTwoTypes) {
        None
    } else if terrain.has_unique(UniqueType::RegionRequirePercentSingleType) {
        Some(terrain.get_matching_uniques(UniqueType::RegionRequirePercentSingleType)
            .first()
            .unwrap()
            .params[2]
            .parse::<i32>()
            .unwrap())
    } else {
        Some(terrain.get_matching_uniques(UniqueType::RegionRequirePercentTwoTypes)
            .first()
            .unwrap()
            .params[3]
            .parse::<i32>()
            .unwrap())
    }
}

/// Create a fake unique with the same conditionals, but sorted alphabetically.
/// Used to save some memory and time when building resource lists.
pub fn anonymize_unique(unique: &Unique) -> Unique {
    let mut modifiers = unique.modifiers.clone();
    modifiers.sort_by(|a, b| a.text.cmp(&b.text));

    Unique::new(
        format!("RULE{}",
            modifiers.iter()
                .map(|m| format!(" <{}>", m.text))
                .collect::<String>()
        )
    )
}

/// Check if a resource is water-only
pub fn is_water_only_resource(resource: &TileResource, ruleset: &Ruleset) -> bool {
    resource.terrains_can_be_found_on.iter()
        .all(|terrain_name| {
            ruleset.terrains.get(terrain_name)
                .map(|terrain| terrain.r#type == TerrainType::Water)
                .unwrap_or(false)
        })
}

/// Create a fake unique with conditionals that will satisfy the same conditions as terrainsCanBeFoundOn
pub fn get_terrain_rule(terrain: &Terrain, ruleset: &Ruleset) -> Unique {
    if terrain.r#type == TerrainType::TerrainFeature {
        if terrain.has_unique(UniqueType::VisibilityElevation) {
            Unique::new(format!("RULE <in [{}] tiles>", terrain.name))
        } else {
            let elevation_terrains: Vec<_> = ruleset.terrains.values()
                .filter(|t| t.r#type == TerrainType::TerrainFeature && t.has_unique(UniqueType::VisibilityElevation))
                .map(|t| format!("<in tiles without [{}]>", t.name))
                .collect();

            Unique::new(format!("RULE <in [{}] tiles> {}",
                terrain.name,
                elevation_terrains.join(" ")
            ))
        }
    } else {
        Unique::new(format!("RULE <in [Featureless] [{}] tiles>", terrain.name))
    }
}