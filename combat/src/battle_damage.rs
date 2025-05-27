use std::collections::HashMap;
use std::f32;
use std::sync::Arc;

use crate::battle::battle_constants::BattleConstants;
use crate::battle::i_combatant::ICombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::battle::city_combatant::CityCombatant;
use crate::battle::great_general_implementation::GreatGeneralImplementation;


/// Enum representing different combat actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatAction {
    Attack,
    Defend,
    Intercept,
}

/// Module for combat damage calculations
pub mod battle_damage {
    use crate::city;

    use super::*;
    use std::cmp::max;

    /// Gets a string description of a modifier from a unique
    fn get_modifier_string_from_unique(unique: &Unique) -> String {
        let source = match unique.source_object_type {
            UniqueTarget::Unit => tr("Unit ability"),
            UniqueTarget::Nation => tr("National ability"),
            UniqueTarget::Global => GlobalUniques::get_unique_source_description(unique),
            _ => format!("[{}] ([{}])", unique.source_object_name, unique.get_source_name_for_user()),
        };

        if unique.modifiers.is_empty() {
            return source;
        }

        let conditionals_text = unique.modifiers.iter()
            .map(|m| tr(&m.text))
            .collect::<Vec<String>>()
            .join(" ");

        format!("{} - {}", source, conditionals_text)
    }

    /// Gets the state for conditionals based on combat action and combatants
    fn get_state_for_conditionals(
        combat_action: CombatAction,
        combatant: &dyn ICombatant,
        enemy: &dyn ICombatant,
    ) -> StateForConditionals {
        let attacked_tile = if combat_action == CombatAction::Attack {
            enemy.get_tile()
        } else {
            combatant.get_tile()
        };

        StateForConditionals::new(
            combatant.get_civ_info(),
            combatant.as_any().downcast_ref::<CityCombatant>().map(|c| c.city.clone()),
            Some(combatant.clone()),
            Some(enemy.clone()),
            Some(attacked_tile),
            Some(combat_action),
        )
    }

    /// Gets general modifiers for a combatant
    fn get_general_modifiers(
        combatant: &dyn ICombatant,
        enemy: &dyn ICombatant,
        combat_action: CombatAction,
        tile_to_attack_from: &Tile,
    ) -> Counter<String> {
        let mut modifiers = Counter::new();
        let conditional_state = Self::get_state_for_conditionals(combat_action, combatant, enemy);
        let civ_info = combatant.get_civ_info();

        if let Some(map_unit_combatant) = combatant.as_any().downcast_ref::<MapUnitCombatant>() {
            Self::add_unit_unique_modifiers(map_unit_combatant, enemy, &conditional_state, tile_to_attack_from, &mut modifiers);
            Self::add_resource_lacking_malus(map_unit_combatant, &mut modifiers);

            let (great_general_name, great_general_bonus) = GreatGeneralImplementation::get_great_general_bonus(
                map_unit_combatant, enemy, combat_action
            );

            if great_general_bonus != 0 {
                modifiers.add(great_general_name, great_general_bonus);
            }

            for unique in map_unit_combatant.unit.get_matching_uniques(UniqueType::StrengthWhenStacked) {
                let mut stacked_units_bonus = 0;
                if map_unit_combatant.unit.get_tile().get_units().iter()
                    .any(|u| u.matches_filter(&unique.params[1])) {
                    stacked_units_bonus += unique.params[0].parse::<i32>().unwrap_or(0);
                }

                if stacked_units_bonus > 0 {
                    modifiers.add(format!("Stacked with [{}]", unique.params[1]), stacked_units_bonus);
                }
            }
        } else if let Some(city_combatant) = combatant.as_any().downcast_ref::<CityCombatant>() {
            for unique in city_combatant.city.get_matching_uniques(UniqueType::StrengthForCities, &conditional_state) {
                modifiers.add(Self::get_modifier_string_from_unique(unique), unique.params[0].parse::<i32>().unwrap_or(0));
            }
        }

        if enemy.get_civ_info().is_barbarian {
            modifiers.add(
                "Difficulty".to_string(),
                (civ_info.game_info.get_difficulty().barbarian_bonus * 100.0) as i32
            );
        }

        modifiers
    }

