use crate::battle::battle::CombatAction;
use crate::battle::i_combatant::ICombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::battle::target_helper::target_helper;

use std::collections::HashMap;
use std::sync::Arc;

/// Module for Great General implementation
pub mod GreatGeneralImplementation {
    use super::*;

    /// Data structure for Great General bonus information
    #[derive(Debug, Clone)]
    struct GeneralBonusData {
        /// The Great General unit
        general: Arc<MapUnit>,
        /// The radius of effect
        radius: i32,
        /// The filter for affected units
        filter: String,
        /// The bonus percentage
        bonus: i32,
    }

    impl GeneralBonusData {
        /// Creates a new GeneralBonusData from a Great General unit and a unique
        fn new(general: Arc<MapUnit>, unique: &Unique) -> Self {
            GeneralBonusData {
                general,
                radius: unique.params[2].parse::<i32>().unwrap_or(0),
                filter: unique.params[1].clone(),
                bonus: unique.params[0].parse::<i32>().unwrap_or(0),
            }
        }
    }

    /// Determine the "Great General" bonus for a unit by searching for units carrying the StrengthBonusInRadius unique in the vicinity.
    ///
    /// Used by BattleDamage.getGeneralModifiers.
    ///
    /// # Arguments
    ///
    /// * `our_unit_combatant` - The unit to calculate the bonus for
    /// * `enemy` - The enemy unit
    /// * `combat_action` - The type of combat action being performed
    ///
    /// # Returns
    ///
    /// A tuple of unit's name and bonus (percentage) as i32 (typically 15), or 0 if no applicable Great General equivalents found
    pub fn get_great_general_bonus(
        our_unit_combatant: &MapUnitCombatant,
        enemy: &dyn ICombatant,
        combat_action: CombatAction
    ) -> (String, i32) {
        let unit = &our_unit_combatant.unit;
        let civ_info = &unit.civ;

        let all_generals: Vec<_> = civ_info.units.get_civ_units()
            .iter()
            .filter(|u| u.cache.has_strength_bonus_in_radius_unique)
            .collect();

        if all_generals.is_empty() {
            return (String::new(), 0);
        }

        let state = StateForConditionals::new(
            Some(our_unit_combatant.clone()),
            Some(Box::new(enemy.clone())),
            civ_info.clone(),
            Some(combat_action),
        );

        let great_generals: Vec<_> = all_generals.iter()
            .flat_map(|general| {
                general.get_matching_uniques(UniqueType::StrengthBonusInRadius, &state)
                    .iter()
                    .map(|unique| GeneralBonusData::new(general.clone(), unique))
                    .collect::<Vec<_>>()
            })
            .filter(|data| {
                // Support the border case when a mod unit has several
                // GreatGeneralAura uniques (e.g. +50% as radius 1, +25% at radius 2, +5% at radius 3)
                // The "Military" test is also supported deep down in unit.matchesFilter, a small
                // optimization for the most common case, as this function is only called for `MapUnitCombatant`s
                data.general.current_tile.aerial_distance_to(unit.get_tile()) <= data.radius
                    && (data.filter == "Military" || unit.matches_filter(&data.filter))
            })
            .collect();

        let great_general_modifier = match great_generals.iter()
            .max_by_key(|data| data.bonus) {
                Some(data) => data,
                None => return (String::new(), 0),
            };

        if unit.has_unique(UniqueType::GreatGeneralProvidesDoubleCombatBonus, true)
            && great_general_modifier.general.is_great_person_of_type("War") { // apply only on "true" generals
            return (great_general_modifier.general.name.clone(), great_general_modifier.bonus * 2);
        }

        (great_general_modifier.general.name.clone(), great_general_modifier.bonus)
    }

    /// Find a tile for accompanying a military unit where the total bonus for all affected units is maximized.
    ///
    /// Used by SpecificUnitAutomation.automateGreatGeneral.
    ///
    /// # Arguments
    ///
    /// * `general` - The Great General unit
    ///
    /// # Returns
    ///
    /// The best tile for the Great General to move to, or None if no suitable tile is found
    pub fn get_best_affected_troops_tile(general: &MapUnit) -> Option<Arc<Tile>> {
        // Normally we have only one Unique here. But a mix is not forbidden, so let's try to support mad modders.
        // (imagine several GreatGeneralAura uniques - +50% at radius 1, +25% at radius 2, +5% at radius 3 - possibly learnable from promotions via buildings or natural wonders?)

        // Map out the uniques sorted by bonus, as later only the best bonus will apply.
        let mut general_bonus_data: Vec<_> = general.get_matching_uniques(UniqueType::StrengthBonusInRadius)
            .iter()
            .map(|unique| GeneralBonusData::new(general.clone(), unique))
            .collect();

        // Sort by bonus (descending) and then by radius
        general_bonus_data.sort_by(|a, b| {
            b.bonus.cmp(&a.bonus)
                .then(a.radius.cmp(&b.radius))
        });

        // Get candidate units to 'follow', coarsely.
        // The mapUnitFilter of the unique won't apply here but in the ranking of the "Aura" effectiveness.
        let unit_max_movement = general.get_max_movement();
        let military_unit_tiles_in_distance: Vec<_> = general.movement.get_distance_to_tiles()
            .iter()
            .map(|(tile, _)| tile.clone())
            .filter(|tile| {
                if let Some(military_unit) = &tile.military_unit {
                    military_unit.civ == general.civ
                        && (tile.civilian_unit.is_none() || tile.civilian_unit.as_ref().map_or(false, |u| u == general))
                        && military_unit.get_max_movement() <= unit_max_movement
                        && !tile.is_city_center()
                } else {
                    false
                }
            })
            .collect();

        // rank tiles and find best
        let unit_bonus_radius = general_bonus_data.iter()
            .map(|data| data.radius)
            .max()?;

        let mut military_unit_to_has_attackable_enemies = HashMap::new();

        military_unit_tiles_in_distance.into_iter()
            .max_by_key(|unit_tile| {
                unit_tile.get_tiles_in_distance(unit_bonus_radius)
                    .iter()
                    .map(|affected_tile| {
                        if let Some(military_unit) = &affected_tile.military_unit {
                            if military_unit.civ != general.civ || military_unit.is_embarked() {
                                0
                            } else {
                                let has_attackable_enemies = military_unit_to_has_attackable_enemies
                                    .entry(military_unit.clone())
                                    .or_insert_with(|| {
                                        !target_helper::get_attackable_enemies(
                                            military_unit,
                                            military_unit.movement.get_distance_to_tiles(),
                                            CombatAction::Attack,
                                            false
                                        ).is_empty()
                                    });

                                if *has_attackable_enemies {
                                    0
                                } else {
                                    general_bonus_data.iter()
                                        .find(|data| {
                                            // "Military" as commented above only a small optimization
                                            affected_tile.aerial_distance_to(unit_tile) <= data.radius
                                                && (data.filter == "Military" || military_unit.matches_filter(&data.filter))
                                        })
                                        .map(|data| data.bonus)
                                        .unwrap_or(0)
                                }
                            }
                        } else {
                            0
                        }
                    })
                    .sum::<i32>()
            })
            .cloned()
    }
}