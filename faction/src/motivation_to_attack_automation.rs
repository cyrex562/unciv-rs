use std::collections::HashMap;
use std::cmp::Ordering;

use crate::civilization::civilization::Civilization;
use crate::map::bfs::BFS;
use crate::tile::tile::Tile;
use crate::unique::unique_type::UniqueType;

/// Contains logic for evaluating the motivation of a civilization to attack another civilization.
pub struct MotivationToAttackAutomation;

impl MotivationToAttackAutomation {
    /// Will return the motivation to attack, but might short circuit if the value is guaranteed to
    /// be lower than `at_least`. So any values below `at_least` should not be used for comparison.
    pub fn has_at_least_motivation_to_attack(civ_info: &Civilization, target_civ: &Civilization, at_least: f32) -> f32 {
        let diplomacy_manager = civ_info.get_diplomacy_manager(target_civ).unwrap();
        let personality = civ_info.get_personality();

        let target_cities_with_our_city: Vec<_> = civ_info.threat_manager.get_neighboring_cities_of_other_civs()
            .iter()
            .filter(|(_, city)| city.civ == target_civ)
            .collect();

        let target_cities: Vec<_> = target_cities_with_our_city.iter()
            .map(|(_, city)| city)
            .collect();

        if target_cities_with_our_city.is_empty() {
            return 0.0;
        }

        if target_cities.iter().all(|city| Self::has_no_units_that_can_attack_city_without_dying(civ_info, city)) {
            return 0.0;
        }

        let base_force = 100.0;

        let our_combat_strength = Self::calculate_self_combat_strength(civ_info, base_force);
        let their_combat_strength = Self::calculate_combat_strength_with_protectors(target_civ, base_force, civ_info);

        let mut modifiers: Vec<(String, f32)> = Vec::new();

        // If our personality is to declare war more then we should have a higher base motivation (a negative number closer to 0)
        modifiers.push(("Base motivation".to_string(),
            -(15.0 * personality.inverse_modifier_focus(PersonalityValue::DeclareWar, 0.5))));

        modifiers.push(("Relative combat strength".to_string(),
            Self::get_combat_strength_modifier(civ_info, target_civ, our_combat_strength,
                their_combat_strength + 0.8 * civ_info.threat_manager.get_combined_force_of_warring_civs())));

        // TODO: For now this will be a very high value because the AI can't handle multiple fronts, this should be changed later though
        modifiers.push(("Concurrent wars".to_string(),
            -(civ_info.get_civs_at_war_with().iter().filter(|civ| civ.is_major_civ() && civ != target_civ).count() as f32 * 20.0)));

        modifiers.push(("Their concurrent wars".to_string(),
            target_civ.get_civs_at_war_with().iter().filter(|civ| civ.is_major_civ()).count() as f32 * 3.0));

        modifiers.push(("Their allies".to_string(),
            Self::get_defensive_pact_allies_score(target_civ, civ_info, base_force, our_combat_strength)));

        if civ_info.threat_manager.get_neighboring_civilizations().iter().none(|civ|
                civ != target_civ && civ.is_major_civ() &&
                civ_info.get_diplomacy_manager(civ).unwrap().is_relationship_level_lt(RelationshipLevel::Friend)) {
            modifiers.push(("No other threats".to_string(), 10.0));
        }

        if target_civ.is_major_civ() {
            let score_ratio_modifier = Self::get_score_ratio_modifier(target_civ, civ_info);
            modifiers.push(("Relative score".to_string(), score_ratio_modifier));

            modifiers.push(("Relative technologies".to_string(),
                Self::get_relative_tech_modifier(civ_info, target_civ)));

            if civ_info.stats.get_unit_supply_deficit() != 0 {
                modifiers.push(("Over unit supply".to_string(),
                    (civ_info.stats.get_unit_supply_deficit() as f32 * 2.0).min(20.0)));
            } else if target_civ.stats.get_unit_supply_deficit() == 0 && !target_civ.is_city_state {
                modifiers.push(("Relative production".to_string(),
                    Self::get_production_ratio_modifier(civ_info, target_civ)));
            }
        }

        let min_target_city_distance = target_cities_with_our_city.iter()
            .map(|(our_city, their_city)| their_city.get_center_tile().aerial_distance_to(our_city.get_center_tile()))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0);

