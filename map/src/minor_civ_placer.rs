use std::collections::{HashMap, HashSet};
use std::cmp::min;
use rand::seq::SliceRandom;
use crate::civilization::Civilization;
use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::UniqueType;
use crate::map::mapgenerator::mapregions::map_regions::{MapRegions, ImpactType, Region};
use crate::map::mapgenerator::mapregions::map_gen_tile_data::MapGenTileData;
use crate::map::mapgenerator::mapregions::start_normalizer::StartNormalizer;

/// Handles the placement of minor civilizations (city states) on the map
pub struct MinorCivPlacer;

impl MinorCivPlacer {
    /// Assigns civs to regions or "uninhabited" land and places them. Depends on
    /// assign_luxuries having been called previously.
    /// Note: can silently fail to place all city states if there is too little room.
    /// Currently our GameStarter fills out with random city states, Civ V behavior is to
    /// forget about the discarded city states entirely.
    pub fn place_minor_civs(
        regions: &[Region],
        tile_map: &TileMap,
        civs: &[Civilization],
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset
    ) {
        if civs.is_empty() {
            return;
        }

        // Some but not all city states are assigned to regions directly. Determine the CS density.
        let mut unassigned_civs = Self::assign_minor_civs_directly_to_regions(civs, regions);

        // Some city states are assigned to "uninhabited" continents - unless it's an archipelago type map
        // (Because then every continent will have been assigned to a region anyway)
        let mut uninhabited_coastal = Vec::new();
        let mut uninhabited_hinterland = Vec::new();
        let mut civ_assigned_to_uninhabited = Vec::new();

        if !tile_map.using_archipelago_regions() {
            Self::spread_city_states_between_habited_and_uninhabited(
                tile_map,
                regions,
                tile_data,
                &mut uninhabited_coastal,
                &mut uninhabited_hinterland,
                civs,
                &mut unassigned_civs,
                &mut civ_assigned_to_uninhabited
            );
        }

        Self::assign_city_states_to_regions_with_common_luxuries(regions, &mut unassigned_civs);
        Self::spread_city_states_evenly_between_regions(&mut unassigned_civs, regions);
        Self::assign_remaining_city_states_to_worst_fertile_regions(regions, &mut unassigned_civs);

        // After we've finished assigning, NOW we actually place them
        Self::place_assigned_minor_civs(
            &mut civ_assigned_to_uninhabited,
            tile_map,
            &mut uninhabited_coastal,
            tile_data,
            ruleset,
            &mut uninhabited_hinterland,
            regions
        );
    }

    /// Spreads city states between habited and uninhabited areas
    fn spread_city_states_between_habited_and_uninhabited(
        tile_map: &TileMap,
        regions: &[Region],
        tile_data: &MapGenTileData,
        uninhabited_coastal: &mut Vec<Tile>,
        uninhabited_hinterland: &mut Vec<Tile>,
        civs: &[Civilization],
        unassigned_civs: &mut Vec<Civilization>,
        civ_assigned_to_uninhabited: &mut Vec<Civilization>
    ) {
        let uninhabited_continents: HashSet<i32> = tile_map.continent_sizes.iter()
            .filter(|(_, size)| *size >= 4 && // Don't bother with tiny islands
                regions.iter().none(|region| region.continent_id == **it))
            .map(|(id, _)| *id)
            .collect();

        let mut num_inhabited_tiles = 0;
        let mut num_uninhabited_tiles = 0;

        // Go through the entire map to build the data
        for tile in tile_map.values() {
            if !Self::can_place_minor_civ(tile, tile_data) {
                continue;
            }

            let continent = tile.get_continent();
            if uninhabited_continents.contains(&continent) {
                if tile.is_coastal_tile() {
                    uninhabited_coastal.push(tile.clone());
                } else {
                    uninhabited_hinterland.push(tile.clone());
                }
                num_uninhabited_tiles += 1;
            } else {
                num_inhabited_tiles += 1;
            }
        }

        // Determine how many minor civs to put on uninhabited continents.
        let max_by_uninhabited = (3 * civs.len() * num_uninhabited_tiles) / (num_inhabited_tiles + num_uninhabited_tiles);
        let max_by_ratio = (civs.len() + 1) / 2;
        let target_for_uninhabited = min(max_by_ratio, max_by_uninhabited);

        // Take the first target_for_uninhabited civs from unassigned_civs
        let mut rng = rand::thread_rng();
        unassigned_civs.shuffle(&mut rng);

        let civs_to_assign: Vec<Civilization> = unassigned_civs.drain(..target_for_uninhabited).collect();
        civ_assigned_to_uninhabited.extend(civs_to_assign);
    }

