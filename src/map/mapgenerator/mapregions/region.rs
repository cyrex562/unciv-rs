use std::collections::{HashMap, HashSet};
use std::cmp::{max, min};
use crate::map::tile_map::TileMap;
use crate::map::tile::Tile;
use crate::civilization::Civilization;
use crate::models::ruleset::unique::UniqueType;
use crate::math::rectangle::Rectangle;
use crate::math::vector2::Vector2;

/// Represents a region on the map, which can be used for map generation and civilization placement
pub struct Region {
    /// The tile map this region belongs to
    pub tile_map: TileMap,
    /// The rectangular bounds of this region
    pub rect: Rectangle,
    /// The continent ID this region belongs to, or -1 if not assigned to a continent
    pub continent_id: i32,
    /// The tiles in this region
    pub tiles: HashSet<Tile>,
    /// Counts of different terrain types in this region
    pub terrain_counts: HashMap<String, i32>,
    /// The total fertility of this region
    pub total_fertility: i32,
    /// The type of this region (e.g., "Hybrid", "Desert", etc.)
    pub region_type: String,
    /// The luxury resource type associated with this region
    pub luxury: Option<String>,
    /// The starting position for civilizations in this region
    pub start_position: Option<Vector2>,
    /// Minor civilizations assigned to this region
    pub assigned_minor_civs: Vec<Civilization>,
    /// Whether this region is affected by world wrapping
    pub affected_by_world_wrap: bool,
}

impl Region {
    /// Creates a new region with the given tile map, rectangle, and continent ID
    pub fn new(tile_map: TileMap, rect: Rectangle, continent_id: i32) -> Self {
        Region {
            tile_map,
            rect,
            continent_id,
            tiles: HashSet::new(),
            terrain_counts: HashMap::new(),
            total_fertility: 0,
            region_type: "Hybrid".to_string(), // being an undefined or indeterminate type
            luxury: None,
            start_position: None,
            assigned_minor_civs: Vec::new(),
            affected_by_world_wrap: false,
        }
    }

    /// Recalculates tiles and fertility
    pub fn update_tiles(&mut self, trim: bool) {
        self.total_fertility = 0;
        let mut min_column = 99999.0;
        let mut max_column = -99999.0;
        let mut min_row = 99999.0;
        let mut max_row = -99999.0;

        let mut column_has_tile = HashSet::new();

        self.tiles.clear();
        for tile in self.tile_map.get_tiles_in_rectangle(&self.rect).filter(|t| {
            self.continent_id == -1 || t.get_continent() == self.continent_id
        }) {
            let fertility = tile.get_tile_fertility(self.continent_id != -1);
            self.tiles.insert(tile.clone());
            self.total_fertility += fertility;

            if self.affected_by_world_wrap {
                column_has_tile.insert(tile.get_column());
            }

            if trim {
                let row = tile.get_row() as f32;
                let column = tile.get_column() as f32;
                min_column = min(min_column, column);
                max_column = max(max_column, column);
                min_row = min(min_row, row);
                max_row = max(max_row, row);
            }
        }

        if trim {
            if self.affected_by_world_wrap {
                // Need to be more thorough with origin longitude
                if let Some(max_col) = column_has_tile.iter().filter(|&col| !column_has_tile.contains(&(col - 1))).max() {
                    self.rect.x = *max_col as f32;
                }
            } else {
                self.rect.x = min_column; // ez way for non-wrapping regions
            }
            self.rect.y = min_row;
            self.rect.height = max_row - min_row + 1.0;

            if self.affected_by_world_wrap && min_column < self.rect.x {
                // Thorough way
                self.rect.width = column_has_tile.len() as f32;
            } else {
                self.rect.width = max_column - min_column + 1.0; // ez way
                self.affected_by_world_wrap = false; // also we're not wrapping anymore
            }
        }
    }

    /// Counts the terrains in the Region for type and start determination
    pub fn count_terrains(&mut self) {
        // Count terrains in the region
        self.terrain_counts.clear();
        for tile in &self.tiles {
            let terrains_to_count = if tile.terrain_has_unique(UniqueType::IgnoreBaseTerrainForRegion) {
                tile.terrain_feature_objects.iter().map(|f| f.name.clone())
            } else {
                tile.all_terrains.iter().map(|t| t.name.clone())
            };

            for terrain in terrains_to_count {
                *self.terrain_counts.entry(terrain).or_insert(0) += 1;
            }

            if tile.is_coastal_tile() {
                *self.terrain_counts.entry("Coastal".to_string()).or_insert(0) += 1;
            }
        }
    }

    /// Returns number of terrains with the given name
    pub fn get_terrain_amount(&self, name: &str) -> i32 {
        *self.terrain_counts.get(name).unwrap_or(&0)
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terrain_counts_str = self.terrain_counts.iter()
            .map(|(name, count)| format!("{} {}", count, name))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "Region({}, {} tiles, {})",
            self.region_type,
            self.tiles.len(),
            terrain_counts_str)
    }
}