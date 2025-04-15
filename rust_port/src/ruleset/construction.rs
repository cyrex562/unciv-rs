use std::collections::{HashMap, HashSet};
use std::fmt;
use std::f64;

use crate::models::city::{City, CityConstructions};
use crate::models::civilization::Civilization;
use crate::models::counter::Counter;
use crate::models::ruleset::{
    Ruleset, RulesetObject, unique::{Unique, UniqueType, UniqueTarget, StateForConditionals, IHasUniques},
};
use crate::models::stats::{INamed, Stat};
use crate::models::ui::{Fonts, to_percent};

/// Represents a reason why a construction can be purchased
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseReason {
    /// Construction is allowed to be purchased
    Allowed,
    /// Construction cannot be purchased with this stat
    Invalid,
    /// Construction is explicitly marked as unpurchasable
    Unpurchasable,
    /// Construction is allowed to be purchased due to a unique
    UniqueAllowed,
    /// Construction is not allowed to be purchased
    NotAllowed,
    /// Other reason
    Other,
    /// Other allowed reason
    OtherAllowed,
}

impl PurchaseReason {
    /// Returns whether the construction is purchasable
    pub fn is_purchasable(&self) -> bool {
        match self {
            PurchaseReason::Allowed | PurchaseReason::UniqueAllowed | PurchaseReason::OtherAllowed => true,
            _ => false,
        }
    }
}

/// Represents a type of rejection reason for a construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RejectionReasonType {
    AlreadyBuilt,
    Unbuildable,
    CanOnlyBePurchased,
    ShouldNotBeDisplayed,
    DisabledBySetting,
    HiddenWithoutVictory,
    MustBeOnTile,
    MustNotBeOnTile,
    MustBeNextToTile,
    MustNotBeNextToTile,
    MustOwnTile,
    WaterUnitsInCoastalCities,
    CanOnlyBeBuiltInSpecificCities,
    MaxNumberBuildable,
    UniqueToOtherNation,
    ReplacedByOurUnique,
    CannotBeBuilt,
    CannotBeBuiltUnhappiness,
    Obsoleted,
    RequiresTech,
    RequiresPolicy,
    UnlockedWithEra,
    MorePolicyBranches,
    RequiresNearbyResource,
    CannotBeBuiltWith,
    RequiresBuildingInThisCity,
    RequiresBuildingInAllCities,
    RequiresBuildingInSomeCities,
    RequiresBuildingInSomeCity,
    WonderAlreadyBuilt,
    NationalWonderAlreadyBuilt,
    WonderBeingBuiltElsewhere,
    CityStateWonder,
    PuppetWonder,
    WonderDisabledEra,
    ConsumesResources,
    PopulationRequirement,
    NoSettlerForOneCityPlayers,
    NoPlaceToPutUnit,
}

impl RejectionReasonType {
    /// Returns whether the rejection reason should be shown to the user
    pub fn should_show(&self) -> bool {
        match self {
            RejectionReasonType::CanOnlyBePurchased |
            RejectionReasonType::CannotBeBuiltUnhappiness |
            RejectionReasonType::RequiresBuildingInThisCity |
            RejectionReasonType::RequiresBuildingInAllCities |
            RejectionReasonType::RequiresBuildingInSomeCities |
            RejectionReasonType::RequiresBuildingInSomeCity |
            RejectionReasonType::WonderBeingBuiltElsewhere |
            RejectionReasonType::ConsumesResources |
            RejectionReasonType::PopulationRequirement |
            RejectionReasonType::NoPlaceToPutUnit => true,
            _ => false,
        }
    }

