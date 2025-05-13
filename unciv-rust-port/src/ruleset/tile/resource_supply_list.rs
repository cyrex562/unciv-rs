use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{constants::constants::diplomacy::CITY_STATES, ruleset::tile::tile_resource::TileResource};
use crate::ruleset::ruleset::Ruleset;

/// Represents a supply of a resource from a specific origin
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSupply {
    /// The resource being supplied
    pub resource: TileResource,
    /// The origin of the supply
    pub origin: String,
    /// The amount of the resource being supplied
    pub amount: i32,
}

impl ResourceSupply {
    /// Creates a new ResourceSupply with the specified values
    pub fn new(resource: TileResource, origin: String, amount: i32) -> Self {
        ResourceSupply {
            resource,
            origin,
            amount,
        }
    }

    /// Checks if this supply is from a city state or trade
    pub fn is_city_state_or_trade_origin(&self) -> bool {
        (self.origin == CITY_STATES || self.origin == "Trade") && self.amount > 0
    }
}

impl std::fmt::Display for ResourceSupply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} from {}", self.amount, self.resource.name, self.origin)
    }
}

/// Container for aggregating supply and demand of resources, categorized by origin
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceSupplyList {
    /// The list of resource supplies
    supplies: Vec<ResourceSupply>,
    /// Whether to keep entries with amount 0
    keep_zero_amounts: bool,
}

impl ResourceSupplyList {
    /// Creates a new ResourceSupplyList with the specified keep_zero_amounts value
    pub fn new(keep_zero_amounts: bool) -> Self {
        ResourceSupplyList {
            supplies: Vec::with_capacity(24), // Allows all resources in G&K with just one Vec growth step
            keep_zero_amounts,
        }
    }

    /// Creates an empty ResourceSupplyList
    pub fn empty() -> Self {
        ResourceSupplyList::new(false)
    }

    /// Gets a ResourceSupply entry or None if no match found
    pub fn get(&self, resource: &TileResource, origin: &str) -> Option<&ResourceSupply> {
        self.supplies.iter().find(|s| s.resource.name == resource.name && s.origin == origin)
    }

    /// Gets a mutable ResourceSupply entry or None if no match found
    pub fn get_mut(&mut self, resource: &TileResource, origin: &str) -> Option<&mut ResourceSupply> {
        self.supplies.iter_mut().find(|s| s.resource.name == resource.name && s.origin == origin)
    }

    /// Gets the total amount for a resource by resource name
    pub fn sum_by(&self, resource_name: &str) -> i32 {
        self.supplies.iter()
            .filter(|s| s.resource.name == resource_name)
            .map(|s| s.amount)
            .sum()
    }

    /// Adds a ResourceSupply to the list
    ///
    /// If a supply with the same resource and origin already exists, the amounts are added up.
    /// If the resulting amount is 0 and keep_zero_amounts is false, the entry is removed.
    ///
    /// Returns true if the length of the list changed.
    pub fn add_supply(&mut self, element: ResourceSupply) -> bool {
        if let Some(existing) = self.get_mut(&element.resource, &element.origin) {
            existing.amount += element.amount;
            if !self.keep_zero_amounts && existing.amount == 0 {
                self.supplies.retain(|s| s.resource.name != element.resource.name || s.origin != element.origin);
                return true;
            }
            return false;
        } else {
            if !self.keep_zero_amounts && element.amount == 0 {
                return false;
            }
            self.supplies.push(element);
            return true;
        }
    }

    /// Adds a resource supply with the specified values
    pub fn add(&mut self, resource: TileResource, origin: String, amount: i32) {
        self.add_supply(ResourceSupply::new(resource, origin, amount));
    }

    /// Adds all entries from another ResourceSupplyList
    pub fn add_list(&mut self, resource_supply_list: &ResourceSupplyList) {
        for supply in &resource_supply_list.supplies {
            self.add_supply(supply.clone());
        }
    }

    /// Subtracts resource requirements from the list
    pub fn subtract_resource_requirements(&mut self, resource_requirements: &HashMap<String, i32>, ruleset: &Ruleset, origin: &str) {
        for (resource_name, amount) in resource_requirements {
            if let Some(resource) = ruleset.tile_resources.get(resource_name) {
                self.add(resource.clone(), origin.to_string(), -amount);
            }
        }
    }

    /// Aggregates entries from another list by resource, replacing their origin with a new origin
    pub fn add_by_resource(&mut self, from_list: &ResourceSupplyList, new_origin: &str) -> &mut Self {
        for supply in &from_list.supplies {
            self.add(supply.resource.clone(), new_origin.to_string(), supply.amount);
        }
        self
    }

