use std::collections::HashMap;

use crate::{
    constants::Constants,
    models::ruleset::{
        tile::TerrainType,
        unique::{StateForConditionals, Unique, UniqueType},
    },
    map::mapunit::MapUnit,
};

/// Cache for MapUnit to improve performance of frequently accessed properties
/// Note: Single use in MapUnit and it's transient there, so no need for that here
pub struct MapUnitCache<'a> {
    map_unit: &'a MapUnit,
    /// These are for performance improvements to get_movement_cost_between_adjacent_tiles,
    /// a major component of get_distance_to_tiles_within_turn,
    /// which in turn is a component of get_shortest_path and can_reach
    pub ignores_terrain_cost: bool,
    pub ignores_zone_of_control: bool,
    pub all_tiles_costs_1: bool,
    pub can_move_on_water: bool,
    pub can_pass_through_impassable_tiles: bool,
    pub rough_terrain_penalty: bool,
    /// `true` if movement 0 _or_ has CannotMove unique
    pub cannot_move: bool,
    /// If set causes an early exit in get_movement_cost_between_adjacent_tiles
    /// - means no double movement uniques, rough_terrain_penalty or ignore_hill_movement_cost
    pub no_terrain_movement_uniques: bool,
    /// If set causes a second early exit in get_movement_cost_between_adjacent_tiles
    pub no_base_terrain_or_hill_double_movement_uniques: bool,
    /// If set skips tile.matches_filter tests for double movement in get_movement_cost_between_adjacent_tiles
    pub no_filtered_double_movement_uniques: bool,
    /// Used for get_movement_cost_between_adjacent_tiles only, based on order of testing
    pub double_movement_in_terrain: HashMap<String, DoubleMovement>,
    pub can_enter_ice_tiles: bool,
    pub cannot_enter_ocean_tiles: bool,
    pub can_enter_foreign_terrain: bool,
    pub can_enter_city_states: bool,
    pub cost_to_disembark: Option<f32>,
    pub cost_to_embark: Option<f32>,
    pub paradrop_range: i32,
    pub has_unique_to_build_improvements: bool,    // not can_build_improvements to avoid confusion
    pub has_unique_to_create_water_improvements: bool,
    pub has_strength_bonus_in_radius_unique: bool,
    pub has_citadel_placement_unique: bool,
    pub state: StateForConditionals,
}

/// Used for get_movement_cost_between_adjacent_tiles only, based on order of testing
#[derive(Debug, Clone)]
pub enum DoubleMovementTerrainTarget {
    Feature,
    Base,
    Hill,
    Filter,
}

#[derive(Debug, Clone)]
pub struct DoubleMovement {
    pub terrain_target: DoubleMovementTerrainTarget,
    pub unique: Unique,
}

impl<'a> MapUnitCache<'a> {
    pub fn new(map_unit: &'a MapUnit) -> Self {
        let mut cache = Self {
            map_unit,
            ignores_terrain_cost: false,
            ignores_zone_of_control: false,
            all_tiles_costs_1: false,
            can_move_on_water: false,
            can_pass_through_impassable_tiles: false,
            rough_terrain_penalty: false,
            cannot_move: false,
            no_terrain_movement_uniques: false,
            no_base_terrain_or_hill_double_movement_uniques: false,
            no_filtered_double_movement_uniques: false,
            double_movement_in_terrain: HashMap::new(),
            can_enter_ice_tiles: false,
            cannot_enter_ocean_tiles: false,
            can_enter_foreign_terrain: false,
            can_enter_city_states: false,
            cost_to_disembark: None,
            cost_to_embark: None,
            paradrop_range: 0,
            has_unique_to_build_improvements: false,
            has_unique_to_create_water_improvements: false,
            has_strength_bonus_in_radius_unique: false,
            has_citadel_placement_unique: false,
            state: StateForConditionals::empty(),
        };
        cache.update_uniques();
        cache
    }

