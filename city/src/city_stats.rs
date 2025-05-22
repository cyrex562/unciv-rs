use std::collections::HashMap;
use std::cmp::min;

use crate::counter::Counter;
use crate::debug_utils::DebugUtils;
use crate::simulation::simulation::Stat;
use crate::stats::stats::{StatMap, Stats};
use crate::tile::tile::RoadStatus;
use crate::unique::unique::{LocalUniqueCache, Unique};
use crate::unique::unique_target::UniqueTarget;
use crate::unique::UniqueType;
use crate::ruleset::construction_new::{Construction, ConstructionType};

use super::city::City;

/// A tree structure for organizing statistics by source
pub struct StatTreeNode {
    children: HashMap<String, StatTreeNode>,
    inner_stats: Option<Stats>,
}

impl StatTreeNode {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            inner_stats: None,
        }
    }

    pub fn set_inner_stat(&mut self, stat: Stat, value: f32) {
        if self.inner_stats.is_none() {
            self.inner_stats = Some(Stats::new());
        }
        if let Some(stats) = &mut self.inner_stats {
            stats.set(stat, value);
        }
    }

    fn add_inner_stats(&mut self, stats: &Stats) {
        if self.inner_stats.is_none() {
            self.inner_stats = Some(stats.clone());
        } else if let Some(inner_stats) = &mut self.inner_stats {
            inner_stats.add(stats);
        }
    }

    pub fn add_stats(&mut self, new_stats: Stats, hierarchy_list: &[String]) {
        if hierarchy_list.is_empty() {
            self.add_inner_stats(&new_stats);
            return;
        }

        let child_name = &hierarchy_list[0];
        let child = self.children.entry(child_name.clone())
            .or_insert_with(StatTreeNode::new);

        child.add_stats(new_stats, &hierarchy_list[1..]);
    }

    pub fn add(&mut self, other_tree: &StatTreeNode) {
        if let Some(other_stats) = &other_tree.inner_stats {
            self.add_inner_stats(other_stats);
        }

        for (key, value) in &other_tree.children {
            if !self.children.contains_key(key) {
                self.children.insert(key.clone(), value.clone());
            } else {
                self.children.get_mut(key).unwrap().add(value);
            }
        }
    }

    pub fn clone(&self) -> Self {
        let mut new = Self::new();
        new.inner_stats = self.inner_stats.clone();
        new.children = self.children.clone();
        new
    }

    pub fn total_stats(&self) -> Stats {
        let mut to_return = Stats::new();
        if let Some(stats) = &self.inner_stats {
            to_return.add(stats);
        }
        for child in self.children.values() {
            to_return.add(&child.total_stats());
        }
        to_return
    }
}

/// Holds and calculates Stats for a city.
///
/// No field needs to be saved, all are calculated on the fly,
/// so its field in City is transient and no such annotation is needed here.
pub struct CityStats<'a> {
    city: &'a City<'a>,
    base_stat_tree: StatTreeNode,
    stat_percent_bonus_tree: StatTreeNode,
    final_stat_list: HashMap<String, Stats>,
    happiness_list: HashMap<String, f32>,
    stats_from_tiles: Stats,
    current_city_stats: Stats,  // This is so we won't have to calculate this multiple times - takes a lot of time, especially on phones
}

impl<'a> CityStats<'a> {
    pub fn new(city: &'a City) -> Self {
        Self {
            city,
            base_stat_tree: StatTreeNode::new(),
            stat_percent_bonus_tree: StatTreeNode::new(),
            final_stat_list: HashMap::new(),
            happiness_list: HashMap::new(),
            stats_from_tiles: Stats::new(),
            current_city_stats: Stats::new(),
        }
    }