    /// If there are still unassigned minor civs, assign extra ones to regions that share their
    /// luxury type with two others, as compensation. Because starting close to a city state is good??
    fn assign_city_states_to_regions_with_common_luxuries(
        regions: &[Region],
        unassigned_civs: &mut Vec<Civilization>
    ) {
        if unassigned_civs.is_empty() {
            return;
        }

        let regions_with_common_luxuries: Vec<&Region> = regions.iter()
            .filter(|region| {
                regions.iter().filter(|other| other.luxury == region.luxury).count() >= 3
            })
            .collect();

        // assign one civ each to regions with common luxuries if there are enough to go around
        if !regions_with_common_luxuries.is_empty() &&
           regions_with_common_luxuries.len() <= unassigned_civs.len() {
            for region in regions_with_common_luxuries {
                if let Some(civ_to_assign) = unassigned_civs.pop() {
                    region.assigned_minor_civs.push(civ_to_assign);
                }
            }
        }
    }

    /// Add one extra to each region as long as there are enough to go around
    fn spread_city_states_evenly_between_regions(
        unassigned_civs: &mut Vec<Civilization>,
        regions: &[Region]
    ) {
        if unassigned_civs.is_empty() {
            return;
        }

        while unassigned_civs.len() >= regions.len() {
            for region in regions {
                if let Some(civ_to_assign) = unassigned_civs.pop() {
                    region.assigned_minor_civs.push(civ_to_assign);
                }
            }
        }
    }

    /// At this point there is at least for sure less remaining city states than regions
    /// Sort regions by fertility and put extra city states in the worst ones.
    fn assign_remaining_city_states_to_worst_fertile_regions(
        regions: &[Region],
        unassigned_civs: &mut Vec<Civilization>
    ) {
        if unassigned_civs.is_empty() {
            return;
        }

        let mut worst_regions: Vec<&Region> = regions.iter().collect();
        worst_regions.sort_by_key(|r| r.total_fertility);
        worst_regions.truncate(unassigned_civs.len());

        for region in worst_regions {
            if let Some(civ_to_assign) = unassigned_civs.pop() {
                region.assigned_minor_civs.push(civ_to_assign);
            }
        }
    }

    /// Actually place the minor civs, after they have been sorted into groups and assigned to regions
    fn place_assigned_minor_civs(
        civ_assigned_to_uninhabited: &mut Vec<Civilization>,
        tile_map: &TileMap,
        uninhabited_coastal: &mut Vec<Tile>,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset,
        uninhabited_hinterland: &mut Vec<Tile>,
        regions: &[Region]
    ) {
        // All minor civs are assigned - now place them
        // First place the "uninhabited continent" ones, preferring coastal starts
        Self::try_place_minor_civs_in_tiles(
            civ_assigned_to_uninhabited,
            tile_map,
            uninhabited_coastal,
            tile_data,
            ruleset
        );

        Self::try_place_minor_civs_in_tiles(
            civ_assigned_to_uninhabited,
            tile_map,
            uninhabited_hinterland,
            tile_data,
            ruleset
        );

        // Fallback to a random region for civs that couldn't be placed in the wilderness
        let mut rng = rand::thread_rng();
        for unplaced_civ in civ_assigned_to_uninhabited.drain(..) {
            if let Some(region) = regions.choose(&mut rng) {
                region.assigned_minor_civs.push(unplaced_civ);
            }
        }

        // Now place the ones assigned to specific regions.
        for region in regions {
            let mut region_tiles = region.tiles.clone();
            Self::try_place_minor_civs_in_tiles(
                &mut region.assigned_minor_civs,
                tile_map,
                &mut region_tiles,
                tile_data,
                ruleset
            );
        }
    }

