use crate::civilization::Civilization;
use crate::game_info::GameInfo;
use crate::map::map_unit::MapUnit;
use crate::math::Vector2;

/// Helper struct for making decisions about more abstract information that may be displayed on the world map
/// (or fair to use in AI), but which does not have any direct influence on save state, rules, or behaviour.
pub struct MapVisualization<'a> {
    game_info: &'a GameInfo,
    viewing_civ: &'a Civilization,
}

impl<'a> MapVisualization<'a> {
    /// Creates a new MapVisualization instance
    pub fn new(game_info: &'a GameInfo, viewing_civ: &'a Civilization) -> Self {
        Self {
            game_info,
            viewing_civ,
        }
    }

    /// Returns whether a unit's past movements should be visible to the player.
    /// Past should always be visible for own units. Past should be visible for foreign units
    /// if the unit is visible and both its current tile and previous tiles are visible.
    pub fn is_unit_past_visible(&self, unit: &MapUnit) -> bool {
        if unit.civ == self.viewing_civ {
            return true;
        }

        // Check if all positions (current and past) are in viewable tiles
        let check_positions = unit.movement_memories.iter()
            .map(|memory| memory.position)
            .chain(std::iter::once(unit.get_tile().position));

        let all_positions_visible = check_positions.all(|pos| {
            self.game_info.tile_map.get(&pos)
                .map(|tile| self.viewing_civ.viewable_tiles.contains(tile))
                .unwrap_or(false)
        });

        // Check if unit is invisible and if the tile is in viewable invisible units tiles
        let invisible_check = !unit.is_invisible(self.viewing_civ)
            || self.viewing_civ.viewable_invisible_units_tiles.contains(&unit.get_tile());

        all_positions_visible && invisible_check
    }

    /// Returns whether a unit's planned movements should be visible to the player.
    /// Plans should be visible always for own units and never for foreign units.
    pub fn is_unit_future_visible(&self, unit: &MapUnit) -> bool {
        self.viewing_civ.is_spectator() || unit.civ == self.viewing_civ
    }

    /// Returns whether an attack by a unit to a target should be visible to the player.
    /// Attacks by the player civ should always be visible, and attacks by foreign civs
    /// should be visible if either the tile they targeted or the attacker's tile are visible.
    /// E.G. Civ V shows bombers coming out of the Fog of War.
    pub fn is_attack_visible(&self, attacker: &Civilization, source: Vector2, target: Vector2) -> bool {
        attacker == self.viewing_civ
            || self.game_info.tile_map.get(&source)
                .map(|tile| self.viewing_civ.viewable_tiles.contains(tile))
                .unwrap_or(false)
            || self.game_info.tile_map.get(&target)
                .map(|tile| self.viewing_civ.viewable_tiles.contains(tile))
                .unwrap_or(false)
    }
}