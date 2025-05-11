// Port of orig_src/core/src/com/unciv/ui/screens/worldscreen/worldmap/WorldMapTileUpdater.kt

use std::rc::Rc;
use std::cell::RefCell;
use egui::Color32;

use crate::game::battle::{AttackableTile, TargetHelper};
use crate::game::city::City;
use crate::game::civilization::Civilization;
use crate::game::map::MapPathing;
use crate::game::map::map_unit::MapUnit;
use crate::game::map::tile::Tile;
use crate::models::Spy;
use crate::models::ruleset::unique::{LocalUniqueCache, UniqueType};
use crate::ui::screens::worldscreen::worldmap::WorldMapHolder;
use crate::logic::automation::unit::CityLocationTileRanker;

pub struct WorldMapTileUpdater {
    world_map_holder: Rc<RefCell<WorldMapHolder>>,
}

impl WorldMapTileUpdater {
    pub fn new(world_map_holder: Rc<RefCell<WorldMapHolder>>) -> Self {
        Self { world_map_holder }
    }

    pub fn update_tiles(&self, viewing_civ: &Civilization) {
        let holder = self.world_map_holder.borrow();

        // Handle map reveal if enabled
        if self.is_map_reveal_enabled(viewing_civ) {
            for tile_group in holder.tile_groups.values() {
                let mut tile = tile_group.borrow_mut();
                tile.tile.borrow_mut().set_explored(viewing_civ, true);
                tile.is_force_visible = true;
            }
        }

        // General update of all tiles
        let unique_cache = LocalUniqueCache::new(true);
        for tile_group in holder.tile_groups.values() {
            tile_group.borrow_mut().update(viewing_civ, &unique_cache);
        }

        // Update tiles based on selected unit/city
        let unit_table = holder.world_screen.borrow().bottom_unit_table.clone();
        let unit_table = unit_table.borrow();

        if let Some(spy) = &unit_table.selected_spy {
            self.update_tiles_for_selected_spy(spy);
        } else if let Some(city) = &unit_table.selected_city {
            self.update_bombardable_tiles_for_selected_city(city);

            // Show road paths to selected city if connecting road
            if unit_table.selected_unit_is_connecting_road {
                if let Some(unit) = unit_table.selected_units.first() {
                    self.update_tiles_for_selected_unit(unit);
                }
            }
        } else if let Some(unit) = &unit_table.selected_unit {
            for unit in &unit_table.selected_units {
                self.update_tiles_for_selected_unit(unit);
            }
        }
    }

    fn update_tiles_for_selected_spy(&self, spy: &Spy) {
        let holder = self.world_map_holder.borrow();
        let viewing_civ = &holder.world_screen.borrow().viewing_civ;

        // TODO: Implement spy tile updates
        // This should show tiles where spy actions are possible
    }

    fn update_bombardable_tiles_for_selected_city(&self, city: &Rc<RefCell<City>>) {
        let holder = self.world_map_holder.borrow();
        let city = city.borrow();

        // Get tiles in bombard range
        let target_helper = TargetHelper::new();
        let attackable_tiles = target_helper.get_bombardable_tiles(&city);

        // Update tile visuals for bombardable tiles
        for tile in attackable_tiles {
            if let Some(tile_group) = holder.tile_groups.get(&tile.borrow().get_id()) {
                tile_group.borrow_mut().update_for_bombardment();
            }
        }
    }

    fn update_tiles_for_selected_unit(&self, unit: &Rc<RefCell<MapUnit>>) {
        let holder = self.world_map_holder.borrow();
        let unit = unit.borrow();
        let viewing_civ = &holder.world_screen.borrow().viewing_civ;

        // Get reachable tiles
        let map_pathing = MapPathing::new();
        let reachable_tiles = map_pathing.get_reachable_tiles(&unit, viewing_civ);

        // Update tile visuals
        for tile in reachable_tiles {
            if let Some(tile_group) = holder.tile_groups.get(&tile.borrow().get_id()) {
                tile_group.borrow_mut().update_for_movement();
            }
        }

        // Handle attack ranges
        if unit.can_attack() {
            let target_helper = TargetHelper::new();
            let attackable_tiles = target_helper.get_attackable_tiles(&unit);

            for tile in attackable_tiles {
                if let Some(tile_group) = holder.tile_groups.get(&tile.borrow().get_id()) {
                    tile_group.borrow_mut().update_for_attack();
                }
            }
        }
    }

    fn is_map_reveal_enabled(&self, viewing_civ: &Civilization) -> bool {
        // TODO: Implement map reveal check based on game rules/debug settings
        false
    }
}

// Additional helper functions and implementations
// TODO: Add more functionality as needed