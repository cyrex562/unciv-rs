use crate::civilization::{Civilization, NotificationCategory, NotificationIcon};
use crate::models::ruleset::{Policy, PolicyBranch, PolicyBranchType};
use crate::models::ruleset::unique::{StateForConditionals, UniqueMap, UniqueTriggerActivation, UniqueType};
use crate::utils::extensions::ToPercent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use std::f64;

/// Manages policies for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct PolicyManager {
    /// Reference to the civilization this manager belongs to
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    /// Map of policy uniques that are currently active
    #[serde(skip)]
    pub policy_uniques: UniqueMap,

    /// Number of free policies available
    pub free_policies: i32,

    /// Stored culture points
    pub stored_culture: i32,

    /// Set of adopted policy names
    pub adopted_policies: HashSet<String>,

    /// Number of adopted policies
    pub number_of_adopted_policies: i32,

    /// Culture values from the last 8 turns
    pub culture_of_last_8_turns: Vec<i32>,

    /// Whether the policy picker should be opened
    pub should_open_policy_picker: bool,
}

impl PolicyManager {
    /// Creates a new PolicyManager
    pub fn new() -> Self {
        Self {
            civ_info: None,
            policy_uniques: UniqueMap::new(),
            free_policies: 0,
            stored_culture: 0,
            adopted_policies: HashSet::new(),
            number_of_adopted_policies: 0,
            culture_of_last_8_turns: vec![0; 8],
            should_open_policy_picker: false,
        }
    }