    /// Same as add_by_resource but ignores negative amounts
    pub fn add_positive_by_resource(&mut self, from_list: &ResourceSupplyList, new_origin: &str) {
        for supply in &from_list.supplies {
            if supply.amount > 0 {
                self.add(supply.resource.clone(), new_origin.to_string(), supply.amount);
            }
        }
    }

    /// Creates a new ResourceSupplyList aggregating resources over all origins
    pub fn sum_by_resource(&self, new_origin: &str) -> ResourceSupplyList {
        let mut result = ResourceSupplyList::new(self.keep_zero_amounts);
        result.add_by_resource(self, new_origin);
        result
    }

    /// Removes all entries from a specific origin
    pub fn remove_all(&mut self, origin: &str) -> &mut Self {
        self.supplies.retain(|s| s.origin != origin);
        self
    }

    /// Gets all supplies in the list
    pub fn get_supplies(&self) -> &[ResourceSupply] {
        &self.supplies
    }

    /// Gets a mutable reference to all supplies in the list
    pub fn get_supplies_mut(&mut self) -> &mut Vec<ResourceSupply> {
        &mut self.supplies
    }
}

impl std::ops::Deref for ResourceSupplyList {
    type Target = Vec<ResourceSupply>;

    fn deref(&self) -> &Self::Target {
        &self.supplies
    }
}

impl std::ops::DerefMut for ResourceSupplyList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.supplies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_supply_new() {
        let resource = TileResource::new();
        let supply = ResourceSupply::new(resource.clone(), "Test".to_string(), 5);
        assert_eq!(supply.resource.name, resource.name);
        assert_eq!(supply.origin, "Test");
        assert_eq!(supply.amount, 5);
    }

    #[test]
    fn test_resource_supply_is_city_state_or_trade_origin() {
        let resource = TileResource::new();
        let mut supply = ResourceSupply::new(resource.clone(), CITY_STATES.to_string(), 5);
        assert!(supply.is_city_state_or_trade_origin());

        supply.origin = "Trade".to_string();
        assert!(supply.is_city_state_or_trade_origin());

        supply.origin = "Other".to_string();
        assert!(!supply.is_city_state_or_trade_origin());

        supply.amount = 0;
        assert!(!supply.is_city_state_or_trade_origin());
    }

    #[test]
    fn test_resource_supply_list_new() {
        let list = ResourceSupplyList::new(true);
        assert!(list.supplies.is_empty());
        assert!(list.keep_zero_amounts);

        let list = ResourceSupplyList::new(false);
        assert!(list.supplies.is_empty());
        assert!(!list.keep_zero_amounts);
    }

    #[test]
    fn test_resource_supply_list_add() {
        let mut list = ResourceSupplyList::new(false);
        let resource = TileResource::new();

        list.add(resource.clone(), "Test".to_string(), 5);
        assert_eq!(list.supplies.len(), 1);
        assert_eq!(list.supplies[0].amount, 5);

        list.add(resource.clone(), "Test".to_string(), 3);
        assert_eq!(list.supplies.len(), 1);
        assert_eq!(list.supplies[0].amount, 8);

        list.add(resource.clone(), "Test".to_string(), -8);
        assert!(list.supplies.is_empty());
    }

    #[test]
    fn test_resource_supply_list_sum_by() {
        let mut list = ResourceSupplyList::new(false);
        let mut resource1 = TileResource::new();
        resource1.name = "Resource1".to_string();
        let mut resource2 = TileResource::new();
        resource2.name = "Resource2".to_string();

        list.add(resource1.clone(), "Test1".to_string(), 5);
        list.add(resource1.clone(), "Test2".to_string(), 3);
        list.add(resource2.clone(), "Test1".to_string(), 2);

        assert_eq!(list.sum_by("Resource1"), 8);
        assert_eq!(list.sum_by("Resource2"), 2);
        assert_eq!(list.sum_by("Resource3"), 0);
    }

    #[test]
    fn test_resource_supply_list_remove_all() {
        let mut list = ResourceSupplyList::new(false);
        let resource = TileResource::new();

        list.add(resource.clone(), "Test1".to_string(), 5);
        list.add(resource.clone(), "Test2".to_string(), 3);

        assert_eq!(list.supplies.len(), 2);

        list.remove_all("Test1");
        assert_eq!(list.supplies.len(), 1);
        assert_eq!(list.supplies[0].origin, "Test2");
    }
}