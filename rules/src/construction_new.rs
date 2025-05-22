use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::models::city::{City, CityConstructions};
use crate::models::civilization::Civilization;
use crate::models::counter::Counter;
use crate::models::ruleset::{
    Building, BaseUnit, Ruleset, RulesetObject, unique::{Unique, UniqueType, UniqueTarget, StateForConditionals, IHasUniques},
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
    MissingTechnology,
    MissingPolicies,
    MissingEra,
    MissingPopulation,
    MissingWonders,
    MissingResources,
    MissingBuildings,
    MissingSpecialists,
    MissingOtherPrerequisites,
    MissingCivInfo,
    MaxNumberOfThisUnitReached,
    CannotBeBuiltWith,
    Obsoleted,
    NotEnoughMaintenance,
    NotEnoughGold,
    NotEnoughFaith,
    NotEnoughCulture,
    NotEnoughScience,
    NotEnoughProduction,
    NotEnoughFood,
    NotEnoughHappiness,
    NotEnoughHealth,
    NotEnoughStability,
    NotEnoughInfluence,
    NotEnoughOtherStats,
    Other,
}

impl RejectionReasonType {
    /// Returns whether the rejection reason should be shown to the user
    pub fn should_show(&self) -> bool {
        match self {
            RejectionReasonType::AlreadyBuilt => true,
            RejectionReasonType::Unbuildable => false,
            RejectionReasonType::CanOnlyBePurchased => true,
            RejectionReasonType::ShouldNotBeDisplayed => false,
            RejectionReasonType::DisabledBySetting => true,
            RejectionReasonType::HiddenWithoutVictory => true,
            RejectionReasonType::MissingTechnology => true,
            RejectionReasonType::MissingPolicies => true,
            RejectionReasonType::MissingEra => true,
            RejectionReasonType::MissingPopulation => true,
            RejectionReasonType::MissingWonders => true,
            RejectionReasonType::MissingResources => true,
            RejectionReasonType::MissingBuildings => true,
            RejectionReasonType::MissingSpecialists => true,
            RejectionReasonType::MissingOtherPrerequisites => true,
            RejectionReasonType::MissingCivInfo => false,
            RejectionReasonType::MaxNumberOfThisUnitReached => true,
            RejectionReasonType::CannotBeBuiltWith => true,
            RejectionReasonType::Obsoleted => true,
            RejectionReasonType::NotEnoughMaintenance => true,
            RejectionReasonType::NotEnoughGold => true,
            RejectionReasonType::NotEnoughFaith => true,
            RejectionReasonType::NotEnoughCulture => true,
            RejectionReasonType::NotEnoughScience => true,
            RejectionReasonType::NotEnoughProduction => true,
            RejectionReasonType::NotEnoughFood => true,
            RejectionReasonType::NotEnoughHappiness => true,
            RejectionReasonType::NotEnoughHealth => true,
            RejectionReasonType::NotEnoughStability => true,
            RejectionReasonType::NotEnoughInfluence => true,
            RejectionReasonType::NotEnoughOtherStats => true,
            RejectionReasonType::Other => true,
        }
    }

