from abc import ABC, abstractmethod
from typing import Optional, cast
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UncivSound
from com.unciv.models.ruleset.unit import UnitType
from com.unciv.logic.battle.city_combatant import CityCombatant
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant

class ICombatant(ABC):
    @abstractmethod
    def get_name(self) -> str:
        pass

    @abstractmethod
    def get_health(self) -> int:
        pass

    @abstractmethod
    def get_max_health(self) -> int:
        pass

    @abstractmethod
    def get_unit_type(self) -> UnitType:
        pass

    @abstractmethod
    def get_attacking_strength(self) -> int:
        pass

    @abstractmethod
    def get_defending_strength(self, attacked_by_ranged: bool = False) -> int:
        pass

    @abstractmethod
    def take_damage(self, damage: int) -> None:
        pass

    @abstractmethod
    def is_defeated(self) -> bool:
        pass

    @abstractmethod
    def get_civ_info(self) -> Civilization:
        pass

    @abstractmethod
    def get_tile(self) -> Tile:
        pass

    @abstractmethod
    def is_invisible(self, to: Civilization) -> bool:
        pass

    @abstractmethod
    def can_attack(self) -> bool:
        pass

    @abstractmethod
    def matches_filter(self, filter: str, multi_filter: bool = True) -> bool:
        """Implements UniqueParameterType.CombatantFilter"""
        pass

    @abstractmethod
    def get_attack_sound(self) -> UncivSound:
        pass

    def is_melee(self) -> bool:
        return not self.is_ranged()

    def is_ranged(self) -> bool:
        if isinstance(self, CityCombatant):
            return True
        unit_combatant = cast(MapUnitCombatant, self)
        return unit_combatant.unit.base_unit.is_ranged()

    def is_air_unit(self) -> bool:
        if isinstance(self, CityCombatant):
            return False
        unit_combatant = cast(MapUnitCombatant, self)
        return unit_combatant.unit.base_unit.is_air_unit()

    def is_water_unit(self) -> bool:
        if isinstance(self, CityCombatant):
            return False
        unit_combatant = cast(MapUnitCombatant, self)
        return unit_combatant.unit.base_unit.is_water_unit

    def is_land_unit(self) -> bool:
        if isinstance(self, CityCombatant):
            return False
        unit_combatant = cast(MapUnitCombatant, self)
        return unit_combatant.unit.base_unit.is_land_unit

    def is_city(self) -> bool:
        return isinstance(self, CityCombatant)

    def is_civilian(self) -> bool:
        if not isinstance(self, MapUnitCombatant):
            return False
        return self.unit.is_civilian()