        // Defensive civs should avoid fighting civilizations that are farther away and don't pose a threat
        let distance_modifier = match min_target_city_distance {
            d if d > 20 => -10.0,
            d if d > 14 => -8.0,
            d if d > 10 => -3.0,
            _ => 0.0,
        } * personality.inverse_modifier_focus(PersonalityValue::Aggressive, 0.2);

        modifiers.push(("Far away cities".to_string(), distance_modifier));

        // Defensive civs want to deal with potential nearby cities to protect themselves
        if min_target_city_distance < 6 {
            modifiers.push(("Close cities".to_string(),
                5.0 * personality.inverse_modifier_focus(PersonalityValue::Aggressive, 1.0)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::ResearchAgreement) {
            modifiers.push(("Research Agreement".to_string(),
                -5.0 * personality.scaled_focus(PersonalityValue::Science) * personality.scaled_focus(PersonalityValue::Commerce)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
            modifiers.push(("Declaration of Friendship".to_string(),
                -10.0 * personality.modifier_focus(PersonalityValue::Loyal, 0.5)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::DefensivePact) {
            modifiers.push(("Defensive Pact".to_string(),
                -15.0 * personality.modifier_focus(PersonalityValue::Loyal, 0.3)));
        }

        modifiers.push(("Relationship".to_string(),
            Self::get_relationship_modifier(diplomacy_manager)));

        if diplomacy_manager.has_flag(DiplomacyFlags::Denunciation) {
            modifiers.push(("Denunciation".to_string(),
                5.0 * personality.inverse_modifier_focus(PersonalityValue::Diplomacy, 0.5)));
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::WaryOf) && diplomacy_manager.get_flag(DiplomacyFlags::WaryOf) < 0 {
            // Completely defensive civs will plan defensively and have a 0 here
            modifiers.push(("PlanningAttack".to_string(),
                -diplomacy_manager.get_flag(DiplomacyFlags::WaryOf) as f32 * personality.scaled_focus(PersonalityValue::Aggressive) / 2.0));
        } else {
            let attacks_planned = civ_info.diplomacy.values().iter()
                .filter(|manager| manager.has_flag(DiplomacyFlags::WaryOf) && manager.get_flag(DiplomacyFlags::WaryOf) < 0)
                .count();
            modifiers.push(("PlanningAttackAgainstOtherCivs".to_string(),
                -(attacks_planned as f32 * 5.0 * personality.inverse_modifier_focus(PersonalityValue::Aggressive, 0.5))));
        }

        if diplomacy_manager.resources_from_trade().iter().any(|resource| resource.amount > 0) {
            modifiers.push(("Receiving trade resources".to_string(),
                -8.0 * personality.modifier_focus(PersonalityValue::Commerce, 0.5)));
        }

        // If their cities don't have any nearby cities that are also targets to us and it doesn't include their capital
        // Then there cities are likely isolated and a good target.
        if !target_cities.contains(&target_civ.get_capital())
            && target_cities.iter().all(|their_city|
                !their_city.neighboring_cities.iter().any(|city| !target_cities.contains(city))) {
            modifiers.push(("Isolated city".to_string(),
                10.0 * personality.modifier_focus(PersonalityValue::Aggressive, 0.8)));
        }
        if target_civ.is_city_state {
            modifiers.push(("Protectors".to_string(),
                -(target_civ.city_state_functions.get_protector_civs().len() as f32 * 3.0)));
            if target_civ.city_state_functions.get_protector_civs().contains(&civ_info) {
                modifiers.push(("Under our protection".to_string(),
                    -15.0 * personality.modifier_focus(PersonalityValue::Diplomacy, 0.8)));
            }
            if target_civ.get_ally_civ() == Some(civ_info.civ_name.clone()) {
                modifiers.push(("Allied City-state".to_string(),
                    -20.0 * personality.modifier_focus(PersonalityValue::Diplomacy, 0.8))); // There had better be a DAMN good reason
            }
        }

        Self::add_wonder_based_motivations(target_civ, &mut modifiers);

        modifiers.push(("War with allies".to_string(),
            Self::get_allied_war_motivation(civ_info, target_civ)));

        // Purely for debugging, remove modifiers that don't have an effect
        modifiers.retain(|(_, value)| *value != 0.0);
        let mut motivation_so_far = modifiers.iter().map(|(_, value)| value).sum();

        // Short-circuit to avoid A-star
        if motivation_so_far < at_least {
            return motivation_so_far;
        }

        motivation_so_far += Self::get_attack_paths_modifier(civ_info, target_civ, &target_cities_with_our_city);

        motivation_so_far
    }

    fn calculate_combat_strength_with_protectors(other_civ: &Civilization, base_force: f32, civ_info: &Civilization) -> f32 {
        let mut their_combat_strength = Self::calculate_self_combat_strength(other_civ, base_force);

        // For city-states, also consider their protectors
        if other_civ.is_city_state && !other_civ.city_state_functions.get_protector_civs().is_empty() {
            their_combat_strength += other_civ.city_state_functions.get_protector_civs()
                .iter()
                .filter(|protector| protector != civ_info)
                .map(|protector| protector.get_stat_for_ranking(RankingType::Force) as f32)
                .sum::<f32>();
        }
        their_combat_strength
    }

    fn calculate_self_combat_strength(civ_info: &Civilization, base_force: f32) -> f32 {
        let mut our_combat_strength = civ_info.get_stat_for_ranking(RankingType::Force) as f32 + base_force;
        if let Some(capital) = civ_info.get_capital() {
            our_combat_strength += CityCombatant::new(capital).get_city_strength();
        }
        our_combat_strength
    }

    fn add_wonder_based_motivations(other_civ: &Civilization, modifiers: &mut Vec<(String, f32)>) {
        let mut wonder_count = 0;
        for city in &other_civ.cities {
            let construction = city.city_constructions.get_current_construction();
            if let Some(building) = construction.downcast_ref::<Building>() {
                if building.has_unique(UniqueType::TriggersCulturalVictory) {
                    modifiers.push(("About to win".to_string(), 15.0));
                }
            }
            if let Some(unit) = construction.downcast_ref::<BaseUnit>() {
                if unit.has_unique(UniqueType::AddInCapital) {
                    modifiers.push(("About to win".to_string(), 15.0));
                }
            }
            wonder_count += city.city_constructions.get_built_buildings().iter()
                .filter(|building| building.is_wonder)
                .count();
        }

        // The more wonders they have, the more beneficial it is to conquer them
        // Civs need an army to protect their wonders which give the most score
        if wonder_count > 0 {
            modifiers.push(("Owned Wonders".to_string(), wonder_count as f32));
        }
    }

    /// If they are at war with our allies, then we should join in
    fn get_allied_war_motivation(civ_info: &Civilization, other_civ: &Civilization) -> f32 {
        let mut allied_war_motivation = 0.0;
        for third_civ in civ_info.get_diplomacy_manager(other_civ).unwrap().get_common_known_civs() {
            let third_civ_diplo_manager = civ_info.get_diplomacy_manager(third_civ).unwrap();
            if third_civ_diplo_manager.is_relationship_level_lt(RelationshipLevel::Friend) {
                continue;
            }

            if third_civ.get_diplomacy_manager(other_civ).unwrap().has_flag(DiplomacyFlags::Denunciation) {
                allied_war_motivation += 2.0;
            }

            if third_civ.is_at_war_with(other_civ) {
                allied_war_motivation += if third_civ_diplo_manager.has_flag(DiplomacyFlags::DefensivePact) {
                    15.0
                } else if third_civ_diplo_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
                    5.0
                } else {
                    2.0
                };
            }
        }
        allied_war_motivation * civ_info.get_personality().modifier_focus(PersonalityValue::Loyal, 0.5)
    }

    fn get_relationship_modifier(diplomacy_manager: &DiplomacyManager) -> f32 {
        let relationship_modifier = match diplomacy_manager.relationship_ignore_afraid() {
            RelationshipLevel::Unforgivable => 10.0,
            RelationshipLevel::Enemy => 5.0,
            RelationshipLevel::Competitor => 2.0,
            RelationshipLevel::Favorable => -2.0,
            RelationshipLevel::Friend => -5.0,
            RelationshipLevel::Ally => -10.0, // this is so that ally + DoF is not too unbalanced -
            // still possible for AI to declare war for isolated city
            _ => 0.0,
        };
        relationship_modifier * diplomacy_manager.civ_info.get_personality().modifier_focus(PersonalityValue::Loyal, 0.3)
    }

    fn get_relative_tech_modifier(civ_info: &Civilization, other_civ: &Civilization) -> f32 {
        let relative_tech = civ_info.get_stat_for_ranking(RankingType::Technologies) -
                           other_civ.get_stat_for_ranking(RankingType::Technologies);
        match relative_tech {
            t if t > 6 => 10.0,
            t if t > 3 => 5.0,
            t if t > -3 => 0.0,
            t if t > -6 => -2.0,
            t if t > -9 => -5.0,
            _ => -10.0,
        }
    }

    fn get_production_ratio_modifier(civ_info: &Civilization, other_civ: &Civilization) -> f32 {
        // If either of our Civs are suffering from a supply deficit, our army must be too large
        // There is no easy way to check the raw production if a civ has a supply deficit
        // We might try to divide the current production by the getUnitSupplyProductionPenalty()
        // but it only is true for our turn and not the previous turn and might result in odd values

        let production_ratio = civ_info.get_stat_for_ranking(RankingType::Production) as f32 /
                              other_civ.get_stat_for_ranking(RankingType::Production) as f32;
        match production_ratio {
            r if r > 2.0 => 10.0,
            r if r > 1.5 => 5.0,
            r if r > 1.2 => 3.0,
            r if r > 0.8 => 0.0,
            r if r > 0.5 => -5.0,
            r if r > 0.25 => -10.0,
            _ => -15.0,
        }
    }

    fn get_score_ratio_modifier(other_civ: &Civilization, civ_info: &Civilization) -> f32 {
        // Civs with more score are more threatening to our victory
        // Bias towards attacking civs with a high score and low military
        // Bias against attacking civs with a low score and a high military
        // Designed to mitigate AIs declaring war on weaker civs instead of their rivals
        let score_ratio = other_civ.get_stat_for_ranking(RankingType::Score) as f32 /
                         civ_info.get_stat_for_ranking(RankingType::Score) as f32;
        let score_ratio_modifier = match score_ratio {
            r if r > 2.0 => 15.0,
            r if r > 1.5 => 10.0,
            r if r > 1.25 => 5.0,
            r if r > 1.0 => 2.0,
            r if r > 0.8 => 0.0,
            r if r > 0.5 => -2.0,
            r if r > 0.25 => -5.0,
            _ => -10.0,
        };
        score_ratio_modifier * civ_info.get_personality().modifier_focus(PersonalityValue::Culture, 0.3)
    }

    fn get_defensive_pact_allies_score(other_civ: &Civilization, civ_info: &Civilization, base_force: f32, our_combat_strength: f32) -> f32 {
        let mut their_allies_value = 0.0;
        for third_civ in other_civ.diplomacy.values().iter()
            .filter(|manager| manager.has_flag(DiplomacyFlags::DefensivePact) && manager.other_civ() != civ_info) {
            let third_civ_combat_strength_ratio = other_civ.get_stat_for_ranking(RankingType::Force) as f32 + base_force / our_combat_strength;
            their_allies_value += match third_civ_combat_strength_ratio {
                r if r > 5.0 => -15.0,
                r if r > 2.5 => -10.0,
                r if r > 2.0 => -8.0,
                r if r > 1.5 => -5.0,
                r if r > 0.8 => -2.0,
                _ => 0.0,
            };
        }
        their_allies_value
    }

    fn get_combat_strength_modifier(civ_info: &Civilization, target_civ: &Civilization, our_combat_strength: f32, their_combat_strength: f32) -> f32 {
        let mut combat_strength_ratio = our_combat_strength / their_combat_strength;

        // At higher difficulty levels the AI gets a unit production boost.
        // In that case while we may have more units than them, we don't necessarily want to be more aggressive.
        // This is to reduce the amount that the AI targets players at these higher levels somewhat.
        if civ_info.is_ai() && target_civ.is_human() && combat_strength_ratio > 1.0 {
            let our_combat_modifiers = civ_info.game_info.get_difficulty().ai_unit_cost_modifier;
            let their_combat_modifiers = civ_info.game_info.get_difficulty().unit_cost_modifier;
            combat_strength_ratio *= our_combat_modifiers / their_combat_modifiers;
        }
        match combat_strength_ratio {
            r if r > 5.0 => 20.0,
            r if r > 4.0 => 15.0,
            r if r > 3.0 => 12.0,
            r if r > 2.0 => 10.0,
            r if r > 1.8 => 8.0,
            r if r > 1.6 => 6.0,
            r if r > 1.4 => 4.0,
            r if r > 1.2 => 2.0,
            r if r > 0.8 => -5.0,
            r if r > 0.6 => -10.0,
            r if r > 0.4 => -20.0,
            _ => -40.0,
        }
    }

    fn has_no_units_that_can_attack_city_without_dying(civ_info: &Civilization, their_city: &City) -> bool {
        civ_info.units.get_civ_units().iter()
            .filter(|unit| unit.is_military())
            .none(|unit| {
                let damage_received_when_attacking = BattleDamage::calculate_damage_to_attacker(
                    MapUnitCombatant::new(unit),
                    CityCombatant::new(their_city)
                );
                damage_received_when_attacking < 100
            })
    }

    /// Checks the routes of attack against [other_civ] using [target_cities_with_our_city].
    ///
    /// The more routes of attack and shorter the path the higher a motivation will be returned.
    /// Sea attack routes are less valuable
    ///
    /// @return The motivation ranging from -30 to around +10
    fn get_attack_paths_modifier(civ_info: &Civilization, other_civ: &Civilization, target_cities_with_our_city: &[(&City, &City)]) -> f32 {
        fn is_tile_can_move_through(civ_info: &Civilization, tile: &Tile, other_civ: &Civilization) -> bool {
            let owner = tile.get_owner();
            !tile.is_impassible() &&
                (owner == Some(other_civ.civ_name.clone()) || owner.is_none() ||
                 civ_info.diplomacy_functions.can_pass_through_tiles(owner.unwrap()))
        }

        fn is_land_tile_can_move_through(civ_info: &Civilization, tile: &Tile, other_civ: &Civilization) -> bool {
            tile.is_land && is_tile_can_move_through(civ_info, tile, other_civ)
        }

        let mut attack_paths: Vec<Vec<Tile>> = Vec::new();
        let mut attack_path_modifiers: f32 = -3.0;

        // For each city, we want to calculate if there is an attack path to the enemy
        let grouped_by_city: HashMap<&City, Vec<(&City, &City)>> = target_cities_with_our_city.iter()
            .fold(HashMap::new(), |mut acc, (our_city, their_city)| {
                acc.entry(our_city).or_insert_with(Vec::new).push((our_city, their_city));
                acc
            });

        for (city_to_attack_from, attacks) in grouped_by_city {
            let mut city_attack_value = 0.0;

            // We only want to calculate the best attack path and use it's value
            // Land routes are clearly better than sea routes
            for (_, city_to_attack) in attacks {
                let land_attack_path = MapPathing::get_connection(
                    civ_info,
                    city_to_attack_from.get_center_tile(),
                    city_to_attack.get_center_tile(),
                    |tile| is_land_tile_can_move_through(civ_info, tile, other_civ)
                );

                if let Some(path) = land_attack_path {
                    if path.len() < 16 {
                        attack_paths.push(path);
                        city_attack_value = 3.0;
                        break;
                    }
                }

                if city_attack_value > 0.0 {
                    continue;
                }

                let land_and_sea_attack_path = MapPathing::get_connection(
                    civ_info,
                    city_to_attack_from.get_center_tile(),
                    city_to_attack.get_center_tile(),
                    |tile| is_tile_can_move_through(civ_info, tile, other_civ)
                );

                if let Some(path) = land_and_sea_attack_path {
                    if path.len() < 16 {
                        attack_paths.push(path);
                        city_attack_value += 1.0;
                    }
                }
            }
            attack_path_modifiers += city_attack_value;
        }

        if attack_paths.is_empty() {
            // Do an expensive BFS to find any possible attack path
            let reachable_enemy_cities_bfs = BFS::new(civ_info.get_capital().unwrap().get_center_tile(),
                |tile| is_tile_can_move_through(civ_info, tile, other_civ));

            reachable_enemy_cities_bfs.step_to_end();

            let reachable_enemy_cities: Vec<_> = other_civ.cities.iter()
                .filter(|city| reachable_enemy_cities_bfs.has_reached_tile(city.get_center_tile()))
                .collect();

            if reachable_enemy_cities.is_empty() {
                return -50.0; // Can't even reach the enemy city, no point in war.
            }

            let min_attack_distance = reachable_enemy_cities.iter()
                .map(|city| reachable_enemy_cities_bfs.get_path_to(city.get_center_tile()).len())
                .min()
                .unwrap_or(0);

            // Longer attack paths are worse, but if the attack path is too far away we shouldn't completely discard the possibility
            attack_path_modifiers -= (min_attack_distance as f32 - 10.0).max(0.0).min(30.0);
        }

        attack_path_modifiers
    }
}