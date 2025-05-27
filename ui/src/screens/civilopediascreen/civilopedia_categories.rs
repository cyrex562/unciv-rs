use bevy::prelude::*;
use bevy_egui::egui::{self, Image};
use std::collections::HashMap;
use std::collections::BTreeMap;

use crate::models::ruleset::Ruleset;
use crate::models::ruleset::belief::Belief as BaseBelief;
use crate::models::ruleset::unit::UnitType as BaseUnitType;
use crate::ui::components::input::KeyboardBinding;
use crate::ui::screens::basescreen::TutorialController;
use crate::ui::screens::civilopediascreen::civilopedia_image_getters::CivilopediaImageGetters;
use crate::ui::screens::civilopediascreen::ICivilopediaText;

/// Enum used as keys for Civilopedia "pages" (categories).
///
/// Note names are singular on purpose - a "link" allows both key and label
/// Order of values determines ordering of the categories in the Civilopedia top bar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CivilopediaCategories {
    /// Buildings category
    Building,

    /// Wonders category
    Wonder,

    /// Resources category
    Resource,

    /// Terrains category
    Terrain,

    /// Tile Improvements category
    Improvement,

    /// Units category
    Unit,

    /// Unit types category
    UnitType,

    /// Nations category
    Nation,

    /// Technologies category
    Technology,

    /// Promotions category
    Promotion,

    /// Policies category
    Policy,

    /// Religions and Beliefs category
    Belief,

    /// Tutorials category
    Tutorial,

    /// Difficulty levels category
    Difficulty,

    /// Eras category
    Era,

    /// Speeds category
    Speed,
}

