use std::collections::{HashMap, HashSet};
use crate::map::tile::Tile;
use crate::map::tile_map::TileMap;
use crate::map::mapgenerator::map_generation_randomness::MapGenerationRandomness;
use crate::map::mapgenerator::map_parameters::MapParameters;
use crate::map::mapgenerator::map_regions::MapRegions;
use crate::map::mapgenerator::map_elevation_generator::MapElevationGenerator;
use crate::ruleset::Ruleset;
use crate::game::civilization::Civilization;
use crate::map::terrain::TerrainType;
use crate::map::mapgenerator::resource_placement::luxury_resource_placement_logic::LuxuryResourcePlacementLogic;
use crate::map::mapgenerator::resource_placement::strategic_bonus_resource_placement_logic::StrategicBonusResourcePlacementLogic;
use crate::map::resource::ResourceType;
use crate::map::mapgenerator::region_start_finder::RegionStartFinder;
use crate::map::mapgenerator::start_normalizer::StartNormalizer;
use crate::map::vector2::Vector2;
use crate::map::mapgenerator::map_type::MapType;

const WATER_PERCENTAGE_MODIFIER: f64 = 0.3;
const WATER_THRESHOLD: f64 = -0.1;
const COAST_THRESHOLD: f64 = 0.05;
const PANGAEA_CENTER_BIAS: f64 = 0.5;
const CONTINENT_SEPARATION: f64 = 0.3;
const ARCHIPELAGO_WATER_MODIFIER: f64 = 0.2;

/// Handles the generation of game maps, including terrain, resources, and starting positions
pub struct MapGenerator {
    pub map_parameters: MapParameters,
    pub ruleset: Ruleset,
    pub randomness: MapGenerationRandomness,
    pub map_regions: Option<MapRegions>,
    pub map_elevation_generator: MapElevationGenerator,
}

impl MapGenerator {
    /// Creates a new MapGenerator instance
    pub fn new(map_parameters: MapParameters, ruleset: Ruleset) -> Self {
        Self {
            map_parameters,
            ruleset,
            randomness: MapGenerationRandomness::new(),
            map_regions: None,
            map_elevation_generator: MapElevationGenerator::new(),
        }
    }

    /// Seeds the random number generator
    pub fn seed(&mut self, seed: u64) {
        self.randomness.seed_rng(seed);
    }

    /// Generates a new map based on the provided parameters
    pub fn generate_map(&mut self, civilizations: &[Civilization]) -> TileMap {
        let mut tile_map = self.create_empty_map();

        // Initialize map regions if needed
        if self.map_parameters.needs_map_regions() {
            self.map_regions = Some(MapRegions::new(&tile_map, &self.map_parameters));
        }

        // Generate basic terrain
        self.generate_basic_terrain(&mut tile_map);

        // Generate elevation (mountains and hills)
        self.map_elevation_generator.raise_mountains_and_hills(&mut tile_map, &mut self.randomness);

        // Place resources and features
        self.place_resources_and_features(&mut tile_map);

        // Place civilizations and city states
        if !civilizations.is_empty() {
            self.place_civilizations(&mut tile_map, civilizations);
        }

        tile_map
    }

    /// Creates an empty map with the specified dimensions
    fn create_empty_map(&self) -> TileMap {
        let radius = self.map_parameters.radius;
        TileMap::new(radius)
    }

    /// Generates the basic terrain for the map (water, land, etc.)
    fn generate_basic_terrain(&mut self, tile_map: &mut TileMap) {
        let water_threshold = self.calculate_water_threshold();
        let mut land_tiles = Vec::new();

        // First pass: Generate basic water/land distribution using Perlin noise
        for tile in tile_map.tiles.values_mut() {
            let noise_value = self.get_terrain_noise(tile);

            if noise_value > water_threshold {
                tile.terrain_type = TerrainType::Grassland;
                land_tiles.push(tile.position.clone());
            } else if noise_value > WATER_THRESHOLD {
                tile.terrain_type = TerrainType::Coast;
            } else {
                tile.terrain_type = TerrainType::Ocean;
            }
        }

        // Second pass: Ensure proper coastal tiles
        self.process_coastal_tiles(tile_map, &land_tiles);

        // Third pass: Apply map type specific modifications
        self.apply_map_type_modifications(tile_map);
    }