    pub fn update_uniques(&mut self) {
        self.state = StateForConditionals::new(self.map_unit);
        self.all_tiles_costs_1 = self.map_unit.has_unique(UniqueType::AllTilesCost1Move);
        self.can_pass_through_impassable_tiles = self.map_unit.has_unique(UniqueType::CanPassImpassable);
        self.ignores_terrain_cost = self.map_unit.has_unique(UniqueType::IgnoresTerrainCost);
        self.ignores_zone_of_control = self.map_unit.has_unique(UniqueType::IgnoresZOC);
        self.rough_terrain_penalty = self.map_unit.has_unique(UniqueType::RoughTerrainPenalty);
        self.cannot_move = self.map_unit.has_unique(UniqueType::CannotMove) || self.map_unit.base_unit.movement == 0;
        self.can_move_on_water = self.map_unit.has_unique(UniqueType::CanMoveOnWater);

        self.double_movement_in_terrain.clear();
        for unique in self.map_unit.get_matching_uniques(
            UniqueType::DoubleMovementOnTerrain,
            &StateForConditionals::ignore_conditionals(),
            true,
        ) {
            let param = &unique.params[0];
            let terrain = self.map_unit.civ.game_info.ruleset.terrains.get(param);
            let terrain_target = match terrain {
                None => DoubleMovementTerrainTarget::Filter,
                Some(terrain) => {
                    if terrain.name == Constants::HILL {
                        DoubleMovementTerrainTarget::Hill
                    } else if terrain.terrain_type == TerrainType::TerrainFeature {
                        DoubleMovementTerrainTarget::Feature
                    } else if terrain.terrain_type.is_base_terrain() {
                        DoubleMovementTerrainTarget::Base
                    } else {
                        DoubleMovementTerrainTarget::Filter
                    }
                }
            };
            self.double_movement_in_terrain.insert(
                param.clone(),
                DoubleMovement {
                    terrain_target,
                    unique: unique.clone(),
                },
            );
        }

        // Init shortcut flags
        self.no_terrain_movement_uniques = self.double_movement_in_terrain.is_empty()
            && !self.rough_terrain_penalty
            && !self.map_unit.civ.nation.ignore_hill_movement_cost;

        self.no_base_terrain_or_hill_double_movement_uniques = self
            .double_movement_in_terrain
            .values()
            .all(|dm| matches!(dm.terrain_target, DoubleMovementTerrainTarget::Feature));

        self.no_filtered_double_movement_uniques = self
            .double_movement_in_terrain
            .values()
            .all(|dm| !matches!(dm.terrain_target, DoubleMovementTerrainTarget::Filter));

        self.cost_to_disembark = self
            .map_unit
            .get_matching_uniques(UniqueType::ReducedDisembarkCost, true)
            .iter()
            .map(|u| u.params[0].parse::<f32>().unwrap_or(0.0))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        self.cost_to_embark = self
            .map_unit
            .get_matching_uniques(UniqueType::ReducedEmbarkCost, true)
            .iter()
            .map(|u| u.params[0].parse::<f32>().unwrap_or(0.0))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        self.can_enter_ice_tiles = self.map_unit.has_unique(UniqueType::CanEnterIceTiles);
        self.cannot_enter_ocean_tiles = self.map_unit.has_unique(UniqueType::CannotEnterOcean);

        self.has_unique_to_build_improvements = self.map_unit.has_unique(UniqueType::BuildImprovements);
        self.has_unique_to_create_water_improvements = self.map_unit.has_unique(UniqueType::CreateWaterImprovements);

        self.can_enter_foreign_terrain = self.map_unit.has_unique(UniqueType::CanEnterForeignTiles)
            || self.map_unit.has_unique(UniqueType::CanEnterForeignTilesButLosesReligiousStrength);

        self.can_enter_city_states = self.map_unit.has_unique(UniqueType::CanTradeWithCityStateForGoldAndInfluence);

        self.has_strength_bonus_in_radius_unique = self.map_unit.has_unique(
            UniqueType::StrengthBonusInRadius,
            &StateForConditionals::ignore_conditionals(),
        );

        self.has_citadel_placement_unique = self
            .map_unit
            .get_matching_uniques(UniqueType::ConstructImprovementInstantly)
            .iter()
            .filter_map(|u| self.map_unit.civ.game_info.ruleset.tile_improvements.get(&u.params[0]))
            .any(|ti| ti.has_unique(UniqueType::OneTimeTakeOverTilesInRadius));
    }
}

impl<'a> MapUnitCache<'a> {
    /// List of uniques that affect unit movement
    pub const UNIT_MOVEMENT_UNIQUES: &'static [UniqueType] = &[
        UniqueType::AllTilesCost1Move,
        UniqueType::CanPassImpassable,
        UniqueType::IgnoresTerrainCost,
        UniqueType::IgnoresZOC,
        UniqueType::RoughTerrainPenalty,
        UniqueType::CannotMove,
        UniqueType::CanMoveOnWater,
        UniqueType::DoubleMovementOnTerrain,
        UniqueType::ReducedDisembarkCost,
        UniqueType::ReducedEmbarkCost,
        UniqueType::CanEnterIceTiles,
        UniqueType::CanEnterForeignTiles,
        UniqueType::CanEnterForeignTilesButLosesReligiousStrength,
        // Special - applied in Nation and not here, should be moved to mapunitcache as well
        UniqueType::ForestsAndJunglesAreRoads,
        UniqueType::IgnoreHillMovementCost,
        // Movement algorithm avoids damage on route, meaning terrain damage requires caching
        UniqueType::DamagesContainingUnits,
        UniqueType::LandUnitEmbarkation,
        UniqueType::LandUnitsCrossTerrainAfterUnitGained,
        UniqueType::EnemyUnitsSpendExtraMovement,
    ];
}