use std::collections::HashMap;
use std::sync::Arc;
use crate::civilization::Civilization;
use crate::civilization::PlayerType;
use crate::diplomacy::RelationshipLevel;
use crate::map::tile::RoadStatus;
use crate::models::ruleset::Policy;
use crate::models::ruleset::tile::ResourceType;
use crate::models::ruleset::tile::TileImprovement;
use crate::models::ruleset::unique::{StateForConditionals, UniqueTarget, UniqueType};
use crate::models::stats::{Stat, StatMap, Stats};
use crate::constants::Constants;

/// CivInfo class was getting too crowded
pub struct CivInfoStatsForNextTurn {
    civ_info: Arc<Civilization>,

    /// Happiness for next turn
    pub happiness: i32,

    pub stats_for_next_turn: Stats,
}

impl CivInfoStatsForNextTurn {
    /// Creates a new CivInfoStatsForNextTurn
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        Self {
            civ_info,
            happiness: 0,
            stats_for_next_turn: Stats::new(),
        }
    }

    /// Calculates unit maintenance cost
    fn get_unit_maintenance(&self) -> i32 {
        let base_unit_cost = 0.5;
        let mut free_units = 3;

        for unique in self.civ_info.get_matching_uniques(UniqueType::FreeUnits, &self.civ_info.state) {
            free_units += unique.params[0].parse::<i32>().unwrap();
        }

        let mut units_to_pay_for = self.civ_info.units.get_civ_units();

        if self.civ_info.has_unique(UniqueType::UnitsInCitiesNoMaintenance) {
            units_to_pay_for = units_to_pay_for.into_iter()
                .filter(|unit| !(unit.get_tile().is_city_center() && unit.can_garrison()))
                .collect();
        }

        // Each unit starts with 1.0 aka 100% of cost, and then the discount is added.
        // Note all discounts are in the form of -X%, such as -25 for 25% reduction

        let mut costs_to_pay = Vec::new();

        // We IGNORE the conditionals when we get them civ-wide, so we won't need to do the same thing for EVERY unit in the civ.
        // This leads to massive memory and CPU time savings when calculating the maintenance!
        let civwide_discount_uniques = self.civ_info.get_matching_uniques(
            UniqueType::UnitMaintenanceDiscount,
            &StateForConditionals::IgnoreConditionals
        );

        for unit in units_to_pay_for {
            let state_for_conditionals = unit.cache.state.clone();
            let mut unit_maintenance = 1.0;

            let uniques_that_apply = unit.get_matching_uniques(
                UniqueType::UnitMaintenanceDiscount,
                &state_for_conditionals
            ).into_iter()
            .chain(civwide_discount_uniques.iter().filter(|unique|
                unique.conditionals_apply(&state_for_conditionals)
            ));

            for unique in uniques_that_apply {
                unit_maintenance *= unique.params[0].parse::<f32>().unwrap() / 100.0;
            }

            costs_to_pay.push(unit_maintenance);
        }

        // Sort by descending maintenance, then drop most expensive X units to make them free
        // If more free than units left, runs sum on empty sequence
        costs_to_pay.sort_by(|a, b| b.partial_cmp(a).unwrap());

        let costs_to_pay_iter = costs_to_pay.iter().skip(free_units);
        let number_of_units_to_pay_for = costs_to_pay_iter.sum::<f32>().max(0.0);

        // as game progresses Maintenance cost rises
        let turn_limit = self.civ_info.game_info.speed.num_total_turns() as f32;
        let game_progress = (self.civ_info.game_info.turns as f32 / turn_limit).min(1.0);

        let mut cost = base_unit_cost * number_of_units_to_pay_for * (1.0 + game_progress);
        cost = cost.powf(1.0 + game_progress / 3.0); // Why 3? To spread 1 to 1.33

        if !self.civ_info.is_human() {
            cost *= self.civ_info.game_info.get_difficulty().ai_unit_maintenance_modifier;
        }

        cost as i32
    }

    /// Calculates transportation upkeep
    fn get_transportation_upkeep(&self) -> Stats {
        let mut transportation_upkeep = Stats::new();

        // we no longer use .flatMap, because there are a lot of tiles and keeping them all in a list
        // just to go over them once is a waste of memory - there are low-end phones who don't have much ram

        let ignored_tile_types = self.civ_info.get_matching_uniques(UniqueType::NoImprovementMaintenanceInSpecificTiles)
            .iter()
            .map(|unique| unique.params[0].clone())
            .collect::<HashSet<_>>(); // needs to be .toHashSet()ed,
        // Because we go over every tile in every city and check if it's in this list, which can get real heavy.

        let add_maintenance_uniques = |road: &TileImprovement, unique_type: UniqueType, state: &StateForConditionals| {
            for unique in road.get_matching_uniques(unique_type, state) {
                transportation_upkeep.add(Stat::value_of(&unique.params[1]), unique.params[0].parse::<f32>().unwrap());
            }
        };

        for city in &self.civ_info.cities {
            for tile in city.get_tiles() {
                if tile.is_city_center() { continue; }
                if tile.get_unpillaged_road() == RoadStatus::None { continue; } // Cheap checks before pricey checks
                if ignored_tile_types.iter().any(|filter| tile.matches_filter(filter, &self.civ_info)) { continue; }

                let road = tile.get_unpillaged_road_improvement().unwrap(); // covered by RoadStatus.None test
                let state_for_conditionals = StateForConditionals::new(&self.civ_info, Some(tile.clone()));

                add_maintenance_uniques(road, UniqueType::ImprovementMaintenance, &state_for_conditionals);
                add_maintenance_uniques(road, UniqueType::ImprovementAllMaintenance, &state_for_conditionals);
            }
        }

        // tabulate neutral roads
        for position in &self.civ_info.neutral_roads {
            let tile = &self.civ_info.game_info.tile_map[position];
            if tile.get_unpillaged_road() == RoadStatus::None { continue; } // Cheap checks before pricey checks

            let road = tile.get_unpillaged_road_improvement().unwrap(); // covered by RoadStatus.None test
            let state_for_conditionals = StateForConditionals::new(&self.civ_info, Some(tile.clone()));

            add_maintenance_uniques(road, UniqueType::ImprovementAllMaintenance, &state_for_conditionals);
        }

        for unique in self.civ_info.get_matching_uniques(UniqueType::RoadMaintenance) {
            transportation_upkeep.times_in_place(unique.params[0].parse::<f32>().unwrap() / 100.0);
        }

        transportation_upkeep
    }

    /// Calculates unit supply
    pub fn get_unit_supply(&self) -> i32 {
        /* TotalSupply = BaseSupply + NumCities*modifier + Population*modifier
        * In civ5, it seems population modifier is always 0.5, so i hardcoded it down below */
        let mut supply = self.get_base_unit_supply() + self.get_unit_supply_from_cities() + self.get_unit_supply_from_pop();

        if self.civ_info.is_major_civ() && self.civ_info.player_type == PlayerType::AI {
            supply = (supply as f32 * (1.0 + self.civ_info.get_difficulty().ai_unit_supply_modifier)) as i32;
        }

        supply
    }

    /// Calculates base unit supply
    pub fn get_base_unit_supply(&self) -> i32 {
        self.civ_info.get_difficulty().unit_supply_base +
            self.civ_info.get_matching_uniques(UniqueType::BaseUnitSupply)
                .iter()
                .map(|unique| unique.params[0].parse::<i32>().unwrap())
                .sum::<i32>()
    }

    /// Calculates unit supply from cities
    pub fn get_unit_supply_from_cities(&self) -> i32 {
        self.civ_info.cities.len() as i32 *
            (self.civ_info.get_difficulty().unit_supply_per_city +
                self.civ_info.get_matching_uniques(UniqueType::UnitSupplyPerCity)
                    .iter()
                    .map(|unique| unique.params[0].parse::<i32>().unwrap())
                    .sum::<i32>())
    }

    /// Calculates unit supply from population
    pub fn get_unit_supply_from_pop(&self) -> i32 {
        let mut total_supply = self.civ_info.cities.iter()
            .map(|city| city.population.population)
            .sum::<i32>() * self.civ_info.game_info.ruleset.mod_options.constants.unit_supply_per_population;

        for unique in self.civ_info.get_matching_uniques(UniqueType::UnitSupplyPerPop) {
            let applicable_population = self.civ_info.cities.iter()
                .filter(|city| city.matches_filter(&unique.params[2]))
                .map(|city| city.population.population / unique.params[1].parse::<i32>().unwrap())
                .sum::<i32>();

            total_supply += (unique.params[0].parse::<f32>().unwrap() * applicable_population as f32) as i32;
        }

        total_supply
    }

    /// Calculates unit supply deficit
    pub fn get_unit_supply_deficit(&self) -> i32 {
        (self.civ_info.units.get_civ_units_size() - self.get_unit_supply()).max(0)
    }

    /// Per each supply missing, a player gets -10% production. Capped at -70%.
    pub fn get_unit_supply_production_penalty(&self) -> f32 {
        -(self.get_unit_supply_deficit() as f32 * 10.0).min(70.0)
    }

    /// Gets stat map for next turn
    pub fn get_stat_map_for_next_turn(&self) -> StatMap {
        let mut stat_map = StatMap::new();

        for city in &self.civ_info.cities {
            for (key, value) in &city.city_stats.final_stat_list {
                stat_map.add(key, *value);
            }
        }

        // City-States bonuses
        for other_civ in self.civ_info.get_known_civs() {
            if !other_civ.is_city_state { continue; }

            let diplomacy_manager = other_civ.get_diplomacy_manager(&self.civ_info.civ_name).unwrap();
            if diplomacy_manager.relationship_ignore_afraid() != RelationshipLevel::Ally {
                continue;
            }

            for unique in self.civ_info.get_matching_uniques(UniqueType::CityStateStatPercent) {
                let stat = Stat::value_of(&unique.params[0]);
                let value = other_civ.stats.stats_for_next_turn.get(&stat).unwrap_or(&0.0) *
                    unique.params[1].parse::<f32>().unwrap() / 100.0;

                stat_map.add(Constants::CITY_STATES, Stats::new().add(stat, value));
            }
        }

        stat_map.insert("Transportation upkeep".to_string(), self.get_transportation_upkeep() * -1.0);
        stat_map.insert("Unit upkeep".to_string(), Stats::new().add(Stat::Gold, -(self.get_unit_maintenance() as f32)));

        if self.civ_info.get_happiness() > 0 {
            let mut excess_happiness_conversion = Stats::new();

            for unique in self.civ_info.get_matching_uniques(UniqueType::ExcessHappinessToGlobalStat) {
                let stat = Stat::value_of(&unique.params[1]);
                let value = unique.params[0].parse::<f32>().unwrap() / 100.0 * self.civ_info.get_happiness() as f32;

                excess_happiness_conversion.add(stat, value);
            }

            stat_map.add("Policies".to_string(), excess_happiness_conversion);
        }

        // negative gold hurts science
        // if we have - or 0, then the techs will never be complete and the tech button
        // will show a negative number of turns and int.max, respectively
        let gold_sum: f32 = stat_map.values().iter().map(|stats| stats.gold).sum();

        if gold_sum < 0.0 && self.civ_info.gold < 0 {
            let science_sum: f32 = stat_map.values().iter().map(|stats| stats.science).sum();
            let science_deficit = gold_sum.max(1.0 - science_sum); // Leave at least 1

            stat_map.insert("Treasury deficit".to_string(), Stats::new().add(Stat::Science, science_deficit));
        }

        let gold_difference_from_trade: f32 = self.civ_info.diplomacy.values()
            .iter()
            .map(|diplomacy| diplomacy.gold_per_turn())
            .sum();

        if gold_difference_from_trade != 0.0 {
            stat_map.insert("Trade".to_string(), Stats::new().add(Stat::Gold, gold_difference_from_trade));
        }

        for (key, value) in self.get_global_stats_from_uniques() {
            stat_map.add(key, value);
        }

        stat_map
    }

    /// Gets happiness breakdown
    pub fn get_happiness_breakdown(&self) -> HashMap<String, f32> {
        let mut stat_map = HashMap::new();

        stat_map.insert("Base happiness".to_string(), self.civ_info.get_difficulty().base_happiness as f32);

        let mut happiness_per_unique_luxury = 4.0 + self.civ_info.get_difficulty().extra_happiness_per_luxury;

        for unique in self.civ_info.get_matching_uniques(UniqueType::BonusHappinessFromLuxury) {
            happiness_per_unique_luxury += unique.params[0].parse::<f32>().unwrap();
        }

        let owned_luxuries: HashSet<_> = self.civ_info.get_civ_resource_supply()
            .iter()
            .map(|res| &res.resource)
            .filter(|res| res.resource_type == ResourceType::Luxury)
            .collect();

        let relevant_luxuries = self.civ_info.get_civ_resource_supply()
            .iter()
            .map(|res| &res.resource)
            .filter(|res| {
                res.resource_type == ResourceType::Luxury &&
                res.get_matching_uniques(UniqueType::ObsoleteWith)
                    .iter()
                    .all(|unique| !self.civ_info.tech_manager.is_researched(&unique.params[0]))
            })
            .count();

        stat_map.insert("Luxury resources".to_string(), relevant_luxuries as f32 * happiness_per_unique_luxury);

        let happiness_bonus_for_city_state_provided_luxuries = self.civ_info.get_matching_uniques(UniqueType::CityStateLuxuryHappiness)
            .iter()
            .map(|unique| unique.params[0].parse::<f32>().unwrap())
            .sum::<f32>() / 100.0;

        let luxuries_provided_by_city_states = self.civ_info.get_known_civs()
            .iter()
            .filter(|civ| civ.is_city_state && civ.get_ally_civ() == Some(self.civ_info.civ_name.clone()))
            .flat_map(|civ| civ.get_civ_resource_supply().iter().map(|res| &res.resource))
            .collect::<HashSet<_>>()
            .iter()
            .filter(|res| res.resource_type == ResourceType::Luxury && owned_luxuries.contains(*res))
            .count();

        stat_map.insert(
            "City-State Luxuries".to_string(),
            happiness_per_unique_luxury * luxuries_provided_by_city_states as f32 * happiness_bonus_for_city_state_provided_luxuries
        );

        let luxuries_all_of_which_are_traded_away = self.civ_info.detailed_civ_resources
            .iter()
            .filter(|res| {
                res.amount < 0 &&
                res.resource.resource_type == ResourceType::Luxury &&
                (res.origin == "Trade" || res.origin == "Trade request") &&
                !owned_luxuries.contains(&res.resource)
            })
            .count();

        let retain_happiness_percent = self.civ_info.get_matching_uniques(UniqueType::RetainHappinessFromLuxury)
            .iter()
            .map(|unique| unique.params[0].parse::<f32>().unwrap())
            .sum::<f32>() / 100.0;

        stat_map.insert(
            "Traded Luxuries".to_string(),
            luxuries_all_of_which_are_traded_away as f32 * happiness_per_unique_luxury * retain_happiness_percent
        );

        for city in &self.civ_info.cities {
            // There appears to be a concurrency problem? In concurrent thread in ConstructionsTable.getConstructionButtonDTOs
            // Literally no idea how, since happinessList is ONLY replaced, NEVER altered.
            // Oh well, toList() should solve the problem, wherever it may come from.
            for (key, value) in &city.city_stats.happiness_list {
                *stat_map.entry(key.clone()).or_insert(0.0) += value;
            }
        }

        let transport_upkeep = self.get_transportation_upkeep();
        if transport_upkeep.happiness != 0.0 {
            stat_map.insert("Transportation Upkeep".to_string(), -transport_upkeep.happiness);
        }

        for (key, value) in self.get_global_stats_from_uniques() {
            *stat_map.entry(key).or_insert(0.0) += value.happiness;
        }

        stat_map
    }

    /// Gets global stats from uniques
    fn get_global_stats_from_uniques(&self) -> StatMap {
        let mut stat_map = StatMap::new();

        if let Some(religion) = &self.civ_info.religion_manager.religion {
            for unique in religion.founder_belief_unique_map.get_matching_uniques(
                UniqueType::StatsFromGlobalCitiesFollowingReligion,
                &self.civ_info.state
            ) {
                stat_map.add(
                    "Religion".to_string(),
                    unique.stats * self.civ_info.religion_manager.number_of_cities_following_this_religion() as f32
                );
            }

            for unique in religion.founder_belief_unique_map.get_matching_uniques(
                UniqueType::StatsFromGlobalFollowers,
                &self.civ_info.state
            ) {
                let followers = self.civ_info.religion_manager.number_of_followers_following_this_religion(
                    &unique.params[2]
                ) as f32;

                stat_map.add(
                    "Religion".to_string(),
                    unique.stats * followers / unique.params[1].parse::<f32>().unwrap()
                );
            }
        }

        for unique in self.civ_info.get_matching_uniques(UniqueType::StatsPerPolicies) {
            let amount = self.civ_info.policies.get_adopted_policies()
                .iter()
                .filter(|policy| !Policy::is_branch_complete_by_name(policy))
                .count() / unique.params[1].parse::<usize>().unwrap();

            stat_map.add("Policies".to_string(), unique.stats * amount as f32);
        }

        for unique in self.civ_info.get_matching_uniques(UniqueType::Stats) {
            if unique.source_object_type != UniqueTarget::Building && unique.source_object_type != UniqueTarget::Wonder {
                stat_map.add(unique.get_source_name_for_user(), unique.stats);
            }
        }

        for unique in self.civ_info.get_matching_uniques(UniqueType::StatsPerStat) {
            let amount = self.civ_info.get_stat_reserve(Stat::value_of(&unique.params[2])) /
                unique.params[1].parse::<f32>().unwrap();

            stat_map.add("Stats".to_string(), unique.stats * amount);
        }

        let mut stats_per_natural_wonder = Stats::new().add(Stat::Happiness, 1.0);

        for unique in self.civ_info.get_matching_uniques(UniqueType::StatsFromNaturalWonders) {
            stats_per_natural_wonder.add_stats(&unique.stats);
        }

        stat_map.add(
            "Natural Wonders".to_string(),
            stats_per_natural_wonder * self.civ_info.natural_wonders.len() as f32
        );

        if stat_map.contains_key(Constants::CITY_STATES) {
            for unique in self.civ_info.get_matching_uniques(UniqueType::BonusStatsFromCityStates) {
                let bonus_percent = unique.params[0].parse::<f32>().unwrap() / 100.0;
                let bonus_stat = Stat::value_of(&unique.params[1]);

                if let Some(stats) = stat_map.get_mut(Constants::CITY_STATES) {
                    stats.multiply_stat(bonus_stat, bonus_percent);
                }
            }
        }

        stat_map
    }
}