    /// Returns the error message for this rejection reason
    pub fn error_message(&self) -> &'static str {
        match self {
            RejectionReasonType::AlreadyBuilt => "Building already built in this city",
            RejectionReasonType::Unbuildable => "Unbuildable",
            RejectionReasonType::CanOnlyBePurchased => "Can only be purchased",
            RejectionReasonType::ShouldNotBeDisplayed => "Should not be displayed",
            RejectionReasonType::DisabledBySetting => "Disabled by setting",
            RejectionReasonType::HiddenWithoutVictory => "Hidden because a victory type has been disabled",
            RejectionReasonType::MustBeOnTile => "Must be on a specific tile",
            RejectionReasonType::MustNotBeOnTile => "Must not be on a specific tile",
            RejectionReasonType::MustBeNextToTile => "Must be next to a specific tile",
            RejectionReasonType::MustNotBeNextToTile => "Must not be next to a specific tile",
            RejectionReasonType::MustOwnTile => "Must own a specific tile close by",
            RejectionReasonType::WaterUnitsInCoastalCities => "May only built water units in coastal cities",
            RejectionReasonType::CanOnlyBeBuiltInSpecificCities => "Build requirements not met in this city",
            RejectionReasonType::MaxNumberBuildable => "Maximum number have been built or are being constructed",
            RejectionReasonType::UniqueToOtherNation => "Unique to another nation",
            RejectionReasonType::ReplacedByOurUnique => "Our unique replaces this",
            RejectionReasonType::CannotBeBuilt => "Cannot be built by this nation",
            RejectionReasonType::CannotBeBuiltUnhappiness => "Unhappiness",
            RejectionReasonType::Obsoleted => "Obsolete",
            RejectionReasonType::RequiresTech => "Required tech not researched",
            RejectionReasonType::RequiresPolicy => "Requires a specific policy!",
            RejectionReasonType::UnlockedWithEra => "Unlocked when reaching a specific era",
            RejectionReasonType::MorePolicyBranches => "Hidden until more policy branches are fully adopted",
            RejectionReasonType::RequiresNearbyResource => "Requires a certain resource being exploited nearby",
            RejectionReasonType::CannotBeBuiltWith => "Cannot be built at the same time as another building already built",
            RejectionReasonType::RequiresBuildingInThisCity => "Requires a specific building in this city!",
            RejectionReasonType::RequiresBuildingInAllCities => "Requires a specific building in all cities!",
            RejectionReasonType::RequiresBuildingInSomeCities => "Requires a specific building in more cities!",
            RejectionReasonType::RequiresBuildingInSomeCity => "Requires a specific building anywhere in your empire!",
            RejectionReasonType::WonderAlreadyBuilt => "Wonder already built",
            RejectionReasonType::NationalWonderAlreadyBuilt => "National Wonder already built",
            RejectionReasonType::WonderBeingBuiltElsewhere => "Wonder is being built elsewhere",
            RejectionReasonType::CityStateWonder => "No Wonders for city-states",
            RejectionReasonType::PuppetWonder => "No Wonders for Puppets",
            RejectionReasonType::WonderDisabledEra => "This Wonder is disabled when starting in this era",
            RejectionReasonType::ConsumesResources => "Consumes resources which you are lacking",
            RejectionReasonType::PopulationRequirement => "Requires more population",
            RejectionReasonType::NoSettlerForOneCityPlayers => "No settlers for city-states or one-city challengers",
            RejectionReasonType::NoPlaceToPutUnit => "No space to place this unit",
        }
    }

    /// Returns a default instance of RejectionReason for this type
    pub fn to_instance(&self) -> RejectionReason {
        RejectionReason::new(*self, self.error_message().to_string(), self.should_show())
    }

    /// Returns a custom instance of RejectionReason for this type
    pub fn to_custom_instance(&self, error_message: Option<String>, should_show: Option<bool>) -> RejectionReason {
        RejectionReason::new(
            *self,
            error_message.unwrap_or_else(|| self.error_message().to_string()),
            should_show.unwrap_or_else(|| self.should_show()),
        )
    }
}

/// Represents a reason why a construction is rejected
pub struct RejectionReason {
    /// The type of rejection reason
    pub type_: RejectionReasonType,
    /// The error message for this rejection reason
    pub error_message: String,
    /// Whether the rejection reason should be shown to the user
    pub should_show: bool,
}

