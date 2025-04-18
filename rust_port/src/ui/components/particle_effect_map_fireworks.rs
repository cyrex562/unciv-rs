use super::particle_effect_fireworks::ParticleEffectFireworks;
use super::particle_effect_animation::ParticleEffectAnimation;
use ggez::graphics::{self, Rect};
use ggez::{Context, GameResult};
use crate::ui::components::tilegroups::CityTileGroup;
use crate::ui::components::widgets::ZoomableScrollPane;
use crate::game::UncivGame;

/// Display fireworks using the particle effect system, over a map view, centered on a specific tile.
///
/// - Repeats endlessly
/// - Handles the zooming and panning of the map
/// - Intentionally exceeds the bounds of the passed (TileGroup) actor bounds, but not by much
pub struct ParticleEffectMapFireworks {
    base: ParticleEffectFireworks,
    map_holder: ZoomableScrollPane,
    actor_bounds: Rect,
    temp_viewport: Rect,
}

impl ParticleEffectMapFireworks {
    /// Creates a new map fireworks effect if enabled in the game settings
    pub fn create(game: &UncivGame, map_scroll_pane: &CityMapHolder) -> Option<Self> {
        if !Self::is_enabled(game, "ConstructionIcons") {
            return None;
        }

        let mut effect = Self {
            base: ParticleEffectFireworks::new(),
            map_holder: map_scroll_pane.clone(),
            actor_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            temp_viewport: Rect::new(0.0, 0.0, 0.0, 0.0),
        };

        if effect.load().is_ok() {
            Some(effect)
        } else {
            None
        }
    }

    /// Sets the actor bounds based on the tile group
    ///
    /// The factors below are just fine-tuning the looks, and avoid lengthy particle effect file edits
    pub fn set_actor_bounds(&mut self, tile_group: &CityTileGroup) {
        let hexagon_image_width = tile_group.hexagon_image_width();
        self.actor_bounds = Rect::new(
            tile_group.x() + (tile_group.width() - hexagon_image_width) / 2.0,
            tile_group.y() + tile_group.height() / 4.0,
            hexagon_image_width,
            tile_group.height() * 1.667
        );
    }

    /// Checks if the effect is enabled in the game settings
    fn is_enabled(game: &UncivGame, atlas_name: &str) -> bool {
        // This would check game settings to determine if the effect should be enabled
        // For now, we'll assume it's always enabled
        true
    }
}

impl ParticleEffectAnimation for ParticleEffectMapFireworks {
    fn get_scale(&self) -> f32 {
        self.map_holder.scale_x() * 0.667
    }

    fn get_target_bounds(&self) -> Rect {
        // Empiric math - any attempts to ask Gdx via localToStageCoordinates were way off
        let scale = self.map_holder.scale_x(); // just assume scaleX==scaleY
        let viewport = self.map_holder.get_viewport();

        Rect::new(
            (self.actor_bounds.x - viewport.x) * scale,
            (self.actor_bounds.y - viewport.y) * scale,
            self.actor_bounds.w * scale,
            self.actor_bounds.h * scale
        )
    }

    fn configure(&self, effect_data: &mut super::particle_effect_animation::ParticleEffectData) {
        self.base.configure(effect_data);
    }

    fn on_complete(&self, effect_data: &super::particle_effect_animation::ParticleEffectData) -> i32 {
        self.base.on_complete(effect_data)
    }

    fn active_count(&self) -> usize {
        self.base.active_count()
    }

    fn load(&mut self, effects_file: &str, atlas_name: &str, count: i32) -> GameResult {
        self.base.load(effects_file, atlas_name, count)
    }

    fn render(&mut self, ctx: &mut Context, delta: f32) -> GameResult {
        self.base.render(ctx, delta)
    }
}