    /// Returns the error message for this rejection reason
    pub fn error_message(&self) -> &'static str {
        match self {
            RejectionReasonType::AlreadyBuilt => "Already built",
            RejectionReasonType::Unbuildable => "Unbuildable",
            RejectionReasonType::CanOnlyBePurchased => "Can only be purchased",
            RejectionReasonType::ShouldNotBeDisplayed => "Should not be displayed",
            RejectionReasonType::DisabledBySetting => "Disabled by setting",
            RejectionReasonType::HiddenWithoutVictory => "Hidden without victory",
            RejectionReasonType::MissingTechnology => "Missing technology",
            RejectionReasonType::MissingPolicies => "Missing policies",
            RejectionReasonType::MissingEra => "Missing era",
            RejectionReasonType::MissingPopulation => "Missing population",
            RejectionReasonType::MissingWonders => "Missing wonders",
            RejectionReasonType::MissingResources => "Missing resources",
            RejectionReasonType::MissingBuildings => "Missing buildings",
            RejectionReasonType::MissingSpecialists => "Missing specialists",
            RejectionReasonType::MissingOtherPrerequisites => "Missing other prerequisites",
            RejectionReasonType::MissingCivInfo => "Missing civ info",
            RejectionReasonType::MaxNumberOfThisUnitReached => "Max number of this unit reached",
            RejectionReasonType::CannotBeBuiltWith => "Cannot be built with",
            RejectionReasonType::Obsoleted => "Obsoleted",
            RejectionReasonType::NotEnoughMaintenance => "Not enough maintenance",
            RejectionReasonType::NotEnoughGold => "Not enough gold",
            RejectionReasonType::NotEnoughFaith => "Not enough faith",
            RejectionReasonType::NotEnoughCulture => "Not enough culture",
            RejectionReasonType::NotEnoughScience => "Not enough science",
            RejectionReasonType::NotEnoughProduction => "Not enough production",
            RejectionReasonType::NotEnoughFood => "Not enough food",
            RejectionReasonType::NotEnoughHappiness => "Not enough happiness",
            RejectionReasonType::NotEnoughHealth => "Not enough health",
            RejectionReasonType::NotEnoughStability => "Not enough stability",
            RejectionReasonType::NotEnoughInfluence => "Not enough influence",
            RejectionReasonType::NotEnoughOtherStats => "Not enough other stats",
            RejectionReasonType::Other => "Other",
        }
    }

    /// Returns a default instance of RejectionReason for this type
    pub fn to_instance(&self) -> RejectionReason {
        RejectionReason::new(*self, self.error_message().to_string(), self.should_show())
    }

    /// Returns a custom instance of RejectionReason for this type
    pub fn to_custom_instance(
        &self,
        error_message: Option<String>,
        should_show: Option<bool>,
    ) -> RejectionReason {
        RejectionReason::new(
            *self,
            error_message.unwrap_or_else(|| self.error_message().to_string()),
            should_show.unwrap_or_else(|| self.should_show()),
        )
    }
}

/// Represents a reason why a construction is rejected
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionReason {
    /// The type of rejection reason
    pub type_: RejectionReasonType,
    /// The error message for this rejection reason
    pub error_message: String,
    /// Whether this rejection reason should be shown to the user
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
            RejectionReasonType::MissingTechnology
                | RejectionReasonType::MissingPolicies
                | RejectionReasonType::MissingEra
                | RejectionReasonType::MissingWonders
        )
    }

    /// Returns whether this rejection reason has a reason to be removed from the queue
    pub fn has_reason_to_be_removed_from_queue(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::AlreadyBuilt
                | RejectionReasonType::Unbuildable
                | RejectionReasonType::CannotBeBuiltWith
                | RejectionReasonType::Obsoleted
        )
    }

    /// Returns whether this rejection reason is important
    pub fn is_important_rejection(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::AlreadyBuilt
                | RejectionReasonType::Unbuildable
                | RejectionReasonType::CanOnlyBePurchased
                | RejectionReasonType::DisabledBySetting
                | RejectionReasonType::HiddenWithoutVictory
                | RejectionReasonType::MissingCivInfo
                | RejectionReasonType::MaxNumberOfThisUnitReached
                | RejectionReasonType::CannotBeBuiltWith
                | RejectionReasonType::Obsoleted
        )
    }

    /// Returns whether this rejection reason is a construction rejection
    pub fn is_construction_rejection(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::MissingBuildings
                | RejectionReasonType::MissingWonders
                | RejectionReasonType::CannotBeBuiltWith
        )
    }

    /// Returns whether this rejection reason is never visible
    pub fn is_never_visible(&self) -> bool {
        matches!(
            self.type_,
            RejectionReasonType::Unbuildable
                | RejectionReasonType::ShouldNotBeDisplayed
                | RejectionReasonType::MissingCivInfo
                | RejectionReasonType::NotEnoughMaintenance
                | RejectionReasonType::NotEnoughGold
                | RejectionReasonType::NotEnoughFaith
                | RejectionReasonType::NotEnoughCulture
                | RejectionReasonType::NotEnoughScience
                | RejectionReasonType::NotEnoughProduction
                | RejectionReasonType::NotEnoughFood
        )
    }

    /// Returns the rejection precedence
    pub fn get_rejection_precedence(&self) -> i32 {
        match self.type_ {
            RejectionReasonType::AlreadyBuilt => 100,
            RejectionReasonType::Unbuildable => 99,
            RejectionReasonType::CanOnlyBePurchased => 98,
            RejectionReasonType::ShouldNotBeDisplayed => 97,
            RejectionReasonType::DisabledBySetting => 96,
            RejectionReasonType::HiddenWithoutVictory => 95,
            RejectionReasonType::MissingTechnology => 90,
            RejectionReasonType::MissingPolicies => 89,
            RejectionReasonType::MissingEra => 88,
            RejectionReasonType::MissingPopulation => 87,
            RejectionReasonType::MissingWonders => 86,
            RejectionReasonType::MissingResources => 85,
            RejectionReasonType::MissingBuildings => 84,
            RejectionReasonType::MissingSpecialists => 83,
            RejectionReasonType::MissingOtherPrerequisites => 82,
            RejectionReasonType::MissingCivInfo => 81,
            RejectionReasonType::MaxNumberOfThisUnitReached => 80,
            RejectionReasonType::CannotBeBuiltWith => 79,
            RejectionReasonType::Obsoleted => 78,
            _ => 0,
        }
    }
}

