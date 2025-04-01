from dataclasses import dataclass
from typing import Optional, List, Tuple, Dict
from com.unciv.logic.automation.unit import SpecificUnitAutomation
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.unique import StateForConditionals, Unique, UniqueType
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant
from com.unciv.logic.battle.i_combatant import ICombatant
from com.unciv.logic.battle.combat_action import CombatAction
from com.unciv.logic.battle.target_helper import TargetHelper

@dataclass
class GeneralBonusData:
    general: MapUnit
    radius: int
    filter: str
    bonus: int

    @classmethod
    def from_unique(cls, general: MapUnit, unique: Unique) -> 'GeneralBonusData':
        return cls(
            general=general,
            radius=int(unique.params[2]) if unique.params[2].isdigit() else 0,
            filter=unique.params[1],
            bonus=int(unique.params[0]) if unique.params[0].isdigit() else 0
        )

class GreatGeneralImplementation:
    @staticmethod
    def get_great_general_bonus(
        our_unit_combatant: MapUnitCombatant,
        enemy: ICombatant,
        combat_action: CombatAction
    ) -> Tuple[str, int]:
        """
        Determine the "Great General" bonus for our_unit_combatant by searching for units carrying the
        UniqueType.StrengthBonusInRadius in the vicinity.

        Used by BattleDamage.getGeneralModifiers.

        Returns:
            A tuple of unit's name and bonus (percentage) as Int (typically 15), or empty string and 0
            if no applicable Great General equivalents found
        """
        unit = our_unit_combatant.unit
        civ_info = our_unit_combatant.unit.civ
        all_generals = [unit for unit in civ_info.units.get_civ_units()
                       if unit.cache.has_strength_bonus_in_radius_unique]
        if not all_generals:
            return "", 0

        great_generals = []
        for general in all_generals:
            for unique in general.get_matching_uniques(
                UniqueType.StrengthBonusInRadius,
                StateForConditionals(
                    unit.civ,
                    our_combatant=our_unit_combatant,
                    their_combatant=enemy,
                    combat_action=combat_action
                )
            ):
                bonus_data = GeneralBonusData.from_unique(general, unique)
                if (general.current_tile.aerial_distance_to(unit.get_tile()) <= bonus_data.radius
                    and (bonus_data.filter == "Military" or unit.matches_filter(bonus_data.filter))):
                    great_generals.append(bonus_data)

        if not great_generals:
            return "", 0

        great_general_modifier = max(great_generals, key=lambda x: x.bonus)

        if (unit.has_unique(UniqueType.GreatGeneralProvidesDoubleCombatBonus, check_civ_info_uniques=True)
            and great_general_modifier.general.is_great_person_of_type("War")):  # apply only on "true" generals
            return great_general_modifier.general.name, great_general_modifier.bonus * 2
        return great_general_modifier.general.name, great_general_modifier.bonus

    @staticmethod
    def get_best_affected_troops_tile(general: MapUnit) -> Optional[Tile]:
        """
        Find a tile for accompanying a military unit where the total bonus for all affected units is maximized.

        Used by SpecificUnitAutomation.automateGreatGeneral.
        """
        # Normally we have only one Unique here. But a mix is not forbidden, so let's try to support mad modders.
        # (imagine several GreatGeneralAura uniques - +50% at radius 1, +25% at radius 2, +5% at radius 3 -
        # possibly learnable from promotions via buildings or natural wonders?)

        # Map out the uniques sorted by bonus, as later only the best bonus will apply.
        general_bonus_data = sorted(
            [GeneralBonusData.from_unique(general, unique)
             for unique in general.get_matching_uniques(UniqueType.StrengthBonusInRadius)],
            key=lambda x: (-x.bonus, x.radius)  # Sort by bonus descending, then by radius
        )

        # Get candidate units to 'follow', coarsely.
        # The mapUnitFilter of the unique won't apply here but in the ranking of the "Aura" effectiveness.
        unit_max_movement = general.get_max_movement()
        military_unit_tiles_in_distance = [
            tile for tile, _ in general.movement.get_distance_to_tiles().items()
            if (tile.military_unit and tile.military_unit.civ == general.civ
                and (not tile.civilian_unit or tile.civilian_unit == general)
                and tile.military_unit.get_max_movement() <= unit_max_movement
                and not tile.is_city_center())
        ]

        # rank tiles and find best
        unit_bonus_radius = max((data.radius for data in general_bonus_data), default=None)
        if not unit_bonus_radius:
            return None

        military_unit_to_has_attackable_enemies: Dict[MapUnit, bool] = {}

        def get_tile_score(unit_tile: Tile) -> int:
            total_bonus = 0
            for affected_tile in unit_tile.get_tiles_in_distance(unit_bonus_radius):
                military_unit = affected_tile.military_unit
                if (not military_unit or military_unit.civ != general.civ
                    or military_unit.is_embarked()):
                    continue

                has_attackable_enemies = military_unit_to_has_attackable_enemies.get(
                    military_unit,
                    not TargetHelper.get_attackable_enemies(
                        military_unit,
                        military_unit.movement.get_distance_to_tiles()
                    )
                )
                military_unit_to_has_attackable_enemies[military_unit] = has_attackable_enemies
                if has_attackable_enemies:
                    continue

                matching_bonus = next(
                    (data.bonus for data in general_bonus_data
                     if (affected_tile.aerial_distance_to(unit_tile) <= data.radius
                         and (data.filter == "Military" or military_unit.matches_filter(data.filter)))),
                    None
                )
                if matching_bonus is not None:
                    total_bonus += matching_bonus

            return total_bonus

        return max(military_unit_tiles_in_distance, key=get_tile_score, default=None)