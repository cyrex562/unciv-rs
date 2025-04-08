from typing import Sequence
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UncivSound
from com.unciv.models.ruleset.unique import StateForConditionals, Unique, UniqueType
from com.unciv.models.ruleset.unit import UnitType
from com.unciv.logic.battle.i_combatant import ICombatant

class MapUnitCombatant(ICombatant):
    def __init__(self, unit: MapUnit):
        self.unit = unit

    def get_health(self) -> int:
        return self.unit.health

    def get_max_health(self) -> int:
        return 100

    def get_civ_info(self) -> Civilization:
        return self.unit.civ

    def get_tile(self) -> Tile:
        return self.unit.get_tile()

    def get_name(self) -> str:
        return self.unit.name

    def is_defeated(self) -> bool:
        return self.unit.health <= 0

    def is_invisible(self, to: Civilization) -> bool:
        return self.unit.is_invisible(to)

    def can_attack(self) -> bool:
        return self.unit.can_attack()

    def matches_filter(self, filter: str, multi_filter: bool = True) -> bool:
        return self.unit.matches_filter(filter, multi_filter)

    def get_attack_sound(self) -> UncivSound:
        sound = self.unit.base_unit.attack_sound
        return UncivSound.Click if sound is None else UncivSound(sound)

    def take_damage(self, damage: int) -> None:
        self.unit.take_damage(damage)

    def get_attacking_strength(self) -> int:
        return self.unit.base_unit.ranged_strength if self.is_ranged() else self.unit.base_unit.strength

    def get_defending_strength(self, attacked_by_ranged: bool = False) -> int:
        if self.unit.is_embarked() and not self.is_civilian():
            return self.unit.civ.get_era().embark_defense
        elif self.is_ranged() and attacked_by_ranged:
            return self.unit.base_unit.ranged_strength
        return self.unit.base_unit.strength

    def get_unit_type(self) -> UnitType:
        return self.unit.type

    def __str__(self) -> str:
        return f"{self.unit.name} of {self.unit.civ.civ_name}"

    def get_matching_uniques(self, unique_type: UniqueType, conditional_state: StateForConditionals, check_civ_uniques: bool) -> Sequence[Unique]:
        return self.unit.get_matching_uniques(unique_type, conditional_state, check_civ_uniques)

    def has_unique(self, unique_type: UniqueType, conditional_state: StateForConditionals | None = None) -> bool:
        if conditional_state is None:
            return self.unit.has_unique(unique_type)
        return self.unit.has_unique(unique_type, conditional_state)