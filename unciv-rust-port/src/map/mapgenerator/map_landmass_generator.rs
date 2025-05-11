use crate::map::tile_map::TileMap;
use crate::map::hex_math::HexMath;
use crate::map::map_shape::MapShape;
use crate::map::map_type::MapType;
use crate::map::tile::Tile;
use crate::ruleset::Ruleset;
use crate::map::terrain::TerrainType;
use crate::ruleset::unique::UniqueType;
use crate::map::mapgenerator::map_generation_randomness::MapGenerationRandomness;
use crate::map::mapgenerator::perlin::Perlin;

/// Generator for creating landmasses on the map
pub struct MapLandmassGenerator {
    tile_map: TileMap,
    land_terrain_name: String,
    water_terrain_name: String,
    land_only_mod: bool,
    water_threshold: f64,
    randomness: MapGenerationRandomness,
}

impl MapLandmassGenerator {
    /// Creates a new MapLandmassGenerator instance
    pub fn new(tile_map: TileMap, ruleset: &Ruleset, randomness: MapGenerationRandomness) -> Self {
        let land_terrain_name = Self::get_initialization_terrain(ruleset, TerrainType::Land);
        let water_terrain_name = Self::get_initialization_terrain(ruleset, TerrainType::Water)
            .unwrap_or_else(|_| land_terrain_name.clone());
        let land_only_mod = water_terrain_name == land_terrain_name;

        Self {
            tile_map,
            land_terrain_name,
            water_terrain_name,
            land_only_mod,
            water_threshold: 0.0,
            randomness,
        }
    }

    /// Gets the initialization terrain for a given terrain type
    fn get_initialization_terrain(ruleset: &Ruleset, terrain_type: TerrainType) -> Result<String, String> {
        ruleset.terrains.values()
            .find(|terrain| {
                terrain.type_ == terrain_type &&
                !terrain.has_unique(UniqueType::NoNaturalGeneration)
            })
            .map(|terrain| terrain.name.clone())
            .ok_or_else(|| format!("Cannot create map - no {:?} terrains found!", terrain_type))
    }

    /// Generates landmasses based on map parameters
    pub fn generate_land(&mut self) {
        // Handle land-only mods
        if self.land_only_mod {
            for tile in self.tile_map.tiles.values_mut() {
                tile.base_terrain = self.land_terrain_name.clone();
            }
            return;
        }

        self.water_threshold = self.tile_map.map_parameters.water_threshold as f64;

        match self.tile_map.map_parameters.type_ {
            MapType::Pangaea => self.create_pangaea(),
            MapType::InnerSea => self.create_inner_sea(),
            MapType::ContinentAndIslands => self.create_continent_and_islands(),
            MapType::TwoContinents => self.create_two_continents(),
            MapType::ThreeContinents => self.create_three_continents(),
            MapType::FourCorners => self.create_four_corners(),
            MapType::Archipelago => self.create_archipelago(),
            MapType::Perlin => self.create_perlin(),
            MapType::Fractal => self.create_fractal(),
            MapType::Lakes => self.create_lakes(),
            MapType::SmallContinents => self.create_small_continents(),
        }

        if self.tile_map.map_parameters.shape == MapShape::FlatEarth {
            self.generate_flat_earth_extra_water();
        }
    }

    /// Spawns land or water on a tile based on elevation
    fn spawn_land_or_water(&mut self, tile: &mut Tile, elevation: f64) {
        tile.base_terrain = if elevation < self.water_threshold {
            self.water_terrain_name.clone()
        } else {
            self.land_terrain_name.clone()
        };
    }

    /// Retries terrain generation with lower water levels until conditions are met
    fn retry_lowering_water_level<F, P>(&mut self, mut predicate: P, mut function: F)
    where
        F: FnMut(),
        P: FnMut(f32) -> bool,
    {
        let mut retries = 0;
        while retries <= 30 {
            function();
            let water_count = self.tile_map.tiles.values()
                .filter(|tile| tile.base_terrain == self.water_terrain_name)
                .count();
            let water_percent = water_count as f32 / self.tile_map.tiles.len() as f32;

            if self.water_threshold < -1.0 || predicate(water_percent) {
                break;
            }

            // Lower water table with empiric base step and acceleration
            self.water_threshold -= 0.02 *
                (water_percent / 0.7).max(1.0) *
                (retries as f32).powf(0.5);
            retries += 1;
        }
    }

