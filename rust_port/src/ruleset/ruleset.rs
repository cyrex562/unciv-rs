use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::Path;

use crate::models::ruleset::{
    belief::Belief,
    building::Building,
    difficulty::Difficulty,
    era::Era,
    event::Event,
    global_uniques::GlobalUniques,
    nation::Nation,
    personality::Personality,
    policy::Policy,
    policy_branch::PolicyBranch,
    ruin_reward::RuinReward,
    quest::Quest,
    specialist::Specialist,
    technology::Technology,
    tech_column::TechColumn,
    terrain::Terrain,
    tile_improvement::TileImprovement,
    tile_resource::TileResource,
    unit::BaseUnit,
    unit_promotion::Promotion,
    unit_type::UnitType,
    victory::Victory,
    city_state_type::CityStateType,
    mod_options::ModOptions,
    speed::Speed,
};
use crate::models::ruleset::unique::{Unique, UniqueType};
use crate::models::stats::{GameResource, Stat, SubStat};
use crate::models::ruleset::validation::RulesetValidator;
use crate::models::ruleset::validation::unique_validator::UniqueValidator;
use crate::models::ruleset::unique::state_for_conditionals::StateForConditionals;
use crate::models::ruleset::tile::road_status::RoadStatus;
use crate::models::ruleset::IRulesetObject;
use crate::models::ICivilopediaText;
use crate::utils::constants::Constants;
use crate::utils::json;
use crate::utils::log::Log;

/// Enum representing the different files in a ruleset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulesetFile {
    Beliefs,
    Buildings,
    Eras,
    Religions,
    Nations,
    Policies,
    Techs,
    Terrains,
    Tutorials,
    TileImprovements,
    TileResources,
    Specialists,
    Units,
    UnitPromotions,
    UnitTypes,
    VictoryTypes,
    CityStateTypes,
    Personalities,
    Events,
    GlobalUniques,
    ModOptions,
    Speeds,
    Difficulties,
    Quests,
    Ruins,
}

impl RulesetFile {
    /// Get the filename for this ruleset file
    pub fn filename(&self) -> &'static str {
        match self {
            RulesetFile::Beliefs => "Beliefs.json",
            RulesetFile::Buildings => "Buildings.json",
            RulesetFile::Eras => "Eras.json",
            RulesetFile::Religions => "Religions.json",
            RulesetFile::Nations => "Nations.json",
            RulesetFile::Policies => "Policies.json",
            RulesetFile::Techs => "Techs.json",
            RulesetFile::Terrains => "Terrains.json",
            RulesetFile::Tutorials => "Tutorials.json",
            RulesetFile::TileImprovements => "TileImprovements.json",
            RulesetFile::TileResources => "TileResources.json",
            RulesetFile::Specialists => "Specialists.json",
            RulesetFile::Units => "Units.json",
            RulesetFile::UnitPromotions => "UnitPromotions.json",
            RulesetFile::UnitTypes => "UnitTypes.json",
            RulesetFile::VictoryTypes => "VictoryTypes.json",
            RulesetFile::CityStateTypes => "CityStateTypes.json",
            RulesetFile::Personalities => "Personalities.json",
            RulesetFile::Events => "Events.json",
            RulesetFile::GlobalUniques => "GlobalUniques.json",
            RulesetFile::ModOptions => "ModOptions.json",
            RulesetFile::Speeds => "Speeds.json",
            RulesetFile::Difficulties => "Difficulties.json",
            RulesetFile::Quests => "Quests.json",
            RulesetFile::Ruins => "Ruins.json",
        }
    }
}

/// Represents a game ruleset containing all game elements like technologies, units, buildings, etc.
pub struct Ruleset {
    /// If (and only if) this Ruleset is a mod, this will be the source folder.
    /// In other words, this is None for built-in and combined rulesets.
    pub folder_location: Option<String>,

    /// A Ruleset instance can represent a built-in ruleset, a mod or a combined ruleset.
    /// name will be the built-in's fullName, the mod's name as displayed (same as folder name),
    /// or in the case of combined rulesets it will be empty.
    pub name: String,

    /// The list of mods that made up this Ruleset, including the base ruleset.
    pub mods: HashSet<String>,

    // Json fields
    pub beliefs: HashMap<String, Belief>,
    pub buildings: HashMap<String, Building>,
    pub difficulties: HashMap<String, Difficulty>,
    pub eras: HashMap<String, Era>,
    pub speeds: HashMap<String, Speed>,
    pub global_uniques: GlobalUniques,
    pub nations: HashMap<String, Nation>,
    pub policies: HashMap<String, Policy>,
    pub policy_branches: HashMap<String, PolicyBranch>,
    pub religions: Vec<String>,
    pub ruin_rewards: HashMap<String, RuinReward>,
    pub quests: HashMap<String, Quest>,
    pub specialists: HashMap<String, Specialist>,
    pub technologies: HashMap<String, Technology>,
    pub tech_columns: Vec<TechColumn>,
    pub terrains: HashMap<String, Terrain>,
    pub tile_improvements: HashMap<String, TileImprovement>,
    pub tile_resources: HashMap<String, TileResource>,
    pub units: HashMap<String, BaseUnit>,
    pub unit_promotions: HashMap<String, Promotion>,
    pub unit_types: HashMap<String, UnitType>,
    pub victories: HashMap<String, Victory>,
    pub city_state_types: HashMap<String, CityStateType>,
    pub personalities: HashMap<String, Personality>,
    pub events: HashMap<String, Event>,
    pub mod_options: ModOptions,
}

