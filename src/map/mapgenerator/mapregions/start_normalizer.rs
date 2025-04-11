use std::collections::HashMap;
use std::f32;
use crate::constants::Constants;
use crate::map::tile_map::TileMap;
use crate::map::mapgenerator::resourceplacement::map_region_resources::MapRegionResources;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::unique::StateForConditionals;
use crate::models::ruleset::unique::UniqueType;
use crate::models::stats::Stat;
use crate::map::mapgenerator::mapregions::map_gen_tile_data::MapGenTileData;

/// Ensures that starting positions of civs have enough yield that they aren't at a disadvantage
pub struct StartNormalizer;

impl StartNormalizer {
    /// Attempts to improve the start on start_tile as needed to make it decent.
    /// Relies on start_position having been set previously.
    /// Assumes unchanged baseline values ie citizens eat 2 food each, similar production costs
    /// If is_minor_civ is true, different weightings will be used.
    pub fn normalize_start(
        start_tile: &mut Tile,
        tile_map: &TileMap,
        tile_data: &mut MapGenTileData,
        ruleset: &Ruleset,
        is_minor_civ: bool
    ) {
        // Remove ice-like features adjacent to start
        for tile in start_tile.neighbors() {
            if let Some(last_terrain) = tile.terrain_feature_objects.iter()
                .filter(|t| t.impassable)
                .last() {
                tile.remove_terrain_feature(&last_terrain.name);
            }
        }

        if !is_minor_civ && tile_map.map_parameters.get_strategic_balance() {
            Self::place_strategic_balance_resources(start_tile, ruleset, tile_data);
        }

        Self::normalize_production(start_tile, is_minor_civ, ruleset, tile_data);

        let food_bonuses_needed = Self::calculate_food_bonuses_needed(start_tile, is_minor_civ, ruleset, tile_map);
        Self::place_food_bonuses(is_minor_civ, start_tile, ruleset, food_bonuses_needed);

        // Minor civs are done, go on with grassiness checks for major civs
        if is_minor_civ {
            return;
        }

        Self::add_production_bonuses(start_tile, ruleset);
    }

    fn normalize_production(
        start_tile: &Tile,
        is_minor_civ: bool,
        ruleset: &Ruleset,
        tile_data: &mut MapGenTileData
    ) {
        // evaluate production potential
        let inner_production = start_tile.neighbors()
            .map(|t| Self::get_potential_yield(t, Stat::Production) as i32)
            .sum::<i32>();

        let outer_production = start_tile.get_tiles_at_distance(2)
            .map(|t| Self::get_potential_yield(t, Stat::Production) as i32)
            .sum::<i32>();

        // for very early production we ideally want tiles that also give food
        let early_production = start_tile.get_tiles_in_distance_range(1..=2)
            .map(|t| {
                if Self::get_potential_yield(t, Stat::Food, true) > 0.0 {
                    Self::get_potential_yield(t, Stat::Production, true) as i32
                } else {
                    0
                }
            })
            .sum::<i32>();

        // If terrible, try adding a hill to a dry flat tile
        if inner_production == 0 || (inner_production < 2 && outer_production < 8) || (is_minor_civ && inner_production < 4) {
            let hill_spots: Vec<&Tile> = start_tile.neighbors()
                .filter(|t| t.is_land && t.terrain_features.is_empty() && !t.is_adjacent_to(&Constants::fresh_water) && !t.is_impassible())
                .collect();

            if let Some(hill_spot) = hill_spots.choose(&mut rand::thread_rng()) {
                let hill_equivalent = ruleset.terrains.values()
                    .find(|t| t.terrain_type == TerrainType::TerrainFeature &&
                              t.production >= 2 &&
                              !t.has_unique(UniqueType::RareFeature))
                    .map(|t| t.name.clone());

                if let Some(hill_name) = hill_equivalent {
                    hill_spot.add_terrain_feature(&hill_name);
                }
            }
        }

        // If bad early production, add a small strategic resource to SECOND ring (not for minors)
        if !is_minor_civ && inner_production < 3 && early_production < 6 {
            let last_era_number = ruleset.eras.values()
                .map(|e| e.era_number)
                .max()
                .unwrap_or(0);

            let early_eras: Vec<_> = ruleset.eras.values()
                .filter(|e| e.era_number <= last_era_number / 3)
                .collect();

            let valid_resources: Vec<_> = ruleset.tile_resources.values()
                .filter(|r| r.resource_type == ResourceType::Strategic &&
                           (r.revealed_by.is_none() ||
                            early_eras.iter().any(|e| e.name == ruleset.technologies[r.revealed_by.as_ref().unwrap()].era())))
                .collect();

            let mut candidate_tiles: Vec<&Tile> = start_tile.get_tiles_at_distance(2).collect();
            candidate_tiles.shuffle(&mut rand::thread_rng());

            for resource in valid_resources {
                let resources_added = MapRegionResources::try_adding_resource_to_tiles(
                    tile_data, resource, 1, &candidate_tiles, false);
                if resources_added > 0 {
                    break;
                }
            }
        }
    }

