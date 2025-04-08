import random
from typing import Optional, Sequence, Tuple
from dataclasses import dataclass

@dataclass
class DamageDealt:
    attacker_dealt: int
    defender_dealt: int

    @classmethod
    def none(cls) -> 'DamageDealt':
        return cls(0, 0)

class MapUnitCombatant:
    def __init__(self, unit):
        self.unit = unit

    def get_name(self) -> str:
        return self.unit.name

    def get_civ_info(self):
        return self.unit.civ

    def get_tile(self):
        return self.unit.current_tile

    def is_defeated(self) -> bool:
        return self.unit.health <= 0

    def take_damage(self, damage: int):
        self.unit.health = max(0, self.unit.health - damage)

class AirInterception:
    @staticmethod
    def air_sweep(attacker: MapUnitCombatant, attacked_tile) -> None:
        # Air Sweep counts as an attack, even if nothing else happens
        attacker.unit.attacks_this_turn += 1

        # Use up movement
        if (attacker.unit.has_unique("CanMoveAfterAttacking") or
            attacker.unit.max_attacks_per_turn() > attacker.unit.attacks_this_turn):
            if not attacker.unit.base_unit.moves_like_air_units:
                attacker.unit.use_movement_points(1.0)
        else:
            attacker.unit.current_movement = 0.0

        attacker_name = attacker.get_name()

        # Make sequence of all potential interceptors from all civs at war
        potential_interceptors = []
        for intercepting_civ in attacker.get_civ_info().game_info.civilizations:
            if attacker.get_civ_info().is_at_war_with(intercepting_civ):
                potential_interceptors.extend(
                    unit for unit in intercepting_civ.units.get_civ_units()
                    if unit.can_intercept(attacked_tile)
                )

        # First priority: only air units
        if any(unit.base_unit.is_air_unit() for unit in potential_interceptors):
            potential_interceptors = [
                unit for unit in potential_interceptors
                if unit.base_unit.is_air_unit()
            ]

        # Pick highest chance interceptor
        for interceptor in sorted(
            random.sample(potential_interceptors, len(potential_interceptors)),
            key=lambda x: x.intercept_chance(),
            reverse=True
        ):
            interceptor.attacks_this_turn += 1

            if not interceptor.base_unit.is_air_unit():
                interceptor_name = interceptor.name
                attacker_text = f"Our [{attacker_name}] ([-0] HP) was attacked by an intercepting [{interceptor_name}] ([-0] HP)"
                interceptor_text = f"Our [{interceptor_name}] ([-0] HP) intercepted and attacked an enemy [{attacker_name}] ([-0] HP)"

                attacker.get_civ_info().add_notification(
                    attacker_text,
                    [(interceptor.current_tile.position, attacker.unit.current_tile.position)],
                    "War",
                    attacker_name,
                    "War",
                    interceptor_name
                )

                interceptor.civ.add_notification(
                    interceptor_text,
                    [(interceptor.current_tile.position, attacker.unit.current_tile.position)],
                    "War",
                    interceptor_name,
                    "War",
                    attacker_name
                )

                attacker.unit.action = None
                return

            # Damage if Air v Air should work similar to Melee
            damage_dealt = Battle.take_damage(attacker, MapUnitCombatant(interceptor))

            # 5 XP to both
            Battle.add_xp(MapUnitCombatant(interceptor), 5, attacker)
            Battle.add_xp(attacker, 5, MapUnitCombatant(interceptor))

            locations_interceptor_unknown = [
                (attacked_tile.position, attacker.unit.current_tile.position)
            ]
            locations = [
                (interceptor.current_tile.position, attacker.unit.current_tile.position)
            ]

            AirInterception._add_air_sweep_interception_notifications(
                attacker,
                interceptor,
                damage_dealt,
                locations_interceptor_unknown,
                locations
            )
            attacker.unit.action = None
            return

        # No interceptions available
        attacker_text = f"Nothing tried to intercept our [{attacker_name}]"
        attacker.get_civ_info().add_notification(
            attacker_text,
            "War",
            attacker_name
        )
        attacker.unit.action = None

    @staticmethod
    def try_intercept_air_attack(
        attacker: MapUnitCombatant,
        attacked_tile,
        intercepting_civ,
        defender: Optional[MapUnitCombatant]
    ) -> DamageDealt:
        if attacker.unit.has_unique(
            "CannotBeIntercepted",
            StateForConditionals(
                attacker.get_civ_info(),
                our_combatant=attacker,
                their_combatant=defender,
                attacked_tile=attacked_tile
            )
        ):
            return DamageDealt.none()

        # Pick highest chance interceptor
        interceptors = [
            unit for unit in intercepting_civ.units.get_civ_units()
            if unit.can_intercept(attacked_tile)
        ]

        interceptors.sort(key=lambda x: x.intercept_chance(), reverse=True)

        for unit in interceptors:
            conditional_state = StateForConditionals(
                intercepting_civ,
                our_combatant=MapUnitCombatant(unit),
                their_combatant=attacker,
                combat_action="Intercept",
                attacked_tile=attacked_tile
            )

            if (unit.get_matching_uniques("CannotInterceptUnits", conditional_state)
                and unit != (defender.unit if isinstance(defender, MapUnitCombatant) else None)):
                continue

            interceptor = unit
            break
        else:
            return DamageDealt.none()

        interceptor.attacks_this_turn += 1

        # Does intercept happen?
        if random.random() > interceptor.intercept_chance() / 100.0:
            return DamageDealt.none()

        damage = BattleDamage.calculate_damage_to_defender(
            MapUnitCombatant(interceptor),
            attacker
        )

        damage_factor = 1.0 + interceptor.intercept_damage_percent_bonus() / 100.0
        damage_factor *= attacker.unit.received_intercept_damage_factor()

        damage = min(
            int(damage * damage_factor),
            attacker.unit.health
        )

        attacker.take_damage(damage)
        if damage > 0:
            Battle.add_xp(MapUnitCombatant(interceptor), 2, attacker)

        AirInterception._add_interception_notifications(attacker, interceptor, damage)

        return DamageDealt(0, damage)

    @staticmethod
    def _add_air_sweep_interception_notifications(
        attacker: MapUnitCombatant,
        interceptor,
        damage_dealt: DamageDealt,
        locations_interceptor_unknown: Sequence[Tuple],
        locations: Sequence[Tuple]
    ) -> None:
        attacker_name = attacker.get_name()
        interceptor_name = interceptor.name

        attacker_text = (
            f"Our [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP) was destroyed by an intercepting [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP)"
            if attacker.is_defeated() and interceptor.get_tile() in attacker.get_civ_info().viewable_tiles
            else f"Our [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP) was destroyed by an unknown interceptor"
            if attacker.is_defeated()
            else f"Our [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP) destroyed an intercepting [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP)"
            if MapUnitCombatant(interceptor).is_defeated()
            else f"Our [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP) was attacked by an intercepting [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP)"
        )

        attacker.get_civ_info().add_notification(
            attacker_text,
            locations_interceptor_unknown,
            "War",
            attacker_name,
            "War",
            "Question"
        )

        interceptor_text = (
            f"Our [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP) intercepted and destroyed an enemy [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP)"
            if attacker.is_defeated()
            else f"Our [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP) intercepted and was destroyed by an unknown enemy"
            if MapUnitCombatant(interceptor).is_defeated() and attacker.get_tile() not in interceptor.civ.viewable_tiles
            else f"Our [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP) intercepted and was destroyed by an enemy [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP)"
            if MapUnitCombatant(interceptor).is_defeated()
            else f"Our [{interceptor_name}] ([-{damage_dealt.attacker_dealt}] HP) intercepted and attacked an enemy [{attacker_name}] ([-{damage_dealt.defender_dealt}] HP)"
        )

        interceptor.civ.add_notification(
            interceptor_text,
            locations,
            "War",
            interceptor_name,
            "War",
            attacker_name
        )

    @staticmethod
    def _add_interception_notifications(
        attacker: MapUnitCombatant,
        interceptor,
        damage: int
    ) -> None:
        attacker_name = attacker.get_name()
        interceptor_name = interceptor.name

        locations = [(interceptor.current_tile.position, attacker.unit.current_tile.position)]

        attacker_text = (
            f"Our [{attacker_name}] ([-{damage}] HP) was attacked by an intercepting [{interceptor_name}] ([-0] HP)"
            if not attacker.is_defeated()
            else f"Our [{attacker_name}] ([-{damage}] HP) was destroyed by an intercepting [{interceptor_name}] ([-0] HP)"
            if interceptor.get_tile() in attacker.get_civ_info().viewable_tiles
            else f"Our [{attacker_name}] ([-{damage}] HP) was destroyed by an unknown interceptor"
        )

        attacker.get_civ_info().add_notification(
            attacker_text,
            interceptor.current_tile.position,
            "War",
            attacker_name,
            "War",
            interceptor_name
        )

        interceptor_text = (
            f"Our [{interceptor_name}] ([-0] HP) intercepted and destroyed an enemy [{attacker_name}] ([-{damage}] HP)"
            if attacker.is_defeated()
            else f"Our [{interceptor_name}] ([-0] HP) intercepted and attacked an enemy [{attacker_name}] ([-{damage}] HP)"
        )

        interceptor.civ.add_notification(
            interceptor_text,
            locations,
            "War",
            interceptor_name,
            "War",
            attacker_name
        )