    /// Calculates the water threshold based on map parameters
    fn calculate_water_threshold(&self) -> f64 {
        let base_threshold = WATER_THRESHOLD;
        let water_modifier = match self.map_parameters.water_level {
            WaterLevel::High => WATER_PERCENTAGE_MODIFIER,
            WaterLevel::Low => -WATER_PERCENTAGE_MODIFIER,
            _ => 0.0,
        };
        base_threshold + water_modifier
    }

    /// Gets the terrain noise value for a tile
    fn get_terrain_noise(&mut self, tile: &Tile) -> f64 {
        // Use different seeds for different map types
        let seed = match self.map_parameters.map_type {
            MapType::Pangaea => 42.0,
            MapType::Continents => 123.0,
            MapType::Archipelago => 456.0,
            _ => 789.0,
        };

        self.randomness.get_perlin_noise_default(tile, seed, 30.0)
    }

    /// Processes coastal tiles to ensure proper water/land transitions
    fn process_coastal_tiles(&self, tile_map: &mut TileMap, land_tiles: &[Vector2]) {
        for position in land_tiles {
            let neighbors = tile_map.get_tile_neighbors(position);
            for neighbor in neighbors {
                if let Some(tile) = tile_map.tiles.get_mut(&neighbor) {
                    if tile.terrain_type == TerrainType::Ocean {
                        tile.terrain_type = TerrainType::Coast;
                    }
                }
            }
        }
    }

    /// Applies modifications based on the map type
    fn apply_map_type_modifications(&mut self, tile_map: &mut TileMap) {
        match self.map_parameters.map_type {
            MapType::Pangaea => self.modify_for_pangaea(tile_map),
            MapType::Continents => self.modify_for_continents(tile_map),
            MapType::Archipelago => self.modify_for_archipelago(tile_map),
            _ => {}
        }
    }

    /// Modifies the map for Pangaea type
    fn modify_for_pangaea(&mut self, tile_map: &mut TileMap) {
        // For Pangaea, we want to bias land towards the center
        let center = Vector2::new(0, 0);

        for tile in tile_map.tiles.values_mut() {
            let distance_to_center = tile.position.distance_to(&center) as f64 / self.map_parameters.radius as f64;
            let noise = self.randomness.get_perlin_noise_default(tile, 111.0, 30.0);

            // The further from center, the more likely to be water
            if distance_to_center > PANGAEA_CENTER_BIAS && noise < distance_to_center - PANGAEA_CENTER_BIAS {
                if tile.terrain_type != TerrainType::Ocean {
                    tile.terrain_type = TerrainType::Coast;
                }
            }
        }

        // Ensure coastal tiles around land masses
        self.process_coastal_tiles(tile_map, &tile_map.get_land_tiles());
    }

    /// Modifies the map for Continents type
    fn modify_for_continents(&mut self, tile_map: &mut TileMap) {
        // For continents, we want to create distinct landmasses
        let mut land_tiles = Vec::new();

        for tile in tile_map.tiles.values_mut() {
            let noise1 = self.randomness.get_perlin_noise_default(tile, 222.0, 40.0);
            let noise2 = self.randomness.get_perlin_noise_default(tile, 333.0, 20.0);

            // Create separation between continents using multiple noise channels
            if noise1 > CONTINENT_SEPARATION || noise2 > CONTINENT_SEPARATION {
                if tile.terrain_type != TerrainType::Ocean && tile.terrain_type != TerrainType::Coast {
                    land_tiles.push(tile.position.clone());
                }
            } else {
                tile.terrain_type = TerrainType::Ocean;
            }
        }

        // Process coastal tiles for each continent
        self.process_coastal_tiles(tile_map, &land_tiles);

        // Ensure continents are properly separated
        self.separate_continents(tile_map);
    }