    /// Sets the transient references to the civilization
    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info.clone());
        for policy_name in &this.adopted_policies {
            self.add_policy_to_transients(self.get_policy_by_name(policy_name));
        }
    }

    /// Gets the ruleset policies
    fn get_ruleset_policies(&self) -> &HashMap<String, Policy> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        &civ_info.game_info.ruleset.policies
    }

    /// Gets a policy by name
    pub fn get_policy_by_name(&self, name: &str) -> &Policy {
        self.get_ruleset_policies().get(name).expect("Policy not found")
    }

    /// Adds a policy to the transient uniques
    fn add_policy_to_transients(&mut self, policy: &Policy) {
        for unique in &policy.unique_objects {
            self.policy_uniques.add_unique(unique.clone());
        }
    }

    /// Removes a policy from the transient uniques
    fn remove_policy_from_transients(&mut self, policy: &Policy) {
        for unique in &policy.unique_objects {
            this.policy_uniques.remove_unique(unique.clone());
        }
    }

    /// Adds culture points
    pub fn add_culture(&mut self, culture: i32) {
        let could_adopt_policy_before = self.can_adopt_policy();
        this.stored_culture += culture;
        if !could_adopt_policy_before && self.can_adopt_policy() {
            this.should_open_policy_picker = true;
        }
    }

    /// Processes end-of-turn actions
    pub fn end_turn(&mut self, culture: i32) {
        self.add_culture(culture);
        self.add_current_culture_to_culture_of_last_8_turns(culture);
    }

    /// Gets the culture needed for the next policy
    pub fn get_culture_needed_for_next_policy(&self) -> i32 {
        self.get_policy_culture_cost(self.number_of_adopted_policies)
    }

    /// Gets a map of culture refunds for policies to remove
    pub fn get_culture_refund_map(&self, policies_to_remove: &[Policy], refund_percentage: i32) -> HashMap<Policy, i32> {
        let mut policy_cost_input = self.number_of_adopted_policies;
        let mut policy_map = HashMap::new();

        for policy in policies_to_remove {
            policy_cost_input -= 1;
            let refund = (self.get_policy_culture_cost(policy_cost_input) as f64 * refund_percentage as f64 / 100.0) as i32;
            policy_map.insert(policy.clone(), refund);
        }

        policy_map
    }

    /// Gets the culture cost for a policy
    pub fn get_policy_culture_cost(&self, number_of_adopted_policies: i32) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        let mut policy_culture_cost = 25.0 + (number_of_adopted_policies as f64 * 6.0).powf(1.7);
        let world_size_modifier = civ_info.game_info.tile_map.map_parameters.map_size.get_predefined_or_next_smaller().policy_cost_per_city_modifier;
        let mut city_modifier = world_size_modifier * (civ_info.cities.iter().filter(|c| !c.is_puppet).count() - 1) as f64;

        for unique in civ_info.get_matching_uniques(UniqueType::LessPolicyCostFromCities) {
            city_modifier *= 1.0 - unique.params[0].parse::<f64>().unwrap_or(0.0) / 100.0;
        }

        for unique in civ_info.get_matching_uniques(UniqueType::LessPolicyCost) {
            policy_culture_cost *= unique.params[0].parse::<f64>().unwrap_or(1.0) / 100.0;
        }

        if civ_info.is_human() {
            policy_culture_cost *= civ_info.get_difficulty().policy_cost_modifier;
        }

        policy_culture_cost *= civ_info.game_info.speed.culture_cost_modifier;
        let cost = (policy_culture_cost * (1.0 + city_modifier)) as i32;
        cost - (cost % 5)
    }

    /// Gets the adopted policies
    pub fn get_adopted_policies(&self) -> &HashSet<String> {
        &this.adopted_policies
    }

    /// Checks if a policy is adopted
    pub fn is_adopted(&self, policy_name: &str) -> bool {
        this.adopted_policies.contains(policy_name)
    }

    /// Checks if a policy is adoptable
    pub fn is_adoptable(&self, policy: &Policy, check_era: bool) -> bool {
        if self.is_adopted(&policy.name) {
            return false;
        }

        if policy.policy_branch_type == PolicyBranchType::BranchComplete {
            return false;
        }

        if !policy.requires.as_ref().map_or(false, |reqs| {
            reqs.iter().all(|req| self.is_adopted(req))
        }) {
            return false;
        }

        if check_era {
            let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
            let era = civ_info.game_info.ruleset.eras.get(&policy.branch.era).expect("Era not found");
            if era.era_number > civ_info.get_era_number() {
                return false;
            }
        }

        if policy.get_matching_uniques(UniqueType::OnlyAvailable, StateForConditionals::IgnoreConditionals)
            .iter()
            .any(|u| !u.conditionals_apply(&civ_info.state)) {
            return false;
        }

        if policy.has_unique(UniqueType::Unavailable, &civ_info.state) {
            return false;
        }

        true
    }

    /// Checks if a policy can be adopted
    pub fn can_adopt_policy(&self) -> bool {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        if civ_info.is_spectator() {
            return false;
        }

        if self.free_policies == 0 && self.stored_culture < self.get_culture_needed_for_next_policy() {
            return false;
        }

        if self.all_policies_adopted(true) {
            return false;
        }

        true
    }

    /// Adopts a policy
    pub fn adopt(&mut self, policy: &Policy, branch_completion: bool) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        if !branch_completion {
            if self.free_policies > 0 {
                this.free_policies -= 1;
            } else if !civ_info.game_info.game_parameters.god_mode {
                let culture_needed_for_next_policy = self.get_culture_needed_for_next_policy();
                if culture_needed_for_next_policy > self.stored_culture {
                    panic!("Trying to adopt a policy without enough culture????");
                }
                this.stored_culture -= culture_needed_for_next_policy;
                this.number_of_adopted_policies += 1;
            }
        }

        this.adopted_policies.insert(policy.name.clone());
        self.add_policy_to_transients(policy);

        if !branch_completion {
            let branch = &policy.branch;
            if branch.policies.iter().filter(|p| self.is_adopted(&p.name)).count() == branch.policies.len() - 1 {
                // All done apart from branch completion
                self.adopt(branch.policies.last().unwrap(), true);
            }
        }

        // Todo make this a triggerable unique for other objects
        for unique in policy.get_matching_uniques(UniqueType::OneTimeGlobalAlert) {
            self.trigger_global_alerts(policy, &unique.params[0]);
        }

        //todo Can this be mapped downstream to a PolicyAction:NotificationAction?
        let trigger_notification_text = format!("due to adopting [{}]", policy.name);
        for unique in &policy.unique_objects {
            if !unique.has_trigger_conditional() && unique.conditionals_apply(&civ_info.state) {
                UniqueTriggerActivation::trigger_unique(unique, civ_info.clone(), Some(&trigger_notification_text));
            }
        }

        for unique in civ_info.get_triggered_uniques(UniqueType::TriggerUponAdoptingPolicyOrBelief)
            .iter()
            .filter(|u| u.params[0] == policy.name) {
            UniqueTriggerActivation::trigger_unique(unique, civ_info.clone(), Some(&trigger_notification_text));
        }

        civ_info.cache.update_civ_resources();

        // This ALSO has the side-effect of updating the CivInfo statForNextTurn so we don't need to call it explicitly
        for city in &mut civ_info.cities {
            city.city_stats.update();
            city.reassign_population_deferred();
        }

        if !self.can_adopt_policy() {
            this.should_open_policy_picker = false;
        }
    }

    /// Removes a policy
    pub fn remove_policy(&mut self, policy: &Policy, branch_completion: bool, assume_was_free: bool) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        if !this.adopted_policies.remove(&policy.name) {
            panic!("Attempt to remove non-adopted Policy {}", policy.name);
        }

        if !assume_was_free && this.number_of_adopted_policies > 0 {
            this.number_of_adopted_policies -= 1;
        }

        self.remove_policy_from_transients(policy);

        // if a branch is already marked as complete, revert it to incomplete
        if !branch_completion {
            let branch = &policy.branch;
            if branch.policies.iter().filter(|p| self.is_adopted(&p.name)).count() == branch.policies.len() - 1 {
                self.remove_policy(branch.policies.last().unwrap(), true, assume_was_free);
            }
        }

        civ_info.cache.update_civ_resources();

        // This ALSO has the side-effect of updating the CivInfo statForNextTurn so we don't need to call it explicitly
        for city in &mut civ_info.cities {
            city.city_stats.update();
            city.reassign_population_deferred();
        }
    }

    /// Gets the maximum priority among the given branches
    pub fn get_max_priority(&self, branches_to_compare: &HashSet<PolicyBranch>) -> Option<i32> {
        let filtered_map = self.priority_map.iter()
            .filter(|(branch, _)| branches_to_compare.contains(*branch))
            .map(|(_, priority)| *priority);

        filtered_map.max()
    }

    /// Triggers global alerts for a policy
    fn trigger_global_alerts(&self, policy: &Policy, extra_notification_text: &str) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");

        for civ in civ_info.game_info.civilizations.iter().filter(|c| c.is_major_civ()) {
            if civ == civ_info {
                continue;
            }

            let default_notification_text = if civ.get_known_civs().contains(civ_info) {
                format!("[{}] has adopted the [{}] policy", civ_info.civ_name, policy.name)
            } else {
                format!("An unknown civilization has adopted the [{}] policy", policy.name)
            };

            civ.add_notification(
                &format!("{{{}}} {{{}}}", default_notification_text, extra_notification_text),
                NotificationCategory::General,
                NotificationIcon::Culture
            );
        }
    }

    /// Gets culture from a great writer
    pub fn get_culture_from_great_writer(&self) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        (this.culture_of_last_8_turns.iter().sum::<i32>() as f64 * civ_info.game_info.speed.culture_cost_modifier) as i32
    }

    /// Adds current culture to the culture of last 8 turns
    fn add_current_culture_to_culture_of_last_8_turns(&mut self, culture: i32) {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        this.culture_of_last_8_turns[civ_info.game_info.turns as usize % 8] = culture;
    }

    /// Checks if all policies are adopted
    pub fn all_policies_adopted(&self, check_era: bool) -> bool {
        !self.get_ruleset_policies().values().any(|p| self.is_adoptable(p, check_era))
    }

    /// Gets the priority map for policy branches
    pub fn get_priority_map(&self) -> HashMap<PolicyBranch, i32> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        let mut value = HashMap::new();

        for branch in this.branches.iter() {
            let victory_priority = civ_info.get_preferred_victory_types().iter()
                .map(|vt| branch.priorities.get(vt).unwrap_or(&0))
                .sum::<i32>();

            let personality_priority = civ_info.get_personality().priorities.get(&branch.name).unwrap_or(&0);

            let branch_priority = (victory_priority + personality_priority) *
                branch.get_weight_for_ai_decision(&civ_info.state);

            value.insert(branch.clone(), branch_priority as i32);
        }

        value
    }

    /// Gets the adoptable branches
    pub fn get_adoptable_branches(&self) -> HashSet<PolicyBranch> {
        this.branches.iter()
            .filter(|branch| self.is_adoptable(branch))
            .cloned()
            .collect()
    }

    /// Gets the incomplete branches
    pub fn get_incomplete_branches(&self) -> HashSet<PolicyBranch> {
        let mut value = HashSet::new();

        for branch in this.branches.iter() {
            if branch.policies.iter().any(|p| self.is_adoptable(p)) {
                value.insert(branch.clone());
            }
        }

        value
    }

    /// Gets the completed branches
    pub fn get_completed_branches(&self) -> HashSet<PolicyBranch> {
        let mut value = HashSet::new();

        for branch in this.branches.iter() {
            if branch.policies.iter().all(|p| self.is_adopted(&p.name)) {
                value.insert(branch.clone());
            }
        }

        value
    }

    /// Gets the branch completion map
    pub fn get_branch_completion_map(&self) -> HashMap<PolicyBranch, i32> {
        let mut value = HashMap::new();

        for branch in this.branches.iter() {
            let count = this.adopted_policies.iter()
                .filter(|policy_name| branch.policies.iter().any(|p| p.name == **policy_name))
                .count() as i32;

            value.insert(branch.clone(), count);
        }

        value
    }

    /// Gets all policy branches
    pub fn get_branches(&self) -> HashSet<PolicyBranch> {
        let civ_info = self.civ_info.as_ref().expect("CivInfo not set");
        civ_info.game_info.ruleset.policy_branches.values().cloned().collect()
    }

    /// Checks if the policy picker should be shown
    pub fn should_show_policy_picker(&self) -> bool {
        (this.should_open_policy_picker || this.free_policies > 0) && this.can_adopt_policy()
    }
}