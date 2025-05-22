use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::models::city::{City, CityConstructions};
use crate::models::civilization::Civilization;
use crate::models::conditionals::Conditionals;
use crate::models::counter::Counter;
use crate::models::game_info::GameInfo;
use crate::models::multi_filter::MultiFilter;
use crate::models::purchase_reason::PurchaseReason;
use crate::models::rejection_reason::{RejectionReason, RejectionReasonType};
use crate::models::ruleset::{
    tile::{ResourceType, TileImprovement},
    unique::{LocalUniqueCache, StateForConditionals, Unique, UniqueTarget, UniqueType},
    unit::BaseUnit,
    INonPerpetualConstruction, Ruleset, RulesetObject, RulesetStatsObject,
};
use crate::models::stats::{Stat, Stats};
use crate::ui::object_descriptions::BuildingDescriptions;
use crate::utils::string_utils::{get_need_more_amount_string, to_percent};

/// Replaces all occurrences of `old_building_name` in `city_constructions` with `new_building_name`
/// if the former is not contained in the ruleset.
pub fn change_building_name_if_not_in_ruleset(
    rule_set: &Ruleset,
    city_constructions: &mut CityConstructions,
    old_building_name: &str,
    new_building_name: &str,
) {
    if rule_set.buildings.contains_key(old_building_name) {
        return;
    }

    // Replace in built buildings
    if city_constructions.is_built(old_building_name) {
        city_constructions.remove_building(old_building_name);
        city_constructions.add_building(new_building_name);
    }

    // Replace in construction queue
    if !city_constructions.is_built(new_building_name)
        && !city_constructions
            .construction_queue
            .contains(&new_building_name.to_string())
    {
        city_constructions.construction_queue = city_constructions
            .construction_queue
            .iter()
            .map(|it| {
                if it == old_building_name {
                    new_building_name.to_string()
                } else {
                    it.clone()
                }
            })
            .collect();
    } else {
        city_constructions
            .construction_queue
            .retain(|it| it != old_building_name);
    }

    // Replace in in-progress constructions
    if city_constructions
        .in_progress_constructions
        .contains_key(old_building_name)
    {
        if !city_constructions.is_built(new_building_name)
            && !city_constructions
                .in_progress_constructions
                .contains_key(new_building_name)
        {
            let value = city_constructions
                .in_progress_constructions
                .get(old_building_name)
                .unwrap()
                .clone();
            city_constructions
                .in_progress_constructions
                .insert(new_building_name.to_string(), value);
        }
        city_constructions
            .in_progress_constructions
            .remove(old_building_name);
    }
}

/// Represents a building in the game.
pub struct Building {
    /// Base stats object that this building extends
    base: RulesetStatsObject,

    /// The name of the building
    name: String,

    /// The uniques associated with this building
    uniques: Vec<String>,

    /// The required technology to build this building
    required_tech: Option<String>,

    /// The base cost of the building
    cost: i32,

    /// The maintenance cost of the building
    maintenance: i32,

    /// Percentage stat bonuses for this building
    percent_stat_bonus: Option<Stats>,

    /// Specialist slots provided by this building
    specialist_slots: Counter<String>,

    /// Great person points provided by this building
    great_person_points: Counter<String>,

    /// Extra cost percentage when purchasing
    hurry_cost_modifier: i32,

    /// Whether this building is a wonder
    is_wonder: bool,

    /// Whether this building is a national wonder
    is_national_wonder: bool,

    /// The building that this building requires to be built
    required_building: Option<String>,

    /// A strategic resource that will be consumed by this building
    required_resource: Option<String>,

    /// This Building can only be built if one of these resources is nearby - it must be improved!
    required_nearby_improved_resources: Option<Vec<String>>,

    /// The city strength provided by this building
    city_strength: i32,

    /// The city health provided by this building
    city_health: i32,

    /// The building that this building replaces
    replaces: Option<String>,

    /// The nation that this building is unique to
    unique_to: Option<String>,

    /// The quote associated with this building
    quote: String,

    /// The replacement text for uniques
    replacement_text_for_uniques: String,

    /// The ruleset this building belongs to
    ruleset: Option<Ruleset>,

    /// Cached filter match results
    cached_matches_filter_result: HashMap<String, bool>,

    /// Whether this building has the CreatesOneImprovement unique
    has_creates_one_improvement_unique: Option<bool>,

    /// The improvement to create
    improvement_to_create: Option<TileImprovement>,
}

impl Building {
    /// Creates a new empty building
    pub fn new() -> Self {
        Self {
            base: RulesetStatsObject::new(),
            name: String::new(),
            uniques: Vec::new(),
            required_tech: None,
            cost: -1,
            maintenance: 0,
            percent_stat_bonus: None,
            specialist_slots: Counter::new(),
            great_person_points: Counter::new(),
            hurry_cost_modifier: 0,
            is_wonder: false,
            is_national_wonder: false,
            required_building: None,
            required_resource: None,
            required_nearby_improved_resources: None,
            city_strength: 0,
            city_health: 0,
            replaces: None,
            unique_to: None,
            quote: String::new(),
            replacement_text_for_uniques: String::new(),
            ruleset: None,
            cached_matches_filter_result: HashMap::new(),
            has_creates_one_improvement_unique: None,
            improvement_to_create: None,
        }
    }

