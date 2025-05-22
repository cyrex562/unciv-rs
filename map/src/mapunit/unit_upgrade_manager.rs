use crate::{
    map::mapunit::MapUnit,
    models::ruleset::{RejectionReasonType, BaseUnit},
    models::ruleset::unique::UniqueType,
    civilization::Civilization,
};

/// Manages unit upgrade-related functionality
pub struct UnitUpgradeManager<'a> {
    /// The unit this manager belongs to
    unit: &'a MapUnit,
}

impl<'a> UnitUpgradeManager<'a> {
    /// Creates a new UnitUpgradeManager for the given unit
    pub fn new(unit: &'a MapUnit) -> Self {
        Self { unit }
    }

    /// Check whether this unit can upgrade to the specified unit type.
    /// This does not check or follow the normal upgrade chain defined by `BaseUnit::get_upgrade_units`.
    ///
    /// # Arguments
    /// * `unit_to_upgrade_to` - The unit type to upgrade to
    /// * `ignore_requirements` - Ignore possible tech/policy/building requirements (e.g. resource requirements still count).
    ///                         Used for upgrading units via ancient ruins.
    /// * `ignore_resources` - Ignore resource requirements (tech still counts).
    ///                       Used to display disabled Upgrade button
    pub fn can_upgrade(
        &self,
        unit_to_upgrade_to: &BaseUnit,
        ignore_requirements: bool,
        ignore_resources: bool
    ) -> bool {
        if self.unit.name == unit_to_upgrade_to.name {
            return false;
        }

        let rejection_reasons = unit_to_upgrade_to.get_rejection_reasons(
            &self.unit.civ,
            self.unit.get_resource_requirements_per_turn()
        );

        let mut relevant_rejection_reasons = rejection_reasons.iter()
            .filter(|reason| !reason.is_construction_rejection() && reason.reason_type != RejectionReasonType::Obsoleted)
            .collect::<Vec<_>>();

        if ignore_requirements {
            relevant_rejection_reasons.retain(|reason| !reason.tech_policy_era_wonder_requirements());
        }
        if ignore_resources {
            relevant_rejection_reasons.retain(|reason| reason.reason_type != RejectionReasonType::ConsumesResources);
        }

        relevant_rejection_reasons.is_empty()
    }

    /// Determine gold cost of a Unit Upgrade, potentially over several steps.
    ///
    /// # Arguments
    /// * `unit_to_upgrade_to` - The final BaseUnit. Must be reachable via normal upgrades or else
    ///                         the function will return the cost to upgrade to the last possible and researched normal upgrade.
    ///
    /// # Returns
    /// Gold cost in increments of 5, never negative. Will return 0 for invalid inputs (unit can't upgrade or is already a `unit_to_upgrade_to`)
    ///
    /// # Reference
    /// See [CvUnit::upgradePrice](https://github.com/dmnd/CvGameCoreSource/blob/6501d2398113a5100ffa854c146fb6f113992898/CvGameCoreDLL_Expansion1/CvUnit.cpp#L7728)
    pub fn get_cost_of_upgrade(&self, unit_to_upgrade_to: &BaseUnit) -> i32 {
        // Source rounds to int every step, we don't
        // TODO: From the source, this should apply _Production_ modifiers (Temple of Artemis? GameSpeed! StartEra!), at the moment it doesn't

        let mut gold_cost_of_upgrade = 0;

        let ruleset = &self.unit.civ.game_info.ruleset;
        let constants = &ruleset.mod_options.constants.unit_upgrade_cost;

        // Apply modifiers: Wonders (Pentagon), Policies (Professional Army). Cached outside loop despite
        // the UniqueType being allowed on a BaseUnit - we don't have a MapUnit in the loop.
        // Actually instantiating every intermediate to support such mods: todo
        let mut civ_modifier = 1.0;
        let state_for_conditionals = &self.unit.cache.state;

        for unique in self.unit.civ.get_matching_uniques(UniqueType::UnitUpgradeCost, state_for_conditionals) {
            civ_modifier *= unique.params[0].parse::<f32>().unwrap_or(1.0) / 100.0;
        }

        let mut cost = constants.base;
        cost += (constants.per_production * (unit_to_upgrade_to.cost as f32 - self.unit.base_unit.cost as f32)).max(0.0);

        if let Some(era) = unit_to_upgrade_to.era(ruleset) {
            cost *= 1.0 + era.era_number as f32 * constants.era_multiplier;
        }

        cost = (cost * civ_modifier).powf(constants.exponent);
        cost *= self.unit.civ.game_info.speed.modifier;

        gold_cost_of_upgrade += (cost / constants.round_to as f32).floor() as i32 * constants.round_to;

        gold_cost_of_upgrade
    }

    /// Perform an upgrade, assuming validity checks were already passed.
    ///
    /// # Arguments
    /// * `upgraded_unit` - The unit type to upgrade to
    /// * `is_free` - Whether the upgrade is free (no gold cost)
    /// * `gold_cost_of_upgrade` - Optional gold cost of the upgrade. If None, will be calculated.
    ///
    /// # Note
    /// Continuing to use a reference to this manager or its unit after this call is invalid!
    /// Please use `UnitActionsUpgrade::get_upgrade_actions` instead if at all possible.
    ///
    /// Note - the upgraded unit is a new instance, and it's possible this method will need to place it on a different tile.
    /// It is also possible the placement fails and the original is resurrected - in which case it is a **new instance** as well.
    pub fn perform_upgrade(
        &self,
        upgraded_unit: &BaseUnit,
        is_free: bool,
        gold_cost_of_upgrade: Option<i32>
    ) {
        // When mashing the upgrade button, you can 'queue' 2 upgrade actions
        // If both are performed, what you get is the unit is doubled
        // This prevents this, since we lack another way to do so -_-'
        if self.unit.is_destroyed {
            return;
        }

        self.unit.destroy(false); // destroyTransportedUnit = false
        let civ = &self.unit.civ;
        let position = self.unit.current_tile.position;
        let new_unit = civ.units.place_unit_near_tile(position, upgraded_unit, Some(self.unit.id));

        // We were UNABLE to place the new unit, which means that the unit failed to upgrade!
        // The only known cause of this currently is "land units upgrading to water units" which fail to be placed.
        if new_unit.is_none() {
            let resurrected_unit = civ.units.place_unit_near_tile(position, &self.unit.base_unit, None).unwrap();
            self.unit.copy_statistics_to(&resurrected_unit);
            return;
        }

        let new_unit = new_unit.unwrap();

        // Managed to upgrade
        if !is_free {
            let cost = gold_cost_of_upgrade.unwrap_or_else(|| self.get_cost_of_upgrade(upgraded_unit));
            civ.add_gold(-cost);
        }

        self.unit.copy_statistics_to(&new_unit);
        new_unit.current_movement = 0.0;

        // Wake up if lost ability to fortify
        if new_unit.is_fortified() && !new_unit.can_fortify(true) {
            new_unit.action = None;
        }

        // Wake up from Guarding if can't Withdraw
        if new_unit.is_guarding() && !new_unit.has_unique(UniqueType::WithdrawsBeforeMeleeCombat) {
            new_unit.action = None;
        }
    }
}