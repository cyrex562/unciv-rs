use std::collections::HashSet;
use crate::models::civilization::{Civilization, MapUnitAction, NotificationCategory, NotificationIcon};
use crate::models::map::mapunit::MapUnit;
use crate::models::map::tile::{RoadStatus, Tile};
use crate::models::map::MapPathing;
use crate::models::Vector2;
use crate::utils::{debug, Log};

/// Responsible for automation the "build road to" action
/// This is *pretty bad code* overall and needs to be cleaned up
pub struct RoadToAutomation {
    civ_info: Civilization,
    actual_best_road_available: RoadStatus,
}

impl RoadToAutomation {
    pub fn new(civ_info: Civilization) -> Self {
        let actual_best_road_available = civ_info.tech.get_best_road_available();
        Self {
            civ_info,
            actual_best_road_available,
        }
    }

    /// Automate the process of connecting a road between two points.
    /// Current thoughts:
    /// Will be a special case of MapUnit.automated property
    /// Unit has new attributes startTile endTile
    /// - We will progress towards the end path sequentially, taking absolute least distance w/o regard for movement cost
    /// - Cancel upon risk of capture
    /// - Cancel upon blocked
    /// - End automation upon finish
    pub fn automate_connect_road(&self, unit: &mut MapUnit, tiles_where_we_will_be_captured: &HashSet<Tile>) {
        if self.actual_best_road_available == RoadStatus::None {
            return;
        }

        let current_tile = unit.get_tile();

        if unit.automated_road_connection_destination.is_none() {
            self.stop_and_clean_automation(unit);
            return;
        }

        let destination_tile = unit.civ.game_info.tile_map.get(&unit.automated_road_connection_destination.unwrap());

        let mut path_to_dest = unit.automated_road_connection_path.clone();

        // The path does not exist, create it
        if path_to_dest.is_none() {
            let found_path = MapPathing::get_road_path(unit, current_tile, destination_tile);
            if found_path.is_none() {
                Log::debug(&format!("WorkerAutomation: {} -> connect road failed", unit));
                self.stop_and_clean_automation(unit);
                unit.civ.add_notification(
                    "Connect road failed!",
                    MapUnitAction::new(unit),
                    NotificationCategory::Units,
                    NotificationIcon::Construction,
                );
                return;
            }

            // Convert to a list of positions for serialization
            path_to_dest = Some(found_path.unwrap().iter().map(|tile| tile.position.clone()).collect());
            unit.automated_road_connection_path = path_to_dest.clone();
            debug!("WorkerAutomation: {} -> found connect road path to destination tile: {:?}, {:?}",
                unit, destination_tile, path_to_dest);
        }

        let path_to_dest = path_to_dest.unwrap();
        let curr_tile_index = path_to_dest.iter().position(|pos| *pos == current_tile.position);

        // The worker was somehow moved off its path, cancel the action
        if curr_tile_index.is_none() {
            Log::debug(&format!("{} -> was moved off its connect road path. Operation cancelled.", unit));
            self.stop_and_clean_automation(unit);
            unit.civ.add_notification(
                "Connect road cancelled!",
                MapUnitAction::new(unit),
                NotificationCategory::Units,
                unit.name.clone(),
            );
            return;
        }

        let curr_tile_index = curr_tile_index.unwrap();

        /* Can not build a road on this tile, try to move on.
        * The worker should search for the next furthest tile in the path that:
        * - It can move to
        * - Can be improved/upgraded
        */
        if unit.has_movement() && !self.should_build_road_on_tile(current_tile) {
            if curr_tile_index == path_to_dest.len() - 1 {
                // The last tile in the path is unbuildable or has a road
                self.stop_and_clean_automation(unit);
                unit.civ.add_notification(
                    "Connect road completed",
                    MapUnitAction::new(unit),
                    NotificationCategory::Units,
                    unit.name.clone(),
                );
                return;
            }

            if curr_tile_index < path_to_dest.len() - 1 {
                // Try to move to the next tile in the path
                let tile_map = &unit.civ.game_info.tile_map;
                let mut next_tile = current_tile;

                // Create a new list with tiles where the index is greater than currTileIndex
                let future_tiles = path_to_dest.iter()
                    .skip_while(|pos| **pos != unit.current_tile.position)
                    .skip(1)
                    .map(|pos| tile_map.get(pos).unwrap());

                // Find the furthest tile we can reach in this turn, move to, and does not have a road
                for future_tile in future_tiles {
                    if unit.movement.can_reach_in_current_turn(future_tile) && unit.movement.can_move_to(future_tile) {
                        next_tile = future_tile;
                        if self.should_build_road_on_tile(future_tile) {
                            break; // Stop on this tile
                        }
                    }
                }

                unit.movement.move_to_tile(next_tile);
                let current_tile = unit.get_tile();
            }
        }

        // We need to check current movement again after we've (potentially) moved
        if unit.has_movement() {
            // Repair pillaged roads first
            if current_tile.road_status != RoadStatus::None && current_tile.road_is_pillaged {
                current_tile.set_repaired();
                return;
            }
            if self.should_build_road_on_tile(current_tile)
                && current_tile.improvement_in_progress != Some(self.actual_best_road_available.name()) {
                let improvement = self.actual_best_road_available.improvement(&self.civ_info.game_info.ruleset).unwrap();
                current_tile.start_working_on_improvement(improvement, &self.civ_info, unit);
                return;
            }
        }
    }

    /// Reset side effects from automation, return worker to non-automated state
    pub fn stop_and_clean_automation(&self, unit: &mut MapUnit) {
        unit.automated = false;
        unit.action = None;
        unit.automated_road_connection_destination = None;
        unit.automated_road_connection_path = None;
        unit.current_tile.stop_working_on_improvement();
    }

    /// Conditions for whether it is acceptable to build a road on this tile
    pub fn should_build_road_on_tile(&self, tile: &Tile) -> bool {
        if tile.road_is_pillaged {
            return true;
        }
        !tile.is_city_center() // Can't build road on city tiles
            // Special case for civs that treat forest/jungles as roads (inside their territory).
            // We shouldn't build if railroads aren't unlocked.
            && !(tile.has_connection(&self.civ_info) && self.actual_best_road_available == RoadStatus::Road)
            && tile.road_status != self.actual_best_road_available // Build (upgrade) if possible
    }
}