impl Ruleset {
    /// Creates a new empty Ruleset
    pub fn new() -> Self {
        Self {
            folder_location: None,
            name: String::new(),
            mods: HashSet::new(),
            beliefs: HashMap::new(),
            buildings: HashMap::new(),
            difficulties: HashMap::new(),
            eras: HashMap::new(),
            speeds: HashMap::new(),
            global_uniques: GlobalUniques::new(),
            nations: HashMap::new(),
            policies: HashMap::new(),
            policy_branches: HashMap::new(),
            religions: Vec::new(),
            ruin_rewards: HashMap::new(),
            quests: HashMap::new(),
            specialists: HashMap::new(),
            technologies: HashMap::new(),
            tech_columns: Vec::new(),
            terrains: HashMap::new(),
            tile_improvements: HashMap::new(),
            tile_resources: HashMap::new(),
            units: HashMap::new(),
            unit_promotions: HashMap::new(),
            unit_types: HashMap::new(),
            victories: HashMap::new(),
            city_state_types: HashMap::new(),
            personalities: HashMap::new(),
            events: HashMap::new(),
            mod_options: ModOptions::new(),
        }
    }

    /// Creates a clone of this Ruleset
    pub fn clone(&self) -> Self {
        let mut new_ruleset = Self::new();
        new_ruleset.add(self);
        new_ruleset
    }

    /// Gets a game resource by name
    pub fn get_game_resource(&self, resource_name: &str) -> Option<&GameResource> {
        Stat::safe_value_of(resource_name)
            .or_else(|| SubStat::safe_value_of(resource_name))
            .or_else(|| self.tile_resources.get(resource_name))
    }

    /// Clears all data from this Ruleset
    pub fn clear(&mut self) {
        self.beliefs.clear();
        self.buildings.clear();
        self.difficulties.clear();
        self.eras.clear();
        self.speeds.clear();
        self.global_uniques = GlobalUniques::new();
        self.mods.clear();
        self.nations.clear();
        self.policies.clear();
        self.policy_branches.clear();
        self.quests.clear();
        self.religions.clear();
        self.ruin_rewards.clear();
        self.specialists.clear();
        self.technologies.clear();
        self.tech_columns.clear();
        self.terrains.clear();
        self.tile_improvements.clear();
        self.tile_resources.clear();
        self.unit_promotions.clear();
        self.units.clear();
        self.unit_types.clear();
        self.victories.clear();
        self.city_state_types.clear();
        self.personalities.clear();
        self.events.clear();
    }

    /// Gets a summary of this Ruleset
    pub fn get_summary(&self) -> String {
        let mut string_list = Vec::new();

        if self.mod_options.is_base_ruleset {
            string_list.push("Base Ruleset".to_string());
        }

        if !self.technologies.is_empty() {
            string_list.push(format!("[{}] Techs", self.technologies.len()));
        }

        if !self.nations.is_empty() {
            string_list.push(format!("[{}] Nations", self.nations.len()));
        }

        if !self.units.is_empty() {
            string_list.push(format!("[{}] Units", self.units.len()));
        }

        if !self.buildings.is_empty() {
            string_list.push(format!("[{}] Buildings", self.buildings.len()));
        }

        if !self.tile_resources.is_empty() {
            string_list.push(format!("[{}] Resources", self.tile_resources.len()));
        }

        if !self.tile_improvements.is_empty() {
            string_list.push(format!("[{}] Improvements", self.tile_improvements.len()));
        }

        if !self.religions.is_empty() {
            string_list.push(format!("[{}] Religions", self.religions.len()));
        }

        if !self.beliefs.is_empty() {
            string_list.push(format!("[{}] Beliefs", self.beliefs.len()));
        }

        string_list.join(" ")
    }

    /// Gets a list of errors in this Ruleset
    pub fn get_error_list(&self, try_fix_unknown_uniques: bool) -> Vec<String> {
        RulesetValidator::new(self).get_error_list(try_fix_unknown_uniques)
    }
}

impl fmt::Display for Ruleset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.name.is_empty() {
            write!(f, "{}", self.name)
        } else if self.mods.len() == 1 {
            let first_mod = self.mods.iter().next().unwrap();
            write!(f, "{}", first_mod)
        } else {
            write!(f, "Combined RuleSet ({})", self.mods.iter().collect::<Vec<_>>().join(", "))
        }
    }
}