impl RejectionReason {
    /// Creates a new rejection reason
    pub fn new(type_: RejectionReasonType, error_message: String, should_show: bool) -> Self {
        Self {
            type_,
            error_message,
            should_show,
        }
    }

    /// Returns whether this rejection reason is related to tech, policy, era, or wonder requirements
    pub fn is_tech_policy_era_wonder_requirement(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::Obsoleted |
            RejectionReasonType::RequiresTech |
            RejectionReasonType::RequiresPolicy |
            RejectionReasonType::MorePolicyBranches |
            RejectionReasonType::RequiresBuildingInSomeCity
        )
    }

    /// Returns whether this rejection reason has a reason to be removed from the queue
    pub fn has_reason_to_be_removed_from_queue(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::Obsoleted |
            RejectionReasonType::WonderAlreadyBuilt |
            RejectionReasonType::NationalWonderAlreadyBuilt |
            RejectionReasonType::CannotBeBuiltWith |
            RejectionReasonType::MaxNumberBuildable
        )
    }

    /// Returns whether this rejection reason is important
    pub fn is_important_rejection(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::ShouldNotBeDisplayed |
            RejectionReasonType::WonderBeingBuiltElsewhere |
            RejectionReasonType::RequiresBuildingInAllCities |
            RejectionReasonType::RequiresBuildingInThisCity |
            RejectionReasonType::RequiresBuildingInSomeCity |
            RejectionReasonType::RequiresBuildingInSomeCities |
            RejectionReasonType::CanOnlyBeBuiltInSpecificCities |
            RejectionReasonType::CannotBeBuiltUnhappiness |
            RejectionReasonType::PopulationRequirement |
            RejectionReasonType::ConsumesResources |
            RejectionReasonType::CanOnlyBePurchased |
            RejectionReasonType::MaxNumberBuildable |
            RejectionReasonType::NoPlaceToPutUnit
        )
    }

    /// Returns whether this rejection reason is a construction rejection
    pub fn is_construction_rejection(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::Unbuildable |
            RejectionReasonType::CannotBeBuiltUnhappiness |
            RejectionReasonType::CannotBeBuilt |
            RejectionReasonType::CanOnlyBeBuiltInSpecificCities
        )
    }

    /// Returns whether this rejection reason is never visible
    pub fn is_never_visible(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::AlreadyBuilt |
            RejectionReasonType::WonderAlreadyBuilt |
            RejectionReasonType::NationalWonderAlreadyBuilt |
            RejectionReasonType::DisabledBySetting |
            RejectionReasonType::UniqueToOtherNation |
            RejectionReasonType::ReplacedByOurUnique |
            RejectionReasonType::Obsoleted |
            RejectionReasonType::WonderBeingBuiltElsewhere |
            RejectionReasonType::RequiresTech |
            RejectionReasonType::NoSettlerForOneCityPlayers |
            RejectionReasonType::WaterUnitsInCoastalCities
        )
    }

    /// Returns the rejection precedence
    pub fn get_rejection_precedence(&self) -> i32 {
        let ordered_important_rejection_types = [
            RejectionReasonType::ShouldNotBeDisplayed,
            RejectionReasonType::WonderBeingBuiltElsewhere,
            RejectionReasonType::RequiresBuildingInAllCities,
            RejectionReasonType::RequiresBuildingInThisCity,
            RejectionReasonType::RequiresBuildingInSomeCity,
            RejectionReasonType::RequiresBuildingInSomeCities,
            RejectionReasonType::CanOnlyBeBuiltInSpecificCities,
            RejectionReasonType::CannotBeBuiltUnhappiness,
            RejectionReasonType::PopulationRequirement,
            RejectionReasonType::ConsumesResources,
            RejectionReasonType::CanOnlyBePurchased,
            RejectionReasonType::MaxNumberBuildable,
            RejectionReasonType::NoPlaceToPutUnit,
        ];

        ordered_important_rejection_types.iter()
            .position(|&t| t == self.type_)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }
}