    /// Modifies the map for Archipelago type
    fn modify_for_archipelago(&mut self, tile_map: &mut TileMap) {
        // For archipelago, we want many small islands
        let mut land_tiles = Vec::new();

        for tile in tile_map.tiles.values_mut() {
            let noise = self.randomness.get_perlin_noise_default(tile, 444.0, 10.0);

            // Create many small islands using high-frequency noise
            if noise > 1.0 - ARCHIPELAGO_WATER_MODIFIER {
                if tile.terrain_type != TerrainType::Ocean {
                    land_tiles.push(tile.position.clone());
                }
            } else {
                tile.terrain_type = TerrainType::Ocean;
            }
        }

        // Process coastal tiles for each island
        self.process_coastal_tiles(tile_map, &land_tiles);

        // Ensure islands are properly sized
        self.adjust_island_sizes(tile_map);
    }

    /// Separates continents by ensuring water channels between them
    fn separate_continents(&mut self, tile_map: &mut TileMap) {
        let mut continents = self.identify_continents(tile_map);

        // If we have too few or too many continents, adjust the separation
        let target_continents = 3; // Typical number for a continents map

        if continents.len() < target_continents {
            self.split_large_continents(tile_map, &mut continents);
        } else if continents.len() > target_continents + 1 {
            self.merge_small_continents(tile_map, &mut continents);
        }
    }

    /// Identifies distinct continents on the map
    fn identify_continents(&self, tile_map: &TileMap) -> Vec<HashSet<Vector2>> {
        let mut continents = Vec::new();
        let mut processed = HashSet::new();

        for tile in tile_map.tiles.values() {
            if tile.terrain_type != TerrainType::Ocean &&
               tile.terrain_type != TerrainType::Coast &&
               !processed.contains(&tile.position) {
                let mut continent = HashSet::new();
                self.flood_fill_continent(tile_map, &tile.position, &mut continent, &mut processed);
                if !continent.is_empty() {
                    continents.push(continent);
                }
            }
        }

        continents
    }

    /// Flood fills to find all connected land tiles
    fn flood_fill_continent(
        &self,
        tile_map: &TileMap,
        position: &Vector2,
        continent: &mut HashSet<Vector2>,
        processed: &mut HashSet<Vector2>
    ) {
        if processed.contains(position) {
            return;
        }

        processed.insert(position.clone());

        if let Some(tile) = tile_map.tiles.get(position) {
            if tile.terrain_type != TerrainType::Ocean && tile.terrain_type != TerrainType::Coast {
                continent.insert(position.clone());

                // Recursively process neighbors
                for neighbor in tile_map.get_tile_neighbors(position) {
                    self.flood_fill_continent(tile_map, &neighbor, continent, processed);
                }
            }
        }
    }

    /// Splits large continents by adding water channels
    fn split_large_continents(&mut self, tile_map: &mut TileMap, continents: &mut Vec<HashSet<Vector2>>) {
        for continent in continents.iter() {
            if continent.len() > (tile_map.tiles.len() / 3) {
                // Find a suitable split point and create a water channel
                let center = self.find_continent_center(continent);
                self.create_water_channel(tile_map, &center);
            }
        }
    }

    /// Finds the center point of a continent
    fn find_continent_center(&self, continent: &HashSet<Vector2>) -> Vector2 {
        let mut sum_x = 0;
        let mut sum_y = 0;

        for pos in continent {
            sum_x += pos.x;
            sum_y += pos.y;
        }

        Vector2::new(
            sum_x / continent.len() as i32,
            sum_y / continent.len() as i32
        )
    }

    /// Creates a water channel through the specified point
    fn create_water_channel(&mut self, tile_map: &mut TileMap, center: &Vector2) {
        let channel_width = 2;
        let channel_angle = self.randomness.rng.gen_range(0..360) as f64;

        for tile in tile_map.tiles.values_mut() {
            let dx = tile.position.x - center.x;
            let dy = tile.position.y - center.y;
            let distance = ((dx * dx + dy * dy) as f64).sqrt();

            // Create a channel at the specified angle
            let angle = (dy as f64).atan2(dx as f64).to_degrees();
            let angle_diff = (angle - channel_angle).abs() % 360.0;

            if angle_diff < 30.0 && distance < self.map_parameters.radius as f64 {
                if distance.abs() < channel_width as f64 {
                    tile.terrain_type = TerrainType::Ocean;
                } else if distance.abs() < (channel_width + 1) as f64 {
                    tile.terrain_type = TerrainType::Coast;
                }
            }
        }
    }