    fn get_stats_from_trade_route(&self) -> Stats {
        let mut stats = Stats::new();
        let capital_for_trade_route_purposes = self.city.civ.get_capital().unwrap();

        if self.city != capital_for_trade_route_purposes && self.city.is_connected_to_capital(RoadStatus::Road) {
            stats.gold = capital_for_trade_route_purposes.population.population as f32 * 0.15
                + self.city.population.population as f32 * 1.1 - 1.0; // Calculated by http://civilization.wikia.com/wiki/Trade_route_(Civ5)

            for unique in self.city.get_matching_uniques(UniqueType::StatsFromTradeRoute, Some(&self.city.state), true) {
                stats.add(&unique.stats);
            }

            let mut percentage_stats = Stats::new();
            for unique in self.city.get_matching_uniques(UniqueType::StatPercentFromTradeRoutes, Some(&self.city.state), true) {
                let stat = Stat::from_str(&unique.params[1]).unwrap();
                percentage_stats.set(stat, percentage_stats.get(stat) + unique.params[0].parse::<f32>().unwrap_or(0.0));
            }

            for stat in Stat::iter() {
                stats.set(stat, stats.get(stat) * percentage_stats.get(stat).to_percent());
            }
        }
        stats
    }

    fn get_stats_from_production(&self, production: f32) -> Stats {
        let mut stats = Stats::new();

        if let Some(current_construction) = self.city.city_constructions.current_construction_from_queue() {
            if Stat::stats_with_civ_wide_field().iter().any(|s| s.name() == current_construction.name()) {
                if let Ok(stat) = Stat::from_str(current_construction.name()) {
                    stats.set(stat, production * self.get_stat_conversion_rate(stat));
                }
            }
        }
        stats
    }

    pub fn get_stat_conversion_rate(&self, stat: Stat) -> f32 {
        let mut conversion_rate = 1.0 / 4.0;
        if let Some(conversion_unique) = self.city.civ.get_matching_uniques(UniqueType::ProductionToCivWideStatConversionBonus)
            .iter()
            .find(|u| u.params[0] == stat.name())
        {
            conversion_rate *= conversion_unique.params[1].parse::<f32>().unwrap_or(0.0).to_percent();
        }
        conversion_rate
    }

    fn get_stat_percent_bonuses_from_railroad(&self) -> Stats {
        let mut stats = Stats::new();
        let railroad_improvement = self.city.get_ruleset().railroad_improvement.as_ref()?;
        let tech_enabling_railroad = railroad_improvement.tech_required.as_ref();

        // If we conquered enemy cities connected by railroad, but we don't yet have that tech,
        // we shouldn't get bonuses, it's as if the tracks are laid out but we can't operate them.
        if (tech_enabling_railroad.is_none() || self.city.civ.tech.is_researched(tech_enabling_railroad.unwrap()))
            && (self.city.is_capital() || self.is_connected_to_capital(RoadStatus::Railroad))
        {
            stats.production += 25.0;
        }
        stats
    }

    fn add_stat_percent_bonuses_from_buildings(&self, stat_percent_bonus_tree: &mut StatTreeNode) {
        let local_unique_cache = LocalUniqueCache::new(true);
        for building in self.city.city_constructions.get_built_buildings() {
            stat_percent_bonus_tree.add_stats(
                building.get_stat_percentage_bonuses(self.city, &local_unique_cache),
                &["Buildings".to_string(), building.name.clone()]
            );
        }
    }

    fn get_stat_percent_bonuses_from_puppet_city(&self) -> Stats {
        let mut stats = Stats::new();
        if self.city.is_puppet {
            stats.science -= 25.0;
            stats.culture -= 25.0;
        }
        stats
    }

    pub fn get_growth_bonus(&self, total_food: f32) -> StatMap {
        let mut growth_sources = StatMap::new();
        let state_for_conditionals = &self.city.state;

        // "[amount]% growth [cityFilter]"
        for unique in self.city.get_matching_uniques(UniqueType::GrowthPercentBonus, Some(state_for_conditionals), true) {
            if !self.city.matches_filter(&unique.params[1], None, false) {
                continue;
            }

            growth_sources.add(
                unique.get_source_name_for_user(),
                Stats::new().with_food(unique.params[0].parse::<f32>().unwrap_or(0.0) / 100.0 * total_food)
            );
        }
        growth_sources
    }

    pub fn has_extra_annex_unhappiness(&self) -> bool {
        if self.city.civ.civ_name == self.city.founding_civ || self.city.is_puppet {
            return false;
        }
        !self.city.contains_building_unique(UniqueType::RemoveAnnexUnhappiness, None)
    }