/// Enum representing the specific type of construction
#[derive(Debug, Clone)]
pub enum ConstructionType {
    /// Building construction
    Building(Building),
    /// Unit construction
    Unit(BaseUnit),
    /// Perpetual stat conversion
    PerpetualStat(Stat),
    /// Idle construction (do nothing)
    Idle,
    /// Generic perpetual construction
    Perpetual,
    /// Other construction types can be added as needed
    Other,
}

/// Base construction class that all construction types will use
#[derive(Debug, Clone)]
pub struct Construction {
    /// Name of the construction
    pub name: String,
    /// Description of the construction
    pub description: String,
    /// Whether this is a perpetual construction
    pub is_perpetual: bool,
    /// Cost of the construction (for non-perpetual constructions)
    pub cost: Option<i32>,
    /// Uniques associated with this construction
    pub uniques: Vec<Unique>,
    /// The specific type of construction
    pub construction_type: ConstructionType,
}

impl Construction {
    /// Creates a new construction for a building
    pub fn new_building(building: &Building) -> Self {
        Self {
            name: building.name().to_string(),
            description: building.get_description().unwrap_or_default(),
            is_perpetual: false,
            cost: Some(building.cost()),
            uniques: building.get_uniques().iter().map(|u| u.text.clone()).collect(),
            construction_type: ConstructionType::Building(building.clone()),
        }
    }
    
    /// Creates a new construction
    pub fn new(name: String, description: String, is_perpetual: bool, cost: Option<i32>, construction_type: ConstructionType) -> Self {
        Self {
            name,
            description,
            is_perpetual,
            cost,
            uniques: Vec::new(),
            construction_type,
        }
    }

    /// Returns whether this construction is buildable in the given city
    pub fn is_buildable(&self, city_constructions: &CityConstructions) -> bool {
        // Default implementation - override in specific construction types
        true
    }