/// Trait for constructions that can be built in cities
pub trait IConstruction: INamed {
    /// Returns whether this construction can be built in the given city
    fn is_buildable(&self, city_constructions: &CityConstructions) -> bool;

    /// Returns whether this construction should be displayed in the given city
    fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool;

    /// Returns matching uniques for this construction
    fn get_matching_uniques_not_conflicting(&self, unique_type: UniqueType, state_for_conditionals: &StateForConditionals) -> Vec<&Unique> {
        Vec::new()
    }

    /// Returns the resource requirements per turn for this construction
    fn get_resource_requirements_per_turn(&self, state: Option<&StateForConditionals>) -> Counter<String>;

    /// Returns the required resources for this construction
    fn required_resources(&self, state: &StateForConditionals) -> HashSet<String>;

    /// Returns the stockpiled resource requirements for this construction
    fn get_stockpiled_resource_requirements(&self, state: &StateForConditionals) -> Counter<String>;
}

/// Trait for non-perpetual constructions
pub trait INonPerpetualConstruction: IConstruction + IHasUniques {
    /// Returns the cost of this construction
    fn cost(&self) -> i32;

    /// Returns the hurry cost modifier for this construction
    fn hurry_cost_modifier(&self) -> i32;

    /// Returns the required technology for this construction
    fn required_tech(&self) -> Option<&str>;

    /// Sets the required technology for this construction
    fn set_required_tech(&mut self, tech: Option<String>);

    /// Returns the legacy required technologies for this construction
    fn legacy_required_techs(&self) -> Vec<String> {
        self.required_tech()
            .map(|tech| vec![tech.to_string()])
            .unwrap_or_default()
    }

    /// Returns the production cost for this construction
    fn get_production_cost(&self, civ_info: &Civilization, city: Option<&City>) -> i32;

    /// Returns the stat buy cost for this construction
    fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32>;

    /// Returns the rejection reasons for this construction
    fn get_rejection_reasons(&self, city_constructions: &CityConstructions) -> Vec<RejectionReason>;

    /// Returns whether this construction can be purchased with the given stat
    fn can_be_purchased_with_stat(&self, city: Option<&City>, stat: Stat) -> bool {
        self.can_be_purchased_with_stat_reasons(city, stat).is_purchasable()
    }

    /// Returns the purchase reason for this construction with the given stat
    fn can_be_purchased_with_stat_reasons(&self, city: Option<&City>, stat: Stat) -> PurchaseReason {
        let state_for_conditionals = city.map(|c| c.state()).unwrap_or_else(StateForConditionals::empty_state);

        if stat == Stat::Production || stat == Stat::Happiness {
            return PurchaseReason::Invalid;
        }

        if self.has_unique(UniqueType::CannotBePurchased, &state_for_conditionals) {
            return PurchaseReason::Unpurchasable;
        }

        // Can be purchased with [Stat] [cityFilter]
        if self.get_matching_uniques(UniqueType::CanBePurchasedWithStat, &StateForConditionals::ignore_conditionals())
            .iter()
            .any(|unique| {
                unique.params[0] == stat.name() &&
                (city.is_none() || (unique.conditionals_apply(&state_for_conditionals) && city.unwrap().matches_filter(&unique.params[1])))
            })
        {
            return PurchaseReason::UniqueAllowed;
        }

        // Can be purchased for [amount] [Stat] [cityFilter]
        if self.get_matching_uniques(UniqueType::CanBePurchasedForAmountStat, &StateForConditionals::ignore_conditionals())
            .iter()
            .any(|unique| {
                unique.params[1] == stat.name() &&
                (city.is_none() || (unique.conditionals_apply(&state_for_conditionals) && city.unwrap().matches_filter(&unique.params[2])))
            })
        {
            return PurchaseReason::UniqueAllowed;
        }

        if stat == Stat::Gold && !self.has_unique(UniqueType::Unbuildable, &state_for_conditionals) {
            return PurchaseReason::Allowed;
        }

        PurchaseReason::NotAllowed
    }