    pub fn get_stats_of_specialist(&self, specialist_name: &str, local_unique_cache: &LocalUniqueCache) -> Stats {
        let specialist = match self.city.get_ruleset().specialists.get(specialist_name) {
            Some(s) => s,
            None => return Stats::new(),
        };

        let mut stats = specialist.clone_stats();

        for unique in local_unique_cache.for_city_get_matching_uniques(self.city, UniqueType::StatsFromSpecialist, &self.city.state) {
            if self.city.matches_filter(&unique.params[1], None, false) {
                stats.add(&unique.stats);
            }
        }

        for unique in local_unique_cache.for_city_get_matching_uniques(self.city, UniqueType::StatsFromObject, &self.city.state) {
            if unique.params[1] == specialist_name {
                stats.add(&unique.stats);
            }
        }

        stats
    }

    fn get_stats_from_specialists(&self, specialists: &Counter<String>) -> Stats {
        let mut stats = Stats::new();
        let local_unique_cache = LocalUniqueCache::new(true);

        for (key, value) in specialists.iter().filter(|(_, v)| *v > 0) {
            stats.add(&(self.get_stats_of_specialist(key, &local_unique_cache) * *value as f32));
        }

        stats
    }

    fn get_stats_from_uniques_by_source(&self) -> StatTreeNode {
        let mut source_to_stats = StatTreeNode::new();

        let city_state_stats_multipliers = self.city.civ.get_matching_uniques(UniqueType::BonusStatsFromCityStates).to_vec();

        let add_unique_stats = |unique: &Unique| {
            let mut stats = unique.stats.clone();
            if unique.source_object_type == UniqueTarget::CityState {
                for multiplier_unique in &city_state_stats_multipliers {
                    let stat = Stat::from_str(&multiplier_unique.params[1]).unwrap();
                    stats.set(stat, stats.get(stat) * multiplier_unique.params[0].parse::<f32>().unwrap_or(0.0).to_percent());
                }
            }
            source_to_stats.add_stats(stats, &[unique.get_source_name_for_user(), unique.source_object_name.clone().unwrap_or_default()]);
        };

        for unique in self.city.get_matching_uniques(UniqueType::StatsPerCity, Some(&self.city.state), true) {
            if self.city.matches_filter(&unique.params[1], None, false) {
                add_unique_stats(unique);
            }
        }

        // "[stats] per [amount] population [cityFilter]"
        for unique in self.city.get_matching_uniques(UniqueType::StatsPerPopulation, Some(&self.city.state), true) {
            if self.city.matches_filter(&unique.params[2], None, false) {
                let amount_of_effects = self.city.population.population as f32 / unique.params[1].parse::<f32>().unwrap_or(1.0);
                source_to_stats.add_stats(
                    unique.stats.times(amount_of_effects),
                    &[unique.get_source_name_for_user(), unique.source_object_name.clone().unwrap_or_default()]
                );
            }
        }

        for unique in self.city.get_matching_uniques(UniqueType::StatsFromCitiesOnSpecificTiles, Some(&self.city.state), true) {
            if self.city.get_center_tile().matches_terrain_filter(&unique.params[1], &self.city.civ) {
                add_unique_stats(unique);
            }
        }

        source_to_stats
    }

    fn get_stat_percent_bonuses_from_golden_age(&self, is_golden_age: bool) -> Stats {
        let mut stats = Stats::new();
        if is_golden_age {
            stats.production += 20.0;
            stats.culture += 20.0;
        }
        stats
    }

