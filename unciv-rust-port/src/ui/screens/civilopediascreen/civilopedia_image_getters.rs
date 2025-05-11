use bevy::prelude::*;
use bevy_egui::egui::{self, Color32, Image};
use std::collections::HashMap;

use crate::models::ruleset::Ruleset;
use crate::models::ruleset::tile::{Terrain, TerrainType};
use crate::models::ruleset::unit::UnitMovementType;
use crate::ui::components::tilegroups::{TileGroup, TileSetStrings};
use crate::ui::images::{IconCircleGroup, ImageGetter};
use crate::UncivGame;

/// Encapsulates the knowledge on how to get an icon for each of the Civilopedia categories
pub struct CivilopediaImageGetters;

impl CivilopediaImageGetters {
    /// Policy icon folder path
    const POLICY_ICON_FOLDER: &'static str = "PolicyIcons";

    /// Policy branch icon folder path
    const POLICY_BRANCH_ICON_FOLDER: &'static str = "PolicyBranchIcons";

    /// Policy inner size ratio
    const POLICY_INNER_SIZE: f32 = 0.25;

    /// Get a terrain image
    ///
    /// # Arguments
    ///
    /// * `terrain` - The terrain to get an image for
    /// * `ruleset` - The ruleset containing terrain definitions
    /// * `image_size` - The size of the image
    /// * `tile_set_strings` - Optional tile set strings
    pub fn terrain_image(
        terrain: &Terrain,
        ruleset: &Ruleset,
        image_size: f32,
        tile_set_strings: Option<&TileSetStrings>
    ) -> IconCircleGroup {
        // Create a new tile
        let mut tile = crate::logic::map::tile::Tile::new();
        tile.set_ruleset(ruleset);

        // Get base terrain from occurs on
        let base_terrain_from_occurs_on = terrain.occurs_on.iter()
            .filter_map(|name| ruleset.terrains.get(name))
            .filter(|t| t.type_.is_base_terrain)
            .last()
            .map(|t| t.name.clone())
            .or_else(|| {
                ruleset.terrains.values()
                    .find(|t| t.type_ == TerrainType::Land)
                    .map(|t| t.name.clone())
            })
            .or_else(|| ruleset.terrains.keys().next().cloned())
            .unwrap_or_default();

        // Set terrain properties based on type
        match terrain.type_ {
            TerrainType::NaturalWonder => {
                tile.set_natural_wonder(Some(terrain.name.clone()));
                tile.set_base_terrain(
                    if let Some(turns_into) = &terrain.turns_into {
                        if ruleset.terrains.contains_key(turns_into) {
                            turns_into.clone()
                        } else {
                            base_terrain_from_occurs_on
                        }
                    } else {
                        base_terrain_from_occurs_on
                    }
                );
            },
            TerrainType::TerrainFeature => {
                tile.set_base_terrain(base_terrain_from_occurs_on);
                tile.set_terrain_transients();
                tile.add_terrain_feature(terrain.name.clone());
            },
            _ => {
                tile.set_base_terrain(terrain.name.clone());
            }
        }

        tile.set_terrain_transients();

        // Create tile group
        let tile_set_strings = tile_set_strings.unwrap_or_else(|| {
            TileSetStrings::new(ruleset, &UncivGame::current().settings)
        });

        let mut group = TileGroup::new(
            &tile,
            tile_set_strings,
            image_size * 36.0 / 54.0  // TileGroup normally spills out of its bounding box
        );

        group.set_force_visible(true);
        group.set_for_map_editor_icon(true);
        group.update();

        group
    }

    /// Get a construction image
    pub fn construction(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_construction_portrait(name, size)
    }

    /// Get an improvement image
    pub fn improvement(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_improvement_portrait(name, size)
    }

    /// Get a nation image
    pub fn nation(name: &str, size: f32) -> Option<Image> {
        let nation = ImageGetter::ruleset().nations.get(name)?;
        Some(ImageGetter::get_nation_portrait(nation, size))
    }

    /// Get a policy image
    pub fn policy(name: &str, size: f32) -> Option<IconCircleGroup> {
        // Result is nullable: policy branch complete have no icons but are linked -> nonexistence must be passed down
        fn try_image(path: &str, color: Color32) -> Option<IconCircleGroup> {
            if ImageGetter::image_exists(path) {
                let mut image = ImageGetter::get_image(path)?;
                image.set_size(size * Self::POLICY_INNER_SIZE, size * Self::POLICY_INNER_SIZE);
                image.set_color(color);
                Some(image.surround_with_circle(size))
            } else {
                None
            }
        }

        try_image(&format!("{}/{}", Self::POLICY_BRANCH_ICON_FOLDER, name), ImageGetter::CHARCOAL)
            .or_else(|| try_image(&format!("{}/{}", Self::POLICY_ICON_FOLDER, name), Color32::BROWN))
    }

    /// Get a resource image
    pub fn resource(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_resource_portrait(name, size)
    }

    /// Get a technology image
    pub fn technology(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_tech_icon_portrait(name, size)
    }

    /// Get a promotion image
    pub fn promotion(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_promotion_portrait(name, size)
    }

    /// Get a terrain image
    pub fn terrain(name: &str, size: f32) -> Option<IconCircleGroup> {
        let terrain = ImageGetter::ruleset().terrains.get(name)?;
        Some(Self::terrain_image(terrain, ImageGetter::ruleset(), size, None))
    }

    /// Get a belief image
    pub fn belief(name: &str, size: f32) -> Option<Image> {
        ImageGetter::get_religion_portrait(name, size)
    }

    /// Get a unit type image
    pub fn unit_type(name: &str, size: f32) -> Option<Image> {
        let path = UnitMovementType::iter()
            .find(|t| format!("Domain: [{}]", t.name()) == name)
            .map(|t| format!("UnitTypeIcons/Domain{}", t.name()))
            .unwrap_or_else(|| format!("UnitTypeIcons/{}", name));

        if ImageGetter::image_exists(&path) {
            let mut image = ImageGetter::get_image(&path)?;
            image.set_size(size);
            Some(image)
        } else {
            None
        }
    }
}