    /// Generates extra water for flat earth maps
    fn generate_flat_earth_extra_water(&mut self) {
        for tile in self.tile_map.tiles.values_mut() {
            let is_center_tile = tile.latitude == 0.0 && tile.longitude == 0.0;
            let is_edge_tile = tile.neighbors.len() < 6;

            if !is_center_tile && !is_edge_tile {
                continue;
            }

            // Add water perimeter and center
            tile.base_terrain = self.water_terrain_name.clone();
            for neighbor in &tile.neighbors {
                if let Some(n1) = self.tile_map.tiles.get_mut(neighbor) {
                    n1.base_terrain = self.water_terrain_name.clone();
                    for n2_pos in &n1.neighbors {
                        if let Some(n2) = self.tile_map.tiles.get_mut(n2_pos) {
                            n2.base_terrain = self.water_terrain_name.clone();
                            if !is_center_tile {
                                continue;
                            }
                            for n3_pos in &n2.neighbors {
                                if let Some(n3) = self.tile_map.tiles.get_mut(n3_pos) {
                                    n3.base_terrain = self.water_terrain_name.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets ridged Perlin noise for a tile
    fn get_ridged_perlin_noise(&mut self, tile: &Tile, seed: f64,
        n_octaves: i32, persistence: f64, lacunarity: f64, scale: f64) -> f64 {
        let world_coords = HexMath::hex2world_coords(&tile.position);
        Perlin::ridged_noise_3d(
            world_coords.x as f64,
            world_coords.y as f64,
            seed,
            n_octaves,
            persistence,
            lacunarity,
            scale
        )
    }

    /// Creates a Perlin noise based map
    fn create_perlin(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a fractal based map
    fn create_fractal(&mut self) {
        self.retry_lowering_water_level(
            |water_percent| water_percent <= 0.7,
            || {
                let elevation_seed = self.randomness.rng.gen::<f64>();
                for tile in self.tile_map.tiles.values_mut() {
                    let max_dim = self.tile_map.max_latitude.max(self.tile_map.max_longitude);
                    let mut ratio = max_dim / 32.0;

                    if self.tile_map.map_parameters.shape == MapShape::Hexagonal ||
                       self.tile_map.map_parameters.shape == MapShape::FlatEarth {
                        ratio *= 0.5;
                    }

                    let mut elevation = self.randomness.get_perlin_noise(
                        tile, elevation_seed, 0.8, 1.5, ratio * 30.0
                    );

                    elevation += self.get_ocean_edges_transform(tile);
                    self.spawn_land_or_water(tile, elevation);
                }
            }
        );
    }

    /// Creates a lakes based map
    fn create_lakes(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let elevation = 0.3 - self.get_ridged_perlin_noise(
                tile, elevation_seed, 10, 0.7, 1.5, 10.0
            );
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a small continents map
    fn create_small_continents(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        self.water_threshold += 0.25;
        self.retry_lowering_water_level(
            |water_percent| water_percent <= 0.7,
            || {
                for tile in self.tile_map.tiles.values_mut() {
                    let mut elevation = self.get_ridged_perlin_noise(
                        tile, elevation_seed, 10, 0.5, 2.0, 22.0
                    );
                    elevation += self.get_ocean_edges_transform(tile);
                    self.spawn_land_or_water(tile, elevation);
                }
            }
        );
    }

    /// Creates an archipelago map
    fn create_archipelago(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        self.water_threshold += 0.25;
        for tile in self.tile_map.tiles.values_mut() {
            let elevation = self.get_ridged_perlin_noise(
                tile, elevation_seed, 10, 0.5, 2.0, 10.0
            );
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a Pangaea map
    fn create_pangaea(&mut self) {
        let large_continent_threshold = 25
            .min(self.tile_map.tiles.len() / 4)
            .max((self.tile_map.tiles.len() as f32).powf(0.333) as usize);

        self.retry_lowering_water_level(
            |water_percent| {
                let large_continents = self.tile_map.continent_sizes.values()
                    .filter(|&size| *size > large_continent_threshold)
                    .count();
                large_continents == 1 && water_percent <= 0.7
            },
            || {
                let elevation_seed = self.randomness.rng.gen::<f64>();
                for tile in self.tile_map.tiles.values_mut() {
                    let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
                    elevation = elevation * 0.75 + self.get_elliptic_continent(tile) * 0.25;
                    self.spawn_land_or_water(tile, elevation);
                    tile.set_terrain_transients();
                }
                self.tile_map.assign_continents(TileMap::AssignContinentsMode::Reassign);
            }
        );
        self.tile_map.assign_continents(TileMap::AssignContinentsMode::Clear);
    }

    /// Creates an inner sea map
    fn create_inner_sea(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            elevation -= self.get_elliptic_continent(tile, 0.6) * 0.3;
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a continent and islands map
    fn create_continent_and_islands(&mut self) {
        let is_north = self.randomness.rng.gen_bool(0.5);
        let is_latitude = if self.tile_map.map_parameters.shape == MapShape::Hexagonal ||
                            self.tile_map.map_parameters.shape == MapShape::FlatEarth {
            self.randomness.rng.gen_bool(0.5)
        } else if self.tile_map.map_parameters.map_size.height > self.tile_map.map_parameters.map_size.width {
            true
        } else if self.tile_map.map_parameters.map_size.width > self.tile_map.map_parameters.map_size.height {
            false
        } else {
            self.randomness.rng.gen_bool(0.5)
        };

        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            elevation = (elevation + self.get_continent_and_islands_transform(tile, is_north, is_latitude)) / 2.0;
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a two continents map
    fn create_two_continents(&mut self) {
        let is_latitude = if self.tile_map.map_parameters.shape == MapShape::Hexagonal ||
                            self.tile_map.map_parameters.shape == MapShape::FlatEarth {
            self.randomness.rng.gen_bool(0.5)
        } else if self.tile_map.map_parameters.map_size.height > self.tile_map.map_parameters.map_size.width {
            true
        } else if self.tile_map.map_parameters.map_size.width > self.tile_map.map_parameters.map_size.height {
            false
        } else {
            self.randomness.rng.gen_bool(0.5)
        };

        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            elevation = (elevation + self.get_two_continents_transform(tile, is_latitude)) / 2.0;
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a three continents map
    fn create_three_continents(&mut self) {
        let is_north = self.randomness.rng.gen_bool(0.5);
        let is_east_west = self.tile_map.map_parameters.shape == MapShape::FlatEarth &&
                          self.randomness.rng.gen_bool(0.5);

        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            elevation = (elevation + self.get_three_continents_transform(tile, is_north, is_east_west)) / 2.0;
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Creates a four corners map
    fn create_four_corners(&mut self) {
        let elevation_seed = self.randomness.rng.gen::<f64>();
        for tile in self.tile_map.tiles.values_mut() {
            let mut elevation = self.randomness.get_perlin_noise(tile, elevation_seed);
            elevation = elevation / 2.0 + self.get_four_corners_transform(tile) / 2.0;
            self.spawn_land_or_water(tile, elevation);
        }
    }

    /// Gets the transform for ocean edges
    fn get_ocean_edges_transform(&self, tile: &Tile) -> f64 {
        let mut transform = 0.0;
        let max_dim = self.tile_map.max_latitude.max(self.tile_map.max_longitude);

        if self.tile_map.map_parameters.shape == MapShape::Hexagonal {
            let center_dist = ((tile.latitude - self.tile_map.max_latitude / 2.0).powi(2) +
                             (tile.longitude - self.tile_map.max_longitude / 2.0).powi(2)).sqrt();
            transform = -0.3 * (center_dist / (max_dim / 2.0)).powi(2);
        } else if self.tile_map.map_parameters.shape == MapShape::FlatEarth {
            let center_dist = ((tile.latitude - self.tile_map.max_latitude / 2.0).powi(2) +
                             (tile.longitude - self.tile_map.max_longitude / 2.0).powi(2)).sqrt();
            transform = -0.2 * (center_dist / (max_dim / 2.0)).powi(2);
        }

        transform
    }

    /// Gets the transform for an elliptic continent
    fn get_elliptic_continent(&self, tile: &Tile) -> f64 {
        let center_x = self.tile_map.max_longitude / 2.0;
        let center_y = self.tile_map.max_latitude / 2.0;
        let dx = (tile.longitude - center_x) / center_x;
        let dy = (tile.latitude - center_y) / center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > 1.0 {
            0.0
        } else {
            1.0 - (dist).powi(2)
        }
    }

    /// Gets the transform for continent and islands
    fn get_continent_and_islands_transform(&self, tile: &Tile, is_north: bool, is_latitude: bool) -> f64 {
        let mut transform = 0.0;
        let max_dim = if is_latitude {
            self.tile_map.max_latitude
        } else {
            self.tile_map.max_longitude
        };

        let coord = if is_latitude {
            tile.latitude
        } else {
            tile.longitude
        };

        let center = max_dim / 2.0;
        let dist = (coord - center).abs() / center;

        if is_north {
            transform = if coord > center {
                1.0 - (dist * 2.0).powi(2)
            } else {
                0.0
            };
        } else {
            transform = if coord < center {
                1.0 - (dist * 2.0).powi(2)
            } else {
                0.0
            };
        }

        transform
    }

    /// Gets the transform for two continents
    fn get_two_continents_transform(&self, tile: &Tile, is_latitude: bool) -> f64 {
        let max_dim = if is_latitude {
            self.tile_map.max_latitude
        } else {
            self.tile_map.max_longitude
        };

        let coord = if is_latitude {
            tile.latitude
        } else {
            tile.longitude
        };

        let center = max_dim / 2.0;
        let dist = (coord - center).abs() / center;
        1.0 - (dist * 2.0).powi(2)
    }

    /// Gets the transform for three continents
    fn get_three_continents_transform(&self, tile: &Tile, is_north: bool, is_east_west: bool) -> f64 {
        let mut transform = 0.0;

        if is_east_west {
            let center_x = self.tile_map.max_longitude / 2.0;
            let center_y = self.tile_map.max_latitude / 2.0;
            let dx = (tile.longitude - center_x) / center_x;
            let dy = (tile.latitude - center_y) / center_y;

            if dx.abs() < 0.33 {
                transform = 1.0 - (dx * 3.0).powi(2);
            } else if dx > 0.0 {
                let local_dx = (dx - 0.66) * 3.0;
                let local_dy = dy * 2.0;
                transform = 1.0 - (local_dx * local_dx + local_dy * local_dy);
            } else {
                let local_dx = (dx + 0.66) * 3.0;
                let local_dy = dy * 2.0;
                transform = 1.0 - (local_dx * local_dx + local_dy * local_dy);
            }
        } else {
            let center_y = self.tile_map.max_latitude / 2.0;
            let dy = (tile.latitude - center_y) / center_y;

            if is_north {
                if dy > 0.0 {
                    let center_x = self.tile_map.max_longitude / 2.0;
                    let dx = (tile.longitude - center_x) / center_x;
                    let local_dy = (dy - 0.5) * 2.0;
                    transform = 1.0 - (dx * dx + local_dy * local_dy);
                } else {
                    let third = self.tile_map.max_longitude / 3.0;
                    let dx = if tile.longitude < third {
                        (tile.longitude - third / 2.0) / (third / 2.0)
                    } else if tile.longitude < 2.0 * third {
                        (tile.longitude - 1.5 * third) / (third / 2.0)
                    } else {
                        (tile.longitude - 2.5 * third) / (third / 2.0)
                    };
                    let local_dy = (dy + 0.5) * 2.0;
                    transform = 1.0 - (dx * dx + local_dy * local_dy);
                }
            } else {
                if dy < 0.0 {
                    let center_x = self.tile_map.max_longitude / 2.0;
                    let dx = (tile.longitude - center_x) / center_x;
                    let local_dy = (dy + 0.5) * 2.0;
                    transform = 1.0 - (dx * dx + local_dy * local_dy);
                } else {
                    let third = self.tile_map.max_longitude / 3.0;
                    let dx = if tile.longitude < third {
                        (tile.longitude - third / 2.0) / (third / 2.0)
                    } else if tile.longitude < 2.0 * third {
                        (tile.longitude - 1.5 * third) / (third / 2.0)
                    } else {
                        (tile.longitude - 2.5 * third) / (third / 2.0)
                    };
                    let local_dy = (dy - 0.5) * 2.0;
                    transform = 1.0 - (dx * dx + local_dy * local_dy);
                }
            }
        }

        transform.max(0.0)
    }

    /// Gets the transform for four corners
    fn get_four_corners_transform(&self, tile: &Tile) -> f64 {
        let center_x = self.tile_map.max_longitude / 2.0;
        let center_y = self.tile_map.max_latitude / 2.0;
        let dx = (tile.longitude - center_x) / center_x;
        let dy = (tile.latitude - center_y) / center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.707 {
            0.0
        } else {
            1.0 - ((1.0 - dist) / 0.293).powi(2)
        }
    }
}