    /// Merges small continents by filling in water between them
    fn merge_small_continents(&mut self, tile_map: &mut TileMap, continents: &mut Vec<HashSet<Vector2>>) {
        // Sort continents by size
        continents.sort_by_key(|c| c.len());

        while continents.len() > 3 {
            if let Some(smallest) = continents.first() {
                // Find the nearest continent to merge with
                if let Some(nearest) = self.find_nearest_continent(smallest, continents) {
                    self.create_land_bridge(tile_map, smallest, nearest);
                }
            }

            // Remove the smallest continent as it's been merged
            continents.remove(0);
        }
    }

    /// Finds the nearest continent to the given one
    fn find_nearest_continent(&self, continent: &HashSet<Vector2>, continents: &[HashSet<Vector2>]) -> Option<&HashSet<Vector2>> {
        let center = self.find_continent_center(continent);

        continents.iter()
            .filter(|&c| c != continent)
            .min_by_key(|&c| {
                let other_center = self.find_continent_center(c);
                center.distance_to(&other_center)
            })
    }

    /// Creates a land bridge between two continents
    fn create_land_bridge(&mut self, tile_map: &mut TileMap, continent1: &HashSet<Vector2>, continent2: &HashSet<Vector2>) {
        let center1 = self.find_continent_center(continent1);
        let center2 = self.find_continent_center(continent2);

        // Create a path of land tiles between the centers
        let dx = center2.x - center1.x;
        let dy = center2.y - center1.y;
        let steps = dx.abs().max(dy.abs());

        for i in 0..=steps {
            let x = center1.x + (dx * i) / steps;
            let y = center1.y + (dy * i) / steps;
            let pos = Vector2::new(x, y);

            if let Some(tile) = tile_map.tiles.get_mut(&pos) {
                tile.terrain_type = TerrainType::Plains;
            }

            // Add some width to the bridge
            for neighbor in tile_map.get_tile_neighbors(&pos) {
                if let Some(tile) = tile_map.tiles.get_mut(&neighbor) {
                    if tile.terrain_type == TerrainType::Ocean {
                        tile.terrain_type = TerrainType::Coast;
                    }
                }
            }
        }
    }

    /// Adjusts island sizes for archipelago maps
    fn adjust_island_sizes(&mut self, tile_map: &mut TileMap) {
        let islands = self.identify_continents(tile_map);

        for island in islands {
            // Adjust island size based on desired distribution
            if island.len() < 3 {
                // Remove very small islands
                for pos in island {
                    if let Some(tile) = tile_map.tiles.get_mut(&pos) {
                        tile.terrain_type = TerrainType::Ocean;
                    }
                }
            } else if island.len() > 10 {
                // Reduce size of large islands
                self.erode_island_edges(tile_map, &island);
            }
        }
    }

    /// Erodes the edges of an island to reduce its size
    fn erode_island_edges(&mut self, tile_map: &mut TileMap, island: &HashSet<Vector2>) {
        let center = self.find_continent_center(island);

        for pos in island {
            if let Some(tile) = tile_map.tiles.get_mut(pos) {
                let distance_to_center = pos.distance_to(&center);
                let noise = self.randomness.get_perlin_noise_default(tile, 555.0, 10.0);

                // Erode edges based on distance from center and noise
                if distance_to_center > 3 && noise < 0.3 {
                    tile.terrain_type = TerrainType::Ocean;
                }
            }
        }
    }

    /// Places resources and features on the map
    fn place_resources_and_features(&mut self, tile_map: &mut TileMap) {
        // First place terrain features
        self.place_terrain_features(tile_map);

        // Then place resources
        self.place_resources(tile_map);
    }

    /// Places terrain features on the map
    fn place_terrain_features(&mut self, tile_map: &mut TileMap) {
        for tile in tile_map.tiles.values_mut() {
            // Skip water tiles for most features
            if tile.terrain_type == TerrainType::Ocean || tile.terrain_type == TerrainType::Coast {
                self.try_place_water_features(tile);
                continue;
            }

            // Place features based on terrain type and random chance
            match tile.terrain_type {
                TerrainType::Grassland => self.try_place_grassland_features(tile),
                TerrainType::Plains => self.try_place_plains_features(tile),
                TerrainType::Desert => self.try_place_desert_features(tile),
                TerrainType::Tundra => self.try_place_tundra_features(tile),
                TerrainType::Snow => self.try_place_snow_features(tile),
                _ => {}
            }
        }
    }

