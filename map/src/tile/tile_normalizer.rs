use crate::{
    map::tile::Tile,
    models::ruleset::{Ruleset, StateForConditionals},
    models::ruleset::tile::TerrainType,
};

/// Provides functionality for normalizing tiles to match the ruleset
pub struct TileNormalizer;

impl TileNormalizer {
    /// Normalizes a tile to match the ruleset
    pub fn normalize_to_ruleset(tile: &mut Tile, ruleset: &Ruleset) {
        // Check if the natural wonder exists in the ruleset
        if let Some(natural_wonder) = &tile.natural_wonder {
            if !ruleset.terrains.contains_key(natural_wonder) {
                tile.natural_wonder = None;
            } else {
                let wonder_terrain = tile.get_natural_wonder();
                if let Some(turns_into) = &wonder_terrain.turns_into {
                    tile.base_terrain = turns_into.clone();
                    tile.remove_terrain_features();
                } else {
                    tile.set_terrain_features(
                        tile.terrain_features.iter()
                            .filter(|&feature| wonder_terrain.occurs_on.contains(feature))
                            .cloned()
                            .collect()
                    );
                }
                tile.resource = None;
                tile.clear_improvement();
            }
        }

        // Check if the base terrain exists in the ruleset
        if !ruleset.terrains.contains_key(&tile.base_terrain) {
            // Find the first land terrain that is not impassable
            if let Some(land_terrain) = ruleset.terrains.values()
                .find(|terrain| terrain.terrain_type == TerrainType::Land && !terrain.impassable)
            {
                tile.base_terrain = land_terrain.name.clone();
            }
        }

        // Check if the terrain features are valid
        let new_features: Vec<String> = tile.terrain_features.iter()
            .filter(|&terrain_feature| {
                if let Some(terrain_feature_object) = ruleset.terrains.get(terrain_feature) {
                    terrain_feature_object.occurs_on.is_empty() ||
                    terrain_feature_object.occurs_on.contains(&tile.base_terrain)
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        if new_features.len() != tile.terrain_features.len() {
            tile.set_terrain_features(new_features);
        }

        // Check if the resource is valid
        if let Some(resource) = &tile.resource {
            if !ruleset.tile_resources.contains_key(resource) {
                tile.resource = None;
            } else {
                let resource_object = ruleset.tile_resources.get(resource).unwrap();
                if !resource_object.terrains_can_be_found_on.iter()
                    .any(|terrain| terrain == &tile.base_terrain || tile.terrain_features.contains(terrain))
                {
                    tile.resource = None;
                }
            }
        }

        // Check if the improvement is valid
        if tile.improvement.is_some() {
            Self::normalize_tile_improvement(tile, ruleset);
        }

        // Remove roads from water or impassable tiles
        if tile.is_water || tile.is_impassible() {
            tile.remove_road();
        }
    }

    /// Normalizes a tile's improvement to match the ruleset
    fn normalize_tile_improvement(tile: &mut Tile, ruleset: &Ruleset) {
        if let Some(improvement) = &tile.improvement {
            if let Some(improvement_object) = ruleset.tile_improvements.get(improvement) {
                if tile.improvement_functions.can_improvement_be_built_here(
                    improvement_object,
                    StateForConditionals::IgnoreConditionals,
                    true
                ) {
                    return;
                }
            }
        }
        tile.clear_improvement();
    }
}

impl Tile {
    /// Clears the improvement on this tile
    fn clear_improvement(&mut self) {
        // This runs from mapgen, so don't go through the side-effect-triggering TileImprovementFunctions
        self.improvement = None;
        self.stop_working_on_improvement();
    }
}