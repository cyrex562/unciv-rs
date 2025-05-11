use std::collections::{HashMap, HashSet};
use crate::models::civilization::Civilization;
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::Tile;
use crate::models::city::City;
use crate::models::ruleset::unique::{LocalUniqueCache, StateForConditionals, UniqueType};
use crate::models::ruleset::tile::{ResourceType, TileImprovement, Terrain};
use crate::automation::Automation;
use crate::automation::unit::unit_automation::UnitAutomation;
use crate::automation::unit::road_to_automation::RoadToAutomation;
use crate::automation::unit::road_between_cities_automation::RoadBetweenCitiesAutomation;
use crate::ui::screens::worldscreen::unit::actions::{UnitActions, UnitActionsFromUniques};
use crate::unciv::Constants;
use crate::unciv::UncivGame;

/// Contains the logic for worker automation.
pub struct WorkerAutomation {
    pub civ_info: Civilization,
    pub cached_for_turn: i32,
    pub road_to_automation: RoadToAutomation,
    pub road_between_cities_automation: RoadBetweenCitiesAutomation,
    tile_rankings: HashMap<Tile, TileImprovementRank>,
}

/// Each object has two stages, this first one is checking the basic priority without any improvements.
/// If tile_priority is -1 then it must be a dangerous tile.
/// The improvement_priority and best_improvement are by default not set.
/// Once improvement_priority is set we have already checked for the best improvement, repair_improvement.
#[derive(Debug)]
pub struct TileImprovementRank {
    pub tile_priority: f32,
    pub improvement_priority: Option<f32>,
    pub best_improvement: Option<TileImprovement>,
    pub repair_improvement: Option<bool>,
}

impl WorkerAutomation {
    pub fn new(civ_info: Civilization, cached_for_turn: i32, cloning_source: Option<&WorkerAutomation>) -> Self {
        let road_to_automation = RoadToAutomation::new(civ_info.clone());
        let road_between_cities_automation = match cloning_source {
            Some(source) => source.road_between_cities_automation.clone(),
            None => RoadBetweenCitiesAutomation::new(civ_info.clone(), cached_for_turn, None)
        };

        WorkerAutomation {
            civ_info,
            cached_for_turn,
            road_to_automation,
            road_between_cities_automation,
            tile_rankings: HashMap::new(),
        }
    }

    /// Automate one Worker - decide what to do and where, move, start or continue work.
    pub fn automate_worker_action(&mut self, unit: &mut MapUnit, dangerous_tiles: &HashSet<Tile>, local_unique_cache: &LocalUniqueCache) {
        let current_tile = unit.get_tile();

        // Must be called before any get_priority checks to guarantee the local road cache is processed
        let cities_to_connect = self.road_between_cities_automation.get_nearby_cities_to_connect(unit);

        // Shortcut, we are working a suitable tile, and we're better off minimizing worker-turns by finishing everything on this tile
        if current_tile.improvement_in_progress.is_some() && !dangerous_tiles.contains(&current_tile)
            && self.get_full_priority(&current_tile, unit, local_unique_cache) >= 2.0 {
            return;
        }

        let tile_to_work = self.find_tile_to_work(unit, dangerous_tiles, local_unique_cache);

        if tile_to_work.as_ref() != Some(&current_tile) && tile_to_work.is_some() {
            self.head_towards_tile_to_work(unit, tile_to_work.unwrap(), local_unique_cache);
            return;
        }

        if current_tile.improvement_in_progress.is_some() {
            return; // we're working!
        }

        if tile_to_work.as_ref() == Some(&current_tile) && self.tile_has_work_to_do(&current_tile, unit, local_unique_cache) {
            self.start_work_on_current_tile(unit);
        }

        // Support Alpha Frontier-Style Workers that _also_ have the "May create improvements on water resources" unique
        if unit.cache.has_unique_to_create_water_improvements && self.automate_work_boats(unit) {
            return;
        }

        if self.try_head_towards_undeveloped_city(unit, local_unique_cache, &current_tile) {
            return;
        }

        // Nothing to do, try again to connect cities
        if self.road_between_cities_automation.try_connecting_cities(unit, &cities_to_connect) {
            return;
        }

        debug!("WorkerAutomation: {} -> nothing to do", unit);
        unit.civ.add_notification(
            &format!("{} has no work to do.", unit.short_display_name()),
            MapUnitAction::new(unit),
            NotificationCategory::Units,
            &unit.name,
            "OtherIcons/Sleep"
        );

        // Idle CS units should wander so they don't obstruct players so much
        if unit.civ.is_city_state() {
            UnitAutomation::wander(unit, true, Some(dangerous_tiles));
        }
    }

