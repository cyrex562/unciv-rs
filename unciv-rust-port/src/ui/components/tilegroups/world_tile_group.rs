use ggez::graphics::Color;
use std::sync::Arc;

use crate::models::civilization::Civilization;
use crate::models::map::tile::Tile;
use crate::models::ruleset::unique::LocalUniqueCache;
use crate::ui::components::extensions::{center, darken};
use crate::ui::images::ImageGetter;
use crate::UncivGame;

use super::tile_group::TileGroup;
use super::tile_set_strings::TileSetStrings;

/// A TileGroup for displaying tiles in the world view
pub struct WorldTileGroup {
    /// The base TileGroup that this WorldTileGroup extends
    base: TileGroup,
}

impl WorldTileGroup {
    /// Creates a new WorldTileGroup with the given tile and tile set strings
    pub fn new(tile: Arc<Tile>, tile_set_strings: Arc<TileSetStrings>) -> Self {
        let mut world_tile_group = Self {
            base: TileGroup::new(tile.clone(), tile_set_strings.clone()),
        };

        // Set the misc layer to be non-touchable
        world_tile_group.base.layer_misc().set_touchable(false);

        world_tile_group
    }

    /// Updates the WorldTileGroup with the given viewing civilization and local unique cache
    pub fn update(&mut self, viewing_civ: Option<&Civilization>, local_unique_cache: &LocalUniqueCache) {
        // Call the base update method
        self.base.update(viewing_civ, local_unique_cache);

        // Update the worked icon if we have a viewing civilization
        if let Some(civ) = viewing_civ {
            self.update_worked_icon(civ);
        }
    }

    /// Updates the worked icon for the given viewing civilization
    fn update_worked_icon(&mut self, viewing_civ: &Civilization) {
        // Remove any existing worked icon
        self.base.layer_misc().remove_worked_icon();

        // Check if we should show the worked icon
        let should_show_worked_icon = UncivGame::current().settings().show_worked_tiles() && // Overlay enabled
            self.base.is_viewable(viewing_civ) && // We see tile
            self.base.tile().get_city().map_or(false, |city| city.civ() == viewing_civ) && // Tile belongs to us
            self.base.tile().is_worked(); // Tile is worked

        if !should_show_worked_icon {
            return;
        }

        // Determine which icon to show
        let icon = if self.base.tile().is_locked() {
            // Create a locked icon
            let mut icon = ImageGetter::get_image("TileIcons/Locked");
            icon.set_color(darken(Color::WHITE, 0.5));
            Some(icon)
        } else if self.base.tile().is_worked() && self.base.tile().provides_yield() {
            // Create a worked icon
            let mut icon = ImageGetter::get_image("TileIcons/Worked");
            icon.set_color(darken(Color::WHITE, 0.5));
            Some(icon)
        } else {
            None
        };

        // Add the icon if we have one
        if let Some(mut icon) = icon {
            icon.set_size(20.0, 20.0);
            center(&mut icon, self.base.as_ref());
            icon.set_x(icon.x() + 20.0);
            self.base.layer_misc().add_worked_icon(icon);
        }
    }

    /// Gets the base TileGroup
    pub fn base(&self) -> &TileGroup {
        &self.base
    }

    /// Gets a mutable reference to the base TileGroup
    pub fn base_mut(&mut self) -> &mut TileGroup {
        &mut self.base
    }

    /// Creates a clone of this WorldTileGroup
    pub fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

// Implement the necessary traits for WorldTileGroup
impl std::ops::Deref for WorldTileGroup {
    type Target = TileGroup;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for WorldTileGroup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}