    fn place_strategic_balance_resources(
        start_tile: &Tile,
        ruleset: &Ruleset,
        tile_data: &mut MapGenTileData
    ) {
        let mut candidate_tiles: Vec<&Tile> = start_tile.get_tiles_in_distance_range(1..=2).collect();
        candidate_tiles.extend(start_tile.get_tiles_at_distance(3));
        candidate_tiles.shuffle(&mut rand::thread_rng());

        for resource in ruleset.tile_resources.values()
            .filter(|r| r.has_unique(UniqueType::StrategicBalanceResource)) {
            if MapRegionResources::try_adding_resource_to_tiles(
                tile_data, resource, 1, &candidate_tiles, true) == 0 {
                // Fallback mode - force placement, even on an otherwise inappropriate terrain. Do still respect water and impassible tiles!
                let resource_tiles: Vec<&Tile> = if Self::is_water_only_resource(resource, ruleset) {
                    candidate_tiles.iter()
                        .filter(|t| t.is_water && !t.is_impassible())
                        .cloned()
                        .collect()
                } else {
                    candidate_tiles.iter()
                        .filter(|t| t.is_land && !t.is_impassible())
                        .cloned()
                        .collect()
                };

                MapRegionResources::place_resources_in_tiles(
                    tile_data, 999, &resource_tiles, vec![resource], true);
            }
        }
    }

    /// Check for very food-heavy starts that might still need some stone to help with production
    fn add_production_bonuses(start_tile: &Tile, ruleset: &Ruleset) {
        let mut grass_type_plots: Vec<&Tile> = start_tile.get_tiles_in_distance_range(1..=2)
            .filter(|t| t.is_land &&
                      Self::get_potential_yield(t, Stat::Food, true) >= 2.0 && // Food neutral natively
                      Self::get_potential_yield(t, Stat::Production) == 0.0) // Production can't even be improved
            .collect();

        let plains_type_plots: Vec<&Tile> = start_tile.get_tiles_in_distance_range(1..=2)
            .filter(|t| t.is_land &&
                      Self::get_potential_yield(t, Stat::Food) >= 2.0 && // Something that can be improved to food neutral
                      Self::get_potential_yield(t, Stat::Production, true) >= 1.0) // Some production natively
            .collect();

        let mut production_bonuses_needed = match (grass_type_plots.len(), plains_type_plots.len()) {
            (g, _) if g >= 9 && plains_type_plots.is_empty() => 2,
            (g, p) if g >= 6 && p <= 4 => 1,
            _ => 0
        };

        let production_bonuses: Vec<_> = ruleset.tile_resources.values()
            .filter(|r| r.resource_type == ResourceType::Bonus && r.production > 0)
            .collect();

        if !production_bonuses.is_empty() {
            while production_bonuses_needed > 0 && !grass_type_plots.is_empty() {
                let plot_idx = rand::random::<usize>() % grass_type_plots.len();
                let plot = grass_type_plots.remove(plot_idx);

                if plot.resource.is_some() {
                    continue;
                }

                let valid_bonuses: Vec<_> = production_bonuses.iter()
                    .filter(|b| b.generates_naturally_on(plot))
                    .collect();

                if let Some(bonus_to_place) = valid_bonuses.choose(&mut rand::thread_rng()) {
                    plot.resource = Some(bonus_to_place.name.clone());
                    production_bonuses_needed -= 1;
                }
            }
        }
    }

