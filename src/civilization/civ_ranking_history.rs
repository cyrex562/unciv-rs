use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::civilization::Civilization;
use crate::ui::screens::victoryscreen::RankingType;

/// Records for each turn (key of outer map) what the score (value of inner map) was for each RankingType.
#[derive(Clone, Serialize, Deserialize)]
pub struct CivRankingHistory {
    #[serde(flatten)]
    rankings: HashMap<i32, HashMap<RankingType, i32>>,
}

impl CivRankingHistory {
    pub fn new() -> Self {
        Self {
            rankings: HashMap::new(),
        }
    }

    /// Records the ranking stats for a civilization at the current turn
    pub fn record_ranking_stats(&mut self, civilization: &Civilization) {
        let mut turn_rankings = HashMap::new();
        for ranking_type in RankingType::iter() {
            turn_rankings.insert(*ranking_type, civilization.get_stat_for_ranking(*ranking_type));
        }
        self.rankings.insert(civilization.game_info.turns, turn_rankings);
    }

    /// Gets the rankings for a specific turn
    pub fn get_rankings_for_turn(&self, turn: i32) -> Option<&HashMap<RankingType, i32>> {
        self.rankings.get(&turn)
    }

    /// Gets all recorded turns
    pub fn get_turns(&self) -> Vec<i32> {
        self.rankings.keys().copied().collect()
    }
}

impl Default for CivRankingHistory {
    fn default() -> Self {
        Self::new()
    }
}