    /// Returns whether this construction is purchasable
    fn is_purchasable(&self, city_constructions: &CityConstructions) -> bool {
        let rejection_reasons = self.get_rejection_reasons(city_constructions);
        rejection_reasons.iter().all(|reason| reason.type_ == RejectionReasonType::Unbuildable)
    }

    /// Returns whether this construction can be purchased with any stat
    fn can_be_purchased_with_any_stat(&self, city: &City) -> bool {
        Stat::stats_usable_to_buy().iter().any(|&stat| self.can_be_purchased_with_stat(Some(city), stat))
    }

    /// Returns the civilopedia gold cost for this construction
    fn get_civilopedia_gold_cost(&self) -> i32 {
        // Same as get_base_gold_cost, but without game-specific modifiers
        ((30.0 * self.cost() as f64).powf(0.75) * self.hurry_cost_modifier() as f64 * to_percent() / 10.0) as i32 * 10
    }

    /// Returns the base gold cost for this construction
    fn get_base_gold_cost(&self, civ_info: &Civilization, city: Option<&City>) -> f64 {
        // https://forums.civfanatics.com/threads/rush-buying-formula.393892/
        (30.0 * self.get_production_cost(civ_info, city) as f64).powf(0.75) * self.hurry_cost_modifier() as f64 * to_percent()
    }

    /// Returns the base buy cost for this construction
    fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        let conditional_state = city.state();

        // Can be purchased for [amount] [Stat] [cityFilter]
        let lowest_cost_unique = self.get_matching_uniques(UniqueType::CanBePurchasedForAmountStat, &conditional_state)
            .iter()
            .filter(|unique| unique.params[1] == stat.name() && city.matches_filter(&unique.params[2]))
            .min_by_key(|unique| unique.params[0].parse::<i32>().unwrap_or(i32::MAX));

        if let Some(unique) = lowest_cost_unique {
            let amount = unique.params[0].parse::<i32>().unwrap_or(0);
            return Some(amount as f32 * city.civ().game_info().speed().stat_cost_modifiers().get(&stat).unwrap_or(&1.0));
        }

        if stat == Stat::Gold {
            return Some(self.get_base_gold_cost(city.civ(), Some(city)) as f32);
        }

        // Can be purchased with [Stat] [cityFilter]
        if self.get_matching_uniques(UniqueType::CanBePurchasedWithStat, &conditional_state)
            .iter()
            .any(|unique| unique.params[0] == stat.name() && city.matches_filter(&unique.params[1]))
        {
            return Some(city.civ().get_era().base_unit_buy_cost() as f32 * city.civ().game_info().speed().stat_cost_modifiers().get(&stat).unwrap_or(&1.0));
        }

        None
    }

    /// Returns the cost for constructions increasing in price
    fn get_cost_for_constructions_increasing_in_price(&self, base_cost: i32, increase_cost: i32, previously_bought: i32) -> i32 {
        (base_cost as f32 + increase_cost as f32 / 2.0 * (previously_bought * previously_bought + previously_bought) as f32) as i32
    }

    /// Returns matching uniques for this construction
    fn get_matching_uniques_not_conflicting(&self, unique_type: UniqueType, state_for_conditionals: &StateForConditionals) -> Vec<&Unique> {
        self.get_matching_uniques(unique_type, state_for_conditionals)
    }

    /// Returns the required resources for this construction
    fn required_resources(&self, state: &StateForConditionals) -> HashSet<String> {
        let mut resources = self.get_resource_requirements_per_turn(Some(state)).keys().cloned().collect::<HashSet<String>>();

        for unique in self.get_matching_uniques(UniqueType::CostsResources, state) {
            resources.insert(unique.params[1].clone());
        }

        resources
    }

    /// Returns the stockpiled resource requirements for this construction
    fn get_stockpiled_resource_requirements(&self, state: &StateForConditionals) -> Counter<String> {
        let mut counter = Counter::new();

        for unique in self.get_matching_uniques_not_conflicting(UniqueType::CostsResources, state) {
            let mut amount = unique.params[0].parse::<i32>().unwrap_or(0);

            if unique.is_modified_by_game_speed() {
                amount = (amount as f32 * state.game_info().unwrap().speed().modifier()) as i32;
            }

            counter.add(unique.params[1].clone(), amount);
        }

        counter
    }
}

