from typing import List, Dict, Optional
from dataclasses import dataclass
import math

from com.unciv.logic.battle import AttackableTile, Battle, BattleDamage, CityCombatant, MapUnitCombatant, TargetHelper
from com.unciv.logic.city import City
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.models.ruleset.unique import UniqueType

class BattleHelper:
    """Handles AI automation for battle-related decisions."""

    @staticmethod
    def try_attack_nearby_enemy(unit: MapUnit, stay_on_tile: bool = False) -> bool:
        """Attempt to attack a nearby enemy unit.
        
        Args:
            unit: The unit attempting to attack
            stay_on_tile: Whether to stay on the current tile
            
        Returns:
            bool: True if the unit cannot further move this turn - NOT if an attack was successful!
        """
        if unit.has_unique(UniqueType.CannotAttack):
            return False
            
        distance_to_tiles = unit.movement.get_distance_to_tiles()
        attackable_enemies = [
            enemy for enemy in TargetHelper.get_attackable_enemies(
                unit, unit.movement.get_distance_to_tiles(), stay_on_tile=stay_on_tile
            )
            if (unit.has_unique(UniqueType.SelfDestructs)
                or BattleDamage.calculate_damage_to_attacker(
                    MapUnitCombatant(unit),
                    Battle.get_map_combatant_of_tile(enemy.tile_to_attack)
                ) + unit.get_damage_from_terrain(enemy.tile_to_attack_from) < unit.health
            )
        ]

        enemy_tile_to_attack = BattleHelper._choose_attack_target(unit, attackable_enemies)

        if enemy_tile_to_attack is not None:
            if (enemy_tile_to_attack.tile_to_attack.military_unit is None 
                and unit.base_unit.is_ranged()
                and unit.movement.can_move_to(enemy_tile_to_attack.tile_to_attack)
                and enemy_tile_to_attack.tile_to_attack in distance_to_tiles):
                # Ranged units should move to capture a civilian unit instead of attacking it
                unit.movement.move_to_tile(enemy_tile_to_attack.tile_to_attack)
            else:
                Battle.move_and_attack(MapUnitCombatant(unit), enemy_tile_to_attack)
                
        return not unit.has_movement()

    @staticmethod
    def try_disembark_unit_to_attack_position(unit: MapUnit) -> bool:
        """Attempt to disembark a unit to an attack position.
        
        Args:
            unit: The unit to disembark
            
        Returns:
            bool: Whether the unit was successfully disembarked
        """
        if not (unit.base_unit.is_melee() and unit.base_unit.is_land_unit and unit.is_embarked()):
            return False
            
        unit_distance_to_tiles = unit.movement.get_distance_to_tiles()

        attackable_enemies_next_turn = [
            enemy for enemy in TargetHelper.get_attackable_enemies(unit, unit_distance_to_tiles)
            if (BattleDamage.calculate_damage_to_attacker(
                MapUnitCombatant(unit),
                Battle.get_map_combatant_of_tile(enemy.tile_to_attack)
            ) < unit.health
            and enemy.tile_to_attack_from.is_land)
        ]

        enemy_tile_to_attack_next_turn = BattleHelper._choose_attack_target(unit, attackable_enemies_next_turn)

        if enemy_tile_to_attack_next_turn is not None:
            unit.movement.move_to_tile(enemy_tile_to_attack_next_turn.tile_to_attack_from)
            return True
            
        return False

    @staticmethod
    def _choose_attack_target(unit: MapUnit, attackable_enemies: List[AttackableTile]) -> Optional[AttackableTile]:
        """Choose the best target from attackable enemies.
        
        Args:
            unit: The unit choosing the target
            attackable_enemies: List of possible targets
            
        Returns:
            Optional[AttackableTile]: The chosen target, if any
        """
        # Get the highest valued attackableEnemy
        highest_attack_value = 0
        attack_tile = None
        
        # We always have to calculate the attack value even if there is only one attackableEnemy
        for attackable_enemy in attackable_enemies:
            temp_attack_value = (
                BattleHelper._get_city_attack_value(unit, attackable_enemy.tile_to_attack.get_city())
                if attackable_enemy.tile_to_attack.is_city_center()
                else BattleHelper._get_unit_attack_value(unit, attackable_enemy)
            )
            if temp_attack_value > highest_attack_value:
                highest_attack_value = temp_attack_value
                attack_tile = attackable_enemy
                
        # Only return that tile if it is actually a good tile to attack
        return attack_tile if highest_attack_value > 30 else None

    @staticmethod
    def _get_city_attack_value(attacker: MapUnit, city: City) -> int:
        """Calculate the value of attacking a city.
        
        Args:
            attacker: The attacking unit
            city: The target city
            
        Returns:
            int: The calculated attack value
        """
        attacker_unit = MapUnitCombatant(attacker)
        city_unit = CityCombatant(city)
        is_city_capturable = (
            city.health == 1
            or (attacker.base_unit.is_melee() 
                and city.health <= max(1, BattleDamage.calculate_damage_to_defender(attacker_unit, city_unit)))
        )
        
        if is_city_capturable:
            return 10000 if attacker.base_unit.is_melee() else 0  # Capture the city immediately!

        if attacker.base_unit.is_melee():
            battle_damage = BattleDamage.calculate_damage_to_attacker(attacker_unit, city_unit)
            if (attacker.health - battle_damage * 2 <= 0 
                and not attacker.has_unique(UniqueType.SelfDestructs)):
                # The more friendly units around the city, the more willing we should be to just attack the city
                friendly_units_around_city = sum(
                    1 for tile in city.get_center_tile().get_tiles_in_distance(3)
                    if tile.military_unit and tile.military_unit.civ == attacker.civ
                )
                # If we have more than 4 other units around the city, go for it
                if friendly_units_around_city < 5:
                    attacker_health_modifier = 1.0 + 1.0 / friendly_units_around_city
                    if attacker.health - battle_damage * attacker_health_modifier <= 0:
                        return 0  # We'll probably die next turn if we attack the city

        attack_value = 100
        # Siege units should really only attack the city
        if attacker.base_unit.is_probably_siege_unit():
            attack_value += 100
        # Ranged units don't take damage from the city
        elif attacker.base_unit.is_ranged():
            attack_value += 10
        # Lower health cities have a higher priority to attack ranges from -20 to 30
        attack_value -= (city.health - 60) / 2

        # Add value based on number of units around the city
        defending_city_civ = city.civ
        for tile in city.get_center_tile().get_tiles_in_distance(2):
            if tile.military_unit:
                if tile.military_unit.civ.is_at_war_with(attacker.civ):
                    attack_value -= 5
                if tile.military_unit.civ.is_at_war_with(defending_city_civ):
                    attack_value += 15

        return attack_value

    @staticmethod
    def _get_unit_attack_value(attacker: MapUnit, attack_tile: AttackableTile) -> int:
        """Calculate the value of attacking a unit.
        
        Args:
            attacker: The attacking unit
            attack_tile: The tile containing the target unit
            
        Returns:
            int: The calculated attack value
        """
        # Base attack value, there is nothing there...
        attack_value = float('-inf')
        
        # Prioritize attacking military
        military_unit = attack_tile.tile_to_attack.military_unit
        civilian_unit = attack_tile.tile_to_attack.civilian_unit
        
        if military_unit is not None:
            attack_value = 100
            # Associate enemy units with number of hits from this unit to kill them
            attacks_to_kill = min(
                max(1, military_unit.health / BattleDamage.calculate_damage_to_defender(
                    MapUnitCombatant(attacker), MapUnitCombatant(military_unit)
                )),
                10
            )
            # We can kill them in this turn
            if attacks_to_kill <= 1:
                attack_value += 30
            # On average, this should take around 3 turns, so -15
            else:
                attack_value -= int(attacks_to_kill * 5)
                
        elif civilian_unit is not None:
            attack_value = 50
            # Only melee units should really attack/capture civilian units, ranged units may be able to capture by moving
            if (attacker.base_unit.is_melee() 
                or attacker.movement.can_reach_in_current_turn(attack_tile.tile_to_attack)):
                if civilian_unit.is_great_person():
                    attack_value += 150
                if civilian_unit.has_unique(UniqueType.FoundCity):
                    attack_value += 60
            elif (attacker.base_unit.is_ranged() 
                  and not civilian_unit.has_unique(UniqueType.Uncapturable)):
                return 10  # Don't shoot civilians that we can capture!

        # Prioritise closer units as they are generally more threatening to this unit
        # Moving around less means we are straying less into enemy territory
        # Average should be around 2.5-5 early game and up to 35 for tanks in late game
        attack_value += int(attack_tile.movement_left_after_moving_to_attack_tile * 5)

        return attack_value 