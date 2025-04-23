use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::city::city::City;
use crate::civilization::civ_constructions::CivConstructions;
use crate::civilization::diplomacy::diplomacy_manager::DiplomacyManager;
use crate::civilization::managers::espionage_manager::EspionageManager;
use crate::civilization::managers::great_person_manager::GreatPersonManager;
use crate::civilization::managers::quest_manager::QuestManager;
use crate::civilization::managers::religion_manager::ReligionManager;
use crate::civilization::managers::turn_manager::TurnManager;
use crate::civilization::managers::victory_manager::VictoryManager;
use crate::civilization::transients::civ_info_stats_for_next_turn::CivInfoStatsForNextTurn;
use crate::civilization::transients::civ_info_transient_cache::CivInfoTransientCache;
use crate::game_info::GameInfo;
use crate::models::civilization::{TechManager, ThreatManager};
use crate::models::tile::Tile;
use crate::ruleset::nation::nation::Nation;

/// Represents a civilization in the game
#[derive(Serialize, Deserialize)]
pub struct Civilization {
    /// Unique identifier for this civilization
    pub id: String,

    /// The nation this civilization represents
    pub nation: Arc<Nation>,

    /// The cities belonging to this civilization
    pub cities: Vec<Arc<City>>,

    /// The tiles this civilization owns
    pub owned_tiles: HashSet<Arc<Tile>>,

    /// The civilization's current gold amount
    pub gold: i32,

    /// The civilization's science amount
    pub science: i32,

    /// The civilization's culture amount
    pub culture: i32,

    /// The civilization's faith amount
    pub faith: i32,

    /// The civilization's happiness level
    pub happiness: i32,

    /// The civilization's golden age turns remaining
    pub golden_age_turns: i32,

    /// The civilization's era
    pub era: String,

    /// The civilization's difficulty level
    pub difficulty: String,

    /// Whether this civilization is controlled by AI
    pub is_ai: bool,

    /// Whether this civilization is a city-state
    pub is_city_state: bool,

    /// The civilization's capital city
    pub capital: Option<Arc<City>>,

    /// The civilization's stats for the next turn
    #[serde(skip)]
    pub stats_for_next_turn: Option<CivInfoStatsForNextTurn>,

    /// The civilization's transient cache
    #[serde(skip)]
    pub transient_cache: Option<CivInfoTransientCache>,

    /// Reference to the game info
    #[serde(skip)]
    pub game_info: Arc<GameInfo>,

    // Managers
    pub constructions: CivConstructions,
    pub city_state_manager: CityStateManager,
    pub diplomacy_manager: DiplomacyManager,
    pub espionage_manager: EspionageManager,
    pub gold_manager: GoldManager,
    pub great_person_manager: GreatPersonManager,
    pub notification_manager: NotificationManager,
    pub quest_manager: QuestManager,
    pub religion_manager: ReligionManager,
    pub science_manager: ScienceManager,
    pub tech_manager: TechManager,
    pub threat_manager: ThreatManager,
    pub turn_manager: TurnManager,
    pub victory_manager: VictoryManager,
    pub civ_name: String,
}

