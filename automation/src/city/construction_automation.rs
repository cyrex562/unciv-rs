use std::collections::HashMap;
use std::cmp::max;
use std::f32;


use sdl2::sys::False;

use crate::ai::personality::PersonalityValue;
use crate::city::city_constructions::CityConstructions;
use crate::ruleset::building::Building;
use crate::ruleset::construction_new::Construction;
use crate::ruleset::construction_new::ConstructionType;
use crate::ruleset::construction_new::PerpetualConstruction;
use crate::unique::unique::LocalUniqueCache;

/// Handles automated construction choices for cities
pub struct ConstructionAutomation<'a> {
    city_constructions: &'a CityConstructions,
    relative_cost_effectiveness: Vec<ConstructionChoice>,
    buildable_buildings: HashMap<String, bool>,
    buildable_units: HashMap<String, bool>,
    personality: &'a PersonalityValue,
    constructions_to_avoid: Vec<String>,
    disabled_auto_assign_constructions: Vec<String>,
}

#[derive(Debug, Clone)]
struct ConstructionChoice {
    choice: String,
    choice_modifier: f32,
    remaining_work: i32,
    production: i32,
}

impl<'a> ConstructionAutomation<'a> {
    pub fn new(city_constructions: &'a CityConstructions) -> Self {
        let city = city_constructions.get_city();
        let civ_info = city.get_civilization();
        let personality = civ_info.get_personality();

        // Get constructions to avoid from personality
        let constructions_to_avoid = personality.get_matching_uniques("WillNotBuild", city.get_state())
            .iter()
            .map(|unique| unique.params[0].clone())
            .collect();

        // Get disabled auto-assign constructions from settings
        let disabled_auto_assign_constructions = if civ_info.is_human() {
            // TODO: Get from settings
            Vec::new()
        } else {
            Vec::new()
        };

        ConstructionAutomation {
            city_constructions,
            relative_cost_effectiveness: Vec::new(),
            buildable_buildings: HashMap::new(),
            buildable_units: HashMap::new(),
            personality,
            constructions_to_avoid,
            disabled_auto_assign_constructions,
        }
    }

    fn should_avoid_construction(&self, construction: &Construction) -> bool {
        let state = self.city_constructions.get_city().get_state();
        for to_avoid in &self.constructions_to_avoid {
            match &construction.construction_type {
                ConstructionType::Building(building) => {
                    if building.matches_filter(to_avoid, state) {
                        return true;
                    }
                },
                ConstructionType::Unit(unit) => {
                    if unit.matches_filter(to_avoid, state) {
                        return true;
                    }
                },
                _ => {}, // Other construction types are not avoided
            }
        }
        false
    }

    fn add_choice(&mut self, choice: String, choice_modifier: f32) {
        let remaining_work = self.city_constructions.get_remaining_work(&choice, self.city_constructions.get_city());
        let production = self.city_constructions.production_for_construction(&choice);

        self.relative_cost_effectiveness.push(ConstructionChoice {
            choice,
            choice_modifier,
            remaining_work,
            production,
        });
    }

    pub fn choose_next_construction(&mut self) {
        let current_construction = self.city_constructions.get_current_construction();
        if !current_construction.is_perpetual() {
            return;
        }

        self.add_building_choices();

        if !self.city_constructions.get_city().is_puppet() {
            self.add_spaceship_part_choice();
            self.add_worker_choice();
            self.add_workboat_choice();
            self.add_military_unit_choice();
        }

        let chosen_construction = if self.relative_cost_effectiveness.is_empty() {
            self.choose_perpetual_construction()
        } else if self.relative_cost_effectiveness.iter().any(|c| c.remaining_work < c.production * 30) {
            self.choose_short_term_construction()
        } else {
            self.choose_cheapest_construction()
        };

        // Update current construction
        self.city_constructions.set_current_construction(chosen_construction);

        // Handle notifications
        self.handle_construction_notification(chosen_construction);
    }

