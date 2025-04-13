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

        if !this.unit.has_unit_moved_this_turn()
            && (this.unit.is_fortified() || (this.unit.is_guarding() && this.unit.can_fortify()))
            && this.unit.turns_fortified < 2
        {
            this.unit.turns_fortified += 1;
        }
        if !this.unit.is_fortified() && !this.unit.is_guarding() {
            this.unit.turns_fortified = 0;
        }

        if (!this.unit.has_unit_moved_this_turn() && this.unit.attacks_this_turn == 0)
            || this.unit.has_unique(UniqueType::HealsEvenAfterAction) {
            this.heal_unit();
        }

        if this.unit.is_preparing_paradrop() || this.unit.is_preparing_air_sweep() {
            this.unit.action = None;
        }

        if this.unit.has_unique(UniqueType::ReligiousUnit)
            && this.unit.get_tile().get_owner().is_some()
            && !this.unit.get_tile().get_owner().unwrap().is_city_state
            && !this.unit.civ.diplomacy_functions.can_pass_through_tiles(this.unit.get_tile().get_owner().unwrap())
        {
            let lost_religious_strength = this.unit
                .get_matching_uniques(UniqueType::CanEnterForeignTilesButLosesReligiousStrength)
                .iter()
                .map(|u| u.params[0].parse::<i32>().unwrap_or(0))
                .min();

            if let Some(strength) = lost_religious_strength {
                this.unit.religious_strength_lost += strength;
                if this.unit.religious_strength_lost >= this.unit.base_unit.religious_strength {
                    this.unit.civ.add_notification(
                        format!("Your [{}] lost its faith after spending too long inside enemy territory!", this.unit.name),
                        this.unit.get_tile().position,
                        NotificationCategory::Units,
                        this.unit.name
                    );
                    this.unit.destroy();
                }
            }
        }

        this.do_citadel_damage();
        this.do_terrain_damage();

        this.unit.add_movement_memory();

        for unique in this.unit.get_triggered_uniques(UniqueType::TriggerUponEndingTurnInTile)
            .filter(|u| this.unit.get_tile().matches_filter(&u.params[0], &this.unit.civ))
        {
            UniqueTriggerActivation::trigger_unique(unique, this.unit);
        }
    }

    /// Heals the unit based on the current tile
    fn heal_unit(&mut this) {
        let amount_to_heal_by = this.unit.get_heal_amount_for_current_tile();
        if amount_to_heal_by == 0 {
            return;
        }

        this.unit.heal_by(amount_to_heal_by);
    }

    /// Applies damage from citadels
    fn do_citadel_damage(&mut this) {
        // Check for Citadel damage - note: 'Damage does not stack with other Citadels'
        let citadel_info = this.unit.current_tile.neighbors
            .iter()
            .filter(|tile| {
                tile.get_owner().is_some()
                    && tile.get_unpillaged_improvement().is_some()
                    && this.unit.civ.is_at_war_with(tile.get_owner().unwrap())
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

        this.unit.take_damage(damage);
        let improvement_name = citadel_tile.improvement.clone();  // guarded by `get_unpillaged_improvement() != null` above
        let improvement_icon = format!("ImprovementIcons/{}", improvement_name);
        let locations = LocationAction::new(citadel_tile.position, this.unit.current_tile.position);

        if this.unit.health <= 0 {
            this.unit.civ.add_notification(
                format!("An enemy [{}] has destroyed our [{}]", improvement_name, this.unit.name),
                locations,
                NotificationCategory::War,
                improvement_icon,
                Some(NotificationIcon::Death),
                this.unit.name
            );

            if let Some(owner) = citadel_tile.get_owner() {
                owner.add_notification(
                    format!("Your [{}] has destroyed an enemy [{}]", improvement_name, this.unit.name),
                    locations,
                    NotificationCategory::War,
                    improvement_icon,
                    Some(NotificationIcon::Death),
                    this.unit.name
                );
            }

            this.unit.destroy();
        } else {
            this.unit.civ.add_notification(
                format!("An enemy [{}] has attacked our [{}]", improvement_name, this.unit.name),
                locations,
                NotificationCategory::War,
                improvement_icon,
                Some(NotificationIcon::War),
                this.unit.name
            );
        }
    }

    /// Applies damage from terrain
    fn do_terrain_damage(&mut this) {
        let tile_damage = this.unit.get_damage_from_terrain();
        if tile_damage == 0 {
            return;
        }

        this.unit.take_damage(tile_damage);

        if this.unit.is_destroyed() {
            this.unit.civ.add_notification(
                format!("Our [{}] took [{}] tile damage and was destroyed", this.unit.name, tile_damage),
                this.unit.current_tile.position,
                NotificationCategory::Units,
                this.unit.name,
                Some(NotificationIcon::Death)
            );
        } else {
            this.unit.civ.add_notification(
                format!("Our [{}] took [{}] tile damage", this.unit.name, tile_damage),
                MapUnitAction::new(this.unit),
                NotificationCategory::Units,
                this.unit.name,
                None
            );
        }
    }

    /// Handles start-of-turn actions for the unit
    pub fn start_turn(&mut this) {
        this.unit.movement.clear_pathfinding_cache();
        this.unit.current_movement = this.unit.get_max_movement() as f32;
        this.unit.attacks_this_turn = 0;
        this.unit.due = true;

        for unique in this.unit.get_triggered_uniques(UniqueType::TriggerUponTurnStart) {
            UniqueTriggerActivation::trigger_unique(unique, this.unit);
        }

        // Wake sleeping units if there's an enemy in vision range:
        // Military units always but civilians only if not protected.
        if this.unit.is_sleeping()
            && (this.unit.is_military()
                || (this.unit.current_tile.military_unit.is_none() && !this.unit.current_tile.is_city_center()))
            && this.unit.current_tile.get_tiles_in_distance(3).iter().any(|tile| {
                tile.military_unit.is_some()
                && this.unit.civ.viewable_tiles.contains(tile)
                && tile.military_unit.as_ref().unwrap().civ.is_at_war_with(&this.unit.civ)
            })
        {
            this.unit.action = None;
        }

        if this.unit.action.is_some() && this.unit.health > 99 {
            if this.unit.is_action_until_healed() {
                this.unit.action = None; // wake up when healed
            }
        }

        let tile_owner = this.unit.get_tile().get_owner();
        if tile_owner.is_some()
            && !this.unit.cache.can_enter_foreign_terrain
            && !this.unit.civ.diplomacy_functions.can_pass_through_tiles(tile_owner.unwrap())
            && !tile_owner.unwrap().is_city_state
        {
            // if an enemy city expanded onto this tile while I was in it
            this.unit.movement.teleport_to_closest_moveable_tile();
        }

        this.unit.add_movement_memory();
        this.unit.attacks_since_turn_start.clear();

        // Update status effects
        let status_names: Vec<String> = this.unit.status_map.values()
            .map(|status| status.name.clone())
            .collect();

        for status_name in status_names {
            if let Some(status) = this.unit.status_map.get_mut(&status_name) {
                status.turns_left -= 1;
                if status.turns_left <= 0 {
                    this.unit.remove_status(&status_name);
                }
            }
        }

        this.unit.update_uniques();
    }
}