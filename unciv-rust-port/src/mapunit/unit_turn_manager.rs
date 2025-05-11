use crate::{
    map::mapunit::MapUnit,
    models::ruleset::unique::{UniqueTriggerActivation, UniqueType},
    civilization::{LocationAction, MapUnitAction, NotificationCategory, NotificationIcon},
};

/// Manages unit turn-related actions and state changes
pub struct UnitTurnManager<'a> {
    /// The unit this manager belongs to
    unit: &'a mut MapUnit,
}

impl<'a> UnitTurnManager<'a> {
    /// Creates a new UnitTurnManager for the given unit
    pub fn new(unit: &'a mut MapUnit) -> Self {
        Self { unit }
    }

    /// Handles end-of-turn actions for the unit
    pub fn end_turn(&mut self) {
        self.unit.movement.clear_pathfinding_cache();

        for unique in self.unit.get_triggered_uniques(UniqueType::TriggerUponTurnEnd) {
            UniqueTriggerActivation::trigger_unique(unique, self.unit);
        }

        if self.unit.has_movement()
            && self.unit.get_tile().improvement_in_progress.is_some()
            && self.unit.can_build_improvement(self.unit.get_tile().get_tile_improvement_in_progress().unwrap())
        {
            let tile = self.unit.get_tile();
            if tile.do_worker_turn(self.unit) {
                if let Some(city) = tile.get_city() {
                    city.should_reassign_population = true;
                }
            }
        }

        if !self.unit.has_unit_moved_this_turn()
            && (self.unit.is_fortified() || (self.unit.is_guarding() && self.unit.can_fortify()))
            && self.unit.turns_fortified < 2
        {
            self.unit.turns_fortified += 1;
        }
        if !self.unit.is_fortified() && !self.unit.is_guarding() {
            self.unit.turns_fortified = 0;
        }

        if (!self.unit.has_unit_moved_this_turn() && self.unit.attacks_this_turn == 0)
            || self.unit.has_unique(UniqueType::HealsEvenAfterAction) {
            self.heal_unit();
        }

        if self.unit.is_preparing_paradrop() || self.unit.is_preparing_air_sweep() {
            self.unit.action = None;
        }

        if self.unit.has_unique(UniqueType::ReligiousUnit)
            && self.unit.get_tile().get_owner().is_some()
            && !self.unit.get_tile().get_owner().unwrap().is_city_state
            && !self.unit.civ.diplomacy_functions.can_pass_through_tiles(self.unit.get_tile().get_owner().unwrap())
        {
            let lost_religious_strength = self.unit
                .get_matching_uniques(UniqueType::CanEnterForeignTilesButLosesReligiousStrength)
                .iter()
                .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
                .min();

            if let Some(strength) = lost_religious_strength {
                self.unit.religious_strength_lost += strength;
                if self.unit.religious_strength_lost >= self.unit.base_unit.religious_strength {
                    self.unit.civ.add_notification(
                        format!("Your [{}] lost its faith after spending too long inside enemy territory!", self.unit.name),
                        self.unit.get_tile().position,
                        NotificationCategory::Units,
                        self.unit.name
                    );
                    self.unit.destroy();
                }
            }
        }

        self.do_citadel_damage();
        self.do_terrain_damage();

        self.unit.add_movement_memory();

        for unique in self.unit.get_triggered_uniques(UniqueType::TriggerUponEndingTurnInTile)
            .filter(|u| self.unit.get_tile().matches_filter(&u.params[0], &self.unit.civ))
        {
            UniqueTriggerActivation::trigger_unique(unique, self.unit);
        }
    }

    /// Heals the unit based on the current tile
    fn heal_unit(&mut self) {
        let amount_to_heal_by = self.unit.get_heal_amount_for_current_tile();
        if amount_to_heal_by == 0 {
            return;
        }

        self.unit.heal_by(amount_to_heal_by);
    }

