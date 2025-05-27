
/// Represents a tile that can be attacked, along with information about the attack position and movement costs.
pub struct AttackableTile {
    /// The tile from which the attack will be launched
    pub tile_to_attack_from: Tile,

    /// The tile that will be attacked
    pub tile_to_attack: Tile,

    /// The amount of movement points that will remain after moving to the attack tile
    pub movement_left_after_moving_to_attack_tile: f32,

    /// The combatant that will be attacked (if any)
    pub combatant: Option<Box<dyn ICombatant>>,
}

impl AttackableTile {
    /// Creates a new AttackableTile with the specified parameters
    pub fn new(
        tile_to_attack_from: Tile,
        tile_to_attack: Tile,
        movement_left_after_moving_to_attack_tile: f32,
        combatant: Option<Box<dyn ICombatant>>
    ) -> Self {
        AttackableTile {
            tile_to_attack_from,
            tile_to_attack,
            movement_left_after_moving_to_attack_tile,
            combatant,
        }
    }
}