    fn try_head_towards_undeveloped_city(
        &self,
        unit: &mut MapUnit,
        local_unique_cache: &LocalUniqueCache,
        current_tile: &Tile
    ) -> bool {
        let mut cities_to_number_of_unimproved_tiles = HashMap::new();

        for city in &unit.civ.cities {
            let count = city.get_tiles()
                .iter()
                .filter(|tile| {
                    tile.is_land
                        && tile.get_units().iter().all(|unit| !unit.cache.has_unique_to_build_improvements)
                        && (tile.is_pillaged() || self.tile_has_work_to_do(tile, unit, local_unique_cache))
                })
                .count();
            cities_to_number_of_unimproved_tiles.insert(city.id.clone(), count);
        }

        let closest_undeveloped_city = unit.civ.cities.iter()
            .filter(|city| cities_to_number_of_unimproved_tiles[&city.id] > 0)
            .max_by_key(|city| city.get_center_tile().aerial_distance_to(current_tile))
            .filter(|city| unit.movement.can_reach(&city.get_center_tile())); //goto most undeveloped city

        if let Some(city) = closest_undeveloped_city {
            if city.get_center_tile() != current_tile.owning_city.as_ref().map(|c| c.get_center_tile()).unwrap_or_default() {
                debug!("WorkerAutomation: {} -> head towards undeveloped city {}", unit, city.name);
                let reached_tile = unit.movement.head_towards(&city.get_center_tile());
                if reached_tile != current_tile {
                    unit.do_action(); // since we've moved, maybe we can do something here - automate
                }
                return true;
            }
        }
        false
    }

    fn start_work_on_current_tile(&self, unit: &mut MapUnit) {
        let current_tile = unit.current_tile;
        let tile_ranking = self.tile_rankings.get(&current_tile).unwrap();

        if tile_ranking.repair_improvement == Some(true) {
            debug!("WorkerAutomation: {} -> repairs {}", unit, current_tile);
            if let Some(action) = UnitActionsFromUniques::get_repair_action(unit) {
                action.action();
            }
            return;
        }

        if let Some(improvement) = &tile_ranking.best_improvement {
            debug!("WorkerAutomation: {} -> start improving {}", unit, current_tile);
            current_tile.start_working_on_improvement(improvement, &unit.civ, unit);
        } else {
            panic!("We didn't find anything to improve on this tile even though there was supposed to be something to improve!");
        }
    }