    /// Applies damage from citadels
    fn do_citadel_damage(&mut self) {
        // Check for Citadel damage - note: 'Damage does not stack with other Citadels'
        let citadel_info = self.unit.current_tile.neighbors
            .iter()
            .filter(|tile| {
                tile.get_owner().is_some()
                    && tile.get_unpillaged_improvement().is_some()
                    && self.unit.civ.is_at_war_with(tile.get_owner().unwrap())
            })
            .map(|tile| {
                let damage = tile.get_tile_improvement().unwrap()
                    .get_matching_uniques(UniqueType::DamagesAdjacentEnemyUnits, &tile.state_this_tile)
                    .iter()
                    .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
                    .sum();
                (tile, damage)
            })
            .max_by_key(|(_, damage)| *damage);

        let (citadel_tile, damage) = match citadel_info {
            Some(info) => info,
            None => return,
        };

        if damage == 0 {
            return;
        }

        self.unit.take_damage(damage);
        let improvement_name = citadel_tile.improvement.clone();  // guarded by `get_unpillaged_improvement() != null` above
        let improvement_icon = format!("ImprovementIcons/{}", improvement_name);
        let locations = LocationAction::new(citadel_tile.position, self.unit.current_tile.position);

        if self.unit.health <= 0 {
            self.unit.civ.add_notification(
                format!("An enemy [{}] has destroyed our [{}]", improvement_name, self.unit.name),
                locations,
                NotificationCategory::War,
                improvement_icon,
                Some(NotificationIcon::Death),
                self.unit.name
            );

            if let Some(owner) = citadel_tile.get_owner() {
                owner.add_notification(
                    format!("Your [{}] has destroyed an enemy [{}]", improvement_name, self.unit.name),
                    locations,
                    NotificationCategory::War,
                    improvement_icon,
                    Some(NotificationIcon::Death),
                    self.unit.name
                );
            }

            self.unit.destroy();
        } else {
            self.unit.civ.add_notification(
                format!("An enemy [{}] has attacked our [{}]", improvement_name, self.unit.name),
                locations,
                NotificationCategory::War,
                improvement_icon,
                Some(NotificationIcon::War),
                self.unit.name
            );
        }
    }

    /// Applies damage from terrain
    fn do_terrain_damage(&mut self) {
        let tile_damage = self.unit.get_damage_from_terrain();
        if tile_damage == 0 {
            return;
        }

        self.unit.take_damage(tile_damage);

        if self.unit.is_destroyed() {
            self.unit.civ.add_notification(
                format!("Our [{}] took [{}] tile damage and was destroyed", self.unit.name, tile_damage),
                self.unit.current_tile.position,
                NotificationCategory::Units,
                self.unit.name,
                Some(NotificationIcon::Death)
            );
        } else {
            self.unit.civ.add_notification(
                format!("Our [{}] took [{}] tile damage", self.unit.name, tile_damage),
                MapUnitAction::new(self.unit),
                NotificationCategory::Units,
                self.unit.name,
                None
            );
        }
    }

    /// Handles start-of-turn actions for the unit
    pub fn start_turn(&mut self) {
        self.unit.movement.clear_pathfinding_cache();
        self.unit.current_movement = self.unit.get_max_movement() as f32;
        self.unit.attacks_this_turn = 0;
        self.unit.due = true;

        for unique in self.unit.get_triggered_uniques(UniqueType::TriggerUponTurnStart) {
            UniqueTriggerActivation::trigger_unique(unique, self.unit);
        }

        // Wake sleeping units if there's an enemy in vision range:
        // Military units always but civilians only if not protected.
        if self.unit.is_sleeping()
            && (self.unit.is_military()
                || (self.unit.current_tile.military_unit.is_none() && !self.unit.current_tile.is_city_center()))
            && self.unit.current_tile.get_tiles_in_distance(3).iter().any(|tile| {
                tile.military_unit.is_some()
                && self.unit.civ.viewable_tiles.contains(tile)
                && tile.military_unit.as_ref().unwrap().civ.is_at_war_with(&self.unit.civ)
            })
        {
            self.unit.action = None;
        }

        if self.unit.action.is_some() && self.unit.health > 99 {
            if self.unit.is_action_until_healed() {
                self.unit.action = None; // wake up when healed
            }
        }

        let tile_owner = self.unit.get_tile().get_owner();
        if tile_owner.is_some()
            && !self.unit.cache.can_enter_foreign_terrain
            && !self.unit.civ.diplomacy_functions.can_pass_through_tiles(tile_owner.unwrap())
            && !tile_owner.unwrap().is_city_state
        {
            // if an enemy city expanded onto this tile while I was in it
            self.unit.movement.teleport_to_closest_moveable_tile();
        }

        self.unit.add_movement_memory();
        self.unit.attacks_since_turn_start.clear();

        // Update status effects
        let status_names: Vec<String> = self.unit.status_map.values()
            .map(|status| status.name.clone())
            .collect();

        for status_name in status_names {
            if let Some(status) = self.unit.status_map.get_mut(&status_name) {
                status.turns_left -= 1;
                if status.turns_left <= 0 {
                    self.unit.remove_status(&status_name);
                }
            }
        }

        self.unit.update_uniques();
    }
}