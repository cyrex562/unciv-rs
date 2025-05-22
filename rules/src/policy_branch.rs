use std::collections::HashMap;
use std::fmt;

use crate::models::ruleset::policy::Policy;
use crate::models::ruleset::unique::UniqueTarget;

/// Represents a policy branch in the game, which is a collection of related policies
pub struct PolicyBranch {
    // Base Policy fields
    pub name: String,
    pub uniques: Vec<String>,
    pub unique_objects: Vec<Unique>,
    pub unique_map: UniqueMap,
    pub branch: PolicyBranchRef,
    pub row: i32,
    pub column: i32,
    pub requires: Option<Vec<String>>,
    pub policy_branch_type: PolicyBranchType,

    // PolicyBranch specific fields
    pub policies: Vec<Policy>,
    pub priorities: HashMap<String, i32>,
    pub era: String,
}

/// A reference to the PolicyBranch itself (used for the branch field)
pub struct PolicyBranchRef {
    pub name: String,
    pub era: String,
}

impl PolicyBranch {
    /// Creates a new PolicyBranch instance
    pub fn new(name: String, era: String) -> Self {
        let branch_ref = PolicyBranchRef {
            name: name.clone(),
            era: era.clone(),
        };

        Self {
            name,
            uniques: Vec::new(),
            unique_objects: Vec::new(),
            unique_map: UniqueMap::new(),
            branch: branch_ref,
            row: 0,
            column: 0,
            requires: None,
            policy_branch_type: PolicyBranchType::BranchStart,
            policies: Vec::new(),
            priorities: HashMap::new(),
            era,
        }
    }

    /// Adds a policy to this branch
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Sets the priority for a policy
    pub fn set_priority(&mut self, policy_name: String, priority: i32) {
        self.priorities.insert(policy_name, priority);
    }

    /// Gets the priority for a policy
    pub fn get_priority(&self, policy_name: &str) -> i32 {
        *self.priorities.get(policy_name).unwrap_or(&0)
    }

    /// Checks if this is a policy branch
    pub fn is_policy_branch(&self) -> bool {
        true
    }
}

impl RulesetObject for PolicyBranch {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_uniques(&self) -> &[String] {
        &self.uniques
    }

    fn get_unique_objects(&self) -> &[Unique] {
        &self.unique_objects
    }

    fn get_unique_map(&self) -> &UniqueMap {
        &self.unique_map
    }

    fn make_link(&self) -> String {
        format!("PolicyBranch/{}", self.name)
    }

    fn get_sort_group(&self, ruleset: &Ruleset) -> i32 {
        let era = ruleset.eras.get(&self.era).unwrap();
        let era_number = era.era_number;
        let branch_index = ruleset
            .policy_branches
            .keys()
            .position(|name| name == &self.name)
            .unwrap_or(0);

        era_number * 10000 + branch_index as i32 * 100 + self.policy_branch_type as i32
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        let mut line_list = Vec::new();

        let era = ruleset.eras.get(&self.era);
        let era_color = era.map(|e| e.get_hex_color()).unwrap_or_default();
        let era_link = era.map(|e| e.make_link()).unwrap_or_default();

        line_list.push(FormattedLine::new(
            format!("{{Unlocked at}} {{{}}}", self.era),
            4,
            Some(era_color),
            Some(era_link),
            None,
        ));

        // Add policies in this branch
        if !self.policies.is_empty() {
            line_list.push(FormattedLine::new(
                "Policies in this branch:".to_string(),
                0,
                None,
                None,
                None,
            ));

            for policy in &self.policies {
                line_list.push(FormattedLine::new(
                    policy.name.clone(),
                    0,
                    None,
                    Some(policy.make_link()),
                    Some(1),
                ));
            }
        }

        uniques_to_civilopedia_text_lines(&mut line_list, this);

        line_list
    }
}

impl IHasUniques for PolicyBranch {
    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Policy
    }
}

impl fmt::Display for PolicyBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
