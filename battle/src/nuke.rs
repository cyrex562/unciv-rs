use crate::battle::air_interception::AirInterception;
use crate::battle::battle::Battle;
use crate::battle::city_combatant::CityCombatant;
use crate::battle::map_unit_combatant::MapUnitCombatant;
use crate::civilization::Civilization;
use crate::civilization::civilopedia_action::CivilopediaAction;
use crate::civilization::location_action::LocationAction;
use crate::civilization::notification::Notification;
use crate::civilization::notification_category::NotificationCategory;
use crate::civilization::notification_icon::NotificationIcon;
use crate::diplomacy::modifiers::DiplomaticModifiers;
use crate::diplomacy::status::DiplomaticStatus;
use crate::map::tile::road_status::RoadStatus;
use crate::map::tile::Tile;
use crate::models::ruleset::unique::unique_type::UniqueType;
use crate::models::vector2::Vector2;
use std::collections::HashSet;
use std::sync::Arc;
use rand::Rng;

/// Module for nuclear weapon mechanics
pub mod Nuke {
    use super::*;

    /// Checks whether a nuke is allowed to nuke a target tile
    ///
    /// - Not if we would need to declare war on someone we can't.
    /// - Disallow nuking the tile the nuke is in, as per Civ5 (but not nuking your own tiles/units otherwise)
    ///
    /// Both BattleTable.simulateNuke and AirUnitAutomation.automateNukes check range, so that check is omitted here.
    pub fn may_use_nuke(nuke: &MapUnitCombatant, target_tile: &Tile) -> bool {
        if nuke.get_tile().as_ref() == target_tile {
            return false;
        }

        // Can only nuke visible Tiles
        if !target_tile.is_visible(nuke.get_civ_info().as_ref()) {
            return false;
        }

        let mut can_nuke = true;
        let attacker_civ = nuke.get_civ_info();

        let check_defender_civ = |defender_civ: Option<&Civilization>| {
            if let Some(defender_civ) = defender_civ {
                // Allow nuking yourself! (Civ5 source: CvUnit::isNukeVictim)
                if defender_civ == attacker_civ.as_ref() || defender_civ.is_defeated() {
                    return;
                }
                if defender_civ.is_barbarian {
                    return;
                }
                // Gleaned from Civ5 source - this disallows nuking unknown civs even in invisible tiles
                // https://github.com/Gedemon/Civ5-DLL/blob/master/CvGameCoreDLL_Expansion1/CvUnit.cpp#L5056
                // https://github.com/Gedemon/Civ5-DLL/blob/master/CvGameCoreDLL_Expansion1/CvTeam.cpp#L986
                if let Some(diplomacy_manager) = attacker_civ.get_diplomacy_manager(defender_civ) {
                    if diplomacy_manager.can_attack() {
                        return;
                    }
                }
                can_nuke = false;
            }
        };

        let blast_radius = nuke.unit.get_nuke_blast_radius();
        for tile in target_tile.get_tiles_in_distance(blast_radius) {
            check_defender_civ(tile.get_owner().as_ref());
            if let Some(combatant) = Battle::get_map_combatant_of_tile(&tile) {
                check_defender_civ(Some(combatant.get_civ_info().as_ref()));
            }
        }

        can_nuke
    }

    /// Detonates a nuclear weapon at the target tile
    pub fn nuke(attacker: &mut MapUnitCombatant, target_tile: &Tile) {
        let attacking_civ = attacker.get_civ_info();

        // Get nuke strength from the unit's uniques
        let nuke_strength = match attacker.unit.get_matching_uniques(UniqueType::NuclearWeapon)
            .first()
            .and_then(|unique| unique.params.get(0))
            .and_then(|param| param.parse::<i32>().ok()) {
                Some(strength) => strength,
                None => return,
            };

        // Get blast radius from the unit's uniques, default to 2 if not specified
        let blast_radius = attacker.unit.get_matching_uniques(UniqueType::BlastRadius)
            .first()
            .and_then(|unique| unique.params.get(0))
            .and_then(|param| param.parse::<i32>().ok())
            .unwrap_or(2);

        let hit_tiles = target_tile.get_tiles_in_distance(blast_radius);

        let (hit_civs_territory, notify_declared_war_civs) =
            declare_war_on_hit_civs(attacking_civ.as_ref(), &hit_tiles, attacker, target_tile);

        add_nuke_notifications(target_tile, attacker, &notify_declared_war_civs, attacking_civ.as_ref(), &hit_civs_territory);

        if attacker.is_defeated() {
            return;
        }

        attacker.unit.attacks_since_turn_start.push(Vector2::new(target_tile.position.x, target_tile.position.y));

        for tile in hit_tiles {
            // Handle complicated effects
            do_nuke_explosion_for_tile(attacker, &tile, nuke_strength, target_tile == &tile);
        }

        // Instead of postBattleAction() just destroy the unit, all other functions are not relevant
        if attacker.unit.has_unique(UniqueType::SelfDestructs) {
            attacker.unit.destroy();
        }

        // It's unclear whether using nukes results in a penalty with all civs, or only affected civs.
        // For now I'll make it give a diplomatic penalty to all known civs, but some testing for this would be appreciated
        for civ in attacking_civ.get_known_civs() {
            if let Some(diplomacy_manager) = civ.get_diplomacy_manager(attacking_civ.as_ref()) {
                diplomacy_manager.set_modifier(DiplomaticModifiers::UsedNuclearWeapons, -50.0);
            }
        }

        if !attacker.is_defeated() {
            attacker.unit.attacks_this_turn += 1;
        }
    }

