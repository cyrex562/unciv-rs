use rand::Rng;

pub struct AirInterception;

impl AirInterception {
    /// Should draw an Interception if available on the tile from any Civ
    /// Land Units deal 0 damage, and no XP for either party
    /// Air Interceptors do Air Combat as if Melee (mutual damage) but using Ranged Strength. 5XP to both
    /// But does not use the Interception mechanic bonuses/promotions
    /// Counts as an Attack for both units
    /// Will always draw out an Interceptor's attack (they cannot miss)
    /// This means the combat against Air Units will execute and always deal damage
    /// Random Civ at War will Intercept, prioritizing Air Units,
    /// sorted by highest Intercept chance (same as regular Intercept)
    pub fn air_sweep(attacker: &mut MapUnitCombatant, attacked_tile: &Tile) {
        // Air Sweep counts as an attack, even if nothing else happens
        attacker.unit.attacks_this_turn += 1;

        // copied and modified from reduceAttackerMovementPointsAndAttacks()
        // use up movement
        if attacker.unit.has_unique(UniqueType::CanMoveAfterAttacking) ||
           attacker.unit.max_attacks_per_turn() > attacker.unit.attacks_this_turn {
            // if it was a melee attack and we won, then the unit ALREADY got movement points deducted,
            // for the movement to the enemy's tile!
            // and if it's an air unit, it only has 1 movement anyway, so...
            if !attacker.unit.base_unit.moves_like_air_units {
                attacker.unit.use_movement_points(1.0);
            }
        } else {
            attacker.unit.current_movement = 0.0;
        }

        let attacker_name = attacker.get_name();

        // Make giant sequence of all potential Interceptors from all Civs isAtWarWith()
        let mut potential_interceptors = Vec::new();
        for intercepting_civ in attacker.get_civ_info().game_info.civilizations.iter()
            .filter(|c| attacker.get_civ_info().is_at_war_with(c)) {
            potential_interceptors.extend(
                intercepting_civ.units.get_civ_units()
                    .filter(|u| u.can_intercept(attacked_tile))
            );
        }

        // first priority, only Air Units
        if potential_interceptors.iter().any(|u| u.base_unit.is_air_unit()) {
            potential_interceptors.retain(|u| u.base_unit.is_air_unit());
        }

        // Pick highest chance interceptor
        for interceptor in potential_interceptors.iter()
            .collect::<Vec<_>>()
            .shuffle(&mut rand::thread_rng())
            .into_iter()
            .sorted_by(|a, b| b.intercept_chance().partial_cmp(&a.intercept_chance()).unwrap_or(std::cmp::Ordering::Equal)) {

            // No chance of Interceptor to miss (unlike regular Interception). Always want to deal damage
            // pairs of LocationAction for Notification
            let locations = LocationAction::new(
                interceptor.current_tile.position,
                attacker.unit.current_tile.position
            );

            interceptor.attacks_this_turn += 1;  // even if you miss, you took the shot

            if !interceptor.base_unit.is_air_unit() {
                let interceptor_name = interceptor.name.clone();
                // Deal no damage (moddable in future?) and no XP
                let attacker_text = format!(
                    "Our [{}] ([-0] HP) was attacked by an intercepting [{}] ([-0] HP)",
                    attacker_name, interceptor_name
                );
                let interceptor_text = format!(
                    "Our [{}] ([-0] HP) intercepted and attacked an enemy [{}] ([-0] HP)",
                    interceptor_name, attacker_name
                );

                attacker.get_civ_info().add_notification(
                    &attacker_text,
                    &locations,
                    NotificationCategory::War,
                    &attacker_name,
                    NotificationIcon::War,
                    &interceptor_name
                );

                interceptor.civ.add_notification(
                    &interceptor_text,
                    &locations,
                    NotificationCategory::War,
                    &interceptor_name,
                    NotificationIcon::War,
                    &attacker_name
                );

                attacker.unit.action = None;
                return;
            }

            // Damage if Air v Air should work similar to Melee
            let damage_dealt = Battle::take_damage(attacker, MapUnitCombatant::new(interceptor));

            // 5 XP to both
            Battle::add_xp(MapUnitCombatant::new(interceptor), 5, attacker);
            Battle::add_xp(attacker, 5, MapUnitCombatant::new(interceptor));

            let locations_interceptor_unknown = LocationAction::new(
                attacked_tile.position,
                attacker.unit.current_tile.position
            );

            Self::add_air_sweep_interception_notifications(
                attacker,
                interceptor,
                damage_dealt,
                &locations_interceptor_unknown,
                &locations
            );

            attacker.unit.action = None;
            return;
        }

        // No Interceptions available
        let attacker_text = format!("Nothing tried to intercept our [{}]", attacker_name);
        attacker.get_civ_info().add_notification(
            &attacker_text,
            NotificationCategory::War,
            &attacker_name
        );

        attacker.unit.action = None;
    }