    /// Assigns minor civs directly to regions based on density
    fn assign_minor_civs_directly_to_regions(
        civs: &[Civilization],
        regions: &[Region]
    ) -> Vec<Civilization> {
        let minor_civ_ratio = civs.len() as f32 / regions.len() as f32;
        let minor_civ_per_region = if minor_civ_ratio > 14.0 {
            10 // lol
        } else if minor_civ_ratio > 11.0 {
            8
        } else if minor_civ_ratio > 8.0 {
            6
        } else if minor_civ_ratio > 5.7 {
            4
        } else if minor_civ_ratio > 4.35 {
            3
        } else if minor_civ_ratio > 2.7 {
            2
        } else if minor_civ_ratio > 1.35 {
            1
        } else {
            0
        };

        let mut unassigned_civs: Vec<Civilization> = civs.to_vec();
        let mut rng = rand::thread_rng();
        unassigned_civs.shuffle(&mut rng);

        if minor_civ_per_region > 0 {
            for region in regions {
                let civs_to_assign: Vec<Civilization> = unassigned_civs.drain(..min(minor_civ_per_region, unassigned_civs.len())).collect();
                region.assigned_minor_civs.extend(civs_to_assign);
            }
        }

        unassigned_civs
    }

    /// Attempts to randomly place civs from civs_to_place in tiles from tile_list. Assumes that
    /// tile_list is pre-vetted and only contains habitable land tiles.
    /// Will modify both civs_to_place and tile_list as it goes!
    fn try_place_minor_civs_in_tiles(
        civs_to_place: &mut Vec<Civilization>,
        tile_map: &TileMap,
        tile_list: &mut Vec<Tile>,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset
    ) {
        let mut rng = rand::thread_rng();

        while !tile_list.is_empty() && !civs_to_place.is_empty() {
            let chosen_index = rng.gen_range(0..tile_list.len());
            let chosen_tile = tile_list.remove(chosen_index);

            if let Some(data) = tile_data.get_mut(chosen_tile.position) {
                // If the randomly chosen tile is too close to a player or a city state, discard it
                if data.impacts.contains_key(&ImpactType::MinorCiv) {
                    continue;
                }

                // Otherwise, go ahead and place the minor civ
                if let Some(civ_to_add) = civs_to_place.pop() {
                    Self::place_minor_civ(civ_to_add, tile_map, &chosen_tile, tile_data, ruleset);
                }
            }
        }
    }

    /// Checks if a minor civ can be placed on a tile
    fn can_place_minor_civ(tile: &Tile, tile_data: &MapGenTileData) -> bool {
        !tile.is_water &&
        !tile.is_impassible() &&
        !tile_data.get(tile.position).map_or(false, |data| data.is_junk) &&
        tile.get_base_terrain().get_matching_uniques(UniqueType::HasQuality)
            .iter()
            .none(|unique| unique.params[0] == "Undesirable") && // So we don't get snow hills
        tile.neighbors.len() == 6 // Avoid map edges
    }

    /// Places a minor civ on a tile
    fn place_minor_civ(
        civ: Civilization,
        tile_map: &TileMap,
        tile: &Tile,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset
    ) {
        tile_map.add_starting_location(&civ.civ_name, tile);

        tile_data.place_impact(ImpactType::MinorCiv, tile, 4);
        tile_data.place_impact(ImpactType::Luxury, tile, 3);
        tile_data.place_impact(ImpactType::Strategic, tile, 0);
        tile_data.place_impact(ImpactType::Bonus, tile, 3);

        StartNormalizer::normalize_start(tile, tile_map, tile_data, ruleset, true);
    }
}