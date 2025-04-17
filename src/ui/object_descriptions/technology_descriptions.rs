use crate::game::civilization::Civilization;
use crate::game::game_state::GameState;
use crate::game::improvement::Improvement;
use crate::game::technology::Technology;
use crate::game::unit::Unit;
use crate::game::building::Building;
use crate::ui::object_descriptions::description_helpers::DescriptionHelpers;
use crate::ui::object_descriptions::base_unit_descriptions::BaseUnitDescriptions;
use crate::ui::object_descriptions::building_descriptions::BuildingDescriptions;
use crate::ui::object_descriptions::improvement_descriptions::ImprovementDescriptions;

pub struct TechnologyDescriptions;

impl TechnologyDescriptions {
    pub fn get_technology_description(tech: &Technology, civ: &Civilization) -> String {
        let mut description = String::new();

        // Add cost and prerequisites
        description.push_str(&format!("Cost: {} production\n", tech.cost));
        if !tech.prerequisites.is_empty() {
            description.push_str(&format!("Prerequisites: {}\n",
                tech.prerequisites.iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")));
        }

        // Add enabled units
        let enabled_units = Self::get_enabled_units(tech, civ);
        if !enabled_units.is_empty() {
            description.push_str("\nEnables units:\n");
            for unit in enabled_units {
                description.push_str(&format!("- {}\n", unit.name));
            }
        }

        // Add enabled buildings
        let enabled_buildings = Self::get_enabled_buildings(tech, civ);
        if !enabled_buildings.is_empty() {
            description.push_str("\nEnables buildings:\n");
            for building in enabled_buildings {
                description.push_str(&format!("- {}\n", building.name));
            }
        }

        // Add enabled improvements
        let enabled_improvements = Self::get_enabled_improvements(tech, civ);
        if !enabled_improvements.is_empty() {
            description.push_str("\nEnables improvements:\n");
            for improvement in enabled_improvements {
                description.push_str(&format!("- {}\n", improvement.name));
            }
        }

        // Add enabled abilities
        let enabled_abilities = Self::get_enabled_abilities(tech, civ);
        if !enabled_abilities.is_empty() {
            description.push_str("\nEnables abilities:\n");
            for ability in enabled_abilities {
                description.push_str(&format!("- {}\n", ability));
            }
        }

        description
    }

    pub fn get_technology_civilopedia(tech: &Technology, civ: &Civilization) -> String {
        let mut civilopedia = String::new();

        // Add description
        civilopedia.push_str(&format!("{}\n\n", tech.description));

        // Add cost and prerequisites
        civilopedia.push_str(&format!("Cost: {} production\n", tech.cost));
        if !tech.prerequisites.is_empty() {
            civilopedia.push_str(&format!("Prerequisites: {}\n",
                tech.prerequisites.iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")));
        }

        // Add enabled units with descriptions
        let enabled_units = Self::get_enabled_units(tech, civ);
        if !enabled_units.is_empty() {
            civilopedia.push_str("\nEnables units:\n");
            for unit in enabled_units {
                civilopedia.push_str(&format!("- {}: {}\n",
                    unit.name,
                    BaseUnitDescriptions::get_unit_description(&unit, civ)));
            }
        }

        // Add enabled buildings with descriptions
        let enabled_buildings = Self::get_enabled_buildings(tech, civ);
        if !enabled_buildings.is_empty() {
            civilopedia.push_str("\nEnables buildings:\n");
            for building in enabled_buildings {
                civilopedia.push_str(&format!("- {}: {}\n",
                    building.name,
                    BuildingDescriptions::get_building_description(&building, civ)));
            }
        }

        // Add enabled improvements with descriptions
        let enabled_improvements = Self::get_enabled_improvements(tech, civ);
        if !enabled_improvements.is_empty() {
            civilopedia.push_str("\nEnables improvements:\n");
            for improvement in enabled_improvements {
                civilopedia.push_str(&format!("- {}: {}\n",
                    improvement.name,
                    ImprovementDescriptions::get_improvement_description(&improvement, civ)));
            }
        }

        // Add enabled abilities with descriptions
        let enabled_abilities = Self::get_enabled_abilities(tech, civ);
        if !enabled_abilities.is_empty() {
            civilopedia.push_str("\nEnables abilities:\n");
            for ability in enabled_abilities {
                civilopedia.push_str(&format!("- {}: {}\n",
                    ability,
                    DescriptionHelpers::get_ability_description(ability)));
            }
        }

        civilopedia
    }

    fn get_enabled_units(tech: &Technology, civ: &Civilization) -> Vec<Unit> {
        // Implementation to get units enabled by this technology
        // This would need to be implemented based on how units are stored and accessed
        Vec::new()
    }

    fn get_enabled_buildings(tech: &Technology, civ: &Civilization) -> Vec<Building> {
        // Implementation to get buildings enabled by this technology
        // This would need to be implemented based on how buildings are stored and accessed
        Vec::new()
    }

    fn get_enabled_improvements(tech: &Technology, civ: &Civilization) -> Vec<Improvement> {
        // Implementation to get improvements enabled by this technology
        // This would need to be implemented based on how improvements are stored and accessed
        Vec::new()
    }

    fn get_enabled_abilities(tech: &Technology, civ: &Civilization) -> Vec<String> {
        // Implementation to get abilities enabled by this technology
        // This would need to be implemented based on how abilities are stored and accessed
        Vec::new()
    }
}