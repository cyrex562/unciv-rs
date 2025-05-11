use std::collections::{HashMap, HashSet, LinkedHashMap};
use std::f64;
use crate::map::hex_math::HexMath;
use crate::map::tile::Tile;
use crate::utils::debug;
use crate::map::mapgenerator::perlin::Perlin;

/// Provides randomness functionality for map generation
pub struct MapGenerationRandomness {
    pub rng: rand::rngs::StdRng,
}

impl MapGenerationRandomness {
    /// Creates a new MapGenerationRandomness with a default seed
    pub fn new() -> Self {
        Self {
            rng: rand::SeedableRng::seed_from_u64(42),
        }
    }

    /// Seeds the random number generator with a specific seed
    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = rand::SeedableRng::seed_from_u64(seed);
    }

    /// Generates a perlin noise channel combining multiple octaves
    /// Default settings generate mostly within [-0.55, 0.55], but clustered around 0.0
    /// About 28% are < -0.1 and 28% are > 0.1
    ///
    /// # Arguments
    ///
    /// * `tile` - Source for x / y coordinates.
    /// * `seed` - Misnomer: actually the z value the Perlin cloud is 'cut' on.
    /// * `n_octaves` - The number of octaves.
    /// * `persistence` - The scaling factor of octave amplitudes.
    /// * `lacunarity` - The scaling factor of octave frequencies.
    /// * `scale` - The distance the noise is observed from.
    pub fn get_perlin_noise(
        &self,
        tile: &Tile,
        seed: f64,
        n_octaves: i32,
        persistence: f64,
        lacunarity: f64,
        scale: f64
    ) -> f64 {
        let world_coords = HexMath::hex2_world_coords(&tile.position);
        Perlin::noise3d(
            world_coords.x as f64,
            world_coords.y as f64,
            seed,
            n_octaves,
            persistence,
            lacunarity,
            scale
        )
    }

    /// Generates a perlin noise channel with default parameters
    pub fn get_perlin_noise_default(
        &self,
        tile: &Tile,
        seed: f64,
        scale: f64
    ) -> f64 {
        self.get_perlin_noise(tile, seed, 6, 0.5, 2.0, scale)
    }

    /// Chooses a number of spread-out locations from a list of suitable tiles
    ///
    /// # Arguments
    ///
    /// * `number` - The number of locations to choose
    /// * `suitable_tiles` - The list of tiles to choose from
    /// * `map_radius` - The radius of the map
    pub fn choose_spread_out_locations(
        &mut self,
        number: i32,
        suitable_tiles: &[&Tile],
        map_radius: i32
    ) -> Vec<&Tile> {
        if number <= 0 {
            return Vec::new();
        }

        // Determine sensible initial distance from number of desired placements and mapRadius
        // empiric formula comes very close to eliminating retries for distance.
        // The `if` means if we need to fill 60% or more of the available tiles, no sense starting with minimum distance 2.
        let sparsity_factor = (HexMath::get_hexagonal_radius_for_area(suitable_tiles.len() as i32) as f32 / map_radius as f32).powf(0.333);
        let initial_distance = if number == 1 || number * 5 >= suitable_tiles.len() as i32 * 3 {
            1
        } else {
            ((map_radius as f32 * 0.666 / HexMath::get_hexagonal_radius_for_area(number).powf(0.9) * sparsity_factor).round() as i32).max(1)
        };

        // If possible, we want to equalize the base terrains upon which
        //  the resources are found, so we save how many have been
        //  found for each base terrain and try to get one from the lowest
        let mut base_terrains_to_chosen_tiles: HashMap<String, i32> = HashMap::new();
        // Once we have a preference to choose from a specific base terrain, we want quick lookup of the available candidates
        let mut suitable_tiles_grouped: LinkedHashMap<String, HashSet<&Tile>> = LinkedHashMap::with_capacity(8);  // 8 is > number of base terrains in vanilla
        // Prefill both with all existing base terrains as keys, and group suitableTiles into base terrain buckets
        for tile in suitable_tiles {
            let terrain = &tile.base_terrain;
            if !base_terrains_to_chosen_tiles.contains_key(terrain) {
                base_terrains_to_chosen_tiles.insert(terrain.clone(), 0);
            }
            suitable_tiles_grouped.entry(terrain.clone())
                .or_insert_with(HashSet::new)
                .insert(tile);
        }

        // Helper function to deep clone the grouped tiles
        fn deep_clone(grouped: &LinkedHashMap<String, HashSet<&Tile>>) -> LinkedHashMap<String, HashSet<&Tile>> {
            let mut result = LinkedHashMap::with_capacity(grouped.len());
            for (key, value) in grouped {
                result.insert(key.clone(), value.clone());
            }
            result
        }

        for distance_between_resources in (1..=initial_distance).rev() {
            let mut available_tiles = deep_clone(&suitable_tiles_grouped);
            let mut chosen_tiles = Vec::with_capacity(number as usize);

            // Reset counts for each terrain
            for terrain in base_terrains_to_chosen_tiles.keys().cloned().collect::<Vec<_>>() {
                *base_terrains_to_chosen_tiles.get_mut(&terrain).unwrap() = 0;
            }

            for _ in 1..=number {
                // Sort terrains by count and find the first one with available tiles
                let ordered_keys: Vec<_> = base_terrains_to_chosen_tiles.iter()
                    .sorted_by_key(|(_, count)| *count)
                    .map(|(key, _)| key.clone())
                    .collect();

                let first_key_with_tiles_left = ordered_keys.iter()
                    .find(|key| !available_tiles.get(*key).unwrap().is_empty())
                    .cloned();

                if let Some(key) = first_key_with_tiles_left {
                    let available_set = available_tiles.get_mut(&key).unwrap();
                    let chosen_tile = available_set.iter()
                        .nth(self.rng.gen_range(0..available_set.len()))
                        .unwrap();

                    let close_tiles: HashSet<_> = chosen_tile.get_tiles_in_distance(distance_between_resources)
                        .collect();

                    // Remove close tiles from all available sets
                    for available_set in available_tiles.values_mut() {
                        available_set.retain(|tile| !close_tiles.contains(tile));
                    }

                    chosen_tiles.push(*chosen_tile);
                    *base_terrains_to_chosen_tiles.get_mut(&key).unwrap() += 1;
                } else {
                    break;
                }
            }

            if chosen_tiles.len() == number as usize || distance_between_resources == 1 {
                // Either we got them all, or we're not going to get anything better
                if distance_between_resources < initial_distance {
                    debug!("chooseSpreadOutLocations: distance {} < initial {}", distance_between_resources, initial_distance);
                }
                return chosen_tiles;
            }
        }

        // unreachable due to last loop iteration always returning and initialDistance >= 1
        panic!("Unreachable code reached!");
    }
}