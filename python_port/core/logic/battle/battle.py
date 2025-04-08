from dataclasses import dataclass
from typing import Optional, List, Sequence, Tuple, Dict, Any
from enum import Enum
import random
from com.badlogic.gdx.math import Vector2
from com.unciv import Constants
from com.unciv import UncivGame
from com.unciv.logic.automation.civilization import NextTurnAutomation
from com.unciv.logic.city import City
from com.unciv.logic.civilization import (
    AlertType, Civilization, LocationAction, MapUnitAction,
    NotificationCategory, NotificationIcon, PopupAlert, PromoteUnitAction
)
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UnitActionType
from com.unciv.models.ruleset.unique import (
    StateForConditionals, Unique, UniqueTriggerActivation, UniqueType
)
from com.unciv.models.stats import Stat, Stats, SubStat
from com.unciv.ui.components import UnitMovementMemoryType
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActionsPillage
from com.unciv.utils import debug

@dataclass
class DamageDealt:
    attacker_dealt: int
    defender_dealt: int

    def __add__(self, other: 'DamageDealt') -> 'DamageDealt':
        return DamageDealt(
            self.attacker_dealt + other.attacker_dealt,
            self.defender_dealt + other.defender_dealt
        )

    @classmethod
    def none(cls) -> 'DamageDealt':
        return cls(0, 0)

class CombatAction(Enum):
    Attack = "Attack"
    Defend = "Defend"
    Intercept = "Intercept"