    /// Adds unit-specific unique modifiers
    fn add_unit_unique_modifiers(
        combatant: &MapUnitCombatant,
        enemy: &dyn ICombatant,
        conditional_state: &StateForConditionals,
        tile_to_attack_from: &Tile,
        modifiers: &mut Counter<String>,
    ) {
        let civ_info = combatant.get_civ_info();

        for unique in combatant.get_matching_uniques(UniqueType::Strength, conditional_state, true) {
            modifiers.add(
                Self::get_modifier_string_from_unique(unique),
                unique.params[0].parse::<i32>().unwrap_or(0)
            );
        }

        // e.g., Mehal Sefari https://civilization.fandom.com/wiki/Mehal_Sefari_(Civ5)
        for unique in combatant.get_matching_uniques(
            UniqueType::StrengthNearCapital,
            conditional_state,
            true
        ) {
            if civ_info.cities.is_empty() || civ_info.get_capital().is_none() {
                break;
            }

            let capital = civ_info.get_capital().unwrap();
            let distance = combatant.get_tile().aerial_distance_to(capital.get_center_tile());

            // https://steamcommunity.com/sharedfiles/filedetails/?id=326411722#464287
            let effect = unique.params[0].parse::<i32>().unwrap_or(0) - 3 * distance;

            if effect > 0 {
                modifiers.add(
                    format!("{} ({})", unique.source_object_name, unique.get_source_name_for_user()),
                    effect
                );
            }
        }

        // https://www.carlsguides.com/strategy/civilization5/war/combatbonuses.php
        let mut adjacent_units = combatant.get_tile().neighbors.iter()
            .flat_map(|t| t.get_units())
            .collect::<Vec<_>>();

        if !combatant.get_tile().neighbors.contains(&enemy.get_tile())
            && combatant.get_tile().neighbors.contains(tile_to_attack_from)
            && enemy.as_any().downcast_ref::<MapUnitCombatant>().is_some() {
            if let Some(map_unit_enemy) = enemy.as_any().downcast_ref::<MapUnitCombatant>() {
                adjacent_units.push(map_unit_enemy.unit.clone());
            }
        }

        // e.g., Maori Warrior - https://civilization.fandom.com/wiki/Maori_Warrior_(Civ5)
        let conditional_state = &StateForConditionals::new(
            combatant.get_civ_info(),
            None,
            Some(combatant.clone()),
            None,
            Some(combatant.get_tile()),
            None,
        );
        let strength_malus = adjacent_units.iter()
            .filter(|u| u.civ.is_at_war_with(combatant.get_civ_info()))
            .flat_map(|u| u.get_matching_uniques(UniqueType::StrengthForAdjacentEnemies))
            .filter(|u| combatant.matches_filter(&u.params[1], conditional_state) && combatant.get_tile().matches_filter(&u.params[2], conditional_state))
            .max_by_key(|u| u.params[0].parse::<i32>().unwrap_or(0));

        if let Some(malus) = strength_malus {
            modifiers.add("Adjacent enemy units".to_string(), malus.params[0].parse::<i32>().unwrap_or(0));
        }
    }

    /// Adds malus for lacking resources
    fn add_resource_lacking_malus(combatant: &MapUnitCombatant, modifiers: &mut Counter<String>) {
        let civ_info = combatant.get_civ_info();
        let civ_resources = civ_info.get_civ_resources_by_name();

        for resource in combatant.unit.get_resource_requirements_per_turn().keys() {
            if civ_resources.get(resource).map_or(false, |&count| count < 0) && !civ_info.is_barbarian {
                modifiers.add("Missing resource".to_string(), BattleConstants::MISSING_RESOURCES_MALUS);
                break;
            }
        }
    }

    /// Gets attack modifiers for a combatant
    pub fn get_attack_modifiers(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
    ) -> Counter<String> {
        let mut modifiers = Self::get_general_modifiers(attacker, defender, CombatAction::Attack, tile_to_attack_from);

        if let Some(map_unit_combatant) = attacker.as_any().downcast_ref::<MapUnitCombatant>() {
            Self::add_terrain_attack_modifiers(map_unit_combatant, defender, tile_to_attack_from, &mut modifiers);

            // Air unit attacking with Air Sweep
            if map_unit_combatant.unit.is_preparing_air_sweep() {
                modifiers.add_all(Self::get_air_sweep_attack_modifiers(attacker));
            }

            if map_unit_combatant.is_melee() {
                let number_of_other_attackers_surrounding_defender = defender.get_tile().neighbors.iter()
                    .filter(|t| {
                        t.military_unit.is_some()
                        && t.military_unit.as_ref().unwrap() != &map_unit_combatant.unit
                        && t.military_unit.as_ref().unwrap().owner == attacker.get_civ_info().civ_name
                        && MapUnitCombatant::new(t.military_unit.as_ref().unwrap().clone()).is_melee()
                    })
                    .count();

                if number_of_other_attackers_surrounding_defender > 0 {
                    let mut flanking_bonus = BattleConstants::BASE_FLANKING_BONUS;

                    // e.g., Discipline policy - https://civilization.fandom.com/wiki/Discipline_(Civ5)
                    for unique in map_unit_combatant.unit.get_matching_uniques(
                        UniqueType::FlankAttackBonus,
                        true,
                        Self::get_state_for_conditionals(CombatAction::Attack, attacker, defender)
                    ) {
                        flanking_bonus *= unique.params[0].parse::<f32>().unwrap_or(1.0) / 100.0;
                    }

                    modifiers.add(
                        "Flanking".to_string(),
                        (flanking_bonus * number_of_other_attackers_surrounding_defender as f32) as i32
                    );
                }
            }
        }

        modifiers
    }

