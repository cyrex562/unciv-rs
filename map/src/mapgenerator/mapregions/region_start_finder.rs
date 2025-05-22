use std::collections::{HashMap, HashSet};
use std::cmp::min;
use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::models::ruleset::tile::TerrainType;
use crate::math::rectangle::Rectangle;
use crate::math::vector2::Vector2;
use crate::map::mapgenerator::mapregions::map_regions::{MapRegions, close_start_penalty_for_ring, first_ring_food_scores, first_ring_prod_scores, maximum_junk, minimum_food_for_ring, minimum_good_for_ring, minimum_prod_for_ring, second_ring_food_scores, second_ring_prod_scores};
use crate::map::mapgenerator::mapregions::map_gen_tile_data::MapGenTileData;
use crate::map::mapgenerator::mapregions::region::Region;
use crate::constants::Constants;

/// Handles finding good starting positions for civilizations within regions
pub struct RegionStartFinder;

impl RegionStartFinder {
    /// Attempts to find a good start close to the center of region. Calls set_region_start with the position
    pub fn find_start(region: &mut Region, tile_data: &mut MapGenTileData) {
        let mut fallback_tiles = HashSet::new();
        // Priority: 1. Adjacent to river, 2. Adjacent to coast or fresh water, 3. Other.

        // First check center rect, then middle. Only check the outer area if no good sites found
        let center_rect = Self::get_central_rectangle(&region.rect, 0.33);
        let center_tiles: Vec<Tile> = region.tile_map.get_tiles_in_rectangle(&center_rect).collect();

        if Self::find_good_position(&center_tiles, region, tile_data, &mut fallback_tiles) {
            return;
        }

        let middle_rect = Self::get_central_rectangle(&region.rect, 0.67);
        let middle_tiles: Vec<Tile> = region.tile_map.get_tiles_in_rectangle(&middle_rect)
            .filter(|t| !center_tiles.iter().any(|ct| ct.position == t.position))
            .collect();

        if Self::find_good_position(&middle_tiles, region, tile_data, &mut fallback_tiles) {
            return;
        }

        // Now check the outer tiles. For these we don't care about rivers, coasts etc
        let outer_tiles: Vec<Tile> = region.tile_map.get_tiles_in_rectangle(&region.rect)
            .filter(|t| !center_tiles.iter().any(|ct| ct.position == t.position) &&
                      !middle_tiles.iter().any(|mt| mt.position == t.position))
            .collect();

        if Self::find_edge_position(&outer_tiles, region, tile_data, &mut fallback_tiles) {
            return;
        }

        Self::find_fallback_position(&fallback_tiles, tile_data, region);
    }

    /// Finds a good position in the given tiles
    fn find_good_position(
        center_tiles: &[Tile],
        region: &Region,
        tile_data: &mut MapGenTileData,
        fallback_tiles: &mut HashSet<Vector2>
    ) -> bool {
        let mut river_tiles = HashSet::new();
        let mut wet_tiles = HashSet::new();
        let mut dry_tiles = HashSet::new();

        for tile in center_tiles {
            if let Some(data) = tile_data.get(tile.position) {
                if data.is_two_from_coast {
                    continue; // Don't even consider tiles two from coast
                }
            }

            if region.continent_id != -1 && region.continent_id != tile.get_continent() {
                continue; // Wrong continent
            }

            if tile.is_land && !tile.is_impassible() {
                Self::evaluate_tile_for_start(tile, tile_data);

                if tile.is_adjacent_to_river() {
                    river_tiles.insert(tile.position);
                } else if tile.is_coastal_tile() || tile.is_adjacent_to(Constants::fresh_water) {
                    wet_tiles.insert(tile.position);
                } else {
                    dry_tiles.insert(tile.position);
                }
            }
        }

        // Did we find a good start position?
        for list in [&river_tiles, &wet_tiles, &dry_tiles] {
            if list.iter().any(|pos| {
                tile_data.get(*pos).map_or(false, |data| data.is_good_start)
            }) {
                let best_pos = list.iter()
                    .filter(|pos| tile_data.get(**pos).map_or(false, |data| data.is_good_start))
                    .max_by_key(|pos| tile_data.get(**pos).map_or(0, |data| data.start_score))
                    .unwrap();

                Self::set_region_start(region, *best_pos, tile_data);
                return true;
            }

            if !list.is_empty() {
                // Save the best not-good-enough spots for later fallback
                if let Some(best_pos) = list.iter()
                    .max_by_key(|pos| tile_data.get(**pos).map_or(0, |data| data.start_score)) {
                    fallback_tiles.insert(*best_pos);
                }
            }
        }

        false
    }

