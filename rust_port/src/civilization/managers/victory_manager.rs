use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::civilization::Civilization;
use crate::models::Counter;
use crate::models::ruleset::{Milestone, Victory};
use crate::models::ruleset::unique::{StateForConditionals, UniqueType};
use crate::utils::{Constants, UncivGame};
use crate::utils::translations::tr;

/// Manages victory conditions and diplomatic voting for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct VictoryManager {
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,

    // There is very likely a typo in this name (currents), but as its saved in save files,
    // fixing it is non-trivial
    pub currents_spaceship_parts: Counter<String>,
    pub has_ever_won_diplomatic_vote: bool,
}

impl VictoryManager {
    pub fn new() -> Self {
        Self {
            civ_info: None,
            currents_spaceship_parts: Counter::new(),
            has_ever_won_diplomatic_vote: false,
        }
    }

    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.currents_spaceship_parts = self.currents_spaceship_parts.clone();
        to_return.has_ever_won_diplomatic_vote = self.has_ever_won_diplomatic_vote;
        to_return
    }

    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info);
    }

    /// Calculates the results of diplomatic voting
    fn calculate_diplomatic_voting_results(&self, votes_cast: &HashMap<String, Option<String>>) -> Counter<String> {
        let mut results = Counter::new();

        // UN Owner gets 2 votes in G&K
        let (_, civ_owning_un) = self.get_un_building_and_owner_names();

        for (voter, voted_for) in votes_cast {
            if voted_for.is_none() {
                continue; // null means Abstained
            }

            let vote_count = if voter == &civ_owning_un {
                2
            } else {
                1
            };

            results.add(voted_for.as_ref().unwrap(), vote_count);
        }

        results
    }

    /// Gets all civilizations that can vote (not barbarian, spectator, or defeated)
    fn get_voting_civs(&self) -> Vec<&Civilization> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        civ_info.game_info.civilizations.iter()
            .filter(|civ| !civ.is_barbarian && !civ.is_spectator() && !civ.is_defeated())
            .collect()
    }

    /// Finds the Building and Owner of the United Nations (or whatever the Mod called it)
    /// - if it's built at all and only if the owner is alive
    ///
    /// # Returns
    ///
    /// `first`: Building name, `second`: Owner civ name; both None if not found
    pub fn get_un_building_and_owner_names(&self) -> (Option<String>, Option<String>) {
        let voting_civs = self.get_voting_civs();

        for civ in voting_civs {
            for city in &civ.cities {
                for building in city.city_constructions.get_built_buildings() {
                    if building.has_unique(UniqueType::OneTimeTriggerVoting, StateForConditionals::IgnoreConditionals) {
                        return (Some(building.name.clone()), Some(civ.civ_name.clone()));
                    }
                }
            }
        }

        (None, None)
    }

    /// Calculates the number of votes needed for diplomatic victory
    fn votes_needed_for_diplomatic_victory(&self) -> i32 {
        // The original counts "teams ever alive", which excludes Observer and Barbarians.
        // The "ever alive" part sounds unfair - could make a Vote unwinnable?

        // So this is a slightly arbitrary decision: Apply original formula to count of available votes
        // - including catering for the possibility we're voting without a UN thanks to razing -
        let (_, civ_owning_un) = self.get_un_building_and_owner_names();
        let vote_count = self.get_voting_civs().len() as i32 + if civ_owning_un.is_some() { 1 } else { 0 };

        // CvGame.cpp::DoUpdateDiploVictory() in the source code of the original - same integer math and rounding!
        // To verify run `(1..30).map { voteCount -> voteCount to voteCount * (67 - (1.1 * voteCount).toInt()) / 100 + 1 }`
        // ... and compare with: "4 votes needed to win in a game with 5 players, 7 with 13 and 11 with 28"
        if vote_count > 28 {
            vote_count * 35 / 100
        } else {
            vote_count * (67 - (1.1 * vote_count as f32) as i32) / 100 + 1
        }
    }

    /// Checks if the civilization has enough votes for diplomatic victory
    pub fn has_enough_votes_for_diplomatic_victory(&self) -> bool {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let results = self.calculate_diplomatic_voting_results(&civ_info.game_info.diplomatic_victory_votes_cast);

        let best_civ = results.iter()
            .max_by_key(|(_, count)| *count);

        if best_civ.is_none() {
            return false;
        }

        let (best_civ_name, best_civ_votes) = best_civ.unwrap();

        // If we don't have the highest score, we have not won anyway
        if best_civ_name != &civ_info.civ_name {
            return false;
        }

        // If we don't have enough votes, we haven't won
        if *best_civ_votes < self.votes_needed_for_diplomatic_victory() {
            return false;
        }

        // If there's a tie, we haven't won either
        !results.iter().any(|(civ_name, votes)| civ_name != best_civ_name && votes == best_civ_votes)
    }

    /// Structure to hold diplomatic victory vote breakdown information
    #[derive(Debug)]
    pub struct DiplomaticVictoryVoteBreakdown {
        pub results: Counter<String>,
        pub winner_text: String,
    }

    /// Gets a breakdown of the diplomatic victory vote
    pub fn get_diplomatic_victory_vote_breakdown(&self) -> DiplomaticVictoryVoteBreakdown {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let results = self.calculate_diplomatic_voting_results(&civ_info.game_info.diplomatic_victory_votes_cast);

        // Group by vote count and get the highest vote count
        let mut vote_groups: HashMap<i32, Vec<String>> = HashMap::new();

        for (civ_name, vote_count) in &results {
            vote_groups.entry(*vote_count).or_insert_with(Vec::new).push(civ_name.clone());
        }

        let max_vote_count = vote_groups.keys().max();

        if max_vote_count.is_none() {
            return DiplomaticVictoryVoteBreakdown {
                results,
                winner_text: "No valid votes were cast.".to_string(),
            };
        }

        let max_vote_count = *max_vote_count.unwrap();
        let winner_list = vote_groups.get(&max_vote_count).unwrap();

        let mut lines = Vec::new();
        let min_votes = self.votes_needed_for_diplomatic_victory();

        if max_vote_count < min_votes {
            lines.push(format!("Minimum votes for electing a world leader: [{}]", min_votes));
        }

        if winner_list.len() > 1 {
            let tied_civs = winner_list.iter()
                .map(|civ_name| tr(civ_name))
                .collect::<Vec<_>>()
                .join(", ");

            lines.push(format!("Tied in first position: [{}]", tied_civs));
        }

        let winner_civ = civ_info.game_info.get_civilization(&winner_list[0]);

        if !lines.is_empty() {
            lines.push("No world leader was elected.".to_string());
        } else if winner_civ.civ_name == civ_info.civ_name {
            lines.push("You have been elected world leader!".to_string());
        } else {
            lines.push(format!("{} has been elected world leader!",
                winner_civ.nation.get_leader_display_name()));
        }

        let winner_text = lines.iter()
            .map(|line| format!("{{{}}}", line))
            .collect::<Vec<_>>()
            .join("\n");

        DiplomaticVictoryVoteBreakdown {
            results,
            winner_text,
        }
    }

    /// Gets the victory type achieved by the civilization
    pub fn get_victory_type_achieved(&self) -> Option<String> {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if !civ_info.is_major_civ() {
            return None;
        }

        let enabled_victories = &civ_info.game_info.game_parameters.victory_types;

        // Check for regular victory types
        for (victory_name, victory) in &civ_info.game_info.ruleset.victories {
            if victory_name != Constants::neutral_victory_type() && enabled_victories.contains(victory_name) {
                if self.get_next_milestone(victory).is_none() {
                    return Some(victory.name.clone());
                }
            }
        }

        // Check for unique-based victory
        if civ_info.has_unique(UniqueType::TriggersVictory) {
            return Some(Constants::neutral_victory_type());
        }

        None
    }

    /// Gets the next milestone for a victory type
    pub fn get_next_milestone(&self, victory: &Victory) -> Option<&Milestone> {
        for milestone in &victory.milestone_objects {
            if !milestone.has_been_completed_by(self.civ_info.as_ref().expect("Civ not set")) {
                return Some(milestone);
            }
        }

        None
    }

    /// Gets the number of milestones completed for a victory type
    pub fn amount_milestones_completed(&self, victory: &Victory) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let mut completed = 0;

        for milestone in &victory.milestone_objects {
            if milestone.has_been_completed_by(civ_info) {
                completed += 1;
            } else {
                break;
            }
        }

        completed
    }

    /// Checks if the civilization has won
    pub fn has_won(&self) -> bool {
        self.get_victory_type_achieved().is_some()
    }
}