    /// Returns whether this building is any kind of wonder
    pub fn is_any_wonder(&self) -> bool {
        self.is_wonder || self.is_national_wonder
    }

    /// Returns a new counter of specialists
    pub fn new_specialists(&self) -> Counter<String> {
        self.specialist_slots.clone()
    }

    /// Gets the unique target for this building
    pub fn get_unique_target(&self) -> UniqueTarget {
        if self.is_any_wonder() {
            UniqueTarget::Wonder
        } else {
            UniqueTarget::Building
        }
    }

    /// Creates a link for this building
    pub fn make_link(&self) -> String {
        if self.is_any_wonder() {
            format!("Wonder/{}", self.name)
        } else {
            format!("Building/{}", self.name)
        }
    }

    /// Gets a short description of this building
    pub fn get_short_description(
        &self,
        multiline: bool,
        unique_inclusion_filter: Option<&dyn Fn(&Unique) -> bool>,
    ) -> String {
        BuildingDescriptions::get_short_description(self, multiline, unique_inclusion_filter)
    }

    /// Gets a description of this building for a city
    pub fn get_description(&self, city: &City, show_additional_info: bool) -> String {
        BuildingDescriptions::get_description(self, city, show_additional_info)
    }

    /// Gets civilopedia text lines for this building
    pub fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        BuildingDescriptions::get_civilopedia_text_lines(self, ruleset)
    }

    /// Checks if this building is unavailable by settings
    pub fn is_unavailable_by_settings(&self, game_info: &GameInfo) -> bool {
        if self.base.is_unavailable_by_settings(game_info) {
            return true;
        }

        if !game_info.game_parameters.nuclear_weapons_enabled
            && self.has_unique(UniqueType::EnablesNuclearWeapons)
        {
            return true;
        }

        self.is_hidden_by_starting_era(game_info)
    }

    /// Checks if this building is hidden by the starting era
    fn is_hidden_by_starting_era(&self, game_info: &GameInfo) -> bool {
        if !self.is_wonder {
            return false;
        }

        // Do not rely on self.ruleset or unit tests break
        let starting_era = game_info
            .ruleset
            .eras
            .get(&game_info.game_parameters.starting_era)?;

        starting_era.starting_obsolete_wonders.contains(&self.name)
    }

    /// Gets the stats for this building in a city
    pub fn get_stats(&self, city: &City, local_unique_cache: &LocalUniqueCache) -> Stats {
        // Calls the clone function of the NamedStats this class is derived from, not a clone function of this class
        let mut stats = self.base.clone_stats();

        let conditional_state = city.state();

        for unique in
            local_unique_cache.for_city_get_matching_uniques(city, UniqueType::StatsFromObject)
        {
            if !self.matches_filter(&unique.params[1], conditional_state) {
                continue;
            }
            stats.add(&unique.stats);
        }

        for unique in self.get_matching_uniques(UniqueType::Stats, conditional_state) {
            stats.add(&unique.stats);
        }

        if !self.is_wonder {
            for unique in local_unique_cache
                .for_city_get_matching_uniques(city, UniqueType::StatsFromBuildings)
            {
                if self.matches_filter(&unique.params[1], conditional_state) {
                    stats.add(&unique.stats);
                }
            }
        }

        stats
    }

    /// Gets the stat percentage bonuses for this building in a city
    pub fn get_stat_percentage_bonuses(
        &self,
        city: Option<&City>,
        local_unique_cache: &LocalUniqueCache,
    ) -> Stats {
        let mut stats = self.percent_stat_bonus.clone().unwrap_or_else(Stats::new);

        if city.is_none() {
            return stats; // Initial stats
        }

        let city = city.unwrap();
        let conditional_state = city.state();

        for unique in local_unique_cache
            .for_city_get_matching_uniques(city, UniqueType::StatPercentFromObject)
        {
            if self.matches_filter(&unique.params[2], conditional_state) {
                stats.add(
                    Stat::value_of(&unique.params[1]),
                    unique.params[0].parse::<f32>().unwrap_or(0.0),
                );
            }
        }

        for unique in local_unique_cache
            .for_city_get_matching_uniques(city, UniqueType::AllStatsPercentFromObject)
        {
            if !self.matches_filter(&unique.params[1], conditional_state) {
                continue;
            }

            for stat in Stat::entries() {
                stats.add(stat, unique.params[0].parse::<f32>().unwrap_or(0.0));
            }
        }

        stats
    }

    /// Gets the production cost for this building
    pub fn get_production_cost(&self, civ_info: &Civilization, city: Option<&City>) -> i32 {
        let mut production_cost = self.cost as f32;
        let state_for_conditionals = city.map(|c| c.state()).unwrap_or_else(|| civ_info.state());

        for unique in
            self.get_matching_uniques(UniqueType::CostIncreasesWhenBuilt, state_for_conditionals)
        {
            production_cost += civ_info
                .civ_constructions
                .built_items_with_increasing_cost
                .get(&self.name)
                .unwrap_or(&0)
                * unique.params[0].parse::<i32>().unwrap_or(0) as f32;
        }

        for unique in
            self.get_matching_uniques(UniqueType::CostIncreasesPerCity, state_for_conditionals)
        {
            production_cost +=
                civ_info.cities.len() as f32 * unique.params[0].parse::<i32>().unwrap_or(0) as f32;
        }

        for unique in
            self.get_matching_uniques(UniqueType::CostPercentageChange, state_for_conditionals)
        {
            production_cost *= to_percent(&unique.params[0]);
        }

        if civ_info.is_city_state() {
            production_cost *= 1.5;
        } else if civ_info.is_human() {
            if !self.is_wonder {
                production_cost *= civ_info.get_difficulty().building_cost_modifier;
            }
        } else {
            production_cost *= if self.is_wonder {
                civ_info.game_info.get_difficulty().ai_wonder_cost_modifier
            } else {
                civ_info
                    .game_info
                    .get_difficulty()
                    .ai_building_cost_modifier
            };
        }

        production_cost *= civ_info.game_info.speed.production_cost_modifier;

        production_cost as i32
    }

    /// Checks if this building can be purchased with a specific stat
    pub fn can_be_purchased_with_stat(&self, city: Option<&City>, stat: Stat) -> bool {
        let purchase_reason = self.can_be_purchased_with_stat_reasons(None, stat);

        if purchase_reason != PurchaseReason::UniqueAllowed
            && stat == Stat::Gold
            && self.is_any_wonder()
        {
            return false;
        }

        if city.is_none() {
            return purchase_reason.purchasable();
        }

        let city = city.unwrap();
        let conditional_state = city.state();

        city.get_matching_uniques(UniqueType::BuyBuildingsIncreasingCost, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[2] == stat.name()
                    && self.matches_filter(&unique.params[0], conditional_state)
                    && city.matches_filter(&unique.params[3])
            })
            || city
                .get_matching_uniques(UniqueType::BuyBuildingsByProductionCost, conditional_state)
                .iter()
                .any(|unique| {
                    unique.params[1] == stat.name()
                        && self.matches_filter(&unique.params[0], conditional_state)
                })
            || city
                .get_matching_uniques(UniqueType::BuyBuildingsWithStat, conditional_state)
                .iter()
                .any(|unique| {
                    unique.params[1] == stat.name()
                        && self.matches_filter(&unique.params[0], conditional_state)
                        && city.matches_filter(&unique.params[2])
                })
            || city
                .get_matching_uniques(UniqueType::BuyBuildingsForAmountStat, conditional_state)
                .iter()
                .any(|unique| {
                    unique.params[2] == stat.name()
                        && self.matches_filter(&unique.params[0], conditional_state)
                        && city.matches_filter(&unique.params[3])
                })
            || self.base.can_be_purchased_with_stat(city, stat)
    }

    /// Gets the base buy cost for this building in a city with a specific stat
    pub fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        let conditional_state = city.state();

        let mut costs = Vec::new();

        // Add base cost if available
        if let Some(base_cost) = self.base.get_base_buy_cost(city, stat) {
            costs.push(base_cost);
        }

        // Add costs from BuyBuildingsIncreasingCost uniques
        for unique in city
            .get_matching_uniques(UniqueType::BuyBuildingsIncreasingCost, conditional_state)
            .iter()
            .filter(|unique| {
                unique.params[2] == stat.name()
                    && self.matches_filter(&unique.params[0], conditional_state)
                    && city.matches_filter(&unique.params[3])
            })
        {
            let base_cost = unique.params[1].parse::<i32>().unwrap_or(0);
            let increment = unique.params[4].parse::<i32>().unwrap_or(0);
            let count = city
                .civ
                .civ_constructions
                .bought_items_with_increasing_price
                .get(&self.name)
                .unwrap_or(&0);

            let cost = self
                .get_cost_for_constructions_increasing_in_price(base_cost, increment, *count)
                * city
                    .civ
                    .game_info
                    .speed
                    .stat_cost_modifiers
                    .get(&stat)
                    .unwrap_or(&1.0);

            costs.push(cost);
        }

        // Add costs from BuyBuildingsByProductionCost uniques
        for unique in city
            .get_matching_uniques(UniqueType::BuyBuildingsByProductionCost, conditional_state)
            .iter()
            .filter(|unique| {
                unique.params[1] == stat.name()
                    && self.matches_filter(&unique.params[0], conditional_state)
            })
        {
            let cost = (self.get_production_cost(&city.civ, Some(city))
                * unique.params[2].parse::<i32>().unwrap_or(0)) as f32;
            costs.push(cost);
        }

        // Add cost if BuyBuildingsWithStat unique exists
        if city
            .get_matching_uniques(UniqueType::BuyBuildingsWithStat, conditional_state)
            .iter()
            .any(|unique| {
                unique.params[1] == stat.name()
                    && self.matches_filter(&unique.params[0], conditional_state)
                    && city.matches_filter(&unique.params[2])
            })
        {
            let cost = city.civ.get_era().base_unit_buy_cost as f32
                * city
                    .civ
                    .game_info
                    .speed
                    .stat_cost_modifiers
                    .get(&stat)
                    .unwrap_or(&1.0);
            costs.push(cost);
        }

        // Add costs from BuyBuildingsForAmountStat uniques
        for unique in city
            .get_matching_uniques(UniqueType::BuyBuildingsForAmountStat, conditional_state)
            .iter()
            .filter(|unique| {
                unique.params[2] == stat.name()
                    && self.matches_filter(&unique.params[0], conditional_state)
                    && city.matches_filter(&unique.params[3])
            })
        {
            let cost = unique.params[1].parse::<i32>().unwrap_or(0) as f32
                * city
                    .civ
                    .game_info
                    .speed
                    .stat_cost_modifiers
                    .get(&stat)
                    .unwrap_or(&1.0);
            costs.push(cost);
        }

        // Return the minimum cost if any costs were found
        if costs.is_empty() {
            None
        } else {
            Some(costs.iter().fold(f32::INFINITY, |a, &b| a.min(b)))
        }
    }

    /// Gets the stat buy cost for this building in a city with a specific stat
    pub fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32> {
        let mut cost = self.get_base_buy_cost(city, stat)?.to_f64()?;
        let conditional_state = city.state();

        for unique in city.get_matching_uniques(UniqueType::BuyItemsDiscount) {
            if stat.name() == unique.params[0] {
                cost *= to_percent(&unique.params[1]);
            }
        }

        for unique in city.get_matching_uniques(UniqueType::BuyBuildingsDiscount) {
            if stat.name() == unique.params[0]
                && self.matches_filter(&unique.params[1], conditional_state)
            {
                cost *= to_percent(&unique.params[2]);
            }
        }

        Some((cost / 10.0) as i32 * 10)
    }

    /// Checks if this building should be displayed in a city's constructions
    pub fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        if city_constructions.is_being_constructed_or_enqueued(&self.name) {
            return false;
        }

        for unique in self.get_matching_uniques(UniqueType::MaxNumberBuildable) {
            if city_constructions
                .city
                .civ
                .civ_constructions
                .count_constructed_objects(self)
                >= unique.params[0].parse::<i32>().unwrap_or(0)
            {
                return false;
            }
        }

        let rejection_reasons: Vec<RejectionReason> =
            self.get_rejection_reasons(city_constructions).collect();

        if self.has_unique(
            UniqueType::ShowsWhenUnbuilable,
            city_constructions.city.state(),
        ) && !rejection_reasons
            .iter()
            .any(|reason| reason.is_never_visible())
        {
            return true;
        }

        if rejection_reasons
            .iter()
            .any(|reason| reason.type_() == RejectionReasonType::RequiresBuildingInSomeCities)
            && city_constructions
                .city
                .civ
                .game_info
                .game_parameters
                .one_city_challenge
        {
            return false; // You will never be able to get more cities, this building is effectively disabled
        }

        if !rejection_reasons.iter().any(|reason| !reason.should_show()) {
            return true;
        }

        self.can_be_purchased_with_any_stat(city_constructions.city)
            && rejection_reasons
                .iter()
                .all(|reason| reason.type_() == RejectionReasonType::Unbuildable)
    }

    /// Gets the rejection reasons for this building in a city's constructions
    pub fn get_rejection_reasons(
        &self,
        city_constructions: &CityConstructions,
    ) -> impl Iterator<Item = RejectionReason> {
        let city = &city_constructions.city;
        let city_center = city.get_center_tile();
        let civ = &city.civ;
        let state_for_conditionals = city.state();

        let mut reasons = Vec::new();

        if city_constructions.is_built(&self.name) {
            reasons.push(RejectionReasonType::AlreadyBuilt.to_instance());
        }

        if self.is_unavailable_by_settings(&civ.game_info) {
            // Repeat the starting era test isHiddenBySettings already did to change the RejectionReasonType
            if self.is_hidden_by_starting_era(&civ.game_info) {
                reasons.push(RejectionReasonType::WonderDisabledEra.to_instance());
            } else {
                reasons.push(RejectionReasonType::DisabledBySetting.to_instance());
            }
        }

        for unique in &self.unique_objects() {
            // Skip uniques that don't have conditionals apply
            // EXCEPT for [UniqueType.OnlyAvailable] and [UniqueType.CanOnlyBeBuiltInCertainCities]
            // since they trigger (reject) only if conditionals ARE NOT met
            if unique.type_() != UniqueType::OnlyAvailable
                && unique.type_() != UniqueType::CanOnlyBeBuiltWhen
                && !unique.conditionals_apply(state_for_conditionals)
            {
                continue;
            }

            match unique.type_() {
                // For buildings that are created as side effects of other things, and not directly built,
                // or for buildings that can only be bought
                UniqueType::Unbuildable => {
                    reasons.push(RejectionReasonType::Unbuildable.to_instance());
                }

                UniqueType::OnlyAvailable => {
                    reasons.extend(self.not_met_rejections(unique, city_constructions));
                }

                UniqueType::CanOnlyBeBuiltWhen => {
                    reasons.extend(self.not_met_rejections(unique, city_constructions, true));
                }

                UniqueType::Unavailable => {
                    reasons.push(RejectionReasonType::ShouldNotBeDisplayed.to_instance());
                }

                UniqueType::RequiresPopulation => {
                    let required_population = unique.params[0].parse::<i32>().unwrap_or(0);
                    if required_population > city.population.population {
                        reasons.push(
                            RejectionReasonType::PopulationRequirement.to_instance(&unique.text),
                        );
                    }
                }

                UniqueType::MustBeOn => {
                    if !city_center.matches_terrain_filter(&unique.params[0], civ) {
                        reasons.push(RejectionReasonType::MustBeOnTile.to_instance(&unique.text));
                    }
                }

                UniqueType::MustNotBeOn => {
                    if city_center.matches_terrain_filter(&unique.params[0], civ) {
                        reasons
                            .push(RejectionReasonType::MustNotBeOnTile.to_instance(&unique.text));
                    }
                }

                UniqueType::MustBeNextTo => {
                    if !city_center.is_adjacent_to(&unique.params[0], civ)
                        && !city_center.matches_filter(&unique.params[0], civ)
                    {
                        reasons
                            .push(RejectionReasonType::MustBeNextToTile.to_instance(&unique.text));
                    }
                }

                UniqueType::MustNotBeNextTo => {
                    if city_center
                        .get_tiles_in_distance(1)
                        .iter()
                        .any(|tile| tile.matches_filter(&unique.params[0], civ))
                    {
                        reasons.push(
                            RejectionReasonType::MustNotBeNextToTile.to_instance(&unique.text),
                        );
                    }
                }

                UniqueType::MustHaveOwnedWithinTiles => {
                    let distance = unique.params[1].parse::<i32>().unwrap_or(0);
                    if city_center
                        .get_tiles_in_distance(distance)
                        .iter()
                        .none(|tile| {
                            tile.matches_filter(&unique.params[0], civ) && tile.get_owner() == civ
                        })
                    {
                        reasons.push(RejectionReasonType::MustOwnTile.to_instance(&unique.text));
                    }
                }

                UniqueType::ObsoleteWith => {
                    if civ.tech.is_researched(&unique.params[0]) {
                        reasons.push(RejectionReasonType::Obsoleted.to_instance(&unique.text));
                    }
                }

                UniqueType::MaxNumberBuildable => {
                    let max_count = unique.params[0].parse::<i32>().unwrap_or(0);
                    if civ.civ_constructions.count_constructed_objects(self) >= max_count {
                        reasons.push(RejectionReasonType::MaxNumberBuildable.to_instance());
                    }
                }

                // To be replaced with `Only available <after [Apollo Project] has been build>`
                UniqueType::SpaceshipPart => {
                    if !civ.has_unique(UniqueType::EnablesConstructionOfSpaceshipParts) {
                        reasons.push(
                            RejectionReasonType::RequiresBuildingInSomeCity
                                .to_instance("Apollo project not built!"),
                        );
                    }
                }

                UniqueType::HiddenBeforeAmountPolicies => {
                    let required_policies = unique.params[0].parse::<i32>().unwrap_or(0);
                    if city_constructions
                        .city
                        .civ
                        .get_completed_policy_branches_count()
                        < required_policies
                    {
                        reasons.push(
                            RejectionReasonType::MorePolicyBranches.to_instance(&unique.text),
                        );
                    }
                }

                _ => {}
            }
        }

        if let Some(unique_to) = &self.unique_to {
            if !civ.matches_filter(unique_to, state_for_conditionals) {
                reasons.push(
                    RejectionReasonType::UniqueToOtherNation
                        .to_instance(&format!("Unique to {}", unique_to)),
                );
            }
        }

        if civ.cache.unique_buildings.iter().any(|building| {
            building
                .replaces
                .as_ref()
                .map_or(false, |r| r == &self.name)
        }) {
            reasons.push(RejectionReasonType::ReplacedByOurUnique.to_instance());
        }

        for required_tech in self.required_techs() {
            if !civ.tech.is_researched(required_tech) {
                reasons.push(
                    RejectionReasonType::RequiresTech
                        .to_instance(&format!("{} not researched!", required_tech)),
                );
            }
        }

        // All Wonders
        if self.is_any_wonder() {
            if civ.cities.iter().any(|c| {
                c != city_constructions.city
                    && c.city_constructions
                        .is_being_constructed_or_enqueued(&self.name)
            }) {
                reasons.push(RejectionReasonType::WonderBeingBuiltElsewhere.to_instance());
            }

            if civ.is_city_state() {
                reasons.push(RejectionReasonType::CityStateWonder.to_instance());
            }

            if city_constructions.city.is_puppet {
                reasons.push(RejectionReasonType::PuppetWonder.to_instance());
            }
        }

        // World Wonders
        if self.is_wonder {
            if civ
                .game_info
                .get_cities()
                .iter()
                .any(|c| c.city_constructions.is_built(&self.name))
            {
                reasons.push(RejectionReasonType::WonderAlreadyBuilt.to_instance());
            }
        }

        // National Wonders
        if self.is_national_wonder {
            if civ
                .cities
                .iter()
                .any(|c| c.city_constructions.is_built(&self.name))
            {
                reasons.push(RejectionReasonType::NationalWonderAlreadyBuilt.to_instance());
            }
        }

        if let Some(required_building) = &self.required_building {
            if !city_constructions.contains_building_or_equivalent(required_building) {
                let equivalent_building = civ.get_equivalent_building(required_building);
                reasons.push(RejectionReasonType::RequiresBuildingInThisCity.to_instance(
                    &format!("Requires a [{}] in this city", equivalent_building),
                ));
            }
        }

        for (resource_name, required_amount) in
            self.get_resource_requirements_per_turn(state_for_conditionals)
        {
            let available_amount = city_constructions
                .city
                .get_available_resource_amount(&resource_name);
            if available_amount < required_amount {
                reasons.push(RejectionReasonType::ConsumesResources.to_instance(
                    &get_need_more_amount_string(
                        &resource_name,
                        required_amount - available_amount,
                    ),
                ));
            }
        }

        // If we've already paid the unit costs, we don't need to pay it again
        if city_constructions.get_work_done(&self.name) == 0 {
            for (resource_name, amount) in
                self.get_stockpiled_resource_requirements(state_for_conditionals)
            {
                let available_resources = city_constructions
                    .city
                    .get_available_resource_amount(&resource_name);
                if available_resources < amount {
                    reasons.push(RejectionReasonType::ConsumesResources.to_instance(
                        &get_need_more_amount_string(&resource_name, amount - available_resources),
                    ));
                }
            }
        }

        if let Some(required_nearby_improved_resources) = &self.required_nearby_improved_resources {
            let contains_resource_with_improvement = city_constructions
                .city
                .get_workable_tiles()
                .iter()
                .any(|tile| {
                    tile.resource.is_some()
                        && required_nearby_improved_resources.contains(&tile.resource.unwrap())
                        && tile.get_owner() == civ
                        && ((tile.get_unpillaged_improvement().is_some()
                            && tile
                                .tile_resource
                                .is_improved_by(tile.improvement.as_ref().unwrap()))
                            || tile.is_city_center()
                            || (tile
                                .get_unpillaged_tile_improvement()
                                .map_or(false, |imp| imp.is_great_improvement())
                                && tile.tile_resource.resource_type == ResourceType::Strategic))
                });

            if !contains_resource_with_improvement {
                reasons.push(
                    RejectionReasonType::RequiresNearbyResource.to_instance(&format!(
                        "Nearby {:?} required",
                        required_nearby_improved_resources
                    )),
                );
            }
        }

        reasons.into_iter()
    }

    /// Handles inverted conditional rejections and cumulative conditional reporting
    fn not_met_rejections(
        &self,
        unique: &Unique,
        city_constructions: &CityConstructions,
        built: bool,
    ) -> Vec<RejectionReason> {
        let civ = &city_constructions.city.civ;
        let mut reasons = Vec::new();

        for conditional in &unique.modifiers() {
            // We yield a rejection only when conditionals are NOT met
            if Conditionals::conditional_applies(
                unique,
                conditional,
                city_constructions.city.state(),
            ) {
                continue;
            }

            match conditional.type_() {
                UniqueType::ConditionalBuildingBuiltAmount => {
                    let building = civ.get_equivalent_building(&conditional.params[0]).name;
                    let amount = conditional.params[1].parse::<i32>().unwrap_or(0);
                    let city_filter = &conditional.params[2];

                    let number_of_cities = civ
                        .cities
                        .iter()
                        .filter(|city| {
                            city.city_constructions
                                .contains_building_or_equivalent(&building)
                                && city.matches_filter(city_filter)
                        })
                        .count();

                    if number_of_cities < amount as usize {
                        reasons.push(
                            RejectionReasonType::RequiresBuildingInSomeCities.to_instance(
                                &format!(
                                    "Requires a [{}] in at least [{}] of [{}] cities ({}/{})",
                                    building,
                                    amount,
                                    city_filter,
                                    number_of_cities,
                                    number_of_cities
                                ),
                            ),
                        );
                    }
                }

                UniqueType::ConditionalBuildingBuiltAll => {
                    let building = civ.get_equivalent_building(&conditional.params[0]).name;
                    let city_filter = &conditional.params[1];

                    if civ.cities.iter().any(|city| {
                        city.matches_filter(city_filter)
                            && !city.is_puppet
                            && !city
                                .city_constructions
                                .contains_building_or_equivalent(&building)
                    }) {
                        reasons.push(
                            RejectionReasonType::RequiresBuildingInAllCities.to_instance(&format!(
                                "Requires a [{}] in all [{}] cities",
                                building, city_filter
                            )),
                        );
                    }
                }

                _ => {
                    if built {
                        reasons.push(
                            RejectionReasonType::CanOnlyBeBuiltInSpecificCities
                                .to_instance(&unique.text),
                        );
                    } else {
                        reasons.push(RejectionReasonType::ShouldNotBeDisplayed.to_instance());
                    }
                }
            }
        }

        reasons
    }

    /// Checks if this building is buildable in a city's constructions
    pub fn is_buildable(&self, city_constructions: &CityConstructions) -> bool {
        self.get_rejection_reasons(city_constructions)
            .next()
            .is_none()
    }

    /// Constructs this building in a city's constructions
    pub fn construct(&self, city_constructions: &mut CityConstructions) {
        let civ_info = &city_constructions.city.civ;

        if civ_info.game_info.space_resources.contains(&self.name) {
            civ_info
                .victory_manager
                .currents_spaceship_parts
                .add(&self.name, 1);
        }

        city_constructions.add_building(self);
    }

    /// Implements [UniqueParameterType.BuildingFilter]
    pub fn matches_filter(&self, filter: &str, state: Option<&StateForConditionals>) -> bool {
        MultiFilter::multi_filter(filter, |it| {
            *self
                .cached_matches_filter_result
                .entry(it.to_string())
                .or_insert_with(|| {
                    self.matches_single_filter(it)
                        || state.is_some() && self.has_unique(it, state.unwrap())
                        || state.is_none() && self.has_tag_unique(it)
                })
        })
    }

    /// Checks if this building matches a single filter
    fn matches_single_filter(&self, filter: &str) -> bool {
        // All cases are constants for performance
        match filter {
            "all" | "All" => true,
            "Building" | "Buildings" => !self.is_any_wonder(),
            "Wonder" | "Wonders" => self.is_any_wonder(),
            "National Wonder" | "National" => self.is_national_wonder,
            "World Wonder" | "World" => self.is_wonder,
            _ => {
                if filter == self.name {
                    return true;
                }

                if let Some(replaces) = &self.replaces {
                    if filter == replaces {
                        return true;
                    }
                }

                if let Some(ruleset) = &self.ruleset {
                    // False when loading ruleset and checking buildingsToRemove
                    for required_tech in self.required_techs() {
                        if let Some(tech) = ruleset.technologies.get(required_tech) {
                            if tech.matches_filter(filter, false) {
                                return true;
                            }
                        }
                    }
                }

                if let Some(stat) = Stat::safe_value_of(filter) {
                    return self.is_stat_related(stat, None);
                }

                false
            }
        }
    }

    /// Checks if this building is related to a specific stat
    pub fn is_stat_related(&self, stat: Stat, city: Option<&City>) -> bool {
        if let Some(city) = city {
            if self
                .get_stats(city, &LocalUniqueCache::new(false))
                .get(stat)
                > 0.0
            {
                return true;
            }

            if self
                .get_stat_percentage_bonuses(Some(city), &LocalUniqueCache::new(false))
                .get(stat)
                > 0.0
            {
                return true;
            }
        } else {
            if self.base.get(stat) > 0.0 {
                return true;
            }

            if self
                .get_matching_uniques(UniqueType::Stats, None)
                .iter()
                .any(|unique| unique.stats.get(stat) > 0.0)
            {
                return true;
            }

            if self
                .get_stat_percentage_bonuses(None, &LocalUniqueCache::new(false))
                .get(stat)
                > 0.0
            {
                return true;
            }
        }

        if self
            .get_matching_uniques(UniqueType::StatsFromTiles, None)
            .iter()
            .any(|unique| unique.stats.get(stat) > 0.0)
        {
            return true;
        }

        if self
            .get_matching_uniques(UniqueType::StatsPerPopulation, None)
            .iter()
            .any(|unique| unique.stats.get(stat) > 0.0)
        {
            return true;
        }

        if stat == Stat::Happiness && self.has_unique(UniqueType::RemoveAnnexUnhappiness) {
            return true;
        }

        false
    }

    /// Checks if this building has the CreatesOneImprovement unique
    pub fn has_create_one_improvement_unique(&self) -> bool {
        if self.has_creates_one_improvement_unique.is_none() {
            self.has_creates_one_improvement_unique =
                Some(self.has_unique(UniqueType::CreatesOneImprovement));
        }

        self.has_creates_one_improvement_unique.unwrap_or(false)
    }

    /// Gets the improvement to create for this building
    fn get_improvement_to_create(&self, ruleset: &Ruleset) -> Option<&TileImprovement> {
        if !self.has_create_one_improvement_unique() {
            return None;
        }

        if self.improvement_to_create.is_none() {
            let improvement_unique = self
                .get_matching_uniques(UniqueType::CreatesOneImprovement, None)
                .first()?;
            self.improvement_to_create = Some(
                ruleset
                    .tile_improvements
                    .get(&improvement_unique.params[0])?
                    .clone(),
            );
        }

        self.improvement_to_create.as_ref()
    }

    /// Gets the improvement to create for this building for a civilization
    pub fn get_improvement_to_create_for_civ(
        &self,
        ruleset: &Ruleset,
        civ_info: &Civilization,
    ) -> Option<&TileImprovement> {
        let improvement = self.get_improvement_to_create(ruleset)?;
        Some(civ_info.get_equivalent_tile_improvement(improvement))
    }

    /// Checks if this building is sellable
    pub fn is_sellable(&self) -> bool {
        !self.is_any_wonder() && !self.has_unique(UniqueType::Unsellable)
    }

    /// Gets the resource requirements per turn for this building
    pub fn get_resource_requirements_per_turn(
        &self,
        state: Option<&StateForConditionals>,
    ) -> HashMap<String, i32> {
        let uniques = self.get_matching_uniques(
            UniqueType::ConsumesResources,
            state.unwrap_or(&StateForConditionals::empty_state()),
        );

        if uniques.is_empty() && self.required_resource.is_none() {
            return HashMap::new();
        }

        let mut resource_requirements = HashMap::new();

        if let Some(required_resource) = &self.required_resource {
            resource_requirements.insert(required_resource.clone(), 1);
        }

        for unique in uniques {
            let resource_name = unique.params[1].clone();
            let amount = unique.params[0].parse::<i32>().unwrap_or(0);

            *resource_requirements.entry(resource_name).or_insert(0) += amount;
        }

        resource_requirements
    }

    /// Gets the stockpiled resource requirements for this building
    pub fn get_stockpiled_resource_requirements(
        &self,
        state: Option<&StateForConditionals>,
    ) -> HashMap<String, i32> {
        let uniques = self.get_matching_uniques(
            UniqueType::ConsumesResourcesOnConstruction,
            state.unwrap_or(&StateForConditionals::empty_state()),
        );

        if uniques.is_empty() {
            return HashMap::new();
        }

        let mut resource_requirements = HashMap::new();

        for unique in uniques {
            let resource_name = unique.params[1].clone();
            let amount = unique.params[0].parse::<i32>().unwrap_or(0);

            *resource_requirements.entry(resource_name).or_insert(0) += amount;
        }

        resource_requirements
    }

    /// Gets the cost for constructions increasing in price
    fn get_cost_for_constructions_increasing_in_price(
        &self,
        base_cost: i32,
        increment: i32,
        count: i32,
    ) -> f32 {
        base_cost as f32 + (count as f32 * increment as f32)
    }
}