    /// Attempts to place features on water tiles
    fn try_place_water_features(&mut self, tile: &mut Tile) {
        let noise = self.randomness.get_perlin_noise_default(tile, 789.0, 20.0);
        if noise > 0.7 {
            tile.terrain_features.insert(TerrainFeature::IcePack);
        }
    }

    /// Attempts to place features on grassland tiles
    fn try_place_grassland_features(&mut self, tile: &mut Tile) {
        let noise = self.randomness.get_perlin_noise_default(tile, 123.0, 20.0);
        if noise > 0.6 {
            tile.terrain_features.insert(TerrainFeature::Forest);
        } else if noise < -0.6 {
            tile.terrain_features.insert(TerrainFeature::Marsh);
        }
    }

    /// Attempts to place features on plains tiles
    fn try_place_plains_features(&mut self, tile: &mut Tile) {
        let noise = self.randomness.get_perlin_noise_default(tile, 456.0, 20.0);
        if noise > 0.5 {
            tile.terrain_features.insert(TerrainFeature::Forest);
        }
    }

    /// Attempts to place features on desert tiles
    fn try_place_desert_features(&mut self, tile: &mut Tile) {
        let noise = self.randomness.get_perlin_noise_default(tile, 789.0, 20.0);
        if noise > 0.7 {
            tile.terrain_features.insert(TerrainFeature::Oasis);
        } else if noise < -0.6 {
            tile.terrain_features.insert(TerrainFeature::FloodPlains);
        }
    }

    /// Attempts to place features on tundra tiles
    fn try_place_tundra_features(&mut self, tile: &mut Tile) {
        let noise = self.randomness.get_perlin_noise_default(tile, 321.0, 20.0);
        if noise > 0.5 {
            tile.terrain_features.insert(TerrainFeature::Forest);
        }
    }

    /// Attempts to place features on snow tiles
    fn try_place_snow_features(&mut self, tile: &mut Tile) {
        // Snow tiles typically don't get features in base game
    }

    /// Places resources on the map
    fn place_resources(&mut self, tile_map: &mut TileMap) {
        // Create resource placement logic instances
        let mut luxury_placer = LuxuryResourcePlacementLogic::new(
            &self.ruleset,
            &self.map_parameters,
            &mut self.randomness
        );

        let mut strategic_bonus_placer = StrategicBonusResourcePlacementLogic::new(
            &self.ruleset,
            &self.map_parameters,
            &mut self.randomness
        );

        // Place strategic and bonus resources
        strategic_bonus_placer.place_strategic_and_bonuses(tile_map);

        // Place luxury resources if we have map regions
        if let Some(map_regions) = &mut self.map_regions {
            luxury_placer.place_luxuries(tile_map, map_regions);
        }

        // Place random resources in empty suitable tiles
        self.place_random_resources(tile_map);
    }

    /// Places random resources in empty suitable tiles
    fn place_random_resources(&mut self, tile_map: &mut TileMap) {
        for tile in tile_map.tiles.values_mut() {
            if tile.resource.is_none() && self.is_suitable_for_random_resource(tile) {
                let noise = self.randomness.get_perlin_noise_default(tile, 999.0, 20.0);
                if noise > 0.8 {
                    self.place_random_resource(tile);
                }
            }
        }
    }

    /// Checks if a tile is suitable for random resource placement
    fn is_suitable_for_random_resource(&self, tile: &Tile) -> bool {
        // Don't place on water or mountains
        if tile.terrain_type == TerrainType::Ocean || tile.terrain_type == TerrainType::Mountain {
            return false;
        }

        // Don't place on tiles that already have certain features
        if tile.terrain_features.contains(&TerrainFeature::Oasis) ||
           tile.terrain_features.contains(&TerrainFeature::NaturalWonder) {
            return false;
        }

        true
    }