impl Civilization {
    /// Creates a new civilization
    pub fn new(nation: Arc<Nation>, game_info: Arc<GameInfo>, is_ai: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            nation,
            cities: Vec::new(),
            owned_tiles: HashSet::new(),
            gold: 0,
            science: 0,
            culture: 0,
            faith: 0,
            happiness: 0,
            golden_age_turns: 0,
            era: "Ancient".to_string(),
            difficulty: "Prince".to_string(),
            is_ai,
            is_city_state: false,
            capital: None,
            stats_for_next_turn: None,
            transient_cache: None,
            game_info,
            constructions: CivConstructions::new(),
            city_state_manager: CityStateManager::new(),
            diplomacy_manager: DiplomacyManager::new(),
            espionage_manager: EspionageManager::new(),
            gold_manager: GoldManager::new(),
            great_person_manager: GreatPersonManager::new(),
            notification_manager: NotificationManager::new(),
            quest_manager: QuestManager::new(),
            religion_manager: ReligionManager::new(),
            science_manager: ScienceManager::new(),
            tech_manager: TechManager::new(),
            threat_manager: ThreatManager::new(),
            turn_manager: TurnManager::new(),
            victory_manager: VictoryManager::new(),
        }
    }

    /// Sets up transient references
    pub fn set_transients(&mut self, game_info: Arc<GameInfo>) {
        self.game_info = game_info.clone();

        // Set up transients for all managers
        self.constructions.set_transients(Arc::new(self.clone()));
        self.city_state_manager
            .set_transients(Arc::new(self.clone()));
        self.diplomacy_manager
            .set_transients(Arc::new(self.clone()));
        self.espionage_manager
            .set_transients(Arc::new(self.clone()));
        self.gold_manager.set_transients(Arc::new(self.clone()));
        self.great_person_manager
            .set_transients(Arc::new(self.clone()));
        self.notification_manager
            .set_transients(Arc::new(self.clone()));
        self.quest_manager.set_transients(Arc::new(self.clone()));
        self.religion_manager.set_transients(Arc::new(self.clone()));
        self.science_manager.set_transients(Arc::new(self.clone()));
        self.tech_manager.set_transients(Arc::new(self.clone()));
        self.threat_manager.set_transients(Arc::new(self.clone()));
        self.turn_manager.set_transients(Arc::new(self.clone()));
        self.victory_manager.set_transients(Arc::new(self.clone()));

        // Set up transients for cities
        for city in &mut self.cities {
            Arc::get_mut(city)
                .unwrap()
                .set_transients(Arc::new(self.clone()));
        }

        // Initialize transient cache
        self.transient_cache = Some(CivInfoTransientCache::new(Arc::new(self.clone())));

        // Initialize stats for next turn
        self.stats_for_next_turn = Some(CivInfoStatsForNextTurn::new(Arc::new(self.clone())));
    }

    /// Gets the civilization's current stats
    pub fn get_stats(&self) -> Stats {
        let mut stats = Stats::new();

        // Add stats from cities
        for city in &self.cities {
            stats.add(&city.get_stats());
        }

        // Add stats from policies, buildings, etc.
        stats.add(&self.get_stats_from_policies());
        stats.add(&self.get_stats_from_buildings());
        stats.add(&self.get_stats_from_uniques());

        stats
    }

    /// Gets stats from civilization's policies
    fn get_stats_from_policies(&self) -> Stats {
        let mut stats = Stats::new();
        // TODO: Implement policy stats calculation
        stats
    }

    /// Gets stats from civilization's buildings
    fn get_stats_from_buildings(&self) -> Stats {
        let mut stats = Stats::new();
        // TODO: Implement building stats calculation
        stats
    }

    /// Gets stats from civilization's unique abilities
    fn get_stats_from_uniques(&self) -> Stats {
        let mut stats = Stats::new();
        // TODO: Implement unique ability stats calculation
        stats
    }

    /// Gets the equivalent building for this civilization
    pub fn get_equivalent_building(&self, building_name: &str) -> Arc<Building> {
        // TODO: Implement building equivalence lookup
        self.game_info
            .ruleset
            .buildings
            .get(building_name)
            .unwrap_or_else(|| panic!("Building {} not found", building_name))
            .clone()
    }

    /// Gets all matching uniques of a certain type
    pub fn get_matching_uniques(
        &self,
        unique_type: UniqueType,
        state: StateForConditionals,
    ) -> Vec<Arc<Unique>> {
        let mut uniques = Vec::new();

        // Add uniques from nation
        uniques.extend(self.nation.get_matching_uniques(unique_type, state.clone()));

        // Add uniques from cities
        for city in &self.cities {
            uniques.extend(city.get_matching_uniques(unique_type, state.clone()));
        }

        // Add uniques from policies, etc.
        // TODO: Implement other sources of uniques

        uniques
    }
}

impl Clone for Civilization {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            nation: self.nation.clone(),
            cities: self.cities.clone(),
            owned_tiles: self.owned_tiles.clone(),
            gold: self.gold,
            science: self.science,
            culture: self.culture,
            faith: self.faith,
            happiness: self.happiness,
            golden_age_turns: self.golden_age_turns,
            era: self.era.clone(),
            difficulty: self.difficulty.clone(),
            is_ai: self.is_ai,
            is_city_state: self.is_city_state,
            capital: self.capital.clone(),
            stats_for_next_turn: None, // Don't clone transients
            transient_cache: None,     // Don't clone transients
            game_info: self.game_info.clone(),
            constructions: self.constructions.clone(),
            city_state_manager: self.city_state_manager.clone(),
            diplomacy_manager: self.diplomacy_manager.clone(),
            espionage_manager: self.espionage_manager.clone(),
            gold_manager: self.gold_manager.clone(),
            great_person_manager: self.great_person_manager.clone(),
            notification_manager: self.notification_manager.clone(),
            quest_manager: self.quest_manager.clone(),
            religion_manager: self.religion_manager.clone(),
            science_manager: self.science_manager.clone(),
            tech_manager: self.tech_manager.clone(),
            threat_manager: self.threat_manager.clone(),
            turn_manager: self.turn_manager.clone(),
            victory_manager: self.victory_manager.clone(),
        }
    }
}
