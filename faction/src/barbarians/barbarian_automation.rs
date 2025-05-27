// Update the import path to the correct module where Civilization is defined


/// Handles automation of barbarian units in the game.
pub struct BarbarianAutomation<'a> {
    civ_info: &'a Civilization,
}

impl<'a> BarbarianAutomation<'a> {
    /// Creates a new BarbarianAutomation instance.
    pub fn new(civ_info: &'a Civilization) -> Self {
        BarbarianAutomation { civ_info }
    }

    /// Automates all barbarian units in a specific order.
    pub fn automate(&self) {
        let civ_units = self.civ_info.units.get_civ_units();

        // Process ranged units first
        for unit in civ_units.iter().filter(|u| u.base_unit.is_ranged()) {
            self.automate_unit(unit);
        }

        // Then process melee units
        for unit in civ_units.iter().filter(|u| u.base_unit.is_melee()) {
            self.automate_unit(unit);
        }

        // Finally process other units
        for unit in civ_units
            .iter()
            .filter(|u| !u.base_unit.is_ranged() && !u.base_unit.is_melee())
        {
            self.automate_unit(unit);
        }

        // Clear popup alerts to reduce save size and ease debugging
        self.civ_info.popup_alerts.clear();
    }

    /// Automates a single unit based on its type and current state.
    fn automate_unit(&self, unit: &mut MapUnit) {
        if unit.is_civilian() {
            self.automate_captured_civilian(unit);
        } else {
            match unit.current_tile.improvement == BARBARIAN_ENCAMPMENT {
                true => {
                    self.automate_unit_on_encampment(unit);
                }
                false => {
                    self.automate_combat_unit(unit);
                }
            }
        }
    }

    /// Handles automation of captured civilian units.
    fn automate_captured_civilian(&self, unit: &mut MapUnit) {
        // 1. Stay on current encampment if already there
        if unit.current_tile.improvement == BARBARIAN_ENCAMPMENT {
            return;
        }

        // 2. Find and move to nearest available encampment
        let camp_tiles: Vec<_> = self
            .civ_info
            .game_info
            .barbarians
            .encampments
            .iter()
            .map(|camp| self.civ_info.game_info.tile_map.get_tile(camp.position))
            .collect();

        let mut camp_tiles: Vec<_> = camp_tiles.into_iter().filter_map(|t| t).collect();

        camp_tiles.sort_by(|a, b| {
            unit.current_tile
                .aerial_distance_to(a)
                .partial_cmp(&unit.current_tile.aerial_distance_to(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_camp = camp_tiles
            .iter()
            .find(|tile| tile.civilian_unit.is_none() && unit.movement.can_reach(tile));

        if let Some(camp) = best_camp {
            unit.movement.head_towards(camp);
        } else {
            // 3. Wander aimlessly if no reachable encampment found
            UnitAutomation::wander(
                unit,
                &mut self.civ_info.game_info,
                &mut self.civ_info.clone(),
            );
        }
    }

    /// Handles automation of units stationed at encampments.
    fn automate_unit_on_encampment(&self, unit: &mut MapUnit) {
        // 1. Try to upgrade
        if UnitAutomation::try_upgrade_unit(unit) {
            return;
        }

        // 2. Try to attack without leaving encampment
        if BattleHelper::try_attack_nearby_enemy(unit, true) {
            return;
        }

        // 3. Fortify if possible
        unit.fortify_if_can();
    }

    /// Handles automation of combat units.
    fn automate_combat_unit(&self, unit: &mut MapUnit) {
        // 1. Try pillaging to restore health (barbs don't auto-heal)
        if unit.health < 50
            && UnitAutomation::try_pillage_improvement(unit, true)
            && !unit.has_movement()
        {
            return;
        }

        // 2. Try to upgrade
        if UnitAutomation::try_upgrade_unit(unit) {
            return;
        }

        // 3. Try to attack enemy
        // If an embarked melee unit can land and attack next turn, do not attack from water
        if BattleHelper::try_disembark_unit_to_attack_position(unit) {
            return;
        }
        if !unit.is_civilian() && BattleHelper::try_attack_nearby_enemy(unit, false) {
            return;
        }

        // 4. Try to pillage tile or route
        while UnitAutomation::try_pillage_improvement(unit, false) {
            if !unit.has_movement() {
                return;
            }
        }

        // 5. Wander
        UnitAutomation::wander(
            unit,
            &mut self.civ_info.game_info,
            &mut self.civ_info.clone(),
        );
    }
}