    /// Returns whether this construction should be displayed in the given city
    pub fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        // Default implementation - override in specific construction types
        self.is_buildable(city_constructions)
    }

    /// Returns matching uniques for this construction
    pub fn get_matching_uniques_not_conflicting(
        &self,
        unique_type: UniqueType,
        state_for_conditionals: &StateForConditionals,
    ) -> Vec<&Unique> {
        self.uniques
            .iter()
            .filter(|unique| {
                unique.unique_type == unique_type
                    && unique.conditional_apply(state_for_conditionals)
            })
            .collect()
    }

    /// Returns the resource requirements per turn for this construction
    pub fn get_resource_requirements_per_turn(
        &self,
        state: Option<&StateForConditionals>,
    ) -> Counter<String> {
        // Default implementation - override in specific construction types
        Counter::new()
    }

    /// Returns the required resources for this construction
    pub fn required_resources(&self, state: &StateForConditionals) -> HashSet<String> {
        // Default implementation - override in specific construction types
        HashSet::new()
    }

    /// Returns the stockpiled resource requirements for this construction
    pub fn get_stockpiled_resource_requirements(
        &self,
        state: &StateForConditionals,
    ) -> Counter<String> {
        // Default implementation - override in specific construction types
        Counter::new()
    }

    /// Returns the cost of this construction
    pub fn cost(&self) -> i32 {
        // Default implementation - override in specific construction types
        self.cost.unwrap_or(-1)
    }

    /// Returns the hurry cost modifier for this construction
    pub fn hurry_cost_modifier(&self) -> f32 {
        // Default implementation - override in specific construction types
        1.0
    }

    /// Returns the legacy required technologies for this construction
    pub fn legacy_required_techs(&self) -> Vec<String> {
        // Default implementation - override in specific construction types
        Vec::new()
    }

    /// Returns whether this construction can be purchased with the given stat
    pub fn can_be_purchased_with_stat(&self, city: Option<&City>, stat: Stat) -> bool {
        // Default implementation - override in specific construction types
        false
    }

    /// Returns the purchase reason for this construction with the given stat
    pub fn can_be_purchased_with_stat_reasons(&self, city: Option<&City>, stat: Stat) -> PurchaseReason {
        // Default implementation - override in specific construction types
        PurchaseReason::Invalid
    }

    /// Returns whether this construction is purchasable
    pub fn is_purchasable(&self, city_constructions: &CityConstructions) -> bool {
        // Default implementation - override in specific construction types
        false
    }

    /// Returns whether this construction can be purchased with any stat
    pub fn can_be_purchased_with_any_stat(&self, city: &City) -> bool {
        // Default implementation - override in specific construction types
        false
    }

    /// Returns the civilopedia gold cost for this construction
    pub fn get_civilopedia_gold_cost(&self) -> i32 {
        // Default implementation - override in specific construction types
        0
    }

    /// Returns the base gold cost for this construction
    pub fn get_base_gold_cost(&self, civ_info: &Civilization, city: Option<&City>) -> f64 {
        // Default implementation - override in specific construction types
        0.0
    }

    /// Returns the base buy cost for this construction
    pub fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        // Default implementation - override in specific construction types
        None
    }

    /// Returns the cost for constructions increasing in price
    pub fn get_cost_for_constructions_increasing_in_price(
        &self,
        base_cost: i32,
        increase_cost: i32,
        previously_bought: i32,
    ) -> i32 {
        // Default implementation - override in specific construction types
        base_cost + increase_cost * previously_bought
    }
}

impl INamed for Construction {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Perpetual construction that converts production to other stats
#[derive(Debug, Clone)]
pub struct PerpetualStatConversion {
    /// Base construction
    pub base: Construction,
}

impl PerpetualStatConversion {
    /// Creates a new perpetual stat conversion
    pub fn new(stat: Stat) -> Self {
        let name = format!("Produce {}", stat);
        let description = format!("Convert production to {}", stat.to_string().to_lowercase());
        
        Self {
            base: Construction::new(name, description, true, None, ConstructionType::PerpetualStat(stat)),
        }
    }

    /// Returns the conversion rate for this stat conversion
    pub fn get_conversion_rate(&self, city: &City) -> i32 {
        // Implementation would depend on city stats and other factors
        4 // Default value
    }

