use std::collections::HashMap;
use std::f64;
use crate::map::map_parameters::MapParameters;
use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::unique::UniqueType;
use crate::utils::log::Log;
use crate::map::mapgenerator::map_generation_randomness::MapGenerationRandomness;

/// Generator for map elevation, including hills and mountains
pub struct MapElevationGenerator {
    tile_map: TileMap,
    ruleset: Ruleset,
    randomness: MapGenerationRandomness,
    flat: Option<String>,
    hill_mutator: Box<dyn ITileMutator>,
    mountain_mutator: Box<dyn ITileMutator>,
    dummy_mutator: Box<dyn ITileMutator>,
}

impl MapElevationGenerator {
    const RISING: &'static str = "~Raising~";
    const LOWERING: &'static str = "~Lowering~";

    /// Creates a new MapElevationGenerator
    pub fn new(tile_map: TileMap, ruleset: Ruleset, randomness: MapGenerationRandomness) -> Self {
        let flat = ruleset.terrains.values()
            .find(|t| !t.impassable && t.terrain_type == TerrainType::Land && !t.has_unique(UniqueType::RoughTerrain))
            .map(|t| t.name.clone());

        let dummy_mutator = Box::new(TileDummyMutator::new());
        let mountain_mutator = Self::get_tile_mutator(UniqueType::OccursInChains, &flat, &ruleset, &dummy_mutator);
        let hill_mutator = Self::get_tile_mutator(UniqueType::OccursInGroups, &flat, &ruleset, &dummy_mutator);

        Self {
            tile_map,
            ruleset,
            randomness,
            flat,
            hill_mutator,
            mountain_mutator,
            dummy_mutator,
        }
    }

    fn get_tile_mutator(
        type_name: UniqueType,
        flat: &Option<String>,
        ruleset: &Ruleset,
        dummy_mutator: &Box<dyn ITileMutator>
    ) -> Box<dyn ITileMutator> {
        if flat.is_none() {
            return dummy_mutator.clone();
        }

        let terrain = ruleset.terrains.values()
            .find(|t| t.has_unique(type_name))
            .cloned();

        match terrain {
            Some(terrain) => {
                if terrain.terrain_type == TerrainType::TerrainFeature {
                    Box::new(TileFeatureMutator::new(terrain.name.clone()))
                } else {
                    Box::new(TileBaseMutator::new(flat.clone().unwrap(), terrain.name.clone()))
                }
            },
            None => dummy_mutator.clone()
        }
    }

    /// Raises mountains and hills on the map
    ///
    /// [MapParameters.elevationExponent] favors high elevation
    pub fn raise_mountains_and_hills(&mut self) {
        if self.flat.is_none() {
            Log::debug("Ruleset seems to contain no flat terrain - can't generate heightmap");
            return;
        }

        let elevation_seed = self.randomness.rng.next_i32() as f64;
        let exponent = 1.0 - self.tile_map.map_parameters.elevation_exponent as f64;

        // Helper function for signed power
        fn pow_signed(value: f64, exponent: f64) -> f64 {
            value.abs().powf(exponent) * value.signum()
        }

        self.tile_map.set_transients(&self.ruleset);

        for tile in self.tile_map.values() {
            if tile.is_water {
                continue;
            }

            let elevation = pow_signed(
                self.randomness.get_perlin_noise(tile, elevation_seed, 2.0),
                exponent
            );

            tile.base_terrain = self.flat.clone().unwrap(); // in case both mutators are TileFeatureMutator
            self.hill_mutator.set_elevated(tile, elevation > 0.5 && elevation <= 0.7);
            self.mountain_mutator.set_elevated(tile, elevation > 0.7);
            tile.set_terrain_transients();
        }

        self.cellular_mountain_ranges();
        self.cellular_hills();
    }

    fn cellular_mountain_ranges(&mut self) {
        if self.mountain_mutator.name().is_empty() {
            return;
        }

        Log::debug("Mountain-like generation for {}", self.mountain_mutator.name());

        let target_mountains = self.mountain_mutator.count(self.tile_map.values()) * 2;
        let impassable_terrains: Vec<String> = self.ruleset.terrains.values()
            .filter(|t| t.impassable)
            .map(|t| t.name.clone())
            .collect();

        for _ in 1..=5 {
            let mut total_mountains = self.mountain_mutator.count(self.tile_map.values());

            for tile in self.tile_map.values() {
                if tile.is_water {
                    continue;
                }

                let adjacent_mountains = self.mountain_mutator.count(tile.neighbors());
                let adjacent_impassible = tile.neighbors()
                    .filter(|t| impassable_terrains.contains(&t.base_terrain))
                    .count();

                if adjacent_mountains == 0 && self.mountain_mutator.is_elevated(tile) {
                    if self.randomness.rng.next_i32(4) == 0 {
                        tile.add_terrain_feature(Self::LOWERING);
                    }
                } else if adjacent_mountains == 1 {
                    if self.randomness.rng.next_i32(10) == 0 {
                        tile.add_terrain_feature(Self::RISING);
                    }
                } else if adjacent_impassible == 3 {
                    if self.randomness.rng.next_i32(2) == 0 {
                        tile.add_terrain_feature(Self::LOWERING);
                    }
                } else if adjacent_impassible > 3 {
                    tile.add_terrain_feature(Self::LOWERING);
                }
            }

            for tile in self.tile_map.values() {
                if tile.is_water {
                    continue;
                }

                if tile.terrain_features.contains(Self::RISING) {
                    tile.remove_terrain_feature(Self::RISING);
                    if total_mountains >= target_mountains {
                        continue;
                    }

                    if !self.mountain_mutator.is_elevated(tile) {
                        total_mountains += 1;
                    }

                    self.hill_mutator.lower(tile);
                    self.mountain_mutator.raise(tile);
                }

                if tile.terrain_features.contains(Self::LOWERING) {
                    tile.remove_terrain_feature(Self::LOWERING);
                    if total_mountains * 2 <= target_mountains {
                        continue;
                    }

                    if self.mountain_mutator.is_elevated(tile) {
                        total_mountains -= 1;
                    }

                    self.mountain_mutator.lower(tile);
                    self.hill_mutator.raise(tile);
                }
            }
        }
    }