class Battle:
    @staticmethod
    def move_and_attack(attacker: 'ICombatant', attackable_tile: 'AttackableTile') -> None:
        if not Battle.move_preparing_attack(attacker, attackable_tile, True):
            return
        Battle.attack_or_nuke(attacker, attackable_tile)

    @staticmethod
    def move_preparing_attack(
        attacker: 'ICombatant',
        attackable_tile: 'AttackableTile',
        try_heal_pillage: bool = False
    ) -> bool:
        if not isinstance(attacker, MapUnitCombatant):
            return True

        tiles_moved_through = attacker.unit.movement.get_distance_to_tiles().get_path_to_tile(
            attackable_tile.tile_to_attack_from
        )
        attacker.unit.movement.move_to_tile(attackable_tile.tile_to_attack_from)

        if attacker.get_tile() != attackable_tile.tile_to_attack_from:
            return False

        combatant = Battle.get_map_combatant_of_tile(attackable_tile.tile_to_attack)
        if combatant is None or combatant.get_civ_info() == attacker.get_civ_info():
            return False

        if (attacker.has_unique(UniqueType.MustSetUp)
                and not attacker.unit.is_set_up_for_siege()
                and attacker.unit.has_movement()):
            attacker.unit.action = UnitActionType.SetUp.value
            attacker.unit.use_movement_points(1.0)

        if try_heal_pillage:
            for tile_to_pillage in tiles_moved_through:
                if attacker.unit.current_movement <= 1.0 or attacker.unit.health > 90:
                    break

                if (UnitActionsPillage.can_pillage(attacker.unit, tile_to_pillage)
                    and tile_to_pillage.can_pillage_tile_improvement()):
                    UnitActionsPillage.get_pillage_action(attacker.unit, tile_to_pillage)?.action?.invoke()

        return attacker.unit.has_movement()

    @staticmethod
    def attack_or_nuke(attacker: 'ICombatant', attackable_tile: 'AttackableTile') -> DamageDealt:
        if (isinstance(attacker, MapUnitCombatant)
            and attacker.unit.is_nuclear_weapon()):
            Nuke.NUKE(attacker, attackable_tile.tile_to_attack)
            return DamageDealt.none()
        else:
            return Battle.attack(
                attacker,
                Battle.get_map_combatant_of_tile(attackable_tile.tile_to_attack)
            )

    @staticmethod
    def attack(attacker: 'ICombatant', defender: 'ICombatant') -> DamageDealt:
        debug("%s %s attacked %s %s",
              attacker.get_civ_info().civ_name,
              attacker.get_name(),
              defender.get_civ_info().civ_name,
              defender.get_name())

        attacked_tile = defender.get_tile()
        if isinstance(attacker, MapUnitCombatant):
            attacker.unit.attacks_since_turn_start.append(Vector2(attacked_tile.position))
        else:
            attacker.get_civ_info().attacks_since_turn_start.append(
                Civilization.HistoricalAttackMemory(
                    None,
                    Vector2(attacker.get_tile().position),
                    Vector2(attacked_tile.position)
                )
            )

        # Handle air interception
        if (isinstance(attacker, MapUnitCombatant)
            and attacker.unit.base_unit.is_air_unit()):
            intercept_damage = AirInterception.try_intercept_air_attack(
                attacker, attacked_tile, defender.get_civ_info(), defender
            )
            if attacker.is_defeated():
                return intercept_damage
        else:
            intercept_damage = DamageDealt.none()

        # Handle withdraw from melee ability
        if (isinstance(attacker, MapUnitCombatant)
            and attacker.is_melee()
            and isinstance(defender, MapUnitCombatant)
            and defender.unit.has_unique(
                UniqueType.WithdrawsBeforeMeleeCombat,
                state_for_conditionals=StateForConditionals(
                    civ_info=defender.get_civ_info(),
                    our_combatant=defender,
                    their_combatant=attacker,
                    tile=attacked_tile
                )
            )
            and Battle.do_withdraw_from_melee_ability(attacker, defender)):
            return DamageDealt.none()

        is_already_defeated_city = (isinstance(defender, CityCombatant)
                                 and defender.is_defeated())

        damage_dealt = Battle.take_damage(attacker, defender)

        # Handle unit capture
        capture_military_unit_success = BattleUnitCapture.try_capture_military_unit(
            attacker, defender, attacked_tile
        )

        if not capture_military_unit_success:
            Battle.post_battle_notifications(
                attacker, defender, attacked_tile,
                attacker.get_tile(), damage_dealt
            )

        # Handle barbarian camp attack
        if (defender.get_civ_info().is_barbarian
            and attacked_tile.improvement == Constants.barbarian_encampment):
            defender.get_civ_info().game_info.barbarians.camp_attacked(
                attacked_tile.position
            )

        # Handle city capture
        if (defender.is_defeated()
            and isinstance(defender, CityCombatant)
            and isinstance(attacker, MapUnitCombatant)
            and attacker.is_melee()
            and not attacker.unit.has_unique(
                UniqueType.CannotCaptureCities,
                check_civ_info_uniques=True
            )):
            if attacker.unit.civ.is_barbarian:
                defender.take_damage(-1)  # Back to 2 HP
                ransom = min(200, defender.city.civ.gold)
                defender.city.civ.add_gold(-ransom)
                defender.city.civ.add_notification(
                    f"Barbarians raided [{defender.city.name}] and stole [{ransom}] Gold from your treasury!",
                    defender.city.location,
                    NotificationCategory.War,
                    NotificationIcon.War
                )
                attacker.unit.destroy()
            else:
                Battle.conquer_city(defender.city, attacker)

        # Handle exploring units
        if (not defender.is_defeated()
            and isinstance(defender, MapUnitCombatant)
            and defender.unit.is_exploring()):
            defender.unit.action = None

        # Handle post-battle effects
        if (defender.is_defeated()
            and isinstance(defender, MapUnitCombatant)
            and not defender.unit.is_civilian()):
            Battle.try_earn_from_killing(attacker, defender)
            Battle.try_heal_after_killing(attacker)

            if isinstance(attacker, MapUnitCombatant):
                Battle.trigger_victory_uniques(attacker, defender, attacked_tile)
            Battle.trigger_defeat_uniques(defender, attacker, attacked_tile)

        elif (attacker.is_defeated()
              and isinstance(attacker, MapUnitCombatant)
              and not attacker.unit.is_civilian()):
            Battle.try_earn_from_killing(defender, attacker)
            Battle.try_heal_after_killing(defender)

            if isinstance(defender, MapUnitCombatant):
                Battle.trigger_victory_uniques(defender, attacker, attacked_tile)
            Battle.trigger_defeat_uniques(attacker, defender, attacked_tile)

        if (isinstance(attacker, MapUnitCombatant)
            and isinstance(defender, MapUnitCombatant)):
            Battle.trigger_damage_uniques_for_unit(
                attacker, defender, attacked_tile, CombatAction.Attack
            )
            if not attacker.is_ranged():
                Battle.trigger_damage_uniques_for_unit(
                    defender, attacker, attacked_tile, CombatAction.Defend
                )

        if isinstance(attacker, MapUnitCombatant):
            if attacker.unit.has_unique(UniqueType.SelfDestructs):
                attacker.unit.destroy()
            elif attacker.unit.is_moving():
                attacker.unit.action = None
            Battle.do_destroy_improvements_ability(attacker, attacked_tile, defender)

        if not capture_military_unit_success:
            Battle.post_battle_move_to_attacked_tile(attacker, defender, attacked_tile)

        Battle.reduce_attacker_movement_points_and_attacks(
            attacker, defender, attacked_tile
        )

        if not is_already_defeated_city:
            Battle.post_battle_add_xp(attacker, defender)

        if isinstance(attacker, CityCombatant):
            city_can_bombard_notification = next(
                (n for n in attacker.get_civ_info().notifications
                 if n.text == f"Your city [{attacker.get_name()}] can bombard the enemy!"),
                None
            )
            if city_can_bombard_notification:
                attacker.get_civ_info().notifications.remove(city_can_bombard_notification)

        return damage_dealt + intercept_damage

    @staticmethod
    def trigger_victory_uniques(
        our_unit: MapUnitCombatant,
        enemy: MapUnitCombatant,
        attacked_tile: Tile
    ) -> None:
        state_for_conditionals = StateForConditionals(
            civ_info=our_unit.get_civ_info(),
            our_combatant=our_unit,
            their_combatant=enemy,
            tile=attacked_tile
        )
        for unique in our_unit.unit.get_triggered_uniques(
            UniqueType.TriggerUponDefeatingUnit,
            state_for_conditionals
        ):
            if enemy.unit.matches_filter(unique.params[0]):
                UniqueTriggerActivation.trigger_unique(
                    unique,
                    our_unit.unit,
                    trigger_notification_text=f"due to our [{our_unit.get_name()}] defeating a [{enemy.get_name()}]"
                )

    @staticmethod
    def trigger_damage_uniques_for_unit(
        triggering_unit: MapUnitCombatant,
        enemy: MapUnitCombatant,
        attacked_tile: Tile,
        combat_action: CombatAction
    ) -> None:
        state_for_conditionals = StateForConditionals(
            civ_info=triggering_unit.get_civ_info(),
            our_combatant=triggering_unit,
            their_combatant=enemy,
            tile=attacked_tile,
            combat_action=combat_action
        )

        for unique in triggering_unit.unit.get_triggered_uniques(
            UniqueType.TriggerUponDamagingUnit,
            state_for_conditionals
        ):
            if enemy.matches_filter(unique.params[0]):
                if unique.params[0] == Constants.target_unit:
                    UniqueTriggerActivation.trigger_unique(
                        unique,
                        enemy.unit,
                        trigger_notification_text=f"due to our [{enemy.get_name()}] being damaged by a [{triggering_unit.get_name()}]"
                    )
                else:
                    UniqueTriggerActivation.trigger_unique(
                        unique,
                        triggering_unit.unit,
                        trigger_notification_text=f"due to our [{triggering_unit.get_name()}] damaging a [{enemy.get_name()}]"
                    )

    @staticmethod
    def trigger_defeat_uniques(
        our_unit: MapUnitCombatant,
        enemy: 'ICombatant',
        attacked_tile: Tile
    ) -> None:
        state_for_conditionals = StateForConditionals(
            civ_info=our_unit.get_civ_info(),
            our_combatant=our_unit,
            their_combatant=enemy,
            tile=attacked_tile
        )
        for unique in our_unit.unit.get_triggered_uniques(
            UniqueType.TriggerUponDefeat,
            state_for_conditionals
        ):
            UniqueTriggerActivation.trigger_unique(
                unique,
                our_unit.unit,
                trigger_notification_text=f"due to our [{our_unit.get_name()}] being defeated by a [{enemy.get_name()}]"
            )

    @staticmethod
    def try_earn_from_killing(
        civ_unit: 'ICombatant',
        defeated_unit: MapUnitCombatant
    ) -> None:
        unit_str = max(
            defeated_unit.unit.base_unit.strength,
            defeated_unit.unit.base_unit.ranged_strength
        )
        unit_cost = defeated_unit.unit.base_unit.cost

        bonus_uniques = Battle.get_kill_unit_plunder_uniques(civ_unit, defeated_unit)

        for unique in bonus_uniques:
            if not defeated_unit.matches_filter(unique.params[1]):
                continue

            yield_percent = float(unique.params[0]) / 100
            defeated_unit_yield_source_type = unique.params[2]
            yield_type_source_amount = (
                unit_cost if defeated_unit_yield_source_type == "Cost"
                else unit_str
            )
            yield_amount = int(yield_type_source_amount * yield_percent)

            resource = civ_unit.get_civ_info().game_info.ruleset.get_game_resource(
                unique.params[3]
            )
            if not resource:
                continue
            civ_unit.get_civ_info().add_game_resource(resource, yield_amount)

        # Handle city state friendship from killing barbarians
        if (defeated_unit.get_civ_info().is_barbarian
            and not defeated_unit.is_civilian()
            and civ_unit.get_civ_info().is_major_civ()):
            for city_state in defeated_unit.get_civ_info().game_info.get_alive_city_states():
                if (civ_unit.get_civ_info().knows(city_state)
                    and defeated_unit.unit.threatens_civ(city_state)):
                    city_state.city_state_functions.threatening_barbarian_killed_by(
                        civ_unit.get_civ_info()
                    )

        # Handle city state war with major pseudo-quest
        for city_state in defeated_unit.get_civ_info().game_info.get_alive_city_states():
            city_state.quest_manager.military_unit_killed_by(
                civ_unit.get_civ_info(),
                defeated_unit.get_civ_info()
            )

    @staticmethod
    def get_kill_unit_plunder_uniques(
        civ_unit: 'ICombatant',
        defeated_unit: MapUnitCombatant
    ) -> List[Unique]:
        bonus_uniques = []

        state_for_conditionals = StateForConditionals(
            civ_info=civ_unit.get_civ_info(),
            our_combatant=civ_unit,
            their_combatant=defeated_unit
        )
        if isinstance(civ_unit, MapUnitCombatant):
            bonus_uniques.extend(
                civ_unit.get_matching_uniques(
                    UniqueType.KillUnitPlunder,
                    state_for_conditionals,
                    True
                )
            )
        else:
            bonus_uniques.extend(
                civ_unit.get_civ_info().get_matching_uniques(
                    UniqueType.KillUnitPlunder,
                    state_for_conditionals
                )
            )

        city_with_religion = next(
            (t.get_city() for t in civ_unit.get_tile().get_tiles_in_distance(4)
             if t.is_city_center() and t.get_city() and t.get_city().get_matching_uniques(
                 UniqueType.KillUnitPlunderNearCity,
                 state_for_conditionals
             )),
            None
        )
        if city_with_religion:
            bonus_uniques.extend(
                city_with_religion.get_matching_uniques(
                    UniqueType.KillUnitPlunderNearCity,
                    state_for_conditionals
                )
            )
        return bonus_uniques

    @staticmethod
    def take_damage(attacker: 'ICombatant', defender: 'ICombatant') -> DamageDealt:
        potential_damage_to_defender = BattleDamage.calculate_damage_to_defender(
            attacker, defender
        )
        potential_damage_to_attacker = BattleDamage.calculate_damage_to_attacker(
            attacker, defender
        )

        attacker_health_before = attacker.get_health()
        defender_health_before = defender.get_health()

        if (isinstance(defender, MapUnitCombatant)
            and defender.unit.is_civilian()
            and attacker.is_melee()):
            BattleUnitCapture.capture_civilian_unit(attacker, defender)
        elif attacker.is_ranged() and not attacker.is_air_unit():
            defender.take_damage(potential_damage_to_defender)
        else:
            while potential_damage_to_defender + potential_damage_to_attacker > 0:
                if random.random() * (potential_damage_to_defender + potential_damage_to_attacker) < potential_damage_to_defender:
                    potential_damage_to_defender -= 1
                    defender.take_damage(1)
                    if defender.is_defeated():
                        break
                else:
                    potential_damage_to_attacker -= 1
                    attacker.take_damage(1)
                    if attacker.is_defeated():
                        break

        defender_damage_dealt = attacker_health_before - attacker.get_health()
        attacker_damage_dealt = defender_health_before - defender.get_health()

        if isinstance(attacker, MapUnitCombatant):
            for unique in attacker.unit.get_triggered_uniques(
                UniqueType.TriggerUponLosingHealth
            ):
                if int(unique.params[0]) <= defender_damage_dealt:
                    UniqueTriggerActivation.trigger_unique(
                        unique,
                        attacker.unit,
                        trigger_notification_text=f"due to losing [{defender_damage_dealt}] HP"
                    )

        if isinstance(defender, MapUnitCombatant):
            for unique in defender.unit.get_triggered_uniques(
                UniqueType.TriggerUponLosingHealth
            ):
                if int(unique.params[0]) <= attacker_damage_dealt:
                    UniqueTriggerActivation.trigger_unique(
                        unique,
                        defender.unit,
                        trigger_notification_text=f"due to losing [{attacker_damage_dealt}] HP"
                    )

        Battle.plunder_from_damage(attacker, defender, attacker_damage_dealt)
        return DamageDealt(attacker_damage_dealt, defender_damage_dealt)

    @staticmethod
    def plunder_from_damage(
        plundering_unit: 'ICombatant',
        plundered_unit: 'ICombatant',
        damage_dealt: int
    ) -> None:
        if not isinstance(plundering_unit, MapUnitCombatant):
            return

        civ = plundering_unit.get_civ_info()
        plundered_goods = Stats()

        for unique in plundering_unit.unit.get_matching_uniques(
            UniqueType.DamageUnitsPlunder,
            check_civ_info_uniques=True
        ):
            if not plundered_unit.matches_filter(unique.params[1]):
                continue

            percentage = float(unique.params[0])
            amount = percentage / 100.0 * damage_dealt
            resource_name = unique.params[2]
            resource = plundered_unit.get_civ_info().game_info.ruleset.get_game_resource(
                resource_name
            )
            if not resource:
                continue

            if isinstance(resource, Stat):
                plundered_goods.add(resource, amount)
                continue

            plundered_amount = round(amount)
            civ.add_game_resource(resource, plundered_amount)
            icon = (resource.icon if isinstance(resource, SubStat)
                   else f"ResourceIcons/{resource_name}")
            civ.add_notification(
                f"Your [{plundering_unit.get_name()}] plundered [{plundered_amount}] [{resource_name}] from [{plundered_unit.get_name()}]",
                plundered_unit.get_tile().position,
                NotificationCategory.War,
                plundering_unit.get_name(),
                NotificationIcon.War,
                icon,
                NotificationIcon.City if isinstance(plundered_unit, CityCombatant)
                else plundered_unit.get_name()
            )

        for resource, amount in plundered_goods.items():
            plundered_amount = int(amount)
            if plundered_amount == 0:
                continue
            civ.add_stat(resource, plundered_amount)
            civ.add_notification(
                f"Your [{plundering_unit.get_name()}] plundered [{plundered_amount}] [{resource.name}] from [{plundered_unit.get_name()}]",
                plundered_unit.get_tile().position,
                NotificationCategory.War,
                plundering_unit.get_name(),
                NotificationIcon.War,
                f"StatIcons/{resource.name}",
                NotificationIcon.City if isinstance(plundered_unit, CityCombatant)
                else plundered_unit.get_name()
            )

    @staticmethod
    def post_battle_notifications(
        attacker: 'ICombatant',
        defender: 'ICombatant',
        attacked_tile: Tile,
        attacker_tile: Optional[Tile] = None,
        damage_dealt: Optional[DamageDealt] = None
    ) -> None:
        if attacker.get_civ_info() == defender.get_civ_info():
            return

        battle_action_icon, battle_action_string = (
            (NotificationIcon.War, "was destroyed while attacking")
            if (not isinstance(attacker, CityCombatant) and attacker.is_defeated())
            else (NotificationIcon.War, "has attacked")
            if not defender.is_defeated()
            else (NotificationIcon.War, "has raided")
            if defender.is_city() and attacker.is_melee() and attacker.get_civ_info().is_barbarian
            else (NotificationIcon.War, "has captured")
            if defender.is_city() and attacker.is_melee()
            else (NotificationIcon.Death, "has destroyed")
        )

        attacker_string = (
            f"Enemy city [{attacker.get_name()}]"
            if attacker.is_city()
            else f"An enemy [{attacker.get_name()}]"
        )

        defender_string = (
            f" the defence of [{defender.get_name()}]"
            if defender.is_city() and defender.is_defeated() and attacker.is_ranged()
            else f" [{defender.get_name()}]"
            if defender.is_city()
            else f" our [{defender.get_name()}]"
        )

        attacker_hurt_string = (
            f" ([-{damage_dealt.defender_dealt}] HP)"
            if damage_dealt and damage_dealt.defender_dealt != 0
            else ""
        )
        defender_hurt_string = (
            f" ([-{damage_dealt.attacker_dealt}] HP)"
            if damage_dealt
            else ""
        )

        notification_string = (
            f"[{{{attacker_string}}}{{{attacker_hurt_string}}}] "
            f"[{battle_action_string}] "
            f"[{{{defender_string}}}{{{defender_hurt_string}}}]"
        )

        attacker_icon = (
            NotificationIcon.City
            if isinstance(attacker, CityCombatant)
            else attacker.get_name()
        )
        defender_icon = (
            NotificationIcon.City
            if isinstance(defender, CityCombatant)
            else defender.get_name()
        )

        locations = LocationAction(attacked_tile.position, attacker_tile.position)
        defender.get_civ_info().add_notification(
            notification_string,
            locations,
            NotificationCategory.War,
            attacker_icon,
            battle_action_icon,
            defender_icon
        )

    @staticmethod
    def try_heal_after_killing(attacker: 'ICombatant') -> None:
        if not isinstance(attacker, MapUnitCombatant):
            return

        for unique in attacker.unit.get_matching_uniques(
            UniqueType.HealsAfterKilling,
            check_civ_info_uniques=True
        ):
            amount_to_heal = int(unique.params[0])
            attacker.unit.heal_by(amount_to_heal)

    @staticmethod
    def post_battle_move_to_attacked_tile(
        attacker: 'ICombatant',
        defender: 'ICombatant',
        attacked_tile: Tile
    ) -> None:
        if not attacker.is_melee():
            return
        if not defender.is_defeated() and defender.get_civ_info() != attacker.get_civ_info():
            return
        if isinstance(attacker, MapUnitCombatant) and attacker.unit.cache.cannot_move:
            return

        if (isinstance(attacker, MapUnitCombatant)
            and attacker.unit.movement.can_move_to(attacked_tile)):
            attacker.unit.movement.move_to_tile(
                attacked_tile,
                consider_zone_of_control=False
            )
            attacker.unit.most_recent_move_type = UnitMovementMemoryType.UnitAttacked

    @staticmethod
    def post_battle_add_xp(attacker: 'ICombatant', defender: 'ICombatant') -> None:
        def add_xp(attacker_xp: int, defender_xp: int):
            Battle.add_xp(attacker, attacker_xp, defender)
            Battle.add_xp(defender, defender_xp, attacker)

        if attacker.is_air_unit():
            add_xp(4, 2)
        elif attacker.is_ranged():
            if defender.is_city():
                add_xp(3, 2)
            else:
                add_xp(2, 2)
        elif not defender.is_civilian():
            add_xp(5, 4)

    @staticmethod
    def add_xp(
        this_combatant: 'ICombatant',
        amount: int,
        other_combatant: 'ICombatant'
    ) -> None:
        if not isinstance(this_combatant, MapUnitCombatant):
            return

        civ = this_combatant.get_civ_info()
        other_is_barbarian = other_combatant.get_civ_info().is_barbarian
        promotions = this_combatant.unit.promotions
        mod_constants = civ.game_info.ruleset.mod_options.constants

        if (other_is_barbarian
            and promotions.total_xp_produced() >= mod_constants.max_xp_from_barbarians):
            return

        unit_could_already_promote = promotions.can_be_promoted()

        state_for_conditionals = StateForConditionals(
            civ_info=civ,
            our_combatant=this_combatant,
            their_combatant=other_combatant
        )

        base_xp = amount + sum(
            int(unique.params[0])
            for unique in this_combatant.get_matching_uniques(
                UniqueType.FlatXPGain,
                state_for_conditionals,
                True
            )
        )

        xp_bonus = sum(
            float(unique.params[0])
            for unique in this_combatant.get_matching_uniques(
                UniqueType.PercentageXPGain,
                state_for_conditionals,
                True
            )
        )
        xp_modifier = 1.0 + xp_bonus / 100

        xp_gained = int(base_xp * xp_modifier)
        promotions.xp += xp_gained

        if not other_is_barbarian and civ.is_major_civ():
            great_general_units = [
                unit for unit in civ.game_info.ruleset.great_general_units
                if (unit.has_unique(UniqueType.GreatPersonFromCombat, state_for_conditionals)
                    and not any(
                        not reason.is_construction_rejection()
                        and reason.tech_policy_era_wonder_requirements()
                        for reason in unit.get_rejection_reasons(civ)
                    ))
            ]

            if (not civ.game_info.ruleset.great_general_units
                and "Great General" in civ.game_info.ruleset.units):
                great_general_units.append(civ.game_info.ruleset.units["Great General"])

            for unit in great_general_units:
                great_general_points_bonus = sum(
                    float(unique.params[1])
                    for unique in this_combatant.get_matching_uniques(
                        UniqueType.GreatPersonEarnedFaster,
                        state_for_conditionals,
                        True
                    )
                    if unit.matches_filter(unique.params[0], state_for_conditionals)
                )
                great_general_points_modifier = 1.0 + great_general_points_bonus / 100

                great_general_points_gained = int(xp_gained * great_general_points_modifier)
                civ.great_people.great_general_points_counter[unit.name] += great_general_points_gained

        if (not this_combatant.is_defeated()
            and not unit_could_already_promote
            and promotions.can_be_promoted()):
            pos = this_combatant.get_tile().position
            civ.add_notification(
                f"[{this_combatant.unit.display_name()}] can be promoted!",
                [MapUnitAction(pos), PromoteUnitAction(this_combatant.get_name(), pos)],
                NotificationCategory.Units,
                this_combatant.unit.name
            )

    @staticmethod
    def reduce_attacker_movement_points_and_attacks(
        attacker: 'ICombatant',
        defender: 'ICombatant',
        attacked_tile: Tile
    ) -> None:
        if not isinstance(attacker, MapUnitCombatant):
            if isinstance(attacker, CityCombatant):
                attacker.city.attacked_this_turn = True
            return

        unit = attacker.unit
        if (defender.is_civilian()
            and attacker.get_tile() == defender.get_tile()):
            return

        state_for_conditionals = StateForConditionals(
            attacker, defender, attacked_tile, CombatAction.Attack
        )
        unit.attacks_this_turn += 1
        if (unit.has_unique(UniqueType.CanMoveAfterAttacking, state_for_conditionals)
            or unit.max_attacks_per_turn() > unit.attacks_this_turn):
            if (not attacker.unit.base_unit.moves_like_air_units
                and not (attacker.is_melee() and defender.is_defeated())):
                unit.use_movement_points(1.0)
        else:
            unit.current_movement = 0.0

        if (unit.is_fortified()
            or unit.is_sleeping()
            or unit.is_guarding()):
            attacker.unit.action = None

    @staticmethod
    def conquer_city(city: City, attacker: MapUnitCombatant) -> None:
        attacker_civ = attacker.get_civ_info()

        attacker_civ.add_notification(
            f"We have conquered the city of [{city.name}]!",
            city.location,
            NotificationCategory.War,
            NotificationIcon.War
        )

        city.has_just_been_conquered = True
        city.get_center_tile().apply(
            lambda tile: (
                tile.military_unit.destroy() if tile.military_unit else None,
                BattleUnitCapture.capture_civilian_unit(
                    attacker,
                    MapUnitCombatant(tile.civilian_unit),
                    check_defeat=False
                ) if tile.civilian_unit else None,
                [air_unit.destroy() for air_unit in tile.air_units]
            )
        )

        state_for_conditionals = StateForConditionals(
            civ_info=attacker_civ,
            city=city,
            unit=attacker.unit,
            our_combatant=attacker,
            attacked_tile=city.get_center_tile()
        )

        for unique in attacker.get_matching_uniques(
            UniqueType.CaptureCityPlunder,
            state_for_conditionals,
            True
        ):
            resource = attacker.get_civ_info().game_info.ruleset.get_game_resource(
                unique.params[2]
            )
            if not resource:
                continue
            attacker_civ.add_game_resource(
                resource,
                int(unique.params[0]) * city.city_stats.current_city_stats[
                    Stat(unique.params[1])
                ]
            )

        if attacker_civ.is_barbarian or attacker_civ.is_one_city_challenger():
            city.destroy_city(True)
            return

        if (city.is_original_capital
            and city.founding_civ == attacker_civ.civ_name):
            city.puppet_city(attacker_civ)
            city.annex_city()
        elif (attacker_civ.is_human()
              and not UncivGame.Current.world_screen.auto_play.is_auto_playing_and_full_auto_play_ai()):
            attacker_civ.popup_alerts.append(
                PopupAlert(AlertType.CityConquered, city.id)
            )
        else:
            Battle.automate_city_conquer(attacker_civ, city)

        if attacker_civ.is_current_player():
            UncivGame.Current.settings.add_completed_tutorial_task(
                "Conquer a city"
            )

        for unique in (attacker_civ.get_triggered_uniques(
            UniqueType.TriggerUponConqueringCity,
            state_for_conditionals
        ) + attacker.unit.get_triggered_uniques(
            UniqueType.TriggerUponConqueringCity,
            state_for_conditionals
        )):
            UniqueTriggerActivation.trigger_unique(unique, attacker.unit)

    @staticmethod
    def automate_city_conquer(civ_info: Civilization, city: City) -> None:
        if not city.has_diplomatic_marriage():
            founding_civ = civ_info.game_info.get_civilization(city.founding_civ)
            value_alliance = NextTurnAutomation.value_city_state_alliance(
                civ_info, founding_civ
            )
            if civ_info.get_happiness() < 0:
                value_alliance -= civ_info.get_happiness()
            if (founding_civ.is_city_state
                and city.civ != civ_info
                and founding_civ != civ_info
                and not civ_info.is_at_war_with(founding_civ)
                and value_alliance > 0):
                city.liberate_city(civ_info)
                return

        city.puppet_city(civ_info)
        if ((city.population.population < 4 or civ_info.is_city_state)
            and city.founding_civ != civ_info.civ_name
            and city.can_be_destroyed(just_captured=True)):
            if not civ_info.has_unique(UniqueType.MayNotAnnexCities):
                city.annex_city()
            city.is_being_razed = True

    @staticmethod
    def get_map_combatant_of_tile(tile: Tile) -> Optional['ICombatant']:
        if tile.is_city_center():
            return CityCombatant(tile.get_city())
        if tile.military_unit:
            return MapUnitCombatant(tile.military_unit)
        if tile.civilian_unit:
            return MapUnitCombatant(tile.civilian_unit)
        return None

    @staticmethod
    def destroy_if_defeated(
        attacked_civ: Civilization,
        attacker: Civilization,
        notification_location: Optional[Vector2] = None
    ) -> None:
        if attacked_civ.is_defeated():
            if attacked_civ.is_city_state:
                attacked_civ.city_state_functions.city_state_destroyed(attacker)
            attacked_civ.destroy(notification_location)
            attacker.popup_alerts.append(
                PopupAlert(AlertType.Defeated, attacked_civ.civ_name)
            )

    @staticmethod
    def do_withdraw_from_melee_ability(
        attacker: MapUnitCombatant,
        defender: MapUnitCombatant
    ) -> bool:
        if defender.unit.is_embarked():
            return False
        if defender.unit.cache.cannot_move:
            return False
        if defender.unit.is_escorting():
            return False
        if defender.unit.is_guarding():
            return False

        from_tile = defender.get_tile()
        attacker_tile = attacker.get_tile()

        def can_not_withdraw_to(tile: Tile) -> bool:
            return (not defender.unit.movement.can_move_to(tile)
                   or (defender.is_land_unit() and not tile.is_land)
                   or (tile.is_city_center()
                       and tile.get_owner() != defender.get_civ_info()))

        first_candidate_tiles = [
            tile for tile in from_tile.neighbors
            if tile != attacker_tile and tile not in attacker_tile.neighbors
            and not can_not_withdraw_to(tile)
        ]
        second_candidate_tiles = [
            tile for tile in from_tile.neighbors
            if tile in attacker_tile.neighbors
            and not can_not_withdraw_to(tile)
        ]

        to_tile = None
        if first_candidate_tiles:
            to_tile = random.choice(first_candidate_tiles)
        elif second_candidate_tiles:
            to_tile = random.choice(second_candidate_tiles)
        else:
            return False

        defender.unit.remove_from_tile()
        defender.unit.put_in_tile(to_tile)
        defender.unit.most_recent_move_type = UnitMovementMemoryType.UnitWithdrew

        Battle.reduce_attacker_movement_points_and_attacks(
            attacker, defender, from_tile
        )

        attacker_name = attacker.get_name()
        defender_name = defender.get_name()
        notification_string = f"[{defender_name}] withdrew from a [{attacker_name}]"
        locations = LocationAction(to_tile.position, attacker.get_tile().position)
        defender.get_civ_info().add_notification(
            notification_string,
            locations,
            NotificationCategory.War,
            defender_name,
            NotificationIcon.War,
            attacker_name
        )
        attacker.get_civ_info().add_notification(
            notification_string,
            locations,
            NotificationCategory.War,
            defender_name,
            NotificationIcon.War,
            attacker_name
        )
        return True

    @staticmethod
    def do_destroy_improvements_ability(
        attacker: MapUnitCombatant,
        attacked_tile: Tile,
        defender: 'ICombatant'
    ) -> None:
        if not attacked_tile.improvement:
            return

        conditional_state = StateForConditionals(
            attacker.get_civ_info(),
            our_combatant=attacker,
            their_combatant=defender,
            combat_action=CombatAction.Attack,
            attacked_tile=attacked_tile
        )

        if (not attacked_tile.get_tile_improvement().has_unique(UniqueType.Unpillagable)
            and attacker.has_unique(
                UniqueType.DestroysImprovementUponAttack,
                conditional_state
            )):
            current_tile_improvement = attacked_tile.improvement
            attacked_tile.remove_improvement()
            defender.get_civ_info().add_notification(
                f"An enemy [{attacker.unit.base_unit.name}] has destroyed our tile improvement [{current_tile_improvement}]",
                LocationAction(attacked_tile.position, attacker.get_tile().position),
                NotificationCategory.War,
                attacker.unit.base_unit.name,
                NotificationIcon.War
            )