    fn calculate_food_bonuses_needed(
        start_tile: &Tile,
        minor_civ: bool,
        ruleset: &Ruleset,
        tile_map: &TileMap
    ) -> i32 {
        // evaluate food situation
        // Food²/4 because excess food is really good and lets us work other tiles or run specialists!
        // 2F is worth 1, 3F is worth 2, 4F is worth 4, 5F is worth 6 and so on
        let inner_food = start_tile.neighbors()
            .map(|t| (Self::get_potential_yield(t, Stat::Food).powi(2) / 4.0) as i32)
            .sum::<i32>();

        let outer_food = start_tile.get_tiles_at_distance(2)
            .map(|t| (Self::get_potential_yield(t, Stat::Food).powi(2) / 4.0) as i32)
            .sum::<i32>();

        let total_food = inner_food + outer_food;

        // we want at least some two-food tiles to keep growing
        let inner_native_two_food = start_tile.neighbors()
            .filter(|t| Self::get_potential_yield(t, Stat::Food, true) >= 2.0)
            .count() as i32;

        let outer_native_two_food = start_tile.get_tiles_at_distance(2)
            .filter(|t| Self::get_potential_yield(t, Stat::Food, true) >= 2.0)
            .count() as i32;

        let total_native_two_food = inner_native_two_food + outer_native_two_food;

        // Determine number of needed bonuses. Different weightings for minor and major civs.
        let mut bonuses_needed = if minor_civ {
            match (total_food, inner_food) {
                (t, i) if t < 12 || i < 4 => 2,
                (t, i) if t < 16 || i < 9 => 1,
                _ => 0
            }
        } else {
            match (total_food, inner_food, total_native_two_food, inner_native_two_food) {
                (0, t, _, _) if t < 4 => 5,
                (t, _, _, _) if t < 6 => 4,
                (t, i, _, _) if t < 8 || (t < 12 && i < 5) => 3,
                (t, i, n, _) if (t < 17 && i < 9) || n < 2 => 2,
                (t, i, n, i2) if (t < 24 && i < 11) || n == 2 || i2 == 0 || t < 20 => 1,
                _ => 0
            }
        };

        if tile_map.map_parameters.get_legendary_start() {
            bonuses_needed += 2;
        }

        // Attempt to place one grassland at a plains-only spot (nor for minors)
        if !minor_civ && bonuses_needed < 3 && total_native_two_food == 0 {
            let two_food_terrain = ruleset.terrains.values()
                .find(|t| t.terrain_type == TerrainType::Land && t.food >= 2)
                .map(|t| t.name.clone());

            let candidate_inner_spots: Vec<&Tile> = start_tile.neighbors()
                .filter(|t| t.is_land && !t.is_impassible() && t.terrain_features.is_empty() && t.resource.is_none())
                .collect();

            let candidate_outer_spots: Vec<&Tile> = start_tile.get_tiles_at_distance(2)
                .filter(|t| t.is_land && !t.is_impassible() && t.terrain_features.is_empty() && t.resource.is_none())
                .collect();

            let spot = candidate_inner_spots.choose(&mut rand::thread_rng())
                .or_else(|| candidate_outer_spots.choose(&mut rand::thread_rng()));

            if let (Some(terrain), Some(tile)) = (two_food_terrain, spot) {
                tile.base_terrain = terrain;
            } else {
                bonuses_needed = 3; // Irredeemable plains situation
            }
        }

        bonuses_needed
    }