    /// Adds notifications for all civilizations about the nuclear attack
    fn add_nuke_notifications(
        target_tile: &Tile,
        attacker: &MapUnitCombatant,
        notify_declared_war_civs: &[Arc<Civilization>],
        attacking_civ: &Civilization,
        hit_civs_territory: &[Arc<Civilization>]
    ) {
        let nuke_notification_action = vec![
            LocationAction::new(target_tile.position),
            CivilopediaAction::new(format!("Units/{}", attacker.get_name()))
        ];

        // If the nuke has been intercepted and destroyed then it fails to detonate
        if attacker.is_defeated() {
            // Notify attacker that they are now at war for the attempt
            for defending_civ in notify_declared_war_civs {
                attacking_civ.add_notification(
                    &format!("After an attempted attack by our [{}], [{}] has declared war on us!",
                        attacker.get_name(), defending_civ.civ_name),
                    nuke_notification_action.clone(),
                    NotificationCategory::Diplomacy,
                    defending_civ.civ_name.clone(),
                    NotificationIcon::War,
                    attacker.get_name()
                );
            }
            return;
        }

        // Notify attacker that they are now at war
        for defending_civ in notify_declared_war_civs {
            attacking_civ.add_notification(
                &format!("After being hit by our [{}], [{}] has declared war on us!",
                    attacker.get_name(), defending_civ.civ_name),
                nuke_notification_action.clone(),
                NotificationCategory::Diplomacy,
                defending_civ.civ_name.clone(),
                NotificationIcon::War,
                attacker.get_name()
            );
        }

        // Message all other civs
        for other_civ in attacking_civ.game_info.civilizations.iter() {
            if !other_civ.is_alive() || other_civ == attacking_civ {
                continue;
            }

            if hit_civs_territory.iter().any(|c| c == other_civ) {
                other_civ.add_notification(
                    &format!("A(n) [{}] from [{}] has exploded in our territory!",
                        attacker.get_name(), attacking_civ.civ_name),
                    nuke_notification_action.clone(),
                    NotificationCategory::War,
                    attacking_civ.civ_name.clone(),
                    NotificationIcon::War,
                    attacker.get_name()
                );
            } else if other_civ.knows(attacking_civ) {
                other_civ.add_notification(
                    &format!("A(n) [{}] has been detonated by [{}]!",
                        attacker.get_name(), attacking_civ.civ_name),
                    nuke_notification_action.clone(),
                    NotificationCategory::War,
                    attacking_civ.civ_name.clone(),
                    NotificationIcon::War,
                    attacker.get_name()
                );
            } else {
                other_civ.add_notification(
                    &format!("A(n) [{}] has been detonated by an unknown civilization!",
                        attacker.get_name()),
                    nuke_notification_action.clone(),
                    NotificationCategory::War,
                    NotificationIcon::War,
                    attacker.get_name()
                );
            }
        }
    }

