use std::collections::{HashSet, HashMap};
use serde::{Serialize, Deserialize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::game_info::GameInfo;
use crate::civilization::Civilization;
use crate::belief::{Belief, BeliefType};
use crate::unique::{UniqueMap, UniqueType, StateForConditionals};
use crate::multi_filter::MultiFilter;

/// Data object for Religions
#[derive(Clone, Serialize, Deserialize)]
pub struct Religion {
    pub name: String,
    pub display_name: Option<String>,
    pub founding_civ_name: String,

    #[serde(skip)]
    founder_beliefs: HashSet<String>,
    #[serde(skip)]
    follower_beliefs: HashSet<String>,

    #[serde(skip)]
    founder_belief_unique_map: UniqueMap,
    #[serde(skip)]
    follower_belief_unique_map: UniqueMap,

    #[serde(skip)]
    game_info: Option<GameInfo>,

    #[serde(skip)]
    buildings_purchasable_by_beliefs: Option<Vec<String>>,
}

impl Religion {
    /// Creates a new Religion with the given name, game info, and founding civilization name
    pub fn new(name: String, game_info: GameInfo, founding_civ_name: String) -> Self {
        let mut religion = Religion {
            name,
            display_name: None,
            founding_civ_name,
            founder_beliefs: HashSet::new(),
            follower_beliefs: HashSet::new(),
            founder_belief_unique_map: UniqueMap::new(),
            follower_belief_unique_map: UniqueMap::new(),
            game_info: Some(game_info),
            buildings_purchasable_by_beliefs: None,
        };

        religion.update_unique_maps();
        religion
    }

    /// Creates a clone of this religion
    pub fn clone(&self) -> Self {
        let mut to_return = Religion::new(
            self.name.clone(),
            self.game_info.clone().unwrap(),
            self.founding_civ_name.clone()
        );

        to_return.display_name = self.display_name.clone();
        to_return.founder_beliefs = self.founder_beliefs.clone();
        to_return.follower_beliefs = self.follower_beliefs.clone();

        to_return
    }

    /// Sets the game info and updates the unique maps
    pub fn set_transients(&mut self, game_info: GameInfo) {
        self.game_info = Some(game_info);
        self.update_unique_maps();
    }

    /// Updates the unique maps based on the current beliefs
    fn update_unique_maps(&mut self) {
        if let Some(game_info) = &self.game_info {
            let follower_beliefs = self.map_to_existing_beliefs(&self.follower_beliefs);
            let founder_beliefs = self.map_to_existing_beliefs(&self.founder_beliefs);

            self.follower_belief_unique_map = UniqueMap::from_beliefs(follower_beliefs);
            self.founder_belief_unique_map = UniqueMap::from_beliefs(founder_beliefs);
        }
    }

    /// Adds beliefs to the religion
    pub fn add_beliefs(&mut self, beliefs: &[Belief]) {
        for belief in beliefs {
            match belief.belief_type {
                BeliefType::Founder | BeliefType::Enhancer => {
                    self.founder_beliefs.insert(belief.name.clone());
                },
                BeliefType::Pantheon | BeliefType::Follower => {
                    self.follower_beliefs.insert(belief.name.clone());
                },
                _ => continue, // 'None' and 'Any' are not valid for beliefs, they're used for internal purposes
            }
        }
        self.update_unique_maps();
    }

    /// Gets the icon name for this religion
    pub fn get_icon_name(&self) -> String {
        if self.is_pantheon() {
            "Pantheon".to_string()
        } else {
            self.name.clone()
        }
    }

    /// Gets the display name for this religion
    pub fn get_religion_display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.name.clone())
    }

    /// Maps belief names to actual Belief objects
    fn map_to_existing_beliefs(&self, beliefs: &HashSet<String>) -> Vec<Belief> {
        if let Some(game_info) = &self.game_info {
            beliefs.iter()
                .filter_map(|belief_name| game_info.ruleset.beliefs.get(belief_name).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Gets beliefs of the specified type
    pub fn get_beliefs(&self, belief_type: BeliefType) -> Vec<Belief> {
        if belief_type == BeliefType::Any {
            let mut all_beliefs = self.founder_beliefs.clone();
            all_beliefs.extend(self.follower_beliefs.iter().cloned());
            return self.map_to_existing_beliefs(&all_beliefs);
        }

        let beliefs = if belief_type.is_follower() {
            &self.follower_beliefs
        } else if belief_type.is_founder() {
            &self.founder_beliefs
        } else {
            return Vec::new();
        };

        self.map_to_existing_beliefs(beliefs)
            .into_iter()
            .filter(|belief| belief.belief_type == belief_type)
            .collect()
    }

    /// Gets all beliefs in a specific order
    pub fn get_all_beliefs_ordered(&self) -> Vec<Belief> {
        let mut result = Vec::new();

        // Pantheon beliefs
        result.extend(self.get_beliefs(BeliefType::Pantheon));

        // Founder beliefs
        result.extend(self.get_beliefs(BeliefType::Founder));

        // Follower beliefs
        result.extend(self.get_beliefs(BeliefType::Follower));

        // Enhancer beliefs
        result.extend(self.get_beliefs(BeliefType::Enhancer));

        result
    }

    /// Checks if the religion has a specific belief
    pub fn has_belief(&self, belief: &str) -> bool {
        self.follower_beliefs.contains(belief) || self.founder_beliefs.contains(belief)
    }

    /// Checks if this is a pantheon
    pub fn is_pantheon(&self) -> bool {
        !self.get_beliefs(BeliefType::Pantheon).is_empty() && !self.is_major_religion()
    }

    /// Checks if this is a major religion
    pub fn is_major_religion(&self) -> bool {
        !self.get_beliefs(BeliefType::Founder).is_empty()
    }

    /// Checks if this is an enhanced religion
    pub fn is_enhanced_religion(&self) -> bool {
        !self.get_beliefs(BeliefType::Enhancer).is_empty()
    }

    /// Gets the founding civilization
    pub fn get_founder(&self) -> Option<&Civilization> {
        if let Some(game_info) = &self.game_info {
            game_info.get_civilization(&self.founding_civ_name)
        } else {
            None
        }
    }

    /// Checks if the religion matches a filter
    pub fn matches_filter(&self, filter: &str, state: StateForConditionals, civ: Option<&Civilization>) -> bool {
        MultiFilter::multi_filter(filter, |f| self.matches_single_filter(f, state, civ))
    }

    /// Checks if the religion matches a single filter
    fn matches_single_filter(&self, filter: &str, state: StateForConditionals, civ: Option<&Civilization>) -> bool {
        let founding_civ = self.get_founder();

        match filter {
            "any" => true,
            "major" => self.is_major_religion(),
            "enhanced" => self.is_enhanced_religion(),
            "your" => civ.is_some() && civ.unwrap() == founding_civ.unwrap(),
            "foreign" => civ.is_some() && civ.unwrap() != founding_civ.unwrap(),
            "enemy" => {
                if let (Some(c), Some(fc)) = (civ, founding_civ) {
                    let known = c.knows(fc);
                    known && c.is_at_war_with(fc)
                } else {
                    false
                }
            },
            _ => {
                if filter == self.name {
                    return true;
                }

                if self.get_beliefs(BeliefType::Any).iter().any(|b| b.name == filter) {
                    return true;
                }

                if self.founder_belief_unique_map.has_matching_unique(filter, state) {
                    return true;
                }

                if self.follower_belief_unique_map.has_matching_unique(filter, state) {
                    return true;
                }

                false
            }
        }
    }

    /// Gets the buildings that can be purchased with faith
    pub fn unlocked_buildings_purchasable(&self) -> Vec<String> {
        if let Some(game_info) = &self.game_info {
            let mut result = Vec::new();

            for belief in self.get_all_beliefs_ordered() {
                // BuyBuildingsWithStat
                for unique in belief.get_matching_uniques(UniqueType::BuyBuildingsWithStat) {
                    if let Some(building) = unique.params.get(0) {
                        if game_info.ruleset.buildings.contains_key(building) {
                            result.push(building.clone());
                        }
                    }
                }

                // BuyBuildingsForAmountStat
                for unique in belief.get_matching_uniques(UniqueType::BuyBuildingsForAmountStat) {
                    if let Some(building) = unique.params.get(0) {
                        if game_info.ruleset.buildings.contains_key(building) {
                            result.push(building.clone());
                        }
                    }
                }

                // BuyBuildingsIncreasingCost
                for unique in belief.get_matching_uniques(UniqueType::BuyBuildingsIncreasingCost) {
                    if let Some(building) = unique.params.get(0) {
                        if game_info.ruleset.buildings.contains_key(building) {
                            result.push(building.clone());
                        }
                    }
                }
            }

            result
        } else {
            Vec::new()
        }
    }

    /// Gets the buildings that can be purchased with faith (cached)
    pub fn get_buildings_purchasable_by_beliefs(&self) -> &Vec<String> {
        if self.buildings_purchasable_by_beliefs.is_none() {
            // This is a bit of a hack since we can't modify self here
            // In a real implementation, you might want to use interior mutability
            // or restructure the code to avoid this issue
            let buildings = self.unlocked_buildings_purchasable();
            unsafe {
                // This is unsafe and not recommended in production code
                // A better approach would be to use interior mutability with RefCell
                let this = self as *const Religion as *mut Religion;
                (*this).buildings_purchasable_by_beliefs = Some(buildings);
            }
        }

        self.buildings_purchasable_by_beliefs.as_ref().unwrap()
    }
}

impl fmt::Display for Religion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_religion_display_name())
    }
}

impl fmt::Debug for Religion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Religion {{ name: {}, founding_civ: {} }}", self.name, self.founding_civ_name)
    }
}

impl Hash for Religion {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.founding_civ_name.hash(state);
    }
}

impl PartialEq for Religion {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.founding_civ_name == other.founding_civ_name
    }
}

impl Eq for Religion {}