    fn choose_perpetual_construction(&self) -> String {
        let civ_info = self.city_constructions.get_city().get_civilization();

        if PerpetualConstruction::science().is_buildable(self.city_constructions)
            && !civ_info.tech.all_techs_are_researched() {
            PerpetualConstruction::science().name.to_string()
        } else if PerpetualConstruction::gold().is_buildable(self.city_constructions) {
            PerpetualConstruction::gold().name.to_string()
        } else if PerpetualConstruction::culture().is_buildable(self.city_constructions)
            && !civ_info.policies.all_policies_adopted(true) {
            PerpetualConstruction::culture().name.to_string()
        } else if PerpetualConstruction::faith().is_buildable(self.city_constructions) {
            PerpetualConstruction::faith().name.to_string()
        } else {
            PerpetualConstruction::idle().name.to_string()
        }
    }

    fn choose_short_term_construction(&mut self) -> String {
        self.relative_cost_effectiveness.retain(|c| c.remaining_work < c.production * 30);

        if self.relative_cost_effectiveness.iter().all(|c| c.choice_modifier < 0.0) {
            // Take least negative value
            self.relative_cost_effectiveness.iter()
                .max_by(|a, b| {
                    let a_value = (a.remaining_work as f32 / a.choice_modifier) / a.production.max(1) as f32;
                    let b_value = (b.remaining_work as f32 / b.choice_modifier) / b.production.max(1) as f32;
                    a_value.partial_cmp(&b_value).unwrap()
                })
                .unwrap()
                .choice.clone()
        } else {
            // Remove negative modifiers and take most efficient positive value
            self.relative_cost_effectiveness.retain(|c| c.choice_modifier >= 0.0);
            self.relative_cost_effectiveness.iter()
                .min_by(|a, b| {
                    let a_value = (a.remaining_work as f32 / a.choice_modifier) / a.production.max(1) as f32;
                    let b_value = (b.remaining_work as f32 / b.choice_modifier) / b.production.max(1) as f32;
                    a_value.partial_cmp(&b_value).unwrap()
                })
                .unwrap()
                .choice.clone()
        }
    }

    fn choose_cheapest_construction(&self) -> String {
        self.relative_cost_effectiveness.iter()
            .min_by(|a, b| {
                let a_value = a.remaining_work as f32 / a.production.max(1) as f32;
                let b_value = b.remaining_work as f32 / b.production.max(1) as f32;
                a_value.partial_cmp(&b_value).unwrap()
            })
            .unwrap()
            .choice.clone()
    }

    // Building choices
    fn add_building_choices(&mut self) {
        let local_unique_cache = LocalUniqueCache::new(False);
        let buildings = self.city_constructions.get_ruleset().buildings.values()
            .filter(|b| !self.disabled_auto_assign_constructions.contains(&b.name))
            .filter(|b| {
                let construction = Construction::new_building(b);
                !self.should_avoid_construction(&construction)
            });

        for building in buildings.filter(|b| self.is_buildable(b)) {
            if building.is_wonder() && self.city_constructions.get_city().is_puppet() {
                continue;
            }

            // Don't build wonders in underdeveloped cities/empires
            if building.is_wonder() && (!self.is_city_over_average_production()
                || self.get_total_population() < 12) {
                continue;
            }

            self.add_choice(
                building.name.clone(),
                self.get_value_of_building(building, &local_unique_cache)
            );
        }
    }

    // Check if a building is buildable
    fn is_buildable(&self, building: &Building) -> bool {
        building.is_buildable(self.city_constructions)
    }

    // Helper methods for calculating building value
    fn get_value_of_building(&self, building: &Building, local_unique_cache: &LocalUniqueCache) -> f32 {
        let mut value = 0.0;
        value += self.apply_building_stats(building, local_unique_cache);
        value += self.apply_military_building_value(building);
        value += self.apply_victory_building_value(building);
        value += self.apply_onetime_unique_bonuses(building);
        value
    }

    // ... Additional implementation methods for military units, workers, etc.
    // These would follow the same pattern as the Kotlin code but adapted to Rust idioms
}

// Add mod.rs to expose the module