    /// Finds a position on the edge of the region
    fn find_edge_position(
        outer_donut: &[Tile],
        region: &Region,
        tile_data: &mut MapGenTileData,
        fallback_tiles: &mut HashSet<Vector2>
    ) -> bool {
        let mut dry_tiles = HashSet::new();

        for tile in outer_donut {
            if region.continent_id != -1 && region.continent_id != tile.get_continent() {
                continue; // Wrong continent
            }

            if tile.is_land && !tile.is_impassible() {
                Self::evaluate_tile_for_start(tile, tile_data);
                dry_tiles.insert(tile.position);
            }
        }

        // Were any of them good?
        if dry_tiles.iter().any(|pos| tile_data.get(*pos).map_or(false, |data| data.is_good_start)) {
            // Find the one closest to the center
            let center = region.rect.get_center(Vector2::new(0.0, 0.0));
            let center_tile = region.tile_map.get_if_tile_exists_or_null(
                center.x.round() as i32,
                center.y.round() as i32
            ).unwrap_or_else(|| region.tile_map.values().next().unwrap());

            let closest_to_center = dry_tiles.iter()
                .filter(|pos| tile_data.get(**pos).map_or(false, |data| data.is_good_start))
                .min_by_key(|pos| {
                    let tile = region.tile_map.get_if_tile_exists_or_null(
                        pos.x.round() as i32,
                        pos.y.round() as i32
                    ).unwrap_or_else(|| region.tile_map.values().next().unwrap());

                    center_tile.aerial_distance_to(tile)
                })
                .unwrap();

            Self::set_region_start(region, *closest_to_center, tile_data);
            return true;
        }

        if !dry_tiles.is_empty() {
            // Save the best not-good-enough spots for later fallback
            if let Some(best_pos) = dry_tiles.iter()
                .max_by_key(|pos| tile_data.get(**pos).map_or(0, |data| data.start_score)) {
                fallback_tiles.insert(*best_pos);
            }
        }

        false
    }

    /// Finds a fallback position when no good positions are found
    fn find_fallback_position(
        fallback_tiles: &HashSet<Vector2>,
        tile_data: &mut MapGenTileData,
        region: &mut Region
    ) {
        // Fallback time. Just pick the one with best score
        if let Some(fallback_position) = fallback_tiles.iter()
            .max_by_key(|pos| tile_data.get(**pos).map_or(0, |data| data.start_score)) {
            Self::set_region_start(region, *fallback_position, tile_data);
            return;
        }

        // Something went extremely wrong and there is somehow no place to start. Spawn some land and start there
        let panic_position = region.rect.get_position(Vector2::new(0.0, 0.0));
        let panic_terrain = region.tile_map.ruleset.as_ref().unwrap().terrains.values()
            .find(|t| t.terrain_type == TerrainType::Land)
            .unwrap()
            .name
            .clone();

        region.tile_map.get_mut(panic_position).base_terrain = panic_terrain;
        region.tile_map.get_mut(panic_position).set_terrain_features(Vec::new());
        Self::set_region_start(region, panic_position, tile_data);
    }

    /// Returns a scaled according to proportion Rectangle centered over original_rect
    fn get_central_rectangle(original_rect: &Rectangle, proportion: f32) -> Rectangle {
        let mut scaled_rect = original_rect.clone();

        scaled_rect.width = original_rect.width * proportion;
        scaled_rect.height = original_rect.height * proportion;
        scaled_rect.x = original_rect.x + (original_rect.width - scaled_rect.width) / 2.0;
        scaled_rect.y = original_rect.y + (original_rect.height - scaled_rect.height) / 2.0;

        // round values
        scaled_rect.x = scaled_rect.x.round();
        scaled_rect.y = scaled_rect.y.round();
        scaled_rect.width = scaled_rect.width.round();
        scaled_rect.height = scaled_rect.height.round();

        scaled_rect
    }