    fn get_stats_percent_bonuses_from_uniques_by_source(&self, current_construction: &Construction) -> StatTreeNode {
        let mut source_to_stats = StatTreeNode::new();

        let add_unique_stats = |unique: &Unique, stat: Stat, amount: f32| {
            source_to_stats.add_stats(
                Stats::new().with_stat(stat, amount),
                &[unique.get_source_name_for_user(), unique.source_object_name.clone().unwrap_or_default()]
            );
        };



        for unique in self.city.get_matching_uniques(UniqueType::StatPercentBonus, Some(&self.city.state), true) {
            if let Ok(stat) = Stat::from_str(&unique.params[1]) {
                add_unique_stats(unique, stat, unique.params[0].parse::<f32>().unwrap_or(0.0));
            }
        }

        for unique in self.city.get_matching_uniques(UniqueType::StatPercentBonusCities, Some(&self.city.state), true) {
            if self.city.matches_filter(&unique.params[2], None, false) {
                if let Ok(stat) = Stat::from_str(&unique.params[1]) {
                    add_unique_stats(unique, stat, unique.params[0].parse::<f32>().unwrap_or(0.0));
                }
            }
        }

        // Determine which uniques to check based on construction type
        let uniques_to_check = match &current_construction.construction_type {
            crate::ruleset::construction_new::ConstructionType::Unit(_) =>
                self.city.get_matching_uniques(UniqueType::PercentProductionUnits, Some(&self.city.state), true),
            crate::ruleset::construction_new::ConstructionType::Building(building) if building.is_wonder =>
                self.city.get_matching_uniques(UniqueType::PercentProductionWonders, Some(&self.city.state), true),
            crate::ruleset::construction_new::ConstructionType::Building(_) =>
                self.city.get_matching_uniques(UniqueType::PercentProductionBuildings, Some(&self.city.state), true),
            _ => Vec::new(), // Science/Gold production or other types
        };

        for unique in uniques_to_check {
            if self.construction_matches_filter(current_construction, &unique.params[1])
                && self.city.matches_filter(&unique.params[2], None, false)
            {
                add_unique_stats(unique, Stat::Production, unique.params[0].parse::<f32>().unwrap_or(0.0));
            }
        }

        for unique in self.city.get_matching_uniques(UniqueType::StatPercentFromReligionFollowers, Some(&self.city.state), true) {
            if let Ok(stat) = Stat::from_str(&unique.params[1]) {
                add_unique_stats(
                    unique,
                    stat,
                    min(
                        unique.params[0].parse::<f32>().unwrap_or(0.0) * self.city.religion.get_followers_of_our_religion() as f32,
                        unique.params[2].parse::<f32>().unwrap_or(0.0)
                    )
                );
            }
        }

        // Check if this is a building construction and if it's built in the capital
        if let ConstructionType::Building(_) = &current_construction.construction_type {
            if self.city.civ.get_capital().map_or(false, |cap|
                cap.city_constructions.is_built(current_construction.name()))
            {
                for unique in self.city.get_matching_uniques(UniqueType::PercentProductionBuildingsInCapital, Some(&self.city.state), true) {
                    add_unique_stats(unique, Stat::Production, unique.params[0].parse::<f32>().unwrap_or(0.0));
                }
            }
        }

        source_to_stats
    }

    fn get_stat_percent_bonuses_from_unit_supply(&self) -> Stats {
        let mut stats = Stats::new();
        let supply_deficit = self.city.civ.stats.get_unit_supply_deficit();
        if supply_deficit > 0 {
            stats.production = self.city.civ.stats.get_unit_supply_production_penalty();
        }
        stats
    }

    fn construction_matches_filter(&self, construction: &Construction, filter: &str) -> bool {
        let state = &self.city.state;
        match &construction.construction_type {
            ConstructionType::Building(building) => building.matches_filter(filter, state),
            ConstructionType::Unit(unit) => unit.matches_filter(filter, state),
            _ => false
        }
    }

    pub fn is_connected_to_capital(&self, road_type: RoadStatus) -> bool {
        if self.city.civ.cities.len() < 2 {
            return false; // first city!
        }

        // Railroad, or harbor from railroad
        if road_type == RoadStatus::Railroad {
            self.city.is_connected_to_capital(|road_types|
                road_types.iter().any(|rt| rt.contains(RoadStatus::Railroad.name())))
        } else {
            self.city.is_connected_to_capital(|_| true)
        }
    }

    pub fn get_road_type_of_connection_to_capital(&self) -> RoadStatus {
        if self.is_connected_to_capital(RoadStatus::Railroad) {
            RoadStatus::Railroad
        } else if self.is_connected_to_capital(RoadStatus::Road) {
            RoadStatus::Road
        } else {
            RoadStatus::None
        }
    }

