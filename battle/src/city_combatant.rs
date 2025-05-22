use crate::battle::i_combatant::ICombatant;
use crate::city;

use std::f32;
use std::sync::Arc;

/// Represents a city as a combatant in battle
pub struct CityCombatant {
    /// The city this combatant represents
    pub city: Arc<City>,
}

impl CityCombatant {
    /// Creates a new CityCombatant for the given city
    pub fn new(city: Arc<City>) -> Self {
        CityCombatant { city }
    }

    /// Gets the city strength based on various factors
    ///
    /// # Arguments
    ///
    /// * `combat_action` - The type of combat action being performed
    ///
    /// # Returns
    ///
    /// The calculated city strength
    pub fn get_city_strength(&self, combat_action: CombatAction) -> i32 {
        // Civ fanatics forum, from a modder who went through the original code
        let mod_constants = self.get_civ_info().game_info.ruleset.mod_options.constants;
        let mut strength = mod_constants.city_strength_base;

        // Each 5 pop gives 2 defence
        strength += (self.city.population.population * mod_constants.city_strength_per_pop);

        let city_tile = self.city.get_center_tile();
        for unique in city_tile.all_terrains.iter()
            .flat_map(|t| t.get_matching_uniques(UniqueType::GrantsCityStrength)) {
            strength += unique.params[0].parse::<i32>().unwrap_or(0);
        }

        // as tech progresses so does city strength
        let tech_count = self.get_civ_info().game_info.ruleset.technologies.len();
        let techs_percent_known: f32 = if tech_count > 0 {
            self.city.civ.tech.techs_researched.len() as f32 / tech_count as f32
        } else {
            0.5 // for mods with no tech
        };

        strength += (techs_percent_known * mod_constants.city_strength_from_techs_multiplier)
            .powf(mod_constants.city_strength_from_techs_exponent)
            * mod_constants.city_strength_from_techs_full_multiplier;

        // The way all of this adds up...
        // All ancient techs - 0.5 extra, Classical - 2.7, Medieval - 8, Renaissance - 17.5,
        // Industrial - 32.4, Modern - 51, Atomic - 72.5, All - 118.3

        // Garrisoned unit gives up to 20% of strength to city, health-dependant
        if let Some(military_unit) = &city_tile.military_unit {
            strength += (military_unit.base_unit.strength as f32
                * (military_unit.health as f32 / 100.0)
                * mod_constants.city_strength_from_garrison) as i32;
        }

        let mut buildings_strength = self.city.get_strength();
        let state_for_conditionals = StateForConditionals::new(
            self.get_civ_info(),
            Some(self.city.clone()),
            Some(self.clone()),
            None,
            None,
            Some(combat_action),
        );

        for unique in self.get_civ_info().get_matching_uniques(UniqueType::BetterDefensiveBuildings, &state_for_conditionals) {
            buildings_strength *= unique.params[0].parse::<f32>().unwrap_or(1.0) / 100.0;
        }

        strength += buildings_strength as i32;

        strength
    }
}

impl ICombatant for CityCombatant {
    fn get_max_health(&self) -> i32 {
        self.city.get_max_health()
    }

    fn get_health(&self) -> i32 {
        self.city.health
    }

    fn get_civ_info(&self) -> Arc<crate::civilization::civilization::Civilization> {
        self.city.civ.clone()
    }

    fn get_tile(&self) -> Arc<Tile> {
        self.city.get_center_tile()
    }

    fn get_name(&self) -> String {
        self.city.name.clone()
    }

    fn is_defeated(&self) -> bool {
        self.city.health == 1
    }

    fn is_invisible(&self, to: &crate::civilization::civilization::Civilization) -> bool {
        false
    }

    fn can_attack(&self) -> bool {
        self.city.can_bombard()
    }

    fn matches_filter(&self, filter: &str, multi_filter: bool) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| {
                f == "City" || f == Constants::ALL || self.city.matches_filter(f, false)
            })
        } else {
            filter == "City" || filter == Constants::ALL || self.city.matches_filter(filter, false)
        }
    }

    fn get_attack_sound(&self) -> UncivSound {
        UncivSound::Bombard
    }

    fn take_damage(&mut self, damage: i32) {
        self.city.health -= damage;
        if self.city.health < 1 {
            self.city.health = 1; // min health is 1
        }
    }

    fn get_unit_type(&self) -> UnitType {
        UnitType::City
    }

    fn get_attacking_strength(&self) -> i32 {
        (self.get_city_strength(CombatAction::Attack) as f32 * 0.75) as i32
    }

    fn get_defending_strength(&self, attacked_by_ranged: bool) -> i32 {
        if self.is_defeated() {
            return 1;
        }
        self.get_city_strength(CombatAction::Defend)
    }

    fn is_ranged(&self) -> bool {
        true // Cities can attack at range
    }

    fn is_air_unit(&self) -> bool {
        false
    }

    fn is_water_unit(&self) -> bool {
        false
    }

    fn is_land_unit(&self) -> bool {
        false
    }

    fn is_city(&self) -> bool {
        true
    }

    fn is_civilian(&self) -> bool {
        false
    }
}

impl Clone for CityCombatant {
    fn clone(&self) -> Self {
        CityCombatant {
            city: self.city.clone(),
        }
    }
}

impl std::fmt::Display for CityCombatant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.city.name)
    }
}