    /// Declares war on civilizations affected by the nuclear attack
    fn declare_war_on_hit_civs(
        attacking_civ: &Civilization,
        hit_tiles: &[Arc<Tile>],
        attacker: &MapUnitCombatant,
        target_tile: &Tile
    ) -> (Vec<Arc<Civilization>>, Vec<Arc<Civilization>>) {
        // Declare war on the owners of all hit tiles
        let mut notify_declared_war_civs = Vec::new();

        let try_declare_war = |civ_suffered: &Civilization| {
            if civ_suffered != attacking_civ
                && civ_suffered.knows(attacking_civ)
                && civ_suffered.get_diplomacy_manager(attacking_civ)
                    .map_or(false, |dm| dm.diplomatic_status != DiplomaticStatus::War)
            {
                if let Some(diplomacy_manager) = attacking_civ.get_diplomacy_manager(civ_suffered) {
                    diplomacy_manager.declare_war();
                    if !notify_declared_war_civs.iter().any(|c| c == civ_suffered) {
                        notify_declared_war_civs.push(civ_suffered.clone());
                    }
                }
            }
        };

        let mut hit_civs_territory = Vec::new();
        let hit_civs: HashSet<_> = hit_tiles.iter()
            .filter_map(|tile| tile.get_owner().as_ref())
            .collect();

        for hit_civ in hit_civs {
            hit_civs_territory.push(hit_civ.clone());
            try_declare_war(hit_civ);
        }

        // Declare war on all potentially hit units. They'll try to intercept the nuke before it drops
        let units_in_hit_tiles: Vec<_> = hit_tiles.iter()
            .flat_map(|tile| tile.get_units())
            .collect();

        let civs_whose_unit_was_attacked: HashSet<_> = units_in_hit_tiles.iter()
            .map(|unit| unit.civ.clone())
            .filter(|civ| civ != attacking_civ)
            .collect();

        for civ_whose_unit_was_attacked in civs_whose_unit_was_attacked {
            try_declare_war(&civ_whose_unit_was_attacked);
            if attacker.unit.base_unit.is_air_unit() && !attacker.is_defeated() {
                AirInterception::try_intercept_air_attack(
                    attacker,
                    target_tile,
                    &civ_whose_unit_was_attacked,
                    None
                );
            }
        }

        (hit_civs_territory, notify_declared_war_civs)
    }

    /// Applies nuclear explosion effects to a tile
    fn do_nuke_explosion_for_tile(
        attacker: &MapUnitCombatant,
        tile: &Tile,
        nuke_strength: i32,
        is_ground_zero: bool
    ) {
        // https://forums.civfanatics.com/resources/unit-guide-modern-future-units-g-k.25628/
        // https://www.carlsguides.com/strategy/civilization5/units/aircraft-nukes.ph
        // Testing done by Ravignir
        // original source code: GenerateNuclearExplosionDamage(), ApplyNuclearExplosionDamage()

        let mut damage_modifier_from_missing_resource = 1.0;
        let civ_resources = attacker.get_civ_info().get_civ_resources_by_name();
        for resource in attacker.unit.get_resource_requirements_per_turn().keys() {
            if civ_resources.get(resource).map_or(false, |&amount| amount < 0)
                && !attacker.get_civ_info().is_barbarian {
                damage_modifier_from_missing_resource *= 0.5; // I could not find a source for this number, but this felt about right
                // - Original Civ5 does *not* reduce damage from missing resource, from source inspection
            }
        }

        let mut building_modifier = 1.0;  // Strange, but in Civ5 a bunker mitigates damage to garrison, even if the city is destroyed by the nuke

        // Damage city and reduce its population
        if let Some(city) = tile.get_city() {
            if tile.position == city.location {
                building_modifier = get_aggregate_modifier(&city, UniqueType::GarrisonDamageFromNukes);
                do_nuke_explosion_damage_to_city(&city, nuke_strength, damage_modifier_from_missing_resource);
                Battle::post_battle_notifications(attacker, CityCombatant::new(city.clone()), city.get_center_tile());
                Battle::destroy_if_defeated(city.civ.clone(), attacker.get_civ_info().clone(), city.location);
            }
        }

        // Damage and/or destroy units on the tile
        let units_on_tile: Vec<_> = tile.get_units().iter().cloned().collect(); // toList so if it's destroyed there's no concurrent modification
        for unit in units_on_tile {
            let damage = match nuke_strength {
                _ if is_ground_zero || nuke_strength >= 2 => 100,
                // The following constants are NUKE_UNIT_DAMAGE_BASE / NUKE_UNIT_DAMAGE_RAND_1 / NUKE_UNIT_DAMAGE_RAND_2 in Civ5
                1 => 30 + rand::thread_rng().gen_range(0..40) + rand::thread_rng().gen_range(0..40),
                // Level 0 does not exist in Civ5 (it treats units same as level 2)
                _ => 20 + rand::thread_rng().gen_range(0..30),
            };

            let damage = (damage as f32 * building_modifier * damage_modifier_from_missing_resource + f32::EPSILON) as i32;
            let mut defender = MapUnitCombatant::new(unit.clone());

            if unit.is_civilian() {
                if unit.health - damage <= 40 {
                    unit.destroy();  // Civ5: NUKE_NON_COMBAT_DEATH_THRESHOLD = 60
                }
            } else {
                defender.take_damage(damage);
            }

            Battle::post_battle_notifications(attacker, defender, defender.get_tile());
            Battle::destroy_if_defeated(defender.get_civ_info().clone(), attacker.get_civ_info().clone());
        }

        // Pillage improvements, pillage roads, add fallout
        if tile.is_city_center() {
            return;  // Never touch city centers - if they survived
        }

        if tile.terrain_has_unique(UniqueType::DestroyableByNukesChance) {
            // Note: Safe from concurrent modification exceptions only because removeTerrainFeature
            // *replaces* terrainFeatureObjects and the loop will continue on the old one
            let terrain_features: Vec<_> = tile.terrain_feature_objects.iter().cloned().collect();
            for terrain_feature in terrain_features {
                for unique in terrain_feature.get_matching_uniques(UniqueType::DestroyableByNukesChance) {
                    let chance = unique.params[0].parse::<f32>().unwrap_or(0.0) / 100.0;
                    if !(chance > 0.0 && is_ground_zero) && rand::thread_rng().gen::<f32>() >= chance {
                        continue;
                    }
                    tile.remove_terrain_feature(&terrain_feature.name);
                    apply_pillage_and_fallout(tile);
                }
            }
        } else if is_ground_zero || rand::thread_rng().gen::<f32>() < 0.5 {  // Civ5: NUKE_FALLOUT_PROB
            apply_pillage_and_fallout(tile);
        }
    }

