from dataclasses import dataclass
from enum import Enum
from typing import Optional, Dict, List, Any
import random
from math import max, pow

from com.unciv.logic.map.tile import Tile
from com.unciv.models.counter import Counter
from com.unciv.models.ruleset.global_uniques import GlobalUniques
from com.unciv.models.ruleset.unique import (
    StateForConditionals, Unique, UniqueTarget, UniqueType
)
from com.unciv.models.translations import tr
from com.unciv.ui.components.extensions import to_percent
from com.unciv.logic.battle.battle_constants import BattleConstants
from com.unciv.logic.battle.combat_action import CombatAction
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant
from com.unciv.logic.battle.city_combatant import CityCombatant
from com.unciv.logic.battle.i_combatant import ICombatant
from com.unciv.logic.battle.great_general_implementation import GreatGeneralImplementation

class BattleDamage:
    @staticmethod
    def get_modifier_string_from_unique(unique: Unique) -> str:
        source = {
            UniqueTarget.Unit: "Unit ability",
            UniqueTarget.Nation: "National ability",
            UniqueTarget.Global: GlobalUniques.get_unique_source_description(unique)
        }.get(unique.source_object_type,
              f"[{unique.source_object_name}] ([{unique.get_source_name_for_user()}])")

        source = tr(source)
        if not unique.modifiers:
            return source

        conditionals_text = " - ".join(tr(mod.text) for mod in unique.modifiers)
        return f"{source} - {conditionals_text}"

    @staticmethod
    def get_general_modifiers(
        combatant: ICombatant,
        enemy: ICombatant,
        combat_action: CombatAction,
        tile_to_attack_from: Tile
    ) -> Counter[str]:
        modifiers = Counter[str]()
        conditional_state = BattleDamage.get_state_for_conditionals(
            combat_action, combatant, enemy)
        civ_info = combatant.get_civ_info()

        if isinstance(combatant, MapUnitCombatant):
            BattleDamage.add_unit_unique_modifiers(
                combatant, enemy, conditional_state, tile_to_attack_from, modifiers)
            BattleDamage.add_resource_lacking_malus(combatant, modifiers)

            great_general_name, great_general_bonus = GreatGeneralImplementation.get_great_general_bonus(
                combatant, enemy, combat_action)
            if great_general_bonus != 0:
                modifiers[great_general_name] = great_general_bonus

            for unique in combatant.unit.get_matching_uniques(UniqueType.StrengthWhenStacked):
                stacked_units_bonus = 0
                if any(unit.matches_filter(unique.params[1])
                      for unit in combatant.unit.get_tile().get_units()):
                    stacked_units_bonus += int(unique.params[0])

                if stacked_units_bonus > 0:
                    modifiers[f"Stacked with [{unique.params[1]}]"] = stacked_units_bonus

        elif isinstance(combatant, CityCombatant):
            for unique in combatant.city.get_matching_uniques(
                UniqueType.StrengthForCities, conditional_state):
                modifiers.add(
                    BattleDamage.get_modifier_string_from_unique(unique),
                    int(unique.params[0]))

        if enemy.get_civ_info().is_barbarian:
            modifiers["Difficulty"] = int(
                civ_info.game_info.get_difficulty().barbarian_bonus * 100)

        return modifiers

    @staticmethod
    def get_state_for_conditionals(
        combat_action: CombatAction,
        combatant: ICombatant,
        enemy: ICombatant
    ) -> StateForConditionals:
        attacked_tile = (enemy.get_tile() if combat_action == CombatAction.Attack
                        else combatant.get_tile())

        return StateForConditionals(
            combatant.get_civ_info(),
            city=combatant.city if isinstance(combatant, CityCombatant) else None,
            our_combatant=combatant,
            their_combatant=enemy,
            attacked_tile=attacked_tile,
            combat_action=combat_action
        )

    @staticmethod
    def add_unit_unique_modifiers(
        combatant: MapUnitCombatant,
        enemy: ICombatant,
        conditional_state: StateForConditionals,
        tile_to_attack_from: Tile,
        modifiers: Counter[str]
    ) -> None:
        civ_info = combatant.get_civ_info()

        for unique in combatant.get_matching_uniques(
            UniqueType.Strength, conditional_state, True):
            modifiers.add(
                BattleDamage.get_modifier_string_from_unique(unique),
                int(unique.params[0]))

        for unique in combatant.get_matching_uniques(
            UniqueType.StrengthNearCapital, conditional_state, True):
            if not civ_info.cities or not civ_info.get_capital():
                break
            distance = combatant.get_tile().aerial_distance_to(
                civ_info.get_capital().get_center_tile())
            effect = int(unique.params[0]) - 3 * distance
            if effect > 0:
                modifiers.add(
                    f"{unique.source_object_name} ({unique.get_source_name_for_user()})",
                    effect)

        adjacent_units = [unit for tile in combatant.get_tile().neighbors
                         for unit in tile.get_units()]
        if (enemy.get_tile() not in combatant.get_tile().neighbors and
            tile_to_attack_from in combatant.get_tile().neighbors and
            isinstance(enemy, MapUnitCombatant)):
            adjacent_units.append(enemy.unit)

        strength_malus = max(
            (unique for unit in adjacent_units
             if unit.civ.is_at_war_with(combatant.get_civ_info())
             for unique in unit.get_matching_uniques(UniqueType.StrengthForAdjacentEnemies)
             if (combatant.matches_filter(unique.params[1]) and
                 combatant.get_tile().matches_filter(unique.params[2]))),
            key=lambda x: x.params[0],
            default=None
        )
        if strength_malus:
            modifiers.add("Adjacent enemy units", int(strength_malus.params[0]))

    @staticmethod
    def add_resource_lacking_malus(
        combatant: MapUnitCombatant,
        modifiers: Counter[str]
    ) -> None:
        civ_info = combatant.get_civ_info()
        civ_resources = civ_info.get_civ_resources_by_name()
        for resource in combatant.unit.get_resource_requirements_per_turn():
            if civ_resources[resource] < 0 and not civ_info.is_barbarian:
                modifiers["Missing resource"] = BattleConstants.MISSING_RESOURCES_MALUS

    @staticmethod
    def get_attack_modifiers(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile
    ) -> Counter[str]:
        modifiers = BattleDamage.get_general_modifiers(
            attacker, defender, CombatAction.Attack, tile_to_attack_from)

        if isinstance(attacker, MapUnitCombatant):
            BattleDamage.add_terrain_attack_modifiers(
                attacker, defender, tile_to_attack_from, modifiers)

            if attacker.unit.is_preparing_air_sweep():
                modifiers.add(BattleDamage.get_air_sweep_attack_modifiers(attacker))

            if attacker.is_melee():
                number_of_other_attackers = sum(
                    1 for tile in defender.get_tile().neighbors
                    if (tile.military_unit and tile.military_unit != attacker.unit
                        and tile.military_unit.owner == attacker.get_civ_info().civ_name
                        and MapUnitCombatant(tile.military_unit).is_melee())
                )
                if number_of_other_attackers > 0:
                    flanking_bonus = BattleConstants.BASE_FLANKING_BONUS

                    for unique in attacker.unit.get_matching_uniques(
                        UniqueType.FlankAttackBonus,
                        check_civ_info_uniques=True,
                        state_for_conditionals=BattleDamage.get_state_for_conditionals(
                            CombatAction.Attack, attacker, defender)):
                        flanking_bonus *= to_percent(unique.params[0])

                    modifiers["Flanking"] = int(
                        flanking_bonus * number_of_other_attackers)

        return modifiers

    @staticmethod
    def add_terrain_attack_modifiers(
        attacker: MapUnitCombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile,
        modifiers: Counter[str]
    ) -> None:
        if (attacker.unit.is_embarked() and defender.get_tile().is_land
            and not attacker.unit.has_unique(UniqueType.AttackAcrossCoast)):
            modifiers["Landing"] = BattleConstants.LANDING_MALUS

        if (attacker.unit.type.is_land_unit() and not attacker.get_tile().is_water
            and attacker.is_melee() and defender.get_tile().is_water
            and not attacker.unit.has_unique(UniqueType.AttackAcrossCoast)):
            modifiers["Boarding"] = BattleConstants.BOARDING_MALUS

        if (not attacker.unit.type.is_air_unit() and attacker.is_melee()
            and attacker.get_tile().is_water and not defender.get_tile().is_water
            and not attacker.unit.has_unique(UniqueType.AttackAcrossCoast)
            and not defender.is_city()):
            modifiers["Landing"] = BattleConstants.LANDING_MALUS

        if BattleDamage.is_melee_attacking_across_river_with_no_bridge(
            attacker, tile_to_attack_from, defender):
            modifiers["Across river"] = BattleConstants.ATTACKING_ACROSS_RIVER_MALUS

    @staticmethod
    def is_melee_attacking_across_river_with_no_bridge(
        attacker: MapUnitCombatant,
        tile_to_attack_from: Tile,
        defender: ICombatant
    ) -> bool:
        return (attacker.is_melee()
                and tile_to_attack_from.aerial_distance_to(defender.get_tile()) == 1
                and tile_to_attack_from.is_connected_by_river(defender.get_tile())
                and not attacker.unit.has_unique(UniqueType.AttackAcrossRiver)
                and (not tile_to_attack_from.has_connection(attacker.get_civ_info())
                     or not defender.get_tile().has_connection(attacker.get_civ_info())
                     or not attacker.get_civ_info().tech.roads_connect_across_rivers))

    @staticmethod
    def get_air_sweep_attack_modifiers(attacker: ICombatant) -> Counter[str]:
        modifiers = Counter[str]()
        if isinstance(attacker, MapUnitCombatant):
            for unique in attacker.unit.get_matching_uniques(UniqueType.StrengthWhenAirsweep):
                modifiers.add(
                    BattleDamage.get_modifier_string_from_unique(unique),
                    int(unique.params[0]))
        return modifiers

    @staticmethod
    def get_defence_modifiers(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile
    ) -> Counter[str]:
        modifiers = BattleDamage.get_general_modifiers(
            defender, attacker, CombatAction.Defend, tile_to_attack_from)
        tile = defender.get_tile()

        if (isinstance(defender, MapUnitCombatant)
            and not defender.unit.is_embarked()):
            tile_defence_bonus = tile.get_defensive_bonus(unit=defender.unit)
            if ((not defender.unit.has_unique(UniqueType.NoDefensiveTerrainBonus,
                                           check_civ_info_uniques=True)
                 and tile_defence_bonus > 0)
                or (not defender.unit.has_unique(UniqueType.NoDefensiveTerrainPenalty,
                                               check_civ_info_uniques=True)
                    and tile_defence_bonus < 0)):
                modifiers["Tile"] = int(tile_defence_bonus * 100)

            if defender.unit.is_fortified() or defender.unit.is_guarding():
                modifiers["Fortification"] = (
                    BattleConstants.FORTIFICATION_BONUS *
                    defender.unit.get_fortification_turns())

        return modifiers

    @staticmethod
    def modifiers_to_final_bonus(modifiers: Counter[str]) -> float:
        final_modifier = 1.0
        for modifier_value in modifiers.values():
            final_modifier += modifier_value / 100.0
        return final_modifier

    @staticmethod
    def get_health_dependant_damage_ratio(combatant: ICombatant) -> float:
        if (not isinstance(combatant, MapUnitCombatant)
            or combatant.unit.has_unique(UniqueType.NoDamagePenaltyWoundedUnits,
                                       check_civ_info_uniques=True)):
            return 1.0
        return 1 - (100 - combatant.get_health()) / BattleConstants.DAMAGE_REDUCTION_WOUNDED_UNIT_RATIO_PERCENTAGE

    @staticmethod
    def get_attacking_strength(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile
    ) -> float:
        attack_modifier = BattleDamage.modifiers_to_final_bonus(
            BattleDamage.get_attack_modifiers(attacker, defender, tile_to_attack_from))
        return max(1.0, attacker.get_attacking_strength() * attack_modifier)

    @staticmethod
    def get_defending_strength(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile
    ) -> float:
        defence_modifier = BattleDamage.modifiers_to_final_bonus(
            BattleDamage.get_defence_modifiers(attacker, defender, tile_to_attack_from))
        return max(1.0, defender.get_defending_strength(attacker.is_ranged()) * defence_modifier)

    @staticmethod
    def calculate_damage_to_attacker(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile = None,
        randomness_factor: float = None
    ) -> int:
        if tile_to_attack_from is None:
            tile_to_attack_from = defender.get_tile()
        if randomness_factor is None:
            random_seed = (attacker.get_civ_info().game_info.turns *
                         attacker.get_tile().position.hash_code())
            randomness_factor = random.Random(random_seed).random()

        if attacker.is_ranged() and not attacker.is_air_unit():
            return 0
        if defender.is_civilian():
            return 0

        ratio = (BattleDamage.get_attacking_strength(attacker, defender, tile_to_attack_from) /
                BattleDamage.get_defending_strength(attacker, defender, tile_to_attack_from))
        return int(BattleDamage.damage_modifier(ratio, True, randomness_factor) *
                  BattleDamage.get_health_dependant_damage_ratio(defender))

    @staticmethod
    def calculate_damage_to_defender(
        attacker: ICombatant,
        defender: ICombatant,
        tile_to_attack_from: Tile = None,
        randomness_factor: float = None
    ) -> int:
        if tile_to_attack_from is None:
            tile_to_attack_from = defender.get_tile()
        if randomness_factor is None:
            random_seed = (defender.get_civ_info().game_info.turns *
                         defender.get_tile().position.hash_code())
            randomness_factor = random.Random(random_seed).random()

        if defender.is_civilian():
            return BattleConstants.DAMAGE_TO_CIVILIAN_UNIT

        ratio = (BattleDamage.get_attacking_strength(attacker, defender, tile_to_attack_from) /
                BattleDamage.get_defending_strength(attacker, defender, tile_to_attack_from))
        return int(BattleDamage.damage_modifier(ratio, False, randomness_factor) *
                  BattleDamage.get_health_dependant_damage_ratio(attacker))

    @staticmethod
    def damage_modifier(
        attacker_to_defender_ratio: float,
        damage_to_attacker: bool,
        randomness_factor: float
    ) -> float:
        stronger_to_weaker_ratio = pow(attacker_to_defender_ratio,
                                     -1 if attacker_to_defender_ratio < 1 else 1)
        ratio_modifier = (pow((stronger_to_weaker_ratio + 3) / 4, 4) + 1) / 2
        if ((damage_to_attacker and attacker_to_defender_ratio > 1) or
            (not damage_to_attacker and attacker_to_defender_ratio < 1)):
            ratio_modifier = pow(ratio_modifier, -1)
        random_centered_around_30 = 24 + 12 * randomness_factor
        return random_centered_around_30 * ratio_modifier