impl CivilopediaCategories {
    /// Get the translatable caption for the Civilopedia button
    pub fn label(&self) -> &'static str {
        match self {
            CivilopediaCategories::Building => "Buildings",
            CivilopediaCategories::Wonder => "Wonders",
            CivilopediaCategories::Resource => "Resources",
            CivilopediaCategories::Terrain => "Terrains",
            CivilopediaCategories::Improvement => "Tile Improvements",
            CivilopediaCategories::Unit => "Units",
            CivilopediaCategories::UnitType => "Unit types",
            CivilopediaCategories::Nation => "Nations",
            CivilopediaCategories::Technology => "Technologies",
            CivilopediaCategories::Promotion => "Promotions",
            CivilopediaCategories::Policy => "Policies",
            CivilopediaCategories::Belief => "Religions and Beliefs",
            CivilopediaCategories::Tutorial => "Tutorials",
            CivilopediaCategories::Difficulty => "Difficulty levels",
            CivilopediaCategories::Era => "Eras",
            CivilopediaCategories::Speed => "Speeds",
        }
    }

    /// Get the function to get an image for this category
    pub fn get_image(&self) -> Option<fn(name: &str, size: f32) -> Option<Image>> {
        match self {
            CivilopediaCategories::Building => Some(CivilopediaImageGetters::construction),
            CivilopediaCategories::Wonder => Some(CivilopediaImageGetters::construction),
            CivilopediaCategories::Resource => Some(CivilopediaImageGetters::resource),
            CivilopediaCategories::Terrain => Some(CivilopediaImageGetters::terrain),
            CivilopediaCategories::Improvement => Some(CivilopediaImageGetters::improvement),
            CivilopediaCategories::Unit => Some(CivilopediaImageGetters::construction),
            CivilopediaCategories::UnitType => Some(CivilopediaImageGetters::unit_type),
            CivilopediaCategories::Nation => Some(CivilopediaImageGetters::nation),
            CivilopediaCategories::Technology => Some(CivilopediaImageGetters::technology),
            CivilopediaCategories::Promotion => Some(CivilopediaImageGetters::promotion),
            CivilopediaCategories::Policy => Some(CivilopediaImageGetters::policy),
            CivilopediaCategories::Belief => Some(CivilopediaImageGetters::belief),
            CivilopediaCategories::Tutorial => None,
            CivilopediaCategories::Difficulty => None,
            CivilopediaCategories::Era => None,
            CivilopediaCategories::Speed => None,
        }
    }

    /// Get the keyboard binding for this category
    pub fn binding(&self) -> KeyboardBinding {
        match self {
            CivilopediaCategories::Building => KeyboardBinding::PediaBuildings,
            CivilopediaCategories::Wonder => KeyboardBinding::PediaWonders,
            CivilopediaCategories::Resource => KeyboardBinding::PediaResources,
            CivilopediaCategories::Terrain => KeyboardBinding::PediaTerrains,
            CivilopediaCategories::Improvement => KeyboardBinding::PediaImprovements,
            CivilopediaCategories::Unit => KeyboardBinding::PediaUnits,
            CivilopediaCategories::UnitType => KeyboardBinding::PediaUnitTypes,
            CivilopediaCategories::Nation => KeyboardBinding::PediaNations,
            CivilopediaCategories::Technology => KeyboardBinding::PediaTechnologies,
            CivilopediaCategories::Promotion => KeyboardBinding::PediaPromotions,
            CivilopediaCategories::Policy => KeyboardBinding::PediaPolicies,
            CivilopediaCategories::Belief => KeyboardBinding::PediaBeliefs,
            CivilopediaCategories::Tutorial => KeyboardBinding::PediaTutorials,
            CivilopediaCategories::Difficulty => KeyboardBinding::PediaDifficulties,
            CivilopediaCategories::Era => KeyboardBinding::PediaEras,
            CivilopediaCategories::Speed => KeyboardBinding::PediaSpeeds,
        }
    }

    /// Get the header icon for this category
    pub fn header_icon(&self) -> &'static str {
        match self {
            CivilopediaCategories::Building => "OtherIcons/Cities",
            CivilopediaCategories::Wonder => "OtherIcons/Wonders",
            CivilopediaCategories::Resource => "OtherIcons/Resources",
            CivilopediaCategories::Terrain => "OtherIcons/Terrains",
            CivilopediaCategories::Improvement => "OtherIcons/Improvements",
            CivilopediaCategories::Unit => "OtherIcons/Shield",
            CivilopediaCategories::UnitType => "UnitTypeIcons/UnitTypes",
            CivilopediaCategories::Nation => "OtherIcons/Nations",
            CivilopediaCategories::Technology => "TechIcons/Philosophy",
            CivilopediaCategories::Promotion => "UnitPromotionIcons/Mobility",
            CivilopediaCategories::Policy => "PolicyIcons/Constitution",
            CivilopediaCategories::Belief => "ReligionIcons/Religion",
            CivilopediaCategories::Tutorial => "OtherIcons/ExclamationMark",
            CivilopediaCategories::Difficulty => "OtherIcons/Quickstart",
            CivilopediaCategories::Era => "OtherIcons/Tyrannosaurus",
            CivilopediaCategories::Speed => "OtherIcons/Timer",
        }
    }

    /// Get the iterator for this category
    pub fn get_category_iterator(&self, ruleset: &Ruleset, tutorial_controller: &TutorialController) -> Vec<Box<dyn ICivilopediaText>> {
        match self {
            CivilopediaCategories::Building => {
                ruleset.buildings.values()
                    .filter(|b| !b.is_any_wonder())
                    .map(|b| Box::new(b.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Wonder => {
                ruleset.buildings.values()
                    .filter(|b| b.is_any_wonder())
                    .map(|b| Box::new(b.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Resource => {
                ruleset.tile_resources.values()
                    .map(|r| Box::new(r.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Terrain => {
                ruleset.terrains.values()
                    .map(|t| Box::new(t.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Improvement => {
                ruleset.tile_improvements.values()
                    .map(|i| Box::new(i.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Unit => {
                ruleset.units.values()
                    .map(|u| Box::new(u.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::UnitType => {
                BaseUnitType::get_civilopedia_iterator(ruleset)
            },
            CivilopediaCategories::Nation => {
                ruleset.nations.values()
                    .filter(|n| !n.is_spectator)
                    .map(|n| Box::new(n.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Technology => {
                ruleset.technologies.values()
                    .map(|t| Box::new(t.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Promotion => {
                ruleset.unit_promotions.values()
                    .map(|p| Box::new(p.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Policy => {
                ruleset.policies.values()
                    .map(|p| Box::new(p.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Belief => {
                let mut beliefs: Vec<Box<dyn ICivilopediaText>> = ruleset.beliefs.values()
                    .map(|b| Box::new(b.clone()) as Box<dyn ICivilopediaText>)
                    .collect();

                let religion_entry = BaseBelief::get_civilopedia_religion_entry(ruleset);
                beliefs.push(Box::new(religion_entry) as Box<dyn ICivilopediaText>);

                beliefs
            },
            CivilopediaCategories::Tutorial => {
                tutorial_controller.get_civilopedia_tutorials()
            },
            CivilopediaCategories::Difficulty => {
                ruleset.difficulties.values()
                    .map(|d| Box::new(d.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Era => {
                ruleset.eras.values()
                    .map(|e| Box::new(e.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
            CivilopediaCategories::Speed => {
                ruleset.speeds.values()
                    .map(|s| Box::new(s.clone()) as Box<dyn ICivilopediaText>)
                    .collect()
            },
        }
    }

    /// Get a category from a link name
    pub fn from_link(name: &str) -> Option<Self> {
        // Try to match by enum name
        for category in Self::values() {
            if category.name() == name {
                return Some(category);
            }
        }

        // Try to match by label
        for category in Self::values() {
            if category.label() == name {
                return Some(category);
            }
        }

        None
    }

    /// Get all values of the enum
    pub fn values() -> &'static [Self] {
        &[
            CivilopediaCategories::Building,
            CivilopediaCategories::Wonder,
            CivilopediaCategories::Resource,
            CivilopediaCategories::Terrain,
            CivilopediaCategories::Improvement,
            CivilopediaCategories::Unit,
            CivilopediaCategories::UnitType,
            CivilopediaCategories::Nation,
            CivilopediaCategories::Technology,
            CivilopediaCategories::Promotion,
            CivilopediaCategories::Policy,
            CivilopediaCategories::Belief,
            CivilopediaCategories::Tutorial,
            CivilopediaCategories::Difficulty,
            CivilopediaCategories::Era,
            CivilopediaCategories::Speed,
        ]
    }

    /// Get the name of the enum variant
    pub fn name(&self) -> &'static str {
        match self {
            CivilopediaCategories::Building => "Building",
            CivilopediaCategories::Wonder => "Wonder",
            CivilopediaCategories::Resource => "Resource",
            CivilopediaCategories::Terrain => "Terrain",
            CivilopediaCategories::Improvement => "Improvement",
            CivilopediaCategories::Unit => "Unit",
            CivilopediaCategories::UnitType => "UnitType",
            CivilopediaCategories::Nation => "Nation",
            CivilopediaCategories::Technology => "Technology",
            CivilopediaCategories::Promotion => "Promotion",
            CivilopediaCategories::Policy => "Policy",
            CivilopediaCategories::Belief => "Belief",
            CivilopediaCategories::Tutorial => "Tutorial",
            CivilopediaCategories::Difficulty => "Difficulty",
            CivilopediaCategories::Era => "Era",
            CivilopediaCategories::Speed => "Speed",
        }
    }
}