    /// Places a random resource on a tile
    fn place_random_resource(&mut self, tile: &mut Tile) {
        let resource = match tile.terrain_type {
            TerrainType::Grassland => ResourceType::Cattle,
            TerrainType::Plains => ResourceType::Wheat,
            TerrainType::Desert => ResourceType::Sheep,
            TerrainType::Tundra => ResourceType::Deer,
            _ => return,
        };
        tile.resource = Some(resource);
    }

    /// Places civilizations and city states on the map
    fn place_civilizations(&mut self, tile_map: &mut TileMap, civilizations: &[Civilization]) {
        if let Some(map_regions) = &mut self.map_regions {
            // First assign regions to civilizations
            map_regions.assign_regions(civilizations);

            // Find starting positions for each civilization
            let mut start_finder = RegionStartFinder::new(&self.ruleset);
            let mut start_positions = HashMap::new();

            for (region_id, civ) in map_regions.region_to_civilization.iter() {
                if let Some(region) = map_regions.regions.get(region_id) {
                    if let Some(start_pos) = start_finder.find_start(tile_map, region) {
                        start_positions.insert(civ.clone(), start_pos);
                    }
                }
            }

            // Normalize starting positions to ensure balanced resources
            let mut start_normalizer = StartNormalizer::new(&self.ruleset);
            start_normalizer.normalize_starts(tile_map, &start_positions);

            // Place city states in remaining regions
            self.place_city_states(tile_map, map_regions);
        } else {
            // Fallback placement when no regions are available
            self.place_civilizations_without_regions(tile_map, civilizations);
        }
    }

    /// Places city states in available regions
    fn place_city_states(&mut self, tile_map: &mut TileMap, map_regions: &mut MapRegions) {
        let city_state_regions = map_regions.get_city_state_regions();
        let mut start_finder = RegionStartFinder::new(&self.ruleset);

        for region in city_state_regions {
            if let Some(start_pos) = start_finder.find_start(tile_map, &region) {
                // Place city state at the position
                if let Some(tile) = tile_map.tiles.get_mut(&start_pos) {
                    tile.is_city_state_start = true;
                }
            }
        }
    }

    /// Places civilizations when no regions are available
    fn place_civilizations_without_regions(&mut self, tile_map: &mut TileMap, civilizations: &[Civilization]) {
        let mut suitable_tiles: Vec<&Tile> = tile_map.tiles.values()
            .filter(|tile| self.is_suitable_start_location(tile))
            .collect();

        // Choose spread out locations for the civilizations
        let start_positions = self.randomness.choose_spread_out_locations(
            civilizations.len() as i32,
            &suitable_tiles,
            self.map_parameters.radius
        );

        // Create a mapping of civilizations to their starting positions
        let mut start_positions_map = HashMap::new();
        for (civ, &pos) in civilizations.iter().zip(start_positions.iter()) {
            start_positions_map.insert(civ.clone(), pos.position.clone());
        }

        // Normalize the starting positions
        let mut start_normalizer = StartNormalizer::new(&self.ruleset);
        start_normalizer.normalize_starts(tile_map, &start_positions_map);
    }

