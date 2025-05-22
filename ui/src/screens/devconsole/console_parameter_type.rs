use std::collections::HashSet;

use crate::logic::GameInfo;
use crate::logic::map::mapgenerator::RiverGenerator;
use crate::models::ruleset::tile::TerrainType;
use crate::models::ruleset::unique::{UniqueTarget, UniqueType};
use crate::models::stats::Stat;
use crate::ui::screens::devconsole::cli_input::{CliInput, Method};
use crate::ui::screens::devconsole::dev_console_popup::DevConsolePopup;

/// Enum encapsulates knowledge about console command parameter types
/// - Extensible
/// - Currently limited to supplying autocomplete possibilities: use [get_options]
/// - Supports multi-type parameters via [multi_options]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsoleParameterType {
    /// No parameter type
    None,
    /// Civilization name
    CivName,
    /// Unit name
    UnitName,
    /// Promotion name
    PromotionName,
    /// Improvement name
    ImprovementName,
    /// Feature name
    FeatureName,
    /// Terrain name
    TerrainName,
    /// Resource name
    ResourceName,
    /// Stat
    Stat,
    /// Religion name
    ReligionName,
    /// Building name
    BuildingName,
    /// Direction
    Direction,
    /// Policy name
    PolicyName,
    /// Technology name
    TechName,
    /// City name
    CityName,
    /// Triggered unique template
    TriggeredUniqueTemplate,
    /// Difficulty
    Difficulty,
    /// Boolean
    Boolean,
}

impl ConsoleParameterType {
    /// Get the options for a parameter type
    fn get_options(&self, game_info: &GameInfo) -> Vec<String> {
        match self {
            ConsoleParameterType::None => Vec::new(),
            ConsoleParameterType::CivName => game_info.civilizations.iter()
                .map(|civ| civ.civ_name.clone())
                .collect(),
            ConsoleParameterType::UnitName => game_info.ruleset.units.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::PromotionName => game_info.ruleset.unit_promotions.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::ImprovementName => game_info.ruleset.tile_improvements.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::FeatureName => game_info.ruleset.terrains.values()
                .filter(|t| t.terrain_type == TerrainType::TerrainFeature)
                .map(|t| t.name.clone())
                .collect(),
            ConsoleParameterType::TerrainName => game_info.ruleset.terrains.values()
                .filter(|t| t.terrain_type.is_base_terrain() || t.terrain_type == TerrainType::NaturalWonder)
                .map(|t| t.name.clone())
                .collect(),
            ConsoleParameterType::ResourceName => game_info.ruleset.tile_resources.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::Stat => Stat::names(),
            ConsoleParameterType::ReligionName => game_info.religions.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::BuildingName => game_info.ruleset.buildings.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::Direction => RiverGenerator::RIVER_DIRECTIONS.names()
                .cloned()
                .collect(),
            ConsoleParameterType::PolicyName => {
                let mut policy_names = HashSet::new();
                policy_names.extend(game_info.ruleset.policy_branches.keys().cloned());
                policy_names.extend(game_info.ruleset.policies.keys().cloned());
                policy_names.into_iter().collect()
            },
            ConsoleParameterType::TechName => game_info.ruleset.technologies.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::CityName => game_info.civilizations.iter()
                .flat_map(|civ| civ.cities.iter().map(|city| city.name.clone()))
                .collect(),
            ConsoleParameterType::TriggeredUniqueTemplate => UniqueType::iter()
                .filter(|ut| ut.can_accept_unique_target(UniqueTarget::Triggerable))
                .map(|ut| ut.text().to_string())
                .collect(),
            ConsoleParameterType::Difficulty => game_info.ruleset.difficulties.keys()
                .cloned()
                .collect(),
            ConsoleParameterType::Boolean => vec!["true".to_string(), "false".to_string()],
        }
    }

    /// Whether this parameter type prefers quoted input
    fn prefer_quoted(&self) -> bool {
        matches!(self, ConsoleParameterType::TriggeredUniqueTemplate)
    }

    /// Get the options for a parameter type as CliInput objects
    fn get_options_as_cli_input(&self, console: &DevConsolePopup) -> Vec<CliInput> {
        let options = self.get_options(&console.game_info);
        let method = if self.prefer_quoted() {
            Method::Quoted
        } else {
            Method::Dashed
        };

        options.into_iter()
            .map(|opt| CliInput::new_with_method(opt, method))
            .collect()
    }

    /// Get a ConsoleParameterType from a string name
    pub fn from_str(name: &str) -> ConsoleParameterType {
        match name {
            "civName" => ConsoleParameterType::CivName,
            "unitName" => ConsoleParameterType::UnitName,
            "promotionName" => ConsoleParameterType::PromotionName,
            "improvementName" => ConsoleParameterType::ImprovementName,
            "featureName" => ConsoleParameterType::FeatureName,
            "terrainName" => ConsoleParameterType::TerrainName,
            "resourceName" => ConsoleParameterType::ResourceName,
            "stat" => ConsoleParameterType::Stat,
            "religionName" => ConsoleParameterType::ReligionName,
            "buildingName" => ConsoleParameterType::BuildingName,
            "direction" => ConsoleParameterType::Direction,
            "policyName" => ConsoleParameterType::PolicyName,
            "techName" => ConsoleParameterType::TechName,
            "cityName" => ConsoleParameterType::CityName,
            "triggeredUniqueTemplate" => ConsoleParameterType::TriggeredUniqueTemplate,
            "difficulty" => ConsoleParameterType::Difficulty,
            "boolean" => ConsoleParameterType::Boolean,
            _ => ConsoleParameterType::None,
        }
    }

    /// Get options for a parameter type by name
    pub fn get_options(name: &str, console: &DevConsolePopup) -> Vec<CliInput> {
        ConsoleParameterType::from_str(name).get_options_as_cli_input(console)
    }

    /// Get options for multiple parameter types (separated by |)
    pub fn multi_options(name: &str, console: &DevConsolePopup) -> Vec<CliInput> {
        name.split('|')
            .flat_map(|part| ConsoleParameterType::get_options(part, console))
            .collect()
    }
}