use crate::automation::civilization::next_turn_automation::NextTurnAutomation;

/// Contains logic for automating unit movement towards enemy cities
pub struct HeadTowardsEnemyCityAutomation;

impl HeadTowardsEnemyCityAutomation {
    const MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA: i32 = 5;
    const MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA: i32 = 3;

    /// Attempts to move a unit towards an enemy city
    /// Returns whether the unit has taken this action
    pub fn try_head_towards_enemy_city(unit: &mut MapUnit) -> bool {
        if unit.civ.cities.is_empty() {
            return false;
        }

        // Only focus on *attacking* 1 enemy at a time otherwise you'll lose on both fronts
        let closest_reachable_enemy_city = Self::get_enemy_cities_by_priority(unit)
            .into_iter()
            .find(|city| unit.movement.can_reach(city.get_center_tile()))
            .map(|city| city.get_center_tile());

        let closest_reachable_enemy_city = match closest_reachable_enemy_city {
            Some(city) => city,
            None => return false, // No enemy city reachable
        };

        Self::head_towards_enemy_city(
            unit,
            closest_reachable_enemy_city,
            // This should be cached after the `can_reach` call above
            unit.movement.get_shortest_path(closest_reachable_enemy_city)
        )
    }

    /// Gets enemy cities sorted by priority for attack
    fn get_enemy_cities_by_priority(unit: &MapUnit) -> Vec<&City> {
        let enemies: Vec<&Civilization> = unit.civ.get_known_civs()
            .into_iter()
            .filter(|other_civ| unit.civ.is_at_war_with(other_civ) && !other_civ.cities.is_empty())
            .collect();

        let closest_enemy_city = enemies.iter()
            .flat_map(|enemy| NextTurnAutomation::get_closest_cities(unit.civ, enemy))
            .min_by_key(|pair| pair.aerial_distance)
            .map(|pair| pair.city2);

        let closest_enemy_city = match closest_enemy_city {
            Some(city) => city,
            None => return Vec::new(), // No attackable cities found
        };

        // Our main attack target is the closest city, but we're fine with deviating from that a bit
        let mut enemy_cities_by_priority: Vec<&City> = closest_enemy_city.civ.cities.iter()
            .map(|city| (city, city.get_center_tile().aerial_distance_to(closest_enemy_city.get_center_tile())))
            .filter(|(_, distance)| *distance <= 10) // Anything 10 tiles away from the target is irrelevant
            .sorted_by_key(|(_, distance)| *distance)
            .map(|(city, _)| city)
            .collect();

        if unit.base_unit.is_ranged() {
            // Ranged units don't harm capturable cities, waste of a turn
            enemy_cities_by_priority.retain(|city| city.health > 1);
        }

        enemy_cities_by_priority
    }

    /// Moves a unit towards an enemy city
    /// Returns whether the unit has taken this action
    pub fn head_towards_enemy_city(
        unit: &mut MapUnit,
        closest_reachable_enemy_city: &Tile,
        shortest_path: Vec<&Tile>
    ) -> bool {
        let unit_distance_to_tiles = unit.movement.get_distance_to_tiles();
        let unit_range = unit.get_range();

        if unit_range > 2 {
            // Long-ranged unit, should never be in a bombardable position
            return Self::head_towards_enemy_city_long_range(
                closest_reachable_enemy_city,
                unit_distance_to_tiles,
                unit_range,
                unit
            );
        }

        let next_tile_in_path = &shortest_path[0];

        // None of the stuff below is relevant if we're still quite far away from the city, so we
        // short-circuit here for performance reasons.
        if unit.current_tile.aerial_distance_to(closest_reachable_enemy_city) > Self::MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA
            && shortest_path.len() > Self::MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA as usize {
            unit.movement.move_to_tile(next_tile_in_path);
            return true;
        }

        let our_units_around_enemy_city: Vec<&MapUnit> = closest_reachable_enemy_city.get_tiles_in_distance(6)
            .iter()
            .flat_map(|tile| tile.get_units())
            .filter(|unit| unit.is_military() && unit.civ == unit.civ)
            .collect();

        let city = closest_reachable_enemy_city.get_city().unwrap();

        if Self::cannot_take_city_soon(&our_units_around_enemy_city, city) {
            return Self::head_to_landing_grounds(closest_reachable_enemy_city, unit);
        }

        unit.movement.move_to_tile(next_tile_in_path); // Go for it!
        true
    }

    /// Checks if we cannot take the city within 5 turns
    fn cannot_take_city_soon(
        our_units_around_enemy_city: &[&MapUnit],
        city: &City
    ) -> bool {
        let city_combatant = CityCombatant::new(city);
        let expected_damage_per_turn: i32 = our_units_around_enemy_city.iter()
            .map(|unit| BattleDamage::calculate_damage_to_defender(
                MapUnitCombatant::new(unit),
                &city_combatant
            ))
            .sum();

        let city_healing_per_turn = 20;
        expected_damage_per_turn < city.health && // Cannot take immediately
            (expected_damage_per_turn <= city_healing_per_turn || // No lasting damage
             city.health / (expected_damage_per_turn - city_healing_per_turn) > 5) // Can damage, but will take more than 5 turns
    }

    /// Moves a unit to landing grounds near an enemy city
    fn head_to_landing_grounds(closest_reachable_enemy_city: &Tile, unit: &mut MapUnit) -> bool {
        // Don't head straight to the city, try to head to landing grounds -
        // This is against the AI's brilliant plan of having everyone embarked and attacking via sea when unnecessary.
        let tile_to_head_to = closest_reachable_enemy_city.get_tiles_in_distance_range(
            Self::MIN_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA..=Self::MAX_DISTANCE_FROM_CITY_TO_CONSIDER_FOR_LANDING_AREA
        )
        .iter()
        .filter(|tile| tile.is_land && unit.get_damage_from_terrain(tile) <= 0) // Don't head for hurty terrain
        .sorted_by_key(|tile| tile.aerial_distance_to(unit.current_tile))
        .find(|tile| unit.movement.can_move_to(tile) || *tile == unit.current_tile);

        if let Some(tile) = tile_to_head_to {
            unit.movement.head_towards(tile);
        }
        true
    }

    /// Moves a long-range unit towards an enemy city
    fn head_towards_enemy_city_long_range(
        closest_reachable_enemy_city: &Tile,
        unit_distance_to_tiles: &HashMap<&Tile, PathToTile>,
        unit_range: i32,
        unit: &mut MapUnit
    ) -> bool {
        let tiles_in_bombard_range: HashSet<&Tile> = closest_reachable_enemy_city.get_tiles_in_distance(2)
            .iter()
            .collect();

        let tile_to_move_to = unit_distance_to_tiles.iter()
            .filter(|(tile, _)| {
                tile.aerial_distance_to(closest_reachable_enemy_city) <= unit_range &&
                !tiles_in_bombard_range.contains(tile) &&
                unit.get_damage_from_terrain(tile) <= 0 // Don't set up on a mountain
            })
            .min_by_key(|(_, path)| path.total_movement)
            .map(|(tile, _)| tile);

        match tile_to_move_to {
            Some(tile) => {
                // Move into position far away enough that the bombard doesn't hurt
                unit.movement.head_towards(tile);
                true
            }
            None => false
        }
    }
}