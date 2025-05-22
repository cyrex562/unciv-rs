use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::civilization::Civilization;
use crate::map::mapunit::MapUnit;
use crate::models::ruleset::RuinReward;
use crate::models::ruleset::unique::{StateForConditionals, UniqueTriggerActivation, UniqueType};
use crate::utils::random::Random;

/// Manages ruins-related functionality for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct RuinsManager {
    #[serde(skip)]
    pub civ: Option<Arc<Civilization>>,

    #[serde(skip)]
    pub valid_rewards: Vec<RuinReward>,

    // Last two chosen rewards, used to avoid repetition
    last_chosen_rewards: Vec<String>,
}

impl RuinsManager {
    pub fn new() -> Self {
        Self {
            civ: None,
            valid_rewards: Vec::new(),
            last_chosen_rewards: vec!["".to_string(), "".to_string()],
        }
    }

    pub fn clone(&self) -> Self {
        // needs to deep-clone (the Vec, not the Strings) so undo works
        Self {
            civ: self.civ.clone(),
            valid_rewards: self.valid_rewards.clone(),
            last_chosen_rewards: self.last_chosen_rewards.clone(),
        }
    }

    pub fn set_transients(&mut self, civ: Arc<Civilization>) {
        self.civ = Some(civ.clone());
        self.valid_rewards = civ.game_info.ruleset.ruin_rewards.values().cloned().collect();
    }

    fn remember_reward(&mut self, reward: String) {
        self.last_chosen_rewards[0] = self.last_chosen_rewards[1].clone();
        self.last_chosen_rewards[1] = reward;
    }

    fn get_shuffled_possible_rewards(&self, triggering_unit: &MapUnit) -> Vec<RuinReward> {
        let civ = self.civ.as_ref().expect("Civ not set");

        // Filter possible rewards based on conditions
        let mut candidates = self.valid_rewards.iter()
            .filter(|reward| self.is_possible_reward(reward, triggering_unit))
            // For each possible reward, add (reward.weight) copies to implement 'weight'
            .flat_map(|reward| std::iter::repeat(reward).take(reward.weight))
            .cloned()
            .collect::<Vec<_>>();

        // Shuffle the candidates using a tile-based random to thwart save-scumming
        candidates.shuffle(&mut Random::new(triggering_unit.get_tile().position.hash_code()));

        candidates
    }

    fn is_possible_reward(&self, ruin_reward: &RuinReward, unit: &MapUnit) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");

        // Check if reward was recently chosen
        if self.last_chosen_rewards.contains(&ruin_reward.name) {
            return false;
        }

        // Check if reward is unavailable by settings
        if ruin_reward.is_unavailable_by_settings(&civ.game_info) {
            return false;
        }

        // Check if reward has Unavailable unique
        let state_for_conditionals = StateForConditionals::new(civ.clone(), Some(unit.clone()), Some(unit.get_tile().clone()));
        if ruin_reward.has_unique(UniqueType::Unavailable, &state_for_conditionals) {
            return false;
        }

        // Check if reward has OnlyAvailable unique that doesn't apply
        if ruin_reward.get_matching_uniques(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals())
            .iter()
            .any(|unique| !unique.conditionals_apply(&state_for_conditionals)) {
            return false;
        }

        true
    }

    pub fn select_next_ruins_reward(&mut self, triggering_unit: &MapUnit) {
        for possible_reward in self.get_shuffled_possible_rewards(triggering_unit) {
            let mut at_least_one_unique_had_effect = false;

            for unique in &possible_reward.unique_objects {
                let effect = UniqueTriggerActivation::trigger_unique(
                    unique.clone(),
                    triggering_unit.clone(),
                    Some(possible_reward.notification.clone()),
                    Some("from the ruins".to_string())
                );

                at_least_one_unique_had_effect = at_least_one_unique_had_effect || effect;
            }

            if at_least_one_unique_had_effect {
                self.remember_reward(possible_reward.name.clone());
                break;
            }
        }
    }
}