    fn get_building_maintenance_costs(&self) -> f32 {
        // Same here - will have a different UI display.
        let mut buildings_maintenance = self.city.city_constructions.get_maintenance_costs() as f32; // this is AFTER the bonus calculation!
        if !self.city.civ.is_human() {
            buildings_maintenance *= self.city.civ.game_info.get_difficulty().ai_building_maintenance_modifier;
        }

        for unique in self.city.get_matching_uniques(UniqueType::BuildingMaintenance, Some(&self.city.state), true) {
            buildings_maintenance *= unique.params[0].parse::<f32>().unwrap_or(0.0).to_percent();
        }

        buildings_maintenance
    }

    pub fn update_tile_stats(&mut self, local_unique_cache: &LocalUniqueCache) {
        let mut stats = Stats::new();
        let worked_tiles = self.city.tiles_in_range.iter()
            .filter(|tile| {
                self.city.location == tile.position
                    || self.city.is_worked(tile)
                    || (tile.owning_city.as_ref().map_or(false, |c| c == self.city)
                        && (tile.get_unpillaged_tile_improvement()
                            .map_or(false, |imp| imp.has_unique(UniqueType::TileProvidesYieldWithoutPopulation, &tile.state_this_tile))
                        || tile.terrain_has_unique(UniqueType::TileProvidesYieldWithoutPopulation, &tile.state_this_tile)))
            });

        for tile in worked_tiles {
            if tile.is_blockaded() && self.city.is_worked(tile) {
                self.city.worked_tiles.remove(&tile.position);
                self.city.locked_tiles.remove(&tile.position);
                self.city.should_reassign_population = true;
                continue;
            }
            let tile_stats = tile.stats.get_tile_stats(self.city, &self.city.civ, local_unique_cache);
            stats.add(&tile_stats);
        }
        self.stats_from_tiles = stats;
    }

    // needs to be a separate function because we need to know the global happiness state
    // in order to determine how much food is produced in a city!
    pub fn update_city_happiness(&mut self, stats_from_buildings: &StatTreeNode) {
        let civ_info = &self.city.civ;
        let mut new_happiness_list = HashMap::new();

        // This calculation seems weird to me.
        // Suppose we calculate the modifier for an AI (non-human) player when the game settings has difficulty level 'prince'.
        // We first get the difficulty modifier for this civilization, which results in the 'chieftain' modifier (0.6) being used,
        // as this is a non-human player. Then we multiply that by the ai modifier in general, which is 1.0 for prince.
        // The end result happens to be 0.6, which seems correct. However, if we were playing on chieftain difficulty,
        // we would get back 0.6 twice and the modifier would be 0.36. Thus, in general there seems to be something wrong here
        // I don't know enough about the original whether they do something similar or not and can't be bothered to find where
        // in the source code this calculation takes place, but it would surprise me if they also did this double multiplication thing. ~xlenstra
        let mut unhappiness_modifier = civ_info.get_difficulty().unhappiness_modifier;
        if !civ_info.is_human() {
            unhappiness_modifier *= civ_info.game_info.get_difficulty().ai_unhappiness_modifier;
        }

        let mut unhappiness_from_city = -3.0; // -3 happiness per city
        if self.has_extra_annex_unhappiness() {
            unhappiness_from_city -= 2.0;
        }

        let mut unique_unhappiness_modifier = 0.0;
        for unique in civ_info.get_matching_uniques(UniqueType::UnhappinessFromCitiesPercentage) {
            unique_unhappiness_modifier += unique.params[0].parse::<f32>().unwrap_or(0.0);
        }

        new_happiness_list.insert(
            "Cities".to_string(),
            unhappiness_from_city * unhappiness_modifier * unique_unhappiness_modifier.to_percent()
        );

        let mut unhappiness_from_citizens = self.city.population.population as f32;

        for unique in self.city.get_matching_uniques(UniqueType::UnhappinessFromPopulationTypePercentageChange, Some(&self.city.state), true) {
            if self.city.matches_filter(&unique.params[2], None, false) {
                unhappiness_from_citizens += (unique.params[0].parse::<f32>().unwrap_or(0.0) / 100.0)
                    * self.city.population.get_population_filter_amount(&unique.params[1]) as f32;
            }
        }

        if self.has_extra_annex_unhappiness() {
            unhappiness_from_citizens *= 2.0;
        }

        if unhappiness_from_citizens < 0.0 {
            unhappiness_from_citizens = 0.0;
        }

        new_happiness_list.insert("Population".to_string(), -unhappiness_from_citizens * unhappiness_modifier);

        if self.has_extra_annex_unhappiness() {
            new_happiness_list.insert("Occupied City".to_string(), -2.0); // annexed city
        }

        let happiness_from_specialists = self.get_stats_from_specialists(self.city.population.get_new_specialists()).happiness as f32;
        if happiness_from_specialists > 0.0 {
            new_happiness_list.insert("Specialists".to_string(), happiness_from_specialists);
        }

        new_happiness_list.insert("Buildings".to_string(), stats_from_buildings.total_stats().happiness as f32);
        new_happiness_list.insert("Tile yields".to_string(), self.stats_from_tiles.happiness);

        let happiness_by_source = self.get_stats_from_uniques_by_source();
        for (source, stats) in &happiness_by_source.children {
            if stats.total_stats().happiness != 0.0 {
                let entry = new_happiness_list.entry(source.clone()).or_insert(0.0);
                *entry += stats.total_stats().happiness;
            }
        }

        // we don't want to modify the existing happiness list because that leads
        // to concurrency problems if we iterate on it while changing
        self.happiness_list = new_happiness_list;
    }