    fn place_food_bonuses(
        minor_civ: bool,
        start_tile: &Tile,
        ruleset: &Ruleset,
        food_bonuses_needed: i32
    ) {
        let mut bonuses_still_needed = food_bonuses_needed;

        let oasis_equivalent = ruleset.terrains.values()
            .find(|t| t.terrain_type == TerrainType::TerrainFeature &&
                      t.has_unique(UniqueType::RareFeature) &&
                      t.food >= 2 &&
                      t.food + t.production + t.gold >= 3 &&
                      t.occurs_on.iter().any(|base| ruleset.terrains[base].terrain_type == TerrainType::Land))
            .map(|t| t.name.clone());

        let mut can_place_oasis = oasis_equivalent.is_some(); // One oasis per start is enough. Don't bother finding a place if there is no good oasis equivalent
        let mut placed_in_first = 0; // Attempt to put first 2 in inner ring and next 3 in second ring
        let mut placed_in_second = 0;
        let range_for_bonuses = if minor_civ { 2 } else { 3 };

        // Start with list of candidate plots sorted in ring order 1,2,3
        let mut candidate_plots: Vec<&Tile> = start_tile.get_tiles_in_distance_range(1..=range_for_bonuses)
            .filter(|t| t.resource.is_none() &&
                      !t.terrain_feature_objects.iter().any(|f| Some(&f.name) == oasis_equivalent.as_ref()))
            .collect();

        candidate_plots.shuffle(&mut rand::thread_rng());
        candidate_plots.sort_by(|a, b| {
            a.aerial_distance_to(start_tile).partial_cmp(&b.aerial_distance_to(start_tile)).unwrap()
        });

        // Place food bonuses (and oases) as able
        while bonuses_still_needed > 0 && !candidate_plots.is_empty() {
            let plot = candidate_plots.remove(0); // remove the plot as it has now been tried, whether successfully or not

            if plot.get_base_terrain().has_unique(
                UniqueType::BlocksResources,
                StateForConditionals::new(attacked_tile: Some(plot.clone()))
            ) {
                continue; // Don't put bonuses on snow hills
            }

            let valid_bonuses: Vec<_> = ruleset.tile_resources.values()
                .filter(|r| r.resource_type == ResourceType::Bonus &&
                           r.food >= 1 &&
                           r.generates_naturally_on(plot))
                .collect();

            let good_plot_for_oasis = can_place_oasis &&
                oasis_equivalent.as_ref().map_or(false, |o| {
                    plot.last_terrain.name == *o
                });

            if !valid_bonuses.is_empty() || good_plot_for_oasis {
                if good_plot_for_oasis {
                    if let Some(oasis) = oasis_equivalent.as_ref() {
                        plot.add_terrain_feature(oasis);
                        can_place_oasis = false;
                    }
                } else {
                    let bonus = valid_bonuses.choose(&mut rand::thread_rng()).unwrap();
                    plot.set_tile_resource(bonus);
                }

                if plot.aerial_distance_to(start_tile) == 1.0 {
                    placed_in_first += 1;
                    if placed_in_first == 2 { // Resort the list in ring order 2,3,1
                        candidate_plots.sort_by(|a, b| {
                            (a.aerial_distance_to(start_tile) * 10.0 - 22.0).abs()
                                .partial_cmp(&(b.aerial_distance_to(start_tile) * 10.0 - 22.0).abs())
                                .unwrap()
                        });
                    }
                } else if plot.aerial_distance_to(start_tile) == 2.0 {
                    placed_in_second += 1;
                    if placed_in_second == 3 { // Resort the list in ring order 3,1,2
                        candidate_plots.sort_by(|a, b| {
                            (b.aerial_distance_to(start_tile) * 10.0 - 17.0).abs()
                                .partial_cmp(&(a.aerial_distance_to(start_tile) * 10.0 - 17.0).abs())
                                .unwrap()
                        });
                    }
                }
                bonuses_still_needed -= 1;
            }
        }
    }

    fn get_potential_yield(tile: &Tile, stat: Stat, unimproved: bool) -> f32 {
        let base_yield = tile.stats.get_tile_stats(None::<&String>)[stat];
        if unimproved {
            return base_yield;
        }

        let best_improvement_yield = tile.tile_map.ruleset.as_ref()
            .map(|r| r.tile_improvements.values()
                .filter(|i| !i.has_unique(UniqueType::GreatImprovement) &&
                           i.unique_to.is_none() &&
                           tile.last_terrain.name in i.terrains_can_be_built_on)
                .map(|i| i[stat])
                .max()
                .unwrap_or(0.0))
            .unwrap_or(0.0);

        base_yield + best_improvement_yield
    }

    fn is_water_only_resource(resource: &TileResource, ruleset: &Ruleset) -> bool {
        resource.occurs_on.iter().all(|terrain_name| {
            ruleset.terrains[terrain_name].terrain_type == TerrainType::Water
        })
    }
}