    /// Returns the production tooltip for this construction
    pub fn get_production_tooltip(&self, city: &City, with_icon: bool) -> String {
        let rate = self.get_conversion_rate(city);
        let stat_name = self.stat.to_string();
        
        if with_icon {
            format!("Converts {} [Production] to {} [{}] each turn",
                rate, rate, stat_name)
        } else {
            format!("Converts {} Production to {} {} each turn",
                rate, rate, stat_name)
        }
    }
}

impl INamed for PerpetualStatConversion {
    fn name(&self) -> &str {
        self.base.name()
    }
}

/// Perpetual construction that doesn't do anything
#[derive(Debug, Clone)]
pub struct IdleConstruction {
    /// Base construction
    pub base: Construction,
}

impl IdleConstruction {
    /// Creates a new idle construction
    pub fn new() -> Self {
        Self {
            base: Construction::new(
                "Do nothing".to_string(),
                "No production".to_string(),
                true,
                None,
                ConstructionType::Idle,
            ),
        }
    }
}

impl INamed for IdleConstruction {
    fn name(&self) -> &str {
        self.base.name()
    }
}

/// Standard perpetual construction
#[derive(Debug, Clone)]
pub struct PerpetualConstruction {
    /// Base construction
    pub base: Construction,
}

impl PerpetualConstruction {
    /// Creates a new perpetual construction
    pub fn new(name: String, description: String) -> Self {
        Self {
            base: Construction::new(name, description, true, None, ConstructionType::Perpetual),
        }
    }

    /// Returns the production tooltip for this construction
    pub fn get_production_tooltip(&self, city: &City, with_icon: bool) -> String {
        // Implementation would depend on the specific perpetual construction
        self.base.description.clone()
    }

    /// Returns whether the name represents a perpetual construction
    pub fn is_name_perpetual(name: &str) -> bool {
        name == "Science" || name == "Gold" || name == "Culture" || name == "Faith" || name == "Food"
    }

    /// Returns the perpetual constructions map
    pub fn perpetual_constructions_map() -> HashMap<String, Construction> {
        let mut map = HashMap::new();
        
        // Add perpetual stat conversions
        for stat in [Stat::Science, Stat::Gold, Stat::Culture, Stat::Faith] {
            let conversion = PerpetualStatConversion::new(stat);
            map.insert(conversion.name().to_string(), conversion.base.clone());
        }
        
        // Add idle construction
        let idle = IdleConstruction::new();
        map.insert(idle.name().to_string(), idle.base.clone());
        
        map
    }
    
    /// Gets the building data if this is a building construction
    pub fn as_building(&self) -> Option<&Building> {
        match &self.construction_type {
            ConstructionType::Building(building) => Some(building),
            _ => None,
        }
    }
    
    /// Gets the unit data if this is a unit construction
    pub fn as_unit(&self) -> Option<&BaseUnit> {
        match &self.construction_type {
            ConstructionType::Unit(unit) => Some(unit),
            _ => None,
        }
    }
    
    /// Gets the stat if this is a perpetual stat conversion
    pub fn as_perpetual_stat(&self) -> Option<&Stat> {
        match &self.construction_type {
            ConstructionType::PerpetualStat(stat) => Some(stat),
            _ => None,
        }
    }
}

impl INamed for PerpetualConstruction {
    fn name(&self) -> &str {
        self.base.name()
    }
}

/// Non-perpetual construction that can be built in cities
#[derive(Debug, Clone)]
pub struct NonPerpetualConstruction {
    /// Base construction
    pub base: Construction,
    /// Hurry cost modifier
    pub hurry_cost_modifier: f32,
    /// Required technologies
    pub required_techs: Vec<String>,
}

impl NonPerpetualConstruction {
    /// Creates a new non-perpetual construction for a building
    pub fn new_building(building: Building, cost: i32) -> Self {
        Self {
            base: Construction::new(
                building.name().to_string(),
                building.get_description().unwrap_or_default(),
                false,
                Some(cost),
                ConstructionType::Building(building),
            ),
            hurry_cost_modifier: 1.0,
            required_techs: Vec::new(),
        }
    }
    