    /// Evaluates a tile for starting position, setting is_good_start and start_score in
    /// MapGenTileData. Assumes that all tiles have corresponding MapGenTileData.
    fn evaluate_tile_for_start(tile: &Tile, tile_data: &mut MapGenTileData) {
        let local_data = tile_data.get_mut(tile.position).unwrap();

        let mut total_food = 0;
        let mut total_prod = 0;
        let mut total_good = 0;
        let mut total_junk = 0;
        let mut total_rivers = 0;
        let mut total_score = 0;

        if tile.is_coastal_tile() {
            total_score += 40;
        }

        // Go through all rings
        for ring in 1..=3 {
            // Sum up the values for this ring
            for outer_tile in tile.get_tiles_at_distance(ring) {
                let outer_tile_data = tile_data.get(outer_tile.position).unwrap();
                if outer_tile_data.is_junk {
                    total_junk += 1;
                } else {
                    if outer_tile_data.is_food {
                        total_food += 1;
                    }
                    if outer_tile_data.is_prod {
                        total_prod += 1;
                    }
                    if outer_tile_data.is_good {
                        total_good += 1;
                    }
                    if outer_tile.is_adjacent_to_river() {
                        total_rivers += 1;
                    }
                }
            }

            // Check for minimum levels. We still keep on calculating final score in case of failure
            if total_food < minimum_food_for_ring[ring].unwrap_or(0) ||
               total_prod < minimum_prod_for_ring[ring].unwrap_or(0) ||
               total_good < minimum_good_for_ring[ring].unwrap_or(0) {
                local_data.is_good_start = false;
            }

            // Ring-specific scoring
            match ring {
                1 => {
                    let food_score = first_ring_food_scores.get(&total_food).unwrap_or(&0);
                    let prod_score = first_ring_prod_scores.get(&total_prod).unwrap_or(&0);
                    total_score += food_score + prod_score + total_rivers + (total_good * 2) - (total_junk * 3);
                },
                2 => {
                    let food_score = if total_food > 10 {
                        second_ring_food_scores.values().last().unwrap_or(&0)
                    } else {
                        second_ring_food_scores.get(&total_food).unwrap_or(&0)
                    };

                    let effective_total_prod = if total_prod >= total_food * 2 {
                        total_prod
                    } else {
                        (total_food + 1) / 2 // Can't use all that production without food
                    };

                    let prod_score = if effective_total_prod > 5 {
                        second_ring_prod_scores.values().last().unwrap_or(&0)
                    } else {
                        second_ring_prod_scores.get(&effective_total_prod).unwrap_or(&0)
                    };

                    total_score += food_score + prod_score + total_rivers + (total_good * 2) - (total_junk * 3);
                },
                _ => {
                    total_score += total_food + total_prod + total_good + total_rivers - (total_junk * 2);
                }
            }
        }

        // Too much junk?
        if total_junk > maximum_junk {
            local_data.is_good_start = false;
        }

        // Finally check if this is near another start
        if local_data.close_start_penalty > 0 {
            local_data.is_good_start = false;
            total_score -= (total_score * local_data.close_start_penalty) / 100;
        }

        local_data.start_score = total_score;
    }

    /// Sets the region start position and applies penalties to nearby tiles
    fn set_region_start(region: &mut Region, position: Vector2, tile_data: &mut MapGenTileData) {
        region.start_position = Some(position);

        for (ring, penalty) in close_start_penalty_for_ring.iter() {
            for outer_tile in region.tile_map.get(position).get_tiles_at_distance(*ring)
                .map(|t| t.position) {
                if let Some(data) = tile_data.get_mut(outer_tile) {
                    data.add_close_start_penalty(*penalty);
                }
            }
        }
    }
}