    /// Applies pillaging and fallout effects to a tile
    fn apply_pillage_and_fallout(tile: &mut Tile) {
        if let Some(improvement) = tile.get_unpillaged_improvement() {
            if !improvement.has_unique(UniqueType::Irremovable) {
                if improvement.has_unique(UniqueType::Unpillagable) {
                    tile.remove_improvement();
                } else {
                    tile.set_pillaged();
                }
            }
        }

        if tile.get_unpillaged_road() != RoadStatus::None {
            tile.set_pillaged();
        }

        if tile.is_water || tile.is_impassible() || tile.terrain_features.contains("Fallout") {
            return;
        }

        tile.add_terrain_feature("Fallout");
    }

    /// Applies damage to a city from a nuclear explosion
    fn do_nuke_explosion_damage_to_city(targeted_city: &City, nuke_strength: i32, damage_modifier_from_missing_resource: f32) {
        // Original Capitals must be protected, `canBeDestroyed` is responsible for that check.
        // The `justCaptured = true` parameter is what allows other Capitals to suffer normally.
        if (nuke_strength > 2 || (nuke_strength > 1 && targeted_city.population.population < 5))
            && targeted_city.can_be_destroyed(true) {
            targeted_city.destroy_city();
            return;
        }

        let mut city_combatant = CityCombatant::new(targeted_city.clone());
        city_combatant.take_damage((city_combatant.get_health() as f32 * 0.5 * damage_modifier_from_missing_resource) as i32);

        // Difference to original: Civ5 rounds population loss down twice - before and after bomb shelters
        let population_loss = match nuke_strength {
            0 => 0.0,
            1 => (30.0 + rand::thread_rng().gen_range(0..20) as f32 + rand::thread_rng().gen_range(0..20) as f32) / 100.0,
            2 => (60.0 + rand::thread_rng().gen_range(0..10) as f32 + rand::thread_rng().gen_range(0..10) as f32) / 100.0,
            _ => 1.0,  // hypothetical nukeStrength 3 -> always to 1 pop
        };

        let population_loss = (targeted_city.population.population as f32 *
            get_aggregate_modifier(targeted_city, UniqueType::PopulationLossFromNukes) *
            population_loss) as i32;

        targeted_city.population.add_population(-population_loss);
    }

    /// Gets the aggregate modifier for a city for a specific unique type
    fn get_aggregate_modifier(city: &City, unique_type: UniqueType) -> f32 {
        let mut modifier = 1.0;
        for unique in city.get_matching_uniques(unique_type) {
            if !city.matches_filter(&unique.params[1]) {
                continue;
            }
            modifier *= unique.params[0].parse::<f32>().unwrap_or(0.0) / 100.0;
        }
        modifier
    }
}