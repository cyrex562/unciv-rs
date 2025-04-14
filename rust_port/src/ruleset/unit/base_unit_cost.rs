use std::collections::HashMap;
use crate::models::{
    civilization::Civilization,
    city::City,
    ruleset::{
        unique::{Unique, UniqueType, StateForConditionals},
        unit::BaseUnit,
    },
    stats::Stat,
};
use crate::utils::to_percent;

/// Represents the cost calculations for a base unit
pub struct BaseUnitCost<'a> {
    base_unit: &'a BaseUnit,
}

impl<'a> BaseUnitCost<'a> {
    /// Creates a new BaseUnitCost for the given base unit
    pub fn new(base_unit: &'a BaseUnit) -> Self {
        Self { base_unit }
    }

    /// Gets the production cost for this unit for a civilization and optional city
    pub fn get_production_cost(&self, civ_info: &Civilization, city: Option<&City>) -> i32 {
        let mut production_cost = self.base_unit.cost as f32;

        let state_for_conditionals = city.map(|c| &c.state).unwrap_or(&civ_info.state);

        // Apply cost increases per city
        for unique in self.base_unit.get_matching_uniques(UniqueType::CostIncreasesPerCity, state_for_conditionals) {
            production_cost += (civ_info.cities.len() as i32 * unique.params[0].parse::<i32>().unwrap()) as f32;
        }

        // Apply cost increases when built
        for unique in self.base_unit.get_matching_uniques(UniqueType::CostIncreasesWhenBuilt, state_for_conditionals) {
            let built_count = civ_info.civ_constructions.built_items_with_increasing_cost
                .get(&self.base_unit.name)
                .copied()
                .unwrap_or(0);
            production_cost += (built_count * unique.params[0].parse::<i32>().unwrap()) as f32;
        }

        // Apply cost percentage changes
        for unique in self.base_unit.get_matching_uniques(UniqueType::CostPercentageChange, state_for_conditionals) {
            production_cost *= unique.params[0].to_percent();
        }

        // Apply difficulty modifiers
        production_cost *= if civ_info.is_city_state() {
            1.5
        } else if civ_info.is_human() {
            civ_info.get_difficulty().unit_cost_modifier
        } else {
            civ_info.game_info.get_difficulty().ai_unit_cost_modifier
        };

        // Apply game speed modifier
        production_cost *= civ_info.game_info.speed.production_cost_modifier;

        production_cost as i32
    }

    /// Checks if this unit can be purchased with a specific stat
    pub fn can_be_purchased_with_stat(&self, city: &City, stat: Stat) -> bool {
        let conditional_state = &city.state;

        // Check for BuyUnitsIncreasingCost
        if city.get_matching_uniques(UniqueType::BuyUnitsIncreasingCost, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[2] == stat.name &&
                self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
                city.matches_filter(&unique.params[3])
            }) {
            return true;
        }

        // Check for BuyUnitsByProductionCost
        if city.get_matching_uniques(UniqueType::BuyUnitsByProductionCost, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[1] == stat.name &&
                self.base_unit.matches_filter(&unique.params[0], Some(conditional_state))
            }) {
            return true;
        }

        // Check for BuyUnitsWithStat
        if city.get_matching_uniques(UniqueType::BuyUnitsWithStat, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[1] == stat.name &&
                self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
                city.matches_filter(&unique.params[2])
            }) {
            return true;
        }

        // Check for BuyUnitsForAmountStat
        if city.get_matching_uniques(UniqueType::BuyUnitsForAmountStat, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[2] == stat.name &&
                self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
                city.matches_filter(&unique.params[3])
            }) {
            return true;
        }

        false
    }

    /// Gets the stat buy cost for this unit
    pub fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32> {
        let mut cost = self.base_unit.get_base_buy_cost(city, stat)?;
        let conditional_state = &city.state;

        // Apply BuyUnitsDiscount
        for unique in city.get_matching_uniques(UniqueType::BuyUnitsDiscount, conditional_state) {
            if stat.name == unique.params[0] && self.base_unit.matches_filter(&unique.params[1], Some(conditional_state)) {
                cost *= unique.params[2].to_percent();
            }
        }

        // Apply BuyItemsDiscount
        for unique in city.get_matching_uniques(UniqueType::BuyItemsDiscount, conditional_state) {
            if stat.name == unique.params[0] {
                cost *= unique.params[1].to_percent();
            }
        }

        Some((cost / 10.0) as i32 * 10)
    }

    /// Gets the base buy costs for this unit
    pub fn get_base_buy_costs(&self, city: &City, stat: Stat) -> Vec<f32> {
        let conditional_state = &city.state;
        let mut costs = Vec::new();

        // Add costs from BuyUnitsIncreasingCost
        for unique in city.get_matching_uniques(UniqueType::BuyUnitsIncreasingCost, conditional_state) {
            if unique.params[2] == stat.name &&
               self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
               city.matches_filter(&unique.params[3]) {
                let base_cost = self.base_unit.get_cost_for_constructions_increasing_in_price(
                    unique.params[1].parse::<i32>().unwrap(),
                    unique.params[4].parse::<i32>().unwrap(),
                    city.civ.civ_constructions.bought_items_with_increasing_price
                        .get(&self.base_unit.name)
                        .copied()
                        .unwrap_or(0)
                );
                let speed_modifier = city.civ.game_info.speed.stat_cost_modifiers.get(&stat)
                    .copied()
                    .unwrap_or(1.0);
                costs.push(base_cost * speed_modifier);
            }
        }

        // Add costs from BuyUnitsByProductionCost
        for unique in city.get_matching_uniques(UniqueType::BuyUnitsByProductionCost, conditional_state) {
            if unique.params[1] == stat.name &&
               self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) {
                let production_cost = self.get_production_cost(&city.civ, Some(city));
                let multiplier = unique.params[2].parse::<i32>().unwrap();
                costs.push((production_cost * multiplier) as f32);
            }
        }

        // Add cost from BuyUnitsWithStat
        if city.get_matching_uniques(UniqueType::BuyUnitsWithStat, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[1] == stat.name &&
                self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
                city.matches_filter(&unique.params[2])
            }) {
            let base_cost = city.civ.get_era().base_unit_buy_cost;
            let speed_modifier = city.civ.game_info.speed.stat_cost_modifiers.get(&stat)
                .copied()
                .unwrap_or(1.0);
            costs.push(base_cost * speed_modifier);
        }

        // Add costs from BuyUnitsForAmountStat
        for unique in city.get_matching_uniques(UniqueType::BuyUnitsForAmountStat, conditional_state) {
            if unique.params[2] == stat.name &&
               self.base_unit.matches_filter(&unique.params[0], Some(conditional_state)) &&
               city.matches_filter(&unique.params[3]) {
                let amount = unique.params[1].parse::<i32>().unwrap();
                let speed_modifier = city.civ.game_info.speed.stat_cost_modifiers.get(&stat)
                    .copied()
                    .unwrap_or(1.0);
                costs.push((amount as f32) * speed_modifier);
            }
        }

        costs
    }
}