    fn head_towards_tile_to_work(
        &self,
        unit: &mut MapUnit,
        tile_to_work: Tile,
        local_unique_cache: &LocalUniqueCache
    ) {
        debug!("WorkerAutomation: {} -> head towards {}", unit, tile_to_work);
        let current_tile = unit.get_tile();
        let reached_tile = unit.movement.head_towards(&tile_to_work);

        if tile_to_work.neighbors.contains(&reached_tile)
            && unit.movement.can_move_to(&tile_to_work, true)
            && !unit.movement.can_move_to(&tile_to_work, false)
            && unit.movement.can_unit_swap_to(&tile_to_work)
        {
            // There must be a unit on the target tile! Let's swap with it.
            unit.movement.swap_move_to_tile(&tile_to_work);
        }

        if reached_tile != current_tile {  // otherwise, we get a situation where the worker is automated, so it tries to move but doesn't, then tries to automate, then move, etc, forever. Stack overflow exception!
            unit.do_action();
        }

        // If we have reached a fort tile that is in progress and shouldn't be there, cancel it.
        // TODO: Replace this code entirely and change [choose_improvement] to not continue building the improvement by default
        if reached_tile == tile_to_work
            && reached_tile.improvement_in_progress.as_deref() == Some(Constants::FORT)
            && self.evaluate_fort_surroundings(&current_tile, false) <= 0.0
        {
            debug!("Replacing fort in progress with new improvement");
            reached_tile.stop_working_on_improvement();
        }

        if !unit.has_movement() || reached_tile != tile_to_work {
            return;
        }

        // If there's move still left, and this is even a tile we want, perform action
        // Unit may stop due to Enemy Unit within walking range during do_action() call

        // tile_rankings is updated in get_base_priority, which is only called if is_automation_workable_tile is true
        // Meaning, there are tiles we can't/shouldn't work, and they won't even be in tile_rankings
        if self.tile_has_work_to_do(&unit.current_tile, unit, local_unique_cache) {
            self.start_work_on_current_tile(unit);
        }
    }

    /// Looks for a worthwhile tile to improve
    /// Returns None if no tile to work was found
    fn find_tile_to_work(&mut self, unit: &MapUnit, tiles_to_avoid: &HashSet<Tile>, local_unique_cache: &LocalUniqueCache) -> Option<Tile> {
        let current_tile = unit.get_tile();

        if self.is_automation_workable_tile(&current_tile, tiles_to_avoid, &current_tile, unit)
            && self.get_base_priority(&current_tile, unit) >= 5.0
            && (current_tile.is_pillaged() || current_tile.has_fallout_equivalent() || self.tile_has_work_to_do(&current_tile, unit, local_unique_cache)) {
            return Some(current_tile);
        }

        let workable_tiles_center_first = current_tile.get_tiles_in_distance(4)
            .iter()
            .filter(|tile| {
                self.is_automation_workable_tile(tile, tiles_to_avoid, &current_tile, unit)
                    && self.get_base_priority(tile, unit) > 1.0
            })
            .collect::<Vec<_>>();

        let workable_tiles_prioritized = workable_tiles_center_first.into_iter()
            .group_by(|tile| self.get_base_priority(tile, unit))
            .into_iter()
            .sorted_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap());