    /// Creates a new non-perpetual construction for a unit
    pub fn new_unit(unit: BaseUnit, cost: i32) -> Self {
        Self {
            base: Construction::new(
                unit.name().to_string(),
                unit.get_description().unwrap_or_default(),
                false,
                Some(cost),
                ConstructionType::Unit(unit),
            ),
            hurry_cost_modifier: 1.0,
            required_techs: Vec::new(),
        }
    }
    
    /// Creates a new generic non-perpetual construction
    pub fn new(name: String, description: String, cost: i32) -> Self {
        Self {
            base: Construction::new(name, description, false, Some(cost), ConstructionType::Other),
            hurry_cost_modifier: 1.0,
            required_techs: Vec::new(),
        }
    }

    /// Returns the hurry cost modifier for this construction
    pub fn hurry_cost_modifier(&self) -> f32 {
        self.hurry_cost_modifier
    }

    /// Returns the legacy required technologies for this construction
    pub fn legacy_required_techs(&self) -> Vec<String> {
        self.required_techs.clone()
    }
}

impl INamed for NonPerpetualConstruction {
    fn name(&self) -> &str {
        self.base.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purchase_reason() {
        assert!(PurchaseReason::Allowed.is_purchasable());
        assert!(PurchaseReason::UniqueAllowed.is_purchasable());
        assert!(PurchaseReason::OtherAllowed.is_purchasable());
        assert!(!PurchaseReason::Invalid.is_purchasable());
        assert!(!PurchaseReason::Unpurchasable.is_purchasable());
        assert!(!PurchaseReason::NotAllowed.is_purchasable());
        assert!(!PurchaseReason::Other.is_purchasable());
    }

    #[test]
    fn test_rejection_reason_type() {
        assert!(RejectionReasonType::AlreadyBuilt.should_show());
        assert!(!RejectionReasonType::Unbuildable.should_show());
        assert_eq!(RejectionReasonType::AlreadyBuilt.error_message(), "Already built");
    }

    #[test]
    fn test_rejection_reason() {
        let reason = RejectionReason::new(
            RejectionReasonType::AlreadyBuilt,
            "Already built".to_string(),
            true,
        );
        assert!(reason.is_important_rejection());
        assert!(!reason.is_tech_policy_era_wonder_requirement());
        assert!(reason.has_reason_to_be_removed_from_queue());
        assert!(!reason.is_never_visible());
        assert_eq!(reason.get_rejection_precedence(), 100);
    }

    #[test]
    fn test_construction() {
        let construction = Construction::new(
            "Test Construction".to_string(),
            "Test Description".to_string(),
            false,
            Some(100),
            ConstructionType::Other,
        );
        assert_eq!(construction.name(), "Test Construction");
        assert_eq!(construction.cost(), 100);
        assert!(!construction.is_perpetual);
    }

    #[test]
    fn test_perpetual_construction() {
        let perpetual = PerpetualConstruction::new(
            "Test Perpetual".to_string(),
            "Test Description".to_string(),
        );
        assert_eq!(perpetual.name(), "Test Perpetual");
        assert!(perpetual.base.is_perpetual);
    }

    #[test]
    fn test_perpetual_stat_conversion() {
        let conversion = PerpetualStatConversion::new(Stat::Science);
        assert_eq!(conversion.name(), "Produce Science");
        assert!(conversion.base.is_perpetual);
        assert_eq!(conversion.stat, Stat::Science);
    }

    #[test]
    fn test_idle_construction() {
        let idle = IdleConstruction::new();
        assert_eq!(idle.name(), "Do nothing");
        assert!(idle.base.is_perpetual);
    }

    #[test]
    fn test_non_perpetual_construction() {
        let non_perpetual = NonPerpetualConstruction::new(
            "Test Non-Perpetual".to_string(),
            "Test Description".to_string(),
            100,
        );
        assert_eq!(non_perpetual.name(), "Test Non-Perpetual");
        assert!(!non_perpetual.base.is_perpetual);
        assert_eq!(non_perpetual.base.cost(), 100);
    }
}