    fn cellular_hills(&mut self) {
        if self.hill_mutator.name().is_empty() {
            return;
        }

        Log::debug("Hill-like generation for {}", self.hill_mutator.name());

        let target_hills = self.hill_mutator.count(self.tile_map.values());

        for i in 1..=5 {
            let mut total_hills = self.hill_mutator.count(self.tile_map.values());

            for tile in self.tile_map.values() {
                if tile.is_water || self.mountain_mutator.is_elevated(tile) {
                    continue;
                }

                let adjacent_mountains = self.mountain_mutator.count(tile.neighbors());
                let adjacent_hills = self.hill_mutator.count(tile.neighbors());

                if adjacent_hills <= 1 && adjacent_mountains == 0 && self.randomness.rng.next_i32(2) == 0 {
                    tile.add_terrain_feature(Self::LOWERING);
                } else if adjacent_hills > 3 && adjacent_mountains == 0 && self.randomness.rng.next_i32(2) == 0 {
                    tile.add_terrain_feature(Self::LOWERING);
                } else if (adjacent_hills + adjacent_mountains >= 2 && adjacent_hills + adjacent_mountains <= 3) && self.randomness.rng.next_i32(2) == 0 {
                    tile.add_terrain_feature(Self::RISING);
                }
            }

            for tile in self.tile_map.values() {
                if tile.is_water || self.mountain_mutator.is_elevated(tile) {
                    continue;
                }

                if tile.terrain_features.contains(Self::RISING) {
                    tile.remove_terrain_feature(Self::RISING);
                    if total_hills > target_hills && i != 1 {
                        continue;
                    }

                    if !self.hill_mutator.is_elevated(tile) {
                        self.hill_mutator.raise(tile);
                        total_hills += 1;
                    }
                }

                if tile.terrain_features.contains(Self::LOWERING) {
                    tile.remove_terrain_feature(Self::LOWERING);
                    if total_hills >= target_hills as f32 * 0.9 || i == 1 {
                        if self.hill_mutator.is_elevated(tile) {
                            self.hill_mutator.lower(tile);
                            total_hills -= 1;
                        }
                    }
                }
            }
        }
    }
}

/// Trait for tile mutators that can raise or lower tiles
pub trait ITileMutator: Send + Sync {
    /// Returns the name of the mutator (for logging only)
    fn name(&self) -> &str;

    /// Lowers a tile
    fn lower(&self, tile: &mut Tile);

    /// Raises a tile
    fn raise(&self, tile: &mut Tile);

    /// Checks if a tile is elevated
    fn is_elevated(&self, tile: &Tile) -> bool;

    /// Sets a tile's elevation
    fn set_elevated(&self, tile: &mut Tile, value: bool) {
        if value {
            self.raise(tile);
        } else {
            self.lower(tile);
        }
    }

    /// Counts the number of elevated tiles in a collection
    fn count(&self, tiles: impl Iterator<Item = &Tile>) -> i32 {
        tiles.filter(|t| self.is_elevated(t)).count() as i32
    }
}

/// A dummy mutator that does nothing
pub struct TileDummyMutator;

impl TileDummyMutator {
    pub fn new() -> Self {
        Self
    }
}

impl ITileMutator for TileDummyMutator {
    fn name(&self) -> &str {
        ""
    }

    fn lower(&self, _tile: &mut Tile) {}

    fn raise(&self, _tile: &mut Tile) {}

    fn is_elevated(&self, _tile: &Tile) -> bool {
        false
    }
}

/// A mutator that changes the base terrain of a tile
pub struct TileBaseMutator {
    flat: String,
    elevated: String,
}

impl TileBaseMutator {
    pub fn new(flat: String, elevated: String) -> Self {
        Self {
            flat,
            elevated,
        }
    }
}

impl ITileMutator for TileBaseMutator {
    fn name(&self) -> &str {
        &self.elevated
    }

    fn lower(&self, tile: &mut Tile) {
        tile.base_terrain = self.flat.clone();
    }

    fn raise(&self, tile: &mut Tile) {
        tile.base_terrain = self.elevated.clone();
    }

    fn is_elevated(&self, tile: &Tile) -> bool {
        tile.base_terrain == self.elevated
    }
}

/// A mutator that adds or removes terrain features
pub struct TileFeatureMutator {
    elevated: String,
}

impl TileFeatureMutator {
    pub fn new(elevated: String) -> Self {
        Self {
            elevated,
        }
    }
}

impl ITileMutator for TileFeatureMutator {
    fn name(&self) -> &str {
        &self.elevated
    }

    fn lower(&self, tile: &mut Tile) {
        tile.remove_terrain_feature(&self.elevated);
    }

    fn raise(&self, tile: &mut Tile) {
        tile.add_terrain_feature(&self.elevated);
    }

    fn is_elevated(&self, tile: &Tile) -> bool {
        tile.terrain_features.contains(&self.elevated)
    }
}