    fn update_base_stat_list(&mut self, stats_from_buildings: &StatTreeNode) {
        let mut new_base_stat_tree = StatTreeNode::new();
        let mut new_base_stat_list = StatMap::new();

        new_base_stat_tree.add_stats(
            Stats::new()
                .with_science(self.city.population.population as f32)
                .with_production(self.city.population.get_free_population() as f32),
            &["Population".to_string()]
        );

        new_base_stat_list.insert("Tile yields".to_string(), self.stats_from_tiles.clone());
        new_base_stat_list.insert("Specialists".to_string(), self.get_stats_from_specialists(self.city.population.get_new_specialists()));
        new_base_stat_list.insert("Trade routes".to_string(), self.get_stats_from_trade_route());
        new_base_stat_tree.children.insert("Buildings".to_string(), stats_from_buildings.clone());

        for (source, stats) in new_base_stat_list {
            new_base_stat_tree.add_stats(stats, &[source]);
        }

        new_base_stat_tree.add(&self.get_stats_from_uniques_by_source());
        self.base_stat_tree = new_base_stat_tree;
    }

    fn update_stat_percent_bonus_list(&mut self, current_construction: &Construction) {
        let mut new_stats_bonus_tree = StatTreeNode::new();

        new_stats_bonus_tree.add_stats(
            self.get_stat_percent_bonuses_from_golden_age(self.city.civ.golden_ages.is_golden_age()),
            &["Golden Age".to_string()]
        );

        self.add_stat_percent_bonuses_from_buildings(&mut new_stats_bonus_tree);

        if let Some(railroad_stats) = self.get_stat_percent_bonuses_from_railroad() {
            new_stats_bonus_tree.add_stats(railroad_stats, &["Railroad".to_string()]);
        }

        new_stats_bonus_tree.add_stats(
            self.get_stat_percent_bonuses_from_puppet_city(),
            &["Puppet City".to_string()]
        );

        new_stats_bonus_tree.add_stats(
            self.get_stat_percent_bonuses_from_unit_supply(),
            &["Unit Supply".to_string()]
        );

        new_stats_bonus_tree.add(&self.get_stats_percent_bonuses_from_uniques_by_source(current_construction));

        if DebugUtils::SUPERCHARGED {
            let mut stats = Stats::new();
            for stat in Stat::iter() {
                stats.set(stat, 10000.0);
            }
            new_stats_bonus_tree.add_stats(stats, &["Supercharged".to_string()]);
        }

        self.stat_percent_bonus_tree = new_stats_bonus_tree;
    }

