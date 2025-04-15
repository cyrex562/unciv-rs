use std::collections::HashMap;
use crate::map::tile::Tile;
use crate::models::ruleset::Ruleset;
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::map::mapgenerator::mapregions::MapRegions;

/// Holds a bunch of tile info that is only interesting during map gen
pub struct MapGenTileData<'a> {
    pub tile: &'a Tile,
    pub region: Option<&'a Region>,
    pub close_start_penalty: i32,
    pub impacts: HashMap<MapRegions::ImpactType, i32>,
    pub is_food: bool,
    pub is_prod: bool,
    pub is_good: bool,
    pub is_junk: bool,
    pub is_two_from_coast: bool,
    pub is_good_start: bool,
    pub start_score: i32,
}

impl<'a> MapGenTileData<'a> {
    /// Create a new MapGenTileData instance
    pub fn new(tile: &'a Tile, region: Option<&'a Region>, ruleset: &Ruleset) -> Self {
        let mut data = MapGenTileData {
            tile,
            region,
            close_start_penalty: 0,
            impacts: HashMap::new(),
            is_food: false,
            is_prod: false,
            is_good: false,
            is_junk: false,
            is_two_from_coast: false,
            is_good_start: true,
            start_score: 0,
        };

        data.evaluate(ruleset);
        data
    }

    /// Add a penalty for being close to a start position
    pub fn add_close_start_penalty(&mut self, penalty: i32) {
        if self.close_start_penalty == 0 {
            self.close_start_penalty = penalty;
        } else {
            // Multiple overlapping values - take the higher one and add 20%
            self.close_start_penalty = self.close_start_penalty.max(penalty);
            self.close_start_penalty = (self.close_start_penalty as f32 * 1.2).min(97.0) as i32;
        }
    }

    /// Populates all private-set fields
    fn evaluate(&mut self, ruleset: &Ruleset) {
        // Check if we are two tiles from coast (a bad starting site)
        if !self.tile.is_coastal_tile() && self.tile.neighbors.iter().any(|t| t.is_coastal_tile()) {
            self.is_two_from_coast = true;
        }

        // Check first available out of unbuildable features, then other features, then base terrain
        let terrain_to_check = if self.tile.terrain_features.is_empty() {
            self.tile.get_base_terrain()
        } else {
            self.tile.terrain_feature_objects.iter()
                .find(|f| f.unbuildable)
                .unwrap_or_else(|| self.tile.terrain_feature_objects.first().unwrap())
        };

        // Add all applicable qualities
        for unique in terrain_to_check.get_matching_uniques(
            UniqueType::HasQuality,
            StateForConditionals { region: self.region }
        ) {
            match unique.params[0].as_str() {
                "Food" => self.is_food = true,
                "Desirable" => self.is_good = true,
                "Production" => self.is_prod = true,
                "Undesirable" => self.is_junk = true,
                _ => {}
            }
        }

        // Were there in fact no explicit qualities defined for any region at all? If so let's guess at qualities to preserve mod compatibility.
        if !terrain_to_check.unique_objects.iter().any(|u| u.r#type == UniqueType::HasQuality) {
            if self.tile.is_water {
                return; // Most water type tiles have no qualities
            }

            // is it junk???
            if terrain_to_check.impassable {
                self.is_junk = true;
                return; // Don't bother checking the rest, junk is junk
            }

            // Take possible improvements into account
            let improvements: Vec<_> = ruleset.tile_improvements.values()
                .filter(|imp| {
                    terrain_to_check.name == imp.terrains_can_be_built_on &&
                    imp.unique_to.is_none() &&
                    !imp.has_unique(UniqueType::GreatImprovement)
                })
                .collect();

            let max_food = terrain_to_check.food + improvements.iter()
                .map(|imp| imp.food)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            let max_prod = terrain_to_check.production + improvements.iter()
                .map(|imp| imp.production)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            let best_improvement_value = improvements.iter()
                .map(|imp| imp.food + imp.production + imp.gold + imp.culture + imp.science + imp.faith)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            let max_overall = terrain_to_check.food + terrain_to_check.production + terrain_to_check.gold +
                terrain_to_check.culture + terrain_to_check.science + terrain_to_check.faith + best_improvement_value;

            if max_food >= 2.0 {
                self.is_food = true;
            }
            if max_prod >= 2.0 {
                self.is_prod = true;
            }
            if max_overall >= 3.0 {
                self.is_good = true;
            }
        }
    }
}