        // Search through each group by priority
        // If we can find an eligible best tile in the group lets return that
        // under the assumption that best tile is better than tiles in all lower groups
        for (_, tiles_in_group) in workable_tiles_prioritized {
            let mut best_tile = None;
            for tile in tiles_in_group.sorted_by_key(|tile| unit.get_tile().aerial_distance_to(tile)) {
                // These are the expensive calculations (tile_can_be_improved, can_reach), so we only apply these filters after everything else it done.
                if !self.tile_has_work_to_do(tile, unit, local_unique_cache) {
                    continue;
                }
                if unit.get_tile() == *tile {
                    return Some(*tile);
                }
                if !unit.movement.can_reach(tile) {
                    continue;
                }
                if best_tile.is_none() || self.get_full_priority(tile, unit, local_unique_cache) > self.get_full_priority(best_tile.unwrap(), unit, local_unique_cache) {
                    best_tile = Some(*tile);
                }
            }
            if let Some(tile) = best_tile {
                return Some(tile);
            }
        }
        None
    }

    fn is_automation_workable_tile(
        &self,
        tile: &Tile,
        tiles_to_avoid: &HashSet<Tile>,
        current_tile: &Tile,
        unit: &MapUnit
    ) -> bool {
        if tiles_to_avoid.contains(tile) {
            return false;
        }
        if !(tile == current_tile
            || (unit.is_civilian() && (tile.civilian_unit.is_none() || !tile.civilian_unit.as_ref().unwrap().cache.has_unique_to_build_improvements))
            || (unit.is_military() && (tile.military_unit.is_none() || !tile.military_unit.as_ref().unwrap().cache.has_unique_to_build_improvements))) {
            return false;
        }
        if tile.owning_city.is_some() && tile.get_owner() != Some(&self.civ_info) {
            return false;
        }
        if tile.is_city_center() {
            return false;
        }
        // Don't try to improve tiles we can't benefit from at all
        if !tile.has_viewable_resource(&self.civ_info) && tile.get_tiles_in_distance(self.civ_info.game_info.ruleset.mod_options.constants.city_work_range)
            .iter()
            .all(|t| !t.is_city_center() || t.get_city().map_or(true, |city| city.civ != self.civ_info))
        {
            return false;
        }
        if tile.get_tile_improvement().map_or(false, |imp| imp.has_unique(UniqueType::AutomatedUnitsWillNotReplace)) && !tile.is_pillaged() {
            return false;
        }
        true
    }

    /// Calculate a priority for the tile without accounting for the improvement itself
    /// This is a cheap guess on how helpful it might be to do work on this tile
    fn get_base_priority(&mut self, tile: &Tile, unit: &MapUnit) -> f32 {
        let unit_specific_priority = 2.0 - (tile.aerial_distance_to(&unit.get_tile()) as f32 / 2.0).clamp(0.0, 2.0);
        if let Some(rank) = self.tile_rankings.get(tile) {
            return rank.tile_priority + unit_specific_priority;
        }

        let mut priority = 0.0;
        if tile.get_owner() == Some(&self.civ_info) {
            priority += Automation::rank_stats_value(&tile.stats.get_terrain_stats_breakdown().to_stats(), &self.civ_info);
            if tile.provides_yield() {
                priority += 2.0;
            }
            if tile.is_pillaged() {
                priority += 1.0;
            }
            if tile.has_fallout_equivalent() {
                priority += 1.0;
            }
            if !tile.terrain_features.is_empty() && tile.last_terrain.has_unique(UniqueType::ProductionBonusWhenRemoved) {
                priority += 1.0; // removing our forests is good for tempo
            }
            if tile.terrain_has_unique(UniqueType::FreshWater) {
                priority += 1.0; // we want our farms up when unlocking Civil Service
            }
        }
        // give a minor priority to tiles that we could expand onto
        else if tile.get_owner().is_none() && tile.neighbors.iter().any(|t| t.get_owner() == Some(&self.civ_info)) {
            priority += 1.0;
        }

        if tile.has_viewable_resource(&self.civ_info) {
            priority += 1.0;
            if tile.tile_resource.resource_type == ResourceType::Luxury {
                priority += 3.0; //luxuries are more important than other types of resources
            }
        }

        if self.road_between_cities_automation.tiles_of_roads_map.contains_key(tile) {
            priority += 3.0;
        }

        self.tile_rankings.insert(tile.clone(), TileImprovementRank {
            tile_priority: priority,
            improvement_priority: None,
            best_improvement: None,
            repair_improvement: None,
        });
        priority + unit_specific_priority
    }

    /// Calculates the priority building the improvement on the tile
    fn get_improvement_priority(&mut self, tile: &Tile, unit: &MapUnit, local_unique_cache: &LocalUniqueCache) -> f32 {
        self.get_base_priority(tile, unit);
        let rank = self.tile_rankings.get_mut(tile).unwrap();
        if rank.improvement_priority.is_none() {
            // All values of rank have to be initialized
            rank.improvement_priority = Some(-100.0);
            rank.best_improvement = None;
            rank.repair_improvement = Some(false);

            let best_improvement = self.choose_improvement(unit, tile, local_unique_cache);
            if let Some(improvement) = best_improvement {
                rank.best_improvement = Some(improvement.clone());
                // Increased priority if the improvement has been worked on longer
                let time_spent_priority = if tile.improvement_in_progress.as_deref() == Some(&improvement.name) {
                    improvement.get_turns_to_build(&unit.civ, unit) - tile.turns_to_improvement
                } else {
                    0
                };

                rank.improvement_priority = Some(self.get_improvement_ranking(tile, unit, &improvement.name, local_unique_cache, None) + time_spent_priority as f32);
            }

            if tile.improvement.is_some() && tile.is_pillaged() && tile.owning_city.is_some() {
                // Value repairing higher when it is quicker and is in progress
                let mut repair_bonus_priority = tile.get_improvement_to_repair().unwrap().get_turns_to_build(&unit.civ, unit) as f32
                    - UnitActionsFromUniques::get_repair_turns(unit) as f32;
                if tile.improvement_in_progress.as_deref() == Some(Constants::REPAIR) {
                    repair_bonus_priority += UnitActionsFromUniques::get_repair_turns(unit) as f32 - tile.turns_to_improvement as f32;
                }

                let repair_priority = repair_bonus_priority + Automation::rank_stats_value(
                    &tile.stats.get_stat_diff_for_improvement(tile.get_tile_improvement().unwrap(), &unit.civ, tile.owning_city.as_ref()),
                    &unit.civ
                );
                if repair_priority > rank.improvement_priority.unwrap() {
                    rank.improvement_priority = Some(repair_priority);
                    rank.best_improvement = None;
                    rank.repair_improvement = Some(true);
                }
            }
        }
        // A better tile than this unit can build might have been stored in the cache
        if !rank.repair_improvement.unwrap() && (rank.best_improvement.is_none()
            || !unit.can_build_improvement(&rank.best_improvement.as_ref().unwrap(), tile)) {
            return -100.0;
        }
        rank.improvement_priority.unwrap()
    }

    /// Calculates the full priority of the tile
    fn get_full_priority(&mut self, tile: &Tile, unit: &MapUnit, local_unique_cache: &LocalUniqueCache) -> f32 {
        self.get_base_priority(tile, unit) + self.get_improvement_priority(tile, unit, local_unique_cache)
    }

    /// Returns true if the tile has work that can be done
    fn tile_has_work_to_do(&mut self, tile: &Tile, unit: &MapUnit, local_unique_cache: &LocalUniqueCache) -> bool {
        if self.get_improvement_priority(tile, unit, local_unique_cache) <= 0.0 {
            return false;
        }
        let rank = self.tile_rankings.get(tile).unwrap();
        if !(rank.best_improvement.is_some() || rank.repair_improvement.unwrap()) {
            panic!("There was an improvement_priority > 0 and nothing to do");
        }
        true
    }

    /// Chooses the best improvement for the tile
    fn choose_improvement(&self, unit: &MapUnit, tile: &Tile, local_unique_cache: &LocalUniqueCache) -> Option<TileImprovement> {
        let mut best_improvement = None;
        let mut best_ranking = f32::NEG_INFINITY;

        for improvement in unit.get_buildable_improvements(tile) {
            let ranking = self.get_improvement_ranking(tile, unit, &improvement.name, local_unique_cache, best_improvement.as_ref());
            if ranking > best_ranking {
                best_ranking = ranking;
                best_improvement = Some(improvement);
            }
        }
        best_improvement
    }

    /// Ranks how good an improvement would be on a tile
    fn get_improvement_ranking(
        &self,
        tile: &Tile,
        unit: &MapUnit,
        improvement_name: &str,
        local_unique_cache: &LocalUniqueCache,
        best_improvement: Option<&TileImprovement>
    ) -> f32 {
        let mut ranking = 0.0;
        let improvement = unit.civ.game_info.ruleset.tile_improvements.get(improvement_name).unwrap();

        // Don't build roads/railroads on tiles that already have them
        if improvement.has_unique(UniqueType::Road) && tile.road_status.is_some() {
            return f32::NEG_INFINITY;
        }

        // Don't build improvements that would be removed by other improvements we want to build
        if let Some(best) = best_improvement {
            if best.unique_to.iter().any(|unique| unique.unique_type == UniqueType::RemovesImprovement
                && unique.params[0] == improvement_name) {
                return f32::NEG_INFINITY;
            }
        }

        // Don't build improvements that would be removed by resources we haven't discovered
        if tile.has_resource() && !tile.has_viewable_resource(&self.civ_info) {
            return f32::NEG_INFINITY;
        }

        // Don't build improvements that would be removed by resources we can see
        if tile.has_viewable_resource(&self.civ_info) {
            let resource = tile.tile_resource.as_ref().unwrap();
            if !resource.improvement.is_empty() && !resource.improvement.contains(&improvement_name.to_string()) {
                return f32::NEG_INFINITY;
            }
        }

        // Don't build improvements that would be removed by terrain features
        if !tile.terrain_features.is_empty() {
            for feature in &tile.terrain_features {
                if feature.unique_to.iter().any(|unique| unique.unique_type == UniqueType::RemovesImprovement
                    && unique.params[0] == improvement_name) {
                    return f32::NEG_INFINITY;
                }
            }
        }

        // Don't build improvements that would be removed by other improvements
        if let Some(existing_improvement) = tile.get_tile_improvement() {
            if existing_improvement.unique_to.iter().any(|unique| unique.unique_type == UniqueType::RemovesImprovement
                && unique.params[0] == improvement_name) {
                return f32::NEG_INFINITY;
            }
        }

        // Prioritize roads between cities
        if improvement.has_unique(UniqueType::Road) {
            if self.road_between_cities_automation.tiles_of_roads_map.contains_key(tile) {
                ranking += 10.0;
            } else {
                return f32::NEG_INFINITY;
            }
        }

        // Prioritize resource improvements
        if tile.has_viewable_resource(&self.civ_info) {
            let resource = tile.tile_resource.as_ref().unwrap();
            if resource.improvement.contains(&improvement_name.to_string()) {
                ranking += match resource.resource_type {
                    ResourceType::Luxury => 5.0,
                    ResourceType::Strategic => 4.0,
                    ResourceType::Bonus => 3.0,
                };
            }
        }

        // Consider stat improvements
        if tile.owning_city.is_some() {
            let city = tile.get_city();
            let stat_diff = tile.stats.get_stat_diff_for_improvement(improvement, &unit.civ, city.as_ref());
            ranking += Automation::rank_stats_value(&stat_diff, &self.civ_info);

            // Consider growth needs
            if city.as_ref().map_or(false, |c| c.needs_food()) && stat_diff.food > 0.0 {
                ranking += 2.0;
            }

            // Consider production needs
            if city.as_ref().map_or(false, |c| c.needs_production()) && stat_diff.production > 0.0 {
                ranking += 2.0;
            }
        }

        // Consider improvement build time
        let turns_to_build = improvement.get_turns_to_build(&unit.civ, unit);
        ranking -= turns_to_build as f32 * 0.1;

        // Consider unique effects
        for unique in &improvement.unique_to {
            match unique.unique_type {
                UniqueType::DefensiveBonus => ranking += 1.0,
                UniqueType::StrategicConnection => ranking += 1.0,
                _ => {}
            }
        }

        ranking
    }

    /// Starts work on the current tile if possible
    fn start_work_on_current_tile(&mut self, unit: &mut MapUnit, local_unique_cache: &LocalUniqueCache) -> bool {
        let tile = unit.get_tile();
        self.get_improvement_priority(&tile, unit, local_unique_cache);
        let rank = self.tile_rankings.get(&tile).unwrap();

        if rank.repair_improvement.unwrap() {
            if unit.can_repair_improvement(&tile) {
                unit.repair_improvement(&tile);
                return true;
            }
            return false;
        }

        if let Some(improvement) = &rank.best_improvement {
            if unit.can_build_improvement(improvement, &tile) {
                unit.build_improvement(improvement, &tile);
                return true;
            }
        }
        false
    }

    /// Moves the worker towards a tile that needs work
    fn head_towards_tile_to_work(&mut self, unit: &mut MapUnit, tiles_to_avoid: &HashSet<Tile>, local_unique_cache: &LocalUniqueCache) -> bool {
        if let Some(tile_to_work) = self.find_tile_to_work(unit, tiles_to_avoid, local_unique_cache) {
            if tile_to_work == unit.get_tile() {
                return self.start_work_on_current_tile(unit, local_unique_cache);
            }
            unit.movement.move_towards(&tile_to_work);
            return true;
        }
        false
    }
}