    pub fn update(
        &mut self,
        current_construction: Option<&Construction>,
        update_tile_stats: bool,
        update_civ_stats: bool,
        local_unique_cache: &LocalUniqueCache
    ) {
        let current_construction = current_construction.unwrap_or_else(||
            self.city.city_constructions.get_current_construction().as_ref().unwrap());

        if update_tile_stats {
            self.update_tile_stats(local_unique_cache);
        }

        // We need to compute Tile yields before happiness
        let stats_from_buildings = self.city.city_constructions.get_stats(local_unique_cache); // this is performance heavy, so calculate once
        self.update_base_stat_list(&stats_from_buildings);
        self.update_city_happiness(&stats_from_buildings);
        self.update_stat_percent_bonus_list(current_construction);

        self.update_final_stat_list(current_construction); // again, we don't edit the existing currentCityStats directly, in order to avoid concurrency exceptions

        let mut new_current_city_stats = Stats::new();
        for stats in self.final_stat_list.values() {
            new_current_city_stats.add(stats);
        }
        self.current_city_stats = new_current_city_stats;

        if update_civ_stats {
            self.city.civ.update_stats_for_next_turn();
        }
    }

    fn update_final_stat_list(&mut self, current_construction: &Construction) {
        let mut new_final_stat_list = HashMap::new(); // again, we don't edit the existing currentCityStats directly, in order to avoid concurrency exceptions

        for (key, value) in &self.base_stat_tree.children {
            new_final_stat_list.insert(key.clone(), value.total_stats().clone());
        }

        let stat_percent_bonuses_sum = self.stat_percent_bonus_tree.total_stats();

        for entry in new_final_stat_list.values_mut() {
            entry.production *= stat_percent_bonuses_sum.production.to_percent();
        }

        // We only add the 'extra stats from production' AFTER we calculate the production INCLUDING BONUSES
        let total_production: f32 = new_final_stat_list.values().map(|s| s.production).sum();
        let stats_from_production = self.get_stats_from_production(total_production);

        if !stats_from_production.is_empty() {
            let mut new_base_stat_tree = StatTreeNode::new();
            new_base_stat_tree.children = self.base_stat_tree.children.clone();
            new_base_stat_tree.add_stats(stats_from_production, &["Production".to_string()]);
            self.base_stat_tree = new_base_stat_tree; // concurrency-safe addition

            new_final_stat_list.insert("Construction".to_string(), stats_from_production);
        }

        for entry in new_final_stat_list.values_mut() {
            entry.gold *= stat_percent_bonuses_sum.gold.to_percent();
            entry.culture *= stat_percent_bonuses_sum.culture.to_percent();
            entry.food *= stat_percent_bonuses_sum.food.to_percent();
            entry.faith *= stat_percent_bonuses_sum.faith.to_percent();
        }

        // AFTER we've gotten all the gold stats figured out, only THEN do we plonk that gold into Science
        if self.city.get_ruleset().mod_options.has_unique(UniqueType::ConvertGoldToScience) {
            let total_gold: f32 = new_final_stat_list.values().map(|s| s.gold).sum();
            let amount_converted = (total_gold * self.city.civ.tech.gold_percent_converted_to_science) as i32 as f32;

            if amount_converted > 0.0 { // Don't want you converting negative gold to negative science yaknow
                new_final_stat_list.insert(
                    "Gold -> Science".to_string(),
                    Stats::new()
                        .with_science(amount_converted)
                        .with_gold(-amount_converted)
                );
            }
        }

        for entry in new_final_stat_list.values_mut() {
            entry.science *= stat_percent_bonuses_sum.science.to_percent();
        }

        for unique in self.city.get_matching_uniques(UniqueType::NullifiesStat, Some(&self.city.state), true) {
            if let Ok(stat_to_be_removed) = Stat::from_str(&unique.params[0]) {
                let removed_amount: f32 = new_final_stat_list.values().map(|s| s.get(stat_to_be_removed)).sum();

                let mut nullifying_stats = Stats::new();
                nullifying_stats.set(stat_to_be_removed, -removed_amount);

                new_final_stat_list.insert(
                    unique.get_source_name_for_user(),
                    nullifying_stats
                );
            }
        }

        /* Okay, food calculation is complicated.
        First we see how much food we generate. Then we apply production bonuses to it.
        Up till here, business as usual.
        Then, we deduct food eaten (from the total produced).
        Now we have the excess food, to which "growth" modifiers apply
        Some policies have bonuses for growth only, not general food production. */

        let food_eaten = self.calc_food_eaten();
        if let Some(population_stats) = new_final_stat_list.get_mut("Population") {
            population_stats.food -= food_eaten;
        }

        let total_food: f32 = new_final_stat_list.values().map(|s| s.food).sum();

        // Apply growth modifier only when positive food
        if total_food > 0.0 {
            // Since growth bonuses are special, (applied afterwards) they will be displayed separately in the user interface as well.
            // All bonuses except We Love The King do apply even when unhappy
            let growth_bonuses = self.get_growth_bonus(total_food);
            for (key, value) in growth_bonuses {
                new_final_stat_list.insert(format!("[{}] ([Growth])", key), value);
            }

            if self.city.is_we_love_the_king_day_active() && self.city.civ.get_happiness() >= 0 {
                // We Love The King Day +25%, only if not unhappy
                let we_love_the_king_food = Stats::new().with_food(total_food / 4.0);
                new_final_stat_list.insert("We Love The King Day".to_string(), we_love_the_king_food);
            }

            // recalculate only when all applied - growth bonuses are not multiplicative
            // bonuses can allow a city to grow even with -100% unhappiness penalty, this is intended
            let total_food: f32 = new_final_stat_list.values().map(|s| s.food).sum();
        }

        let buildings_maintenance = self.get_building_maintenance_costs(); // this is AFTER the bonus calculation!
        new_final_stat_list.insert(
            "Maintenance".to_string(),
            Stats::new().with_gold(-(buildings_maintenance as i32) as f32)
        );

        if self.can_convert_food_to_production(total_food, current_construction) {
            new_final_stat_list.insert(
                "Excess food to production".to_string(),
                Stats::new()
                    .with_production(self.get_production_from_excessive_food(total_food))
                    .with_food(-total_food)
            );
        }

        if let Some(growth_nullifying_unique) = self.city.get_matching_uniques(UniqueType::NullifiesGrowth, Some(&self.city.state), true).first() {
            // Does not nullify negative growth (starvation)
            let current_growth: f32 = new_final_stat_list.values().map(|s| s.get(Stat::Food)).sum();
            if current_growth > 0.0 {
                new_final_stat_list.insert(
                    growth_nullifying_unique.get_source_name_for_user(),
                    Stats::new().with_food(-current_growth)
                );
            }
        }

        if self.city.is_in_resistance() {
            new_final_stat_list.clear(); // NOPE
        }

        let total_production: f32 = new_final_stat_list.values().map(|s| s.production).sum();
        if total_production < 1.0 { // Minimum production for things to progress
            new_final_stat_list.insert("Production".to_string(), Stats::new().with_production(1.0));
        }

        self.final_stat_list = new_final_stat_list;
    }