    /// Checks if a tile is suitable as a starting location
    fn is_suitable_start_location(&self, tile: &Tile) -> bool {
        // Must be on land
        if tile.terrain_type == TerrainType::Ocean || tile.terrain_type == TerrainType::Coast {
            return false;
        }

        // Must not be on mountains or natural wonders
        if tile.terrain_type == TerrainType::Mountain ||
           tile.terrain_features.contains(&TerrainFeature::NaturalWonder) {
            return false;
        }

        // Should have some food potential
        if tile.terrain_type == TerrainType::Desert &&
           !tile.terrain_features.contains(&TerrainFeature::FloodPlains) &&
           !tile.terrain_features.contains(&TerrainFeature::Oasis) {
            return false;
        }

        // Should not be on snow unless no choice
        if tile.terrain_type == TerrainType::Snow {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_generator_creation() {
        let params = MapParameters::default();
        let ruleset = Ruleset::default();
        let generator = MapGenerator::new(params, ruleset);
        assert!(generator.map_regions.is_none());
    }

    #[test]
    fn test_basic_terrain_generation() {
        let params = MapParameters::default();
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();
        generator.generate_basic_terrain(&mut tile_map);

        // Verify that the map contains a mix of terrain types
        let mut has_water = false;
        let mut has_land = false;
        for tile in tile_map.tiles.values() {
            match tile.terrain_type {
                TerrainType::Ocean | TerrainType::Coast => has_water = true,
                TerrainType::Grassland => has_land = true,
                _ => {}
            }
            if has_water && has_land {
                break;
            }
        }
        assert!(has_water && has_land);
    }

    #[test]
    fn test_resource_placement() {
        let params = MapParameters::default();
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        // First generate terrain
        generator.generate_basic_terrain(&mut tile_map);

        // Then place resources
        generator.place_resources_and_features(&mut tile_map);

        // Verify that some resources were placed
        let has_resources = tile_map.tiles.values()
            .any(|tile| tile.resource.is_some());

        assert!(has_resources);
    }

    #[test]
    fn test_feature_placement() {
        let params = MapParameters::default();
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        // First generate terrain
        generator.generate_basic_terrain(&mut tile_map);

        // Then place features
        generator.place_terrain_features(&mut tile_map);

        // Verify that some features were placed
        let has_features = tile_map.tiles.values()
            .any(|tile| !tile.terrain_features.is_empty());

        assert!(has_features);
    }

    #[test]
    fn test_civilization_placement() {
        let params = MapParameters::default();
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        // Generate terrain and features
        generator.generate_basic_terrain(&mut tile_map);
        generator.place_resources_and_features(&mut tile_map);

        // Create test civilizations
        let civilizations = vec![
            Civilization::new("Test Civ 1"),
            Civilization::new("Test Civ 2"),
        ];

        // Place civilizations
        generator.place_civilizations(&mut tile_map, &civilizations);

        // Verify that civilizations were placed
        let start_positions = tile_map.tiles.values()
            .filter(|tile| tile.is_start_location)
            .count();

        assert_eq!(start_positions, civilizations.len());
    }

    #[test]
    fn test_city_state_placement() {
        let mut params = MapParameters::default();
        params.number_of_city_states = 2;
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        // Generate terrain and features
        generator.generate_basic_terrain(&mut tile_map);
        generator.place_resources_and_features(&mut tile_map);

        // Initialize map regions
        generator.map_regions = Some(MapRegions::new(&tile_map, &generator.map_parameters));

        // Place city states
        if let Some(ref mut regions) = generator.map_regions {
            generator.place_city_states(&mut tile_map, regions);
        }

        // Verify that city states were placed
        let city_state_positions = tile_map.tiles.values()
            .filter(|tile| tile.is_city_state_start)
            .count();

        assert_eq!(city_state_positions, params.number_of_city_states as usize);
    }

    #[test]
    fn test_pangaea_generation() {
        let mut params = MapParameters::default();
        params.map_type = MapType::Pangaea;
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        generator.generate_basic_terrain(&mut tile_map);

        // Verify that land is more concentrated in the center
        let center_tiles: Vec<_> = tile_map.tiles.values()
            .filter(|tile| {
                tile.position.distance_to(&Vector2::new(0, 0)) < params.radius / 2 &&
                tile.terrain_type != TerrainType::Ocean &&
                tile.terrain_type != TerrainType::Coast
            })
            .collect();

        assert!(center_tiles.len() > tile_map.tiles.len() / 4);
    }

    #[test]
    fn test_continents_generation() {
        let mut params = MapParameters::default();
        params.map_type = MapType::Continents;
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        generator.generate_basic_terrain(&mut tile_map);

        // Verify that we have multiple distinct landmasses
        let continents = generator.identify_continents(&tile_map);
        assert!(continents.len() >= 2);
    }

    #[test]
    fn test_archipelago_generation() {
        let mut params = MapParameters::default();
        params.map_type = MapType::Archipelago;
        let ruleset = Ruleset::default();
        let mut generator = MapGenerator::new(params, ruleset);
        let mut tile_map = generator.create_empty_map();

        generator.generate_basic_terrain(&mut tile_map);

        // Verify that we have many small islands
        let islands = generator.identify_continents(&tile_map);
        assert!(islands.len() > 5);

        // Verify that islands are relatively small
        for island in islands {
            assert!(island.len() < tile_map.tiles.len() / 10);
        }
    }
}