impl RulesetObject for Building {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn uniques(&self) -> &[String] {
        &self.uniques
    }

    fn uniques_mut(&mut self) -> &mut Vec<String> {
        &mut self.uniques
    }

    fn get_unique_target(&self) -> UniqueTarget {
        self.get_unique_target()
    }

    fn make_link(&self) -> String {
        self.make_link()
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<String> {
        self.get_civilopedia_text_lines(ruleset)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl INonPerpetualConstruction for Building {
    fn required_techs(&self) -> Vec<String> {
        self.base.required_techs()
    }

    fn can_be_purchased_with_stat_reasons(
        &self,
        city: Option<&City>,
        stat: Stat,
    ) -> PurchaseReason {
        self.base.can_be_purchased_with_stat_reasons(city, stat)
    }

    fn can_be_purchased_with_any_stat(&self, city: &City) -> bool {
        self.base.can_be_purchased_with_any_stat(city)
    }

    fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        self.base.get_base_buy_cost(city, stat)
    }

    fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32> {
        self.base.get_stat_buy_cost(city, stat)
    }

    fn get_resource_requirements_per_turn(
        &self,
        state: Option<&StateForConditionals>,
    ) -> HashMap<String, i32> {
        self.get_resource_requirements_per_turn(state)
    }

    fn get_stockpiled_resource_requirements(
        &self,
        state: Option<&StateForConditionals>,
    ) -> HashMap<String, i32> {
        self.get_stockpiled_resource_requirements(state)
    }
}

impl fmt::Display for Building {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_building_creation() {
        let building = Building::new();
        assert_eq!(building.name, "");
        assert_eq!(building.cost, -1);
        assert_eq!(building.maintenance, 0);
        assert!(!building.is_wonder);
        assert!(!building.is_national_wonder);
    }

    #[test]
    fn test_is_any_wonder() {
        let mut building = Building::new();
        assert!(!building.is_any_wonder());

        building.is_wonder = true;
        assert!(building.is_any_wonder());

        building.is_wonder = false;
        building.is_national_wonder = true;
        assert!(building.is_any_wonder());
    }

    #[test]
    fn test_matches_filter() {
        let mut building = Building::new();
        building.name = "Barracks".to_string();
        building.is_wonder = false;
        building.is_national_wonder = false;

        assert!(building.matches_filter("all", None));
        assert!(building.matches_filter("Building", None));
        assert!(!building.matches_filter("Wonder", None));
        assert!(building.matches_filter("Barracks", None));

        building.is_wonder = true;
        assert!(!building.matches_filter("Building", None));
        assert!(building.matches_filter("Wonder", None));
        assert!(building.matches_filter("World Wonder", None));
    }
}