    pub fn can_convert_food_to_production(&self, food: f32, current_construction: &Construction) -> bool {
        food > 0.0
            && !current_construction.is_perpetual
            && current_construction.get_matching_uniques_not_conflicting(UniqueType::ConvertFoodToProductionWhenConstructed, &self.city.state).len() > 0
    }

    /// Calculate the conversion of the excessive food to production when
    /// [UniqueType.ConvertFoodToProductionWhenConstructed] is at front of the build queue
    /// @param food is amount of excess Food generates this turn
    /// See for details: https://civilization.fandom.com/wiki/Settler_(Civ5)
    /// @see calc_food_eaten as well for Food consumed this turn
    pub fn get_production_from_excessive_food(&self, food: f32) -> f32 {
        if food >= 4.0 {
            2.0 + (food / 4.0) as i32 as f32
        } else if food >= 2.0 {
            2.0
        } else if food >= 1.0 {
            1.0
        } else {
            0.0
        }
    }

    fn calc_food_eaten(&self) -> f32 {
        let mut food_eaten = self.city.population.population as f32 * 2.0;
        let mut food_eaten_by_specialists = 2.0 * self.city.population.get_number_of_specialists() as f32;

        for unique in self.city.get_matching_uniques(UniqueType::FoodConsumptionBySpecialists, Some(&self.city.state), true) {
            if self.city.matches_filter(&unique.params[1], None, false) {
                food_eaten_by_specialists *= unique.params[0].parse::<f32>().unwrap_or(0.0).to_percent();
            }
        }

        food_eaten -= 2.0 * self.city.population.get_number_of_specialists() as f32 - food_eaten_by_specialists;
        food_eaten
    }
}