    // TODO: Check overlap with addInterceptionNotifications, and unify what we can
    fn add_air_sweep_interception_notifications(
        attacker: &MapUnitCombatant,
        interceptor: &MapUnit,
        damage_dealt: DamageDealt,
        locations_interceptor_unknown: &LocationAction,
        locations: &LocationAction
    ) {
        let attacker_name = attacker.get_name();
        let interceptor_name = interceptor.name.clone();

        let attacker_text = if attacker.is_defeated() {
            if attacker.get_civ_info().viewable_tiles.contains(&interceptor.get_tile()) {
                format!(
                    "Our [{}] ([-{}] HP) was destroyed by an intercepting [{}] ([-{}] HP)",
                    attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
                )
            } else {
                format!(
                    "Our [{}] ([-{}] HP) was destroyed by an unknown interceptor",
                    attacker_name, damage_dealt.defender_dealt
                )
            }
        } else if MapUnitCombatant::new(interceptor).is_defeated() {
            format!(
                "Our [{}] ([-{}] HP) destroyed an intercepting [{}] ([-{}] HP)",
                attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
            )
        } else {
            format!(
                "Our [{}] ([-{}] HP) was attacked by an intercepting [{}] ([-{}] HP)",
                attacker_name, damage_dealt.defender_dealt, interceptor_name, damage_dealt.attacker_dealt
            )
        };

        attacker.get_civ_info().add_notification(
            &attacker_text,
            locations_interceptor_unknown,
            NotificationCategory::War,
            &attacker_name,
            NotificationIcon::War,
            NotificationIcon::Question
        );

        let interceptor_text = if attacker.is_defeated() {
            format!(
                "Our [{}] ([-{}] HP) intercepted and destroyed an enemy [{}] ([-{}] HP)",
                interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
            )
        } else if MapUnitCombatant::new(interceptor).is_defeated() {
            if interceptor.civ.viewable_tiles.contains(&attacker.get_tile()) {
                format!(
                    "Our [{}] ([-{}] HP) intercepted and was destroyed by an enemy [{}] ([-{}] HP)",
                    interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
                )
            } else {
                format!(
                    "Our [{}] ([-{}] HP) intercepted and was destroyed by an unknown enemy",
                    interceptor_name, damage_dealt.attacker_dealt
                )
            }
        } else {
            format!(
                "Our [{}] ([-{}] HP) intercepted and attacked an enemy [{}] ([-{}] HP)",
                interceptor_name, damage_dealt.attacker_dealt, attacker_name, damage_dealt.defender_dealt
            )
        };

        interceptor.civ.add_notification(
            &interceptor_text,
            locations,
            NotificationCategory::War,
            &interceptor_name,
            NotificationIcon::War,
            &attacker_name
        );
    }