/// Represents a perpetual construction
pub struct PerpetualConstruction {
    /// The name of this construction
    name: String,
    /// The description of this construction
    description: String,
}

impl PerpetualConstruction {
    /// Creates a new perpetual construction
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
        }
    }

    /// Returns the production tooltip for this construction
    pub fn get_production_tooltip(&self, city: &City, with_icon: bool) -> String {
        String::new()
    }

    /// Returns whether the name represents a perpetual construction
    pub fn is_name_perpetual(name: &str) -> bool {
        name.is_empty() || Self::perpetual_constructions_map().contains_key(name)
    }

    /// Returns the perpetual constructions map
    pub fn perpetual_constructions_map() -> HashMap<String, Box<dyn IConstruction>> {
        let mut map = HashMap::new();

        let science = Box::new(PerpetualStatConversion::new(Stat::Science));
        let gold = Box::new(PerpetualStatConversion::new(Stat::Gold));
        let culture = Box::new(PerpetualStatConversion::new(Stat::Culture));
        let faith = Box::new(PerpetualStatConversion::new(Stat::Faith));
        let idle = Box::new(IdleConstruction::new());

        map.insert(science.name().to_string(), science);
        map.insert(gold.name().to_string(), gold);
        map.insert(culture.name().to_string(), culture);
        map.insert(faith.name().to_string(), faith);
        map.insert(idle.name().to_string(), idle);

        map
    }
}

impl INamed for PerpetualConstruction {
    fn name(&self) -> &str {
        &self.name
    }
}

impl IConstruction for PerpetualConstruction {
    fn is_buildable(&self, _city_constructions: &CityConstructions) -> bool {
        panic!("Impossible!")
    }

    fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        self.is_buildable(city_constructions)
    }

    fn get_resource_requirements_per_turn(&self, _state: Option<&StateForConditionals>) -> Counter<String> {
        Counter::new()
    }

    fn required_resources(&self, _state: &StateForConditionals) -> HashSet<String> {
        HashSet::new()
    }

    fn get_stockpiled_resource_requirements(&self, _state: &StateForConditionals) -> Counter<String> {
        Counter::new()
    }
}

/// Represents a perpetual stat conversion
pub struct PerpetualStatConversion {
    /// The base perpetual construction
    base: PerpetualConstruction,
    /// The stat to convert to
    stat: Stat,
}

impl PerpetualStatConversion {
    /// Creates a new perpetual stat conversion
    pub fn new(stat: Stat) -> Self {
        Self {
            base: PerpetualConstruction::new(
                stat.name().to_string(),
                format!("Convert production to [{}] at a rate of [rate] to 1", stat.name()),
            ),
            stat,
        }
    }

    /// Returns the conversion rate for this stat conversion
    pub fn get_conversion_rate(&self, city: &City) -> i32 {
        (1.0 / city.city_stats().get_stat_conversion_rate(self.stat)).round() as i32
    }
}

impl INamed for PerpetualStatConversion {
    fn name(&self) -> &str {
        self.base.name()
    }
}

impl IConstruction for PerpetualStatConversion {
    fn is_buildable(&self, city_constructions: &CityConstructions) -> bool {
        let city = city_constructions.city();

        if self.stat == Stat::Faith && !city.civ().game_info().is_religion_enabled() {
            return false;
        }

        let state_for_conditionals = city.state();
        city.civ().get_matching_uniques(UniqueType::EnablesCivWideStatProduction, &state_for_conditionals)
            .iter()
            .any(|unique| unique.params[0] == self.stat.name())
    }

    fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        self.is_buildable(city_constructions)
    }

    fn get_production_tooltip(&self, city: &City, with_icon: bool) -> String {
        let production = city.city_stats().current_city_stats().production();
        let conversion_rate = self.get_conversion_rate(city);
        let amount = (production / conversion_rate as f32).round() as i32;

        let icon = if with_icon { self.stat.character() } else { "" };
        format!("\r\n{}{}/{}", amount, icon, Fonts::turn())
    }

    fn get_resource_requirements_per_turn(&self, _state: Option<&StateForConditionals>) -> Counter<String> {
        Counter::new()
    }

    fn required_resources(&self, _state: &StateForConditionals) -> HashSet<String> {
        HashSet::new()
    }

    fn get_stockpiled_resource_requirements(&self, _state: &StateForConditionals) -> Counter<String> {
        Counter::new()
    }
}

/// Represents an idle construction
pub struct IdleConstruction {
    /// The base perpetual construction
    base: PerpetualConstruction,
}

impl IdleConstruction {
    /// Creates a new idle construction
    pub fn new() -> Self {
        Self {
            base: PerpetualConstruction::new(
                "Nothing".to_string(),
                "The city will not produce anything.".to_string(),
            ),
        }
    }
}

impl INamed for IdleConstruction {
    fn name(&self) -> &str {
        self.base.name()
    }
}

impl IConstruction for IdleConstruction {
    fn is_buildable(&self, _city_constructions: &CityConstructions) -> bool {
        true
    }

    fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        self.is_buildable(city_constructions)
    }

    fn get_resource_requirements_per_turn(&self, _state: Option<&StateForConditionals>) -> Counter<String> {
        Counter::new()
    }

    fn required_resources(&self, _state: &StateForConditionals) -> HashSet<String> {
        HashSet::new()
    }

    fn get_stockpiled_resource_requirements(&self, _state: &StateForConditionals) -> Counter<String> {
        Counter::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purchase_reason() {
        assert!(PurchaseReason::Allowed.is_purchasable());
        assert!(!PurchaseReason::Invalid.is_purchasable());
        assert!(!PurchaseReason::Unpurchasable.is_purchasable());
        assert!(PurchaseReason::UniqueAllowed.is_purchasable());
        assert!(!PurchaseReason::NotAllowed.is_purchasable());
        assert!(!PurchaseReason::Other.is_purchasable());
        assert!(PurchaseReason::OtherAllowed.is_purchasable());
    }

    #[test]
    fn test_rejection_reason_type() {
        assert!(RejectionReasonType::CanOnlyBePurchased.should_show());
        assert!(!RejectionReasonType::AlreadyBuilt.should_show());

        assert_eq!(RejectionReasonType::AlreadyBuilt.error_message(), "Building already built in this city");
        assert_eq!(RejectionReasonType::Unbuildable.error_message(), "Unbuildable");
    }

    #[test]
    fn test_rejection_reason() {
        let reason = RejectionReason::new(
            RejectionReasonType::AlreadyBuilt,
            "Custom error message".to_string(),
            true,
        );

        assert_eq!(reason.type_, RejectionReasonType::AlreadyBuilt);
        assert_eq!(reason.error_message, "Custom error message");
        assert!(reason.should_show);

        assert!(!reason.is_tech_policy_era_wonder_requirement());
        assert!(!reason.has_reason_to_be_removed_from_queue());
        assert!(!reason.is_important_rejection());
        assert!(!reason.is_construction_rejection());
        assert!(reason.is_never_visible());
        assert_eq!(reason.get_rejection_precedence(), -1);
    }

    #[test]
    fn test_perpetual_construction() {
        let construction = PerpetualConstruction::new("Test".to_string(), "Test description".to_string());

        assert_eq!(construction.name(), "Test");
        assert_eq!(construction.get_production_tooltip(&City::new(), false), "");

        assert!(PerpetualConstruction::is_name_perpetual(""));
        assert!(!PerpetualConstruction::is_name_perpetual("NotPerpetual"));
    }
}