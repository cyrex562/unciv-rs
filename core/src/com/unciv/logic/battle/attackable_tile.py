from dataclasses import dataclass
from typing import Optional
from com.unciv.logic.map.tile import Tile
from com.unciv.logic.battle import ICombatant

@dataclass
class AttackableTile:
    tile_to_attack_from: Tile
    tile_to_attack: Tile
    movement_left_after_moving_to_attack_tile: float
    combatant: Optional[ICombatant]