    /// Adds terrain-specific attack modifiers
    fn add_terrain_attack_modifiers(
        attacker: &MapUnitCombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
        modifiers: &mut Counter<String>,
    ) {
        if attacker.unit.is_embarked()
            && defender.get_tile().is_land
            && !attacker.unit.has_unique(UniqueType::AttackAcrossCoast) {
            modifiers.add("Landing".to_string(), BattleConstants::LANDING_MALUS);
        }

        // Land Melee Unit attacking to Water
        if attacker.unit.type_.is_land_unit()
            && !attacker.get_tile().is_water
            && attacker.is_melee()
            && defender.get_tile().is_water
            && !attacker.unit.has_unique(UniqueType::AttackAcrossCoast) {
            modifiers.add("Boarding".to_string(), BattleConstants::BOARDING_MALUS);
        }

        // Melee Unit on water attacking to Land (not City) unit
        if !attacker.unit.type_.is_air_unit()
            && attacker.is_melee()
            && attacker.get_tile().is_water
            && !defender.get_tile().is_water
            && !attacker.unit.has_unique(UniqueType::AttackAcrossCoast)
            && !defender.is_city() {
            modifiers.add("Landing".to_string(), BattleConstants::LANDING_MALUS);
        }

        if Self::is_melee_attacking_across_river_with_no_bridge(attacker, tile_to_attack_from, defender) {
            modifiers.add("Across river".to_string(), BattleConstants::ATTACKING_ACROSS_RIVER_MALUS);
        }
    }

    /// Checks if a melee unit is attacking across a river with no bridge
    fn is_melee_attacking_across_river_with_no_bridge(
        attacker: &MapUnitCombatant,
        tile_to_attack_from: &Tile,
        defender: &dyn ICombatant,
    ) -> bool {
        attacker.is_melee()
            && tile_to_attack_from.aerial_distance_to(defender.get_tile()) == 1
            && tile_to_attack_from.is_connected_by_river(defender.get_tile())
            && !attacker.unit.has_unique(UniqueType::AttackAcrossRiver)
            && (!tile_to_attack_from.has_connection(attacker.get_civ_info())
                || !defender.get_tile().has_connection(attacker.get_civ_info())
                || !attacker.get_civ_info().tech.roads_connect_across_rivers)
    }

    /// Gets air sweep attack modifiers
    pub fn get_air_sweep_attack_modifiers(attacker: &dyn ICombatant) -> Counter<String> {
        let mut modifiers = Counter::new();

        if let Some(map_unit_combatant) = attacker.as_any().downcast_ref::<MapUnitCombatant>() {
            for unique in map_unit_combatant.unit.get_matching_uniques(UniqueType::StrengthWhenAirsweep) {
                modifiers.add(Self::get_modifier_string_from_unique(unique), unique.params[0].parse::<i32>().unwrap_or(0));
            }
        }

        modifiers
    }

    /// Gets defense modifiers for a combatant
    pub fn get_defence_modifiers(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
    ) -> Counter<String> {
        let mut modifiers = Self::get_general_modifiers(defender, attacker, CombatAction::Defend, tile_to_attack_from);
        let tile = defender.get_tile();

        if let Some(map_unit_combatant) = defender.as_any().downcast_ref::<MapUnitCombatant>() {
            if !map_unit_combatant.unit.is_embarked() { // Embarked units get no terrain defensive bonuses
                let tile_defence_bonus = tile.get_defensive_bonus(&map_unit_combatant.unit);

                if (!map_unit_combatant.unit.has_unique(UniqueType::NoDefensiveTerrainBonus, true) && tile_defence_bonus > 0
                    || !map_unit_combatant.unit.has_unique(UniqueType::NoDefensiveTerrainPenalty, true) && tile_defence_bonus < 0) {
                    modifiers.add("Tile".to_string(), (tile_defence_bonus * 100.0) as i32);
                }

                if map_unit_combatant.unit.is_fortified() || map_unit_combatant.unit.is_guarding() {
                    modifiers.add(
                        "Fortification".to_string(),
                        BattleConstants::FORTIFICATION_BONUS * map_unit_combatant.unit.get_fortification_turns()
                    );
                }
            }
        }

        modifiers
    }