    pub fn try_intercept_air_attack(
        attacker: &mut MapUnitCombatant,
        attacked_tile: &Tile,
        intercepting_civ: &Civilization,
        defender: Option<&dyn ICombatant>
    ) -> DamageDealt {
        if attacker.unit.has_unique(
            UniqueType::CannotBeIntercepted,
            StateForConditionals::new(
                attacker.get_civ_info(),
                attacker,
                defender,
                None,
                Some(attacked_tile)
            )
        ) {
            return DamageDealt::None;
        }

        // Pick highest chance interceptor
        let interceptor = intercepting_civ.units.get_civ_units()
            .filter(|u| u.can_intercept(attacked_tile))
            .sorted_by(|a, b| b.intercept_chance().partial_cmp(&a.intercept_chance()).unwrap_or(std::cmp::Ordering::Equal))
            .find(|unit| {
                // Can't intercept if we have a unique preventing it
                let conditional_state = StateForConditionals::new(
                    intercepting_civ,
                    MapUnitCombatant::new(unit),
                    attacker,
                    Some(CombatAction::Intercept),
                    Some(attacked_tile)
                );

                unit.get_matching_uniques(UniqueType::CannotInterceptUnits, conditional_state)
                    .iter()
                    .none(|u| attacker.matches_filter(&u.params[0]))
                    // Defender can't intercept either
                    && unit != defender.and_then(|d| d.downcast_ref::<MapUnitCombatant>()).map(|d| &d.unit).unwrap_or(&None)
            });

        let interceptor = match interceptor {
            Some(i) => i,
            None => return DamageDealt::None,
        };

        interceptor.attacks_this_turn += 1;  // even if you miss, you took the shot

        // Does Intercept happen? If not, exit
        if rand::thread_rng().gen::<f32>() > interceptor.intercept_chance() / 100.0 {
            return DamageDealt::None;
        }

        let mut damage = BattleDamage::calculate_damage_to_defender(
            MapUnitCombatant::new(interceptor),
            attacker
        );

        let mut damage_factor = 1.0 + interceptor.intercept_damage_percent_bonus() as f32 / 100.0;
        damage_factor *= attacker.unit.received_intercept_damage_factor();

        damage = (damage as f32 * damage_factor).min(attacker.unit.health as f32) as i32;

        attacker.take_damage(damage);
        if damage > 0 {
            Battle::add_xp(MapUnitCombatant::new(interceptor), 2, attacker);
        }

        Self::add_interception_notifications(attacker, interceptor, damage);

        DamageDealt::new(0, damage)
    }

    fn add_interception_notifications(
        attacker: &MapUnitCombatant,
        interceptor: &MapUnit,
        damage: i32
    ) {
        let attacker_name = attacker.get_name();
        let interceptor_name = interceptor.name.clone();

        let locations = LocationAction::new(
            interceptor.current_tile.position,
            attacker.unit.current_tile.position
        );

        let attacker_text = if !attacker.is_defeated() {
            format!(
                "Our [{}] ([-{}] HP) was attacked by an intercepting [{}] ([-0] HP)",
                attacker_name, damage, interceptor_name
            )
        } else if attacker.get_civ_info().viewable_tiles.contains(&interceptor.get_tile()) {
            format!(
                "Our [{}] ([-{}] HP) was destroyed by an intercepting [{}] ([-0] HP)",
                attacker_name, damage, interceptor_name
            )
        } else {
            format!(
                "Our [{}] ([-{}] HP) was destroyed by an unknown interceptor",
                attacker_name, damage
            )
        };

        attacker.get_civ_info().add_notification(
            &attacker_text,
            &interceptor.current_tile.position,
            NotificationCategory::War,
            &attacker_name,
            NotificationIcon::War,
            &interceptor_name
        );

        let interceptor_text = if attacker.is_defeated() {
            format!(
                "Our [{}] ([-0] HP) intercepted and destroyed an enemy [{}] ([-{}] HP)",
                interceptor_name, attacker_name, damage
            )
        } else {
            format!(
                "Our [{}] ([-0] HP) intercepted and attacked an enemy [{}] ([-{}] HP)",
                interceptor_name, attacker_name, damage
            )
        };

        interceptor.civ.add_notification(
            &interceptor_text,
            &locations,
            NotificationCategory::War,
            &interceptor_name,
            NotificationIcon::War,
            &attacker_name
        );
    }
}