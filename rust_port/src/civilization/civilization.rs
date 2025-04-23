use crate::city::city::City;
use crate::civilization::civ_constructions::CivConstructions;
use crate::diplomacy::manager::DiplomacyManager;
use crate::civilization::managers::espionage_manager::EspionageManager;
use crate::civilization::managers::great_person_manager::GreatPersonManager;
use crate::civilization::managers::quest_manager::QuestManager;
use crate::civilization::managers::religion_manager::ReligionManager;
use crate::civilization::managers::turn_manager::TurnManager;
use crate::civilization::managers::victory_manager::VictoryManager;
use crate::civilization::transients::civ_info_stats_for_next_turn::CivInfoStatsForNextTurn;
use crate::civilization::transients::civ_info_transient_cache::CivInfoTransientCache;
use crate::game_info::GameInfo;
use crate::models::civilization::{PopupAlert, TechManager, ThreatManager, UnitManager};
use crate::models::map_unit::MapUnit;
use crate::ai::personality::PersonalityValue;
use crate::tile::tile::Tile;
use crate::models::TradeRequest;
use crate::ruleset::building::Building;
use crate::ruleset::nation::nation::Nation;
use crate::stats::stats::Stats;
use crate::unique::state_for_conditionals::StateForConditionals;
use crate::unique::UniqueType;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Represents a civilization in the game
#[derive(Serialize, Deserialize)]
pub struct Civilization {
    pub units: UnitManager,
    // pub game_info: crate::models::game_info::GameInfo,
    pub popup_alerts: Vec<PopupAlert>,
    pub name: String,
    pub is_barbarian: bool,
    pub is_spectator: bool,
    // pub is_city_state: bool,
    pub personality: HashMap<PersonalityValue, f32>,
    pub diplomacy_managers: HashMap<String, crate::models::DiplomacyManager>,
    // pub threat_manager: ThreatManager,
    // pub cities: Vec<crate::models::civilization::City>,
    pub tech: TechManager,
    pub trade_requests: Vec<TradeRequest>,

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

    /// Gets all units belonging to this civilization.
    pub fn get_civ_units(&self) -> Vec<&MapUnit> {
        self.units.get_civ_units()
    }

    /// Gets the personality value for the given personality type.
    pub fn get_personality(&self) -> &HashMap<PersonalityValue, f32> {
        &self.personality
    }

    /// Gets the diplomacy manager for the given civilization.
    pub fn get_diplomacy_manager(
        &self,
        other_civ: &crate::models::civilization::Civilization,
    ) -> Option<&crate::models::DiplomacyManager> {
        self.diplomacy_managers.get(&other_civ.name)
    }

    /// Gets the diplomacy manager for the given civilization (mutable).
    pub fn get_diplomacy_manager_mut(
        &mut self,
        other_civ: &crate::models::civilization::Civilization,
    ) -> Option<&mut crate::models::DiplomacyManager> {
        self.diplomacy_managers.get_mut(&other_civ.name)
    }

    /// Gets the stat for the given ranking type.
    pub fn get_stat_for_ranking(&self, ranking_type: RankingType) -> f32 {
        match ranking_type {
            RankingType::Force => self.threat_manager.get_force(),
            RankingType::Score => self.threat_manager.get_score(),
            _ => 0.0, // Placeholder for other ranking types
        }
    }

    /// Gets the capital city of this civilization.
    pub fn get_capital(&self) -> Option<&crate::models::civilization::City> {
        self.cities.iter().find(|city| city.is_capital)
    }

    /// Gets all civilizations this civilization is at war with.
    pub fn get_civs_at_war_with(&self) -> Vec<&crate::models::civilization::Civilization> {
        let mut war_civs = Vec::new();
        for (civ_name, diplo_manager) in &self.diplomacy_managers {
            if diplo_manager.get_diplomatic_status()
                == crate::models::diplomacy::DiplomaticStatus::War
            {
                // This is a placeholder - in a real implementation, we would look up the civilization by name
                // For now, we'll just return an empty vector
            }
        }
        war_civs
    }

    /// Checks if this civilization is at war with the given civilization.
    pub fn is_at_war_with(&self, other_civ: &crate::models::civilization::Civilization) -> bool {
        if let Some(diplo_manager) = self.get_diplomacy_manager(other_civ) {
            diplo_manager.get_diplomatic_status() == crate::models::diplomacy::DiplomaticStatus::War
        } else {
            false
        }
    }

    /// Checks if this civilization is a major civilization.
    pub fn is_major_civ(&self) -> bool {
        !self.is_barbarian && !self.is_city_state && !self.is_spectator
    }

    /// Checks if this civilization has the given unique type.
    pub fn has_unique(&self, unique_type: crate::unique_type::UniqueType) -> bool {
        // Placeholder implementation
        false
    }

    /// Checks if this civilization has explored the given tile.
    pub fn has_explored(&self, tile: &crate::tile::tile::Tile) -> bool {
        // Placeholder implementation
        false
    }

    /// Adds a notification to this civilization.
    pub fn add_notification(
        &mut self,
        message: String,
        position: crate::models::game_info::Position,
        category: crate::models::civilization::NotificationCategory,
        icon: crate::models::civilization::NotificationIcon,
    ) {
        // Placeholder implementation
    }

    /// Sets the last seen improvement at the given position.
    pub fn set_last_seen_improvement(
        &mut self,
        position: crate::models::game_info::Position,
        improvement: String,
    ) {
        // Placeholder implementation
    }

    /// Checks if this civilization is defeated.
    pub fn is_defeated(&self) -> bool {
        // Placeholder implementation
        false
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