    /// Converts modifiers to a final bonus value
    fn modifiers_to_final_bonus(modifiers: &Counter<String>) -> f32 {
        let mut final_modifier = 1.0;
        for modifier_value in modifiers.values() {
            final_modifier += *modifier_value as f32 / 100.0;
        }
        final_modifier
    }

    /// Gets the health-dependent damage ratio for a combatant
    fn get_health_dependant_damage_ratio(combatant: &dyn ICombatant) -> f32 {
        if !combatant.as_any().downcast_ref::<MapUnitCombatant>().is_some()
            || combatant.as_any().downcast_ref::<MapUnitCombatant>()
                .map_or(false, |c| c.unit.has_unique(UniqueType::NoDamagePenaltyWoundedUnits, true)) {
            return 1.0;
        }
        // Each 3 points of health reduces damage dealt by 1%
        1.0 - (100.0 - combatant.get_health() as f32) / BattleConstants::DAMAGE_REDUCTION_WOUNDED_UNIT_RATIO_PERCENTAGE
    }

    /// Gets the attacking strength including modifiers
    pub fn get_attacking_strength(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
    ) -> f32 {
        let attack_modifier = Self::modifiers_to_final_bonus(&Self::get_attack_modifiers(attacker, defender, tile_to_attack_from));
        f32::max(1.0, attacker.get_attacking_strength() * attack_modifier)
    }

    /// Gets the defending strength including modifiers
    pub fn get_defending_strength(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
    ) -> f32 {
        let defence_modifier = Self::modifiers_to_final_bonus(&Self::get_defence_modifiers(attacker, defender, tile_to_attack_from));
        f32::max(1.0, defender.get_defending_strength(attacker.is_ranged()) * defence_modifier)
    }

    /// Calculates damage to the attacker
    pub fn calculate_damage_to_attacker(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
        randomness_factor: f32,
    ) -> i32 {
        if attacker.is_ranged() && !attacker.is_air_unit() {
            return 0;
        }
        if defender.is_civilian() {
            return 0;
        }

        let ratio = Self::get_attacking_strength(attacker, defender, tile_to_attack_from)
            / Self::get_defending_strength(attacker, defender, tile_to_attack_from);

        (Self::damage_modifier(ratio, true, randomness_factor) * Self::get_health_dependant_damage_ratio(defender)) as i32
    }

    /// Calculates damage to the defender
    pub fn calculate_damage_to_defender(
        attacker: &dyn ICombatant,
        defender: &dyn ICombatant,
        tile_to_attack_from: &Tile,
        randomness_factor: f32,
    ) -> i32 {
        if defender.is_civilian() {
            return BattleConstants::DAMAGE_TO_CIVILIAN_UNIT;
        }

        let ratio = Self::get_attacking_strength(attacker, defender, tile_to_attack_from)
            / Self::get_defending_strength(attacker, defender, tile_to_attack_from);

        (Self::damage_modifier(ratio, false, randomness_factor) * Self::get_health_dependant_damage_ratio(attacker)) as i32
    }

    /// Calculates the damage modifier based on the ratio of strengths
    fn damage_modifier(
        attacker_to_defender_ratio: f32,
        damage_to_attacker: bool,
        randomness_factor: f32,
    ) -> f32 {
        // https://forums.civfanatics.com/threads/getting-the-combat-damage-math.646582/#post-15468029
        let stronger_to_weaker_ratio = if attacker_to_defender_ratio < 1.0 {
            attacker_to_defender_ratio.powf(-1.0)
        } else {
            attacker_to_defender_ratio
        };

        let mut ratio_modifier = (((stronger_to_weaker_ratio + 3.0) / 4.0).powf(4.0) + 1.0) / 2.0;

        if (damage_to_attacker && attacker_to_defender_ratio > 1.0)
            || (!damage_to_attacker && attacker_to_defender_ratio < 1.0) { // damage ratio from the weaker party is inverted
            ratio_modifier = ratio_modifier.powf(-1.0);
        }

        let random_centered_around_30 = 24.0 + 12.0 * randomness_factor;
        random_centered_around_30 * ratio_modifier
    }
}