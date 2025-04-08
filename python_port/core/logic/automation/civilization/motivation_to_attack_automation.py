from typing import List, Tuple, Optional

from com.unciv.logic.battle import BattleDamage
from com.unciv.logic.battle import CityCombatant, MapUnitCombatant
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags, DiplomacyManager, RelationshipLevel
from com.unciv.logic.map import BFS, MapPathing
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset import Building
from com.unciv.models.ruleset.nation import PersonalityValue
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.models.ruleset.unit import BaseUnit
from com.unciv.ui.screens.victoryscreen import RankingType

class MotivationToAttackAutomation:
    """Contains logic for evaluating motivations to attack other civilizations."""

    @staticmethod
    def has_at_least_motivation_to_attack(civ_info: Civilization, target_civ: Civilization, at_least: float) -> float:
        """Calculate the motivation to attack another civilization.
        
        Args:
            civ_info: The civilization considering the attack
            target_civ: The civilization being considered as a target
            at_least: Minimum motivation threshold for short-circuiting
            
        Returns:
            float: The calculated motivation value
        """
        diplomacy_manager = civ_info.get_diplomacy_manager(target_civ)
        personality = civ_info.get_personality()

        target_cities_with_our_city = [
            (our_city, their_city) 
            for our_city, their_city in civ_info.threat_manager.get_neighboring_cities_of_other_civs()
            if their_city.civ == target_civ
        ]
        target_cities = [city for _, city in target_cities_with_our_city]

        if not target_cities_with_our_city:
            return 0.0

        if all(MotivationToAttackAutomation._has_no_units_that_can_attack_city_without_dying(civ_info, city) 
               for city in target_cities):
            return 0.0

        base_force = 100.0

        our_combat_strength = MotivationToAttackAutomation._calculate_self_combat_strength(civ_info, base_force)
        their_combat_strength = MotivationToAttackAutomation._calculate_combat_strength_with_protectors(
            target_civ, base_force, civ_info)

        modifiers: List[Tuple[str, float]] = []

        # If our personality is to declare war more then we should have a higher base motivation
        modifiers.append(("Base motivation", 
                         -15.0 * personality.inverse_modifier_focus(PersonalityValue.DeclareWar, 0.5)))

        modifiers.append(("Relative combat strength", 
                         MotivationToAttackAutomation._get_combat_strength_modifier(
                             civ_info, target_civ, our_combat_strength, 
                             their_combat_strength + 0.8 * civ_info.threat_manager.get_combined_force_of_warring_civs())))

        # TODO: For now this will be a very high value because the AI can't handle multiple fronts
        modifiers.append(("Concurrent wars", 
                         -sum(1 for civ in civ_info.get_civs_at_war_with() 
                             if civ.is_major_civ() and civ != target_civ) * 20.0))
        modifiers.append(("Their concurrent wars", 
                         sum(1 for civ in target_civ.get_civs_at_war_with() 
                             if civ.is_major_civ()) * 3.0))

        modifiers.append(("Their allies", 
                         MotivationToAttackAutomation._get_defensive_pact_allies_score(
                             target_civ, civ_info, base_force, our_combat_strength)))

        if not any(civ != target_civ and civ.is_major_civ()
                  and civ_info.get_diplomacy_manager(civ).is_relationship_level_lt(RelationshipLevel.Friend)
                  for civ in civ_info.threat_manager.get_neighboring_civilizations()):
            modifiers.append(("No other threats", 10.0))

        if target_civ.is_major_civ():
            score_ratio_modifier = MotivationToAttackAutomation._get_score_ratio_modifier(target_civ, civ_info)
            modifiers.append(("Relative score", score_ratio_modifier))

            modifiers.append(("Relative technologies", 
                            MotivationToAttackAutomation._get_relative_tech_modifier(civ_info, target_civ)))

            if civ_info.stats.get_unit_supply_deficit() != 0:
                modifiers.append(("Over unit supply", 
                                min(20.0, civ_info.stats.get_unit_supply_deficit() * 2.0)))
            elif target_civ.stats.get_unit_supply_deficit() == 0 and not target_civ.is_city_state:
                modifiers.append(("Relative production", 
                                MotivationToAttackAutomation._get_production_ratio_modifier(civ_info, target_civ)))

        min_target_city_distance = min(
            their_city.get_center_tile().aerial_distance_to(our_city.get_center_tile())
            for our_city, their_city in target_cities_with_our_city
        )

        # Defensive civs should avoid fighting civilizations that are farther away and don't pose a threat
        distance_modifier = {
            min_target_city_distance > 20: -10.0,
            min_target_city_distance > 14: -8.0,
            min_target_city_distance > 10: -3.0,
            min_target_city_distance <= 10: 0.0
        }[True]
        modifiers.append(("Far away cities", 
                         distance_modifier * personality.inverse_modifier_focus(PersonalityValue.Aggressive, 0.2)))

        # Defensive civs want to deal with potential nearby cities to protect themselves
        if min_target_city_distance < 6:
            modifiers.append(("Close cities", 
                            5.0 * personality.inverse_modifier_focus(PersonalityValue.Aggressive, 1.0)))

        if diplomacy_manager.has_flag(DiplomacyFlags.ResearchAgreement):
            modifiers.append(("Research Agreement", 
                            -5.0 * personality.scaled_focus(PersonalityValue.Science) * 
                            personality.scaled_focus(PersonalityValue.Commerce)))

        if diplomacy_manager.has_flag(DiplomacyFlags.DeclarationOfFriendship):
            modifiers.append(("Declaration of Friendship", 
                            -10.0 * personality.modifier_focus(PersonalityValue.Loyal, 0.5)))

        if diplomacy_manager.has_flag(DiplomacyFlags.DefensivePact):
            modifiers.append(("Defensive Pact", 
                            -15.0 * personality.modifier_focus(PersonalityValue.Loyal, 0.3)))

        modifiers.append(("Relationship", 
                         MotivationToAttackAutomation._get_relationship_modifier(diplomacy_manager)))

        if diplomacy_manager.has_flag(DiplomacyFlags.Denunciation):
            modifiers.append(("Denunciation", 
                            5.0 * personality.inverse_modifier_focus(PersonalityValue.Diplomacy, 0.5)))

        if diplomacy_manager.has_flag(DiplomacyFlags.WaryOf) and diplomacy_manager.get_flag(DiplomacyFlags.WaryOf) < 0:
            # Completely defensive civs will plan defensively and have a 0 here
            modifiers.append(("PlanningAttack", 
                            -diplomacy_manager.get_flag(DiplomacyFlags.WaryOf) * 
                            personality.scaled_focus(PersonalityValue.Aggressive) / 2))
        else:
            attacks_planned = sum(1 for d in civ_info.diplomacy.values() 
                                if d.has_flag(DiplomacyFlags.WaryOf) and d.get_flag(DiplomacyFlags.WaryOf) < 0)
            modifiers.append(("PlanningAttackAgainstOtherCivs", 
                            -attacks_planned * 5.0 * personality.inverse_modifier_focus(PersonalityValue.Aggressive, 0.5)))

        if any(resource.amount > 0 for resource in diplomacy_manager.resources_from_trade()):
            modifiers.append(("Receiving trade resources", 
                            -8.0 * personality.modifier_focus(PersonalityValue.Commerce, 0.5)))

        # If their cities don't have any nearby cities that are also targets to us and it doesn't include their capital
        # Then their cities are likely isolated and a good target
        if (target_civ.get_capital(True) not in target_cities and
            all(not any(city not in target_cities for city in their_city.neighboring_cities)
                for their_city in target_cities)):
            modifiers.append(("Isolated city", 
                            10.0 * personality.modifier_focus(PersonalityValue.Aggressive, 0.8)))

        if target_civ.is_city_state:
            protector_civs = target_civ.city_state_functions.get_protector_civs()
            modifiers.append(("Protectors", -len(protector_civs) * 3.0))
            if civ_info in protector_civs:
                modifiers.append(("Under our protection", 
                                -15.0 * personality.modifier_focus(PersonalityValue.Diplomacy, 0.8)))
            if target_civ.get_ally_civ() == civ_info.civ_name:
                modifiers.append(("Allied City-state", 
                                -20.0 * personality.modifier_focus(PersonalityValue.Diplomacy, 0.8)))

        MotivationToAttackAutomation._add_wonder_based_motivations(target_civ, modifiers)

        modifiers.append(("War with allies", 
                         MotivationToAttackAutomation._get_allied_war_motivation(civ_info, target_civ)))

        # Remove modifiers that don't have an effect
        modifiers = [(name, value) for name, value in modifiers if value != 0.0]
        motivation_so_far = sum(value for _, value in modifiers)

        # Short-circuit to avoid A-star
        if motivation_so_far < at_least:
            return motivation_so_far

        motivation_so_far += MotivationToAttackAutomation._get_attack_paths_modifier(
            civ_info, target_civ, target_cities_with_our_city)

        return motivation_so_far

    @staticmethod
    def _calculate_combat_strength_with_protectors(other_civ: Civilization, base_force: float, civ_info: Civilization) -> float:
        """Calculate combat strength including protector civilizations for city-states."""
        their_combat_strength = MotivationToAttackAutomation._calculate_self_combat_strength(other_civ, base_force)

        # For city-states, also consider their protectors
        if other_civ.is_city_state and other_civ.city_state_functions.get_protector_civs():
            their_combat_strength += sum(
                civ.get_stat_for_ranking(RankingType.Force)
                for civ in other_civ.city_state_functions.get_protector_civs()
                if civ != civ_info
            )
        return their_combat_strength

    @staticmethod
    def _calculate_self_combat_strength(civ_info: Civilization, base_force: float) -> float:
        """Calculate the combat strength of a civilization."""
        our_combat_strength = float(civ_info.get_stat_for_ranking(RankingType.Force)) + base_force
        capital = civ_info.get_capital()
        if capital:
            our_combat_strength += CityCombatant(capital).get_city_strength()
        return our_combat_strength

    @staticmethod
    def _add_wonder_based_motivations(other_civ: Civilization, modifiers: List[Tuple[str, float]]) -> None:
        """Add motivations based on wonders owned by the target civilization."""
        wonder_count = 0
        for city in other_civ.cities:
            construction = city.city_constructions.get_current_construction()
            if isinstance(construction, Building) and construction.has_unique(UniqueType.TriggersCulturalVictory):
                modifiers.append(("About to win", 15.0))
            if isinstance(construction, BaseUnit) and construction.has_unique(UniqueType.AddInCapital):
                modifiers.append(("About to win", 15.0))
            wonder_count += sum(1 for building in city.city_constructions.get_built_buildings() 
                              if building.is_wonder)

        # The more wonders they have, the more beneficial it is to conquer them
        if wonder_count > 0:
            modifiers.append(("Owned Wonders", float(wonder_count)))

    @staticmethod
    def _get_allied_war_motivation(civ_info: Civilization, other_civ: Civilization) -> float:
        """Calculate motivation based on wars with allied civilizations."""
        allied_war_motivation = 0.0
        for third_civ in civ_info.get_diplomacy_manager(other_civ).get_common_known_civs():
            third_civ_diplo_manager = civ_info.get_diplomacy_manager(third_civ)
            if third_civ_diplo_manager.is_relationship_level_lt(RelationshipLevel.Friend):
                continue

            if third_civ.get_diplomacy_manager(other_civ).has_flag(DiplomacyFlags.Denunciation):
                allied_war_motivation += 2.0

            if third_civ.is_at_war_with(other_civ):
                if third_civ_diplo_manager.has_flag(DiplomacyFlags.DefensivePact):
                    allied_war_motivation += 15.0
                elif third_civ_diplo_manager.has_flag(DiplomacyFlags.DeclarationOfFriendship):
                    allied_war_motivation += 5.0
                else:
                    allied_war_motivation += 2.0

        return allied_war_motivation * civ_info.get_personality().modifier_focus(PersonalityValue.Loyal, 0.5)

    @staticmethod
    def _get_relationship_modifier(diplomacy_manager: DiplomacyManager) -> float:
        """Calculate motivation based on diplomatic relationship."""
        relationship_modifier = {
            RelationshipLevel.Unforgivable: 10.0,
            RelationshipLevel.Enemy: 5.0,
            RelationshipLevel.Competitor: 2.0,
            RelationshipLevel.Favorable: -2.0,
            RelationshipLevel.Friend: -5.0,
            RelationshipLevel.Ally: -10.0,
            RelationshipLevel.Neutral: 0.0
        }[diplomacy_manager.relationship_ignore_afraid()]

        return relationship_modifier * diplomacy_manager.civ_info.get_personality().modifier_focus(PersonalityValue.Loyal, 0.3)

    @staticmethod
    def _get_relative_tech_modifier(civ_info: Civilization, other_civ: Civilization) -> float:
        """Calculate motivation based on relative technology levels."""
        relative_tech = (civ_info.get_stat_for_ranking(RankingType.Technologies) - 
                        other_civ.get_stat_for_ranking(RankingType.Technologies))
        
        if relative_tech > 6:
            return 10.0
        elif relative_tech > 3:
            return 5.0
        elif relative_tech > -3:
            return 0.0
        elif relative_tech > -6:
            return -2.0
        elif relative_tech > -9:
            return -5.0
        else:
            return -10.0

    @staticmethod
    def _get_production_ratio_modifier(civ_info: Civilization, other_civ: Civilization) -> float:
        """Calculate motivation based on relative production levels."""
        production_ratio = (civ_info.get_stat_for_ranking(RankingType.Production) / 
                          other_civ.get_stat_for_ranking(RankingType.Production))

        if production_ratio > 2.0:
            return 10.0
        elif production_ratio > 1.5:
            return 5.0
        elif production_ratio > 1.2:
            return 3.0
        elif production_ratio > 0.8:
            return 0.0
        elif production_ratio > 0.5:
            return -5.0
        elif production_ratio > 0.25:
            return -10.0
        else:
            return -15.0

    @staticmethod
    def _get_score_ratio_modifier(other_civ: Civilization, civ_info: Civilization) -> float:
        """Calculate motivation based on relative score."""
        score_ratio = (other_civ.get_stat_for_ranking(RankingType.Score) / 
                      civ_info.get_stat_for_ranking(RankingType.Score))

        if score_ratio > 2.0:
            return 15.0
        elif score_ratio > 1.5:
            return 10.0
        elif score_ratio > 1.25:
            return 5.0
        elif score_ratio > 1.0:
            return 2.0
        elif score_ratio > 0.8:
            return 0.0
        elif score_ratio > 0.5:
            return -2.0
        elif score_ratio > 0.25:
            return -5.0
        else:
            return -10.0 * civ_info.get_personality().modifier_focus(PersonalityValue.Culture, 0.3)

    @staticmethod
    def _get_defensive_pact_allies_score(other_civ: Civilization, civ_info: Civilization, 
                                       base_force: float, our_combat_strength: float) -> float:
        """Calculate motivation based on defensive pacts of the target civilization."""
        their_allies_value = 0.0
        for third_civ in [d.other_civ() for d in other_civ.diplomacy.values() 
                         if d.has_flag(DiplomacyFlags.DefensivePact) and d.other_civ() != civ_info]:
            third_civ_combat_strength_ratio = (other_civ.get_stat_for_ranking(RankingType.Force) + base_force) / our_combat_strength
            
            if third_civ_combat_strength_ratio > 5:
                their_allies_value -= 15.0
            elif third_civ_combat_strength_ratio > 2.5:
                their_allies_value -= 10.0
            elif third_civ_combat_strength_ratio > 2:
                their_allies_value -= 8.0
            elif third_civ_combat_strength_ratio > 1.5:
                their_allies_value -= 5.0
            elif third_civ_combat_strength_ratio > 0.8:
                their_allies_value -= 2.0

        return their_allies_value

    @staticmethod
    def _get_combat_strength_modifier(civ_info: Civilization, target_civ: Civilization, 
                                    our_combat_strength: float, their_combat_strength: float) -> float:
        """Calculate motivation based on relative combat strength."""
        combat_strength_ratio = our_combat_strength / their_combat_strength

        # At higher difficulty levels the AI gets a unit production boost
        if civ_info.is_ai() and target_civ.is_human() and combat_strength_ratio > 1:
            our_combat_modifiers = civ_info.game_info.get_difficulty().ai_unit_cost_modifier
            their_combat_modifiers = civ_info.game_info.get_difficulty().unit_cost_modifier
            combat_strength_ratio *= our_combat_modifiers / their_combat_modifiers

        if combat_strength_ratio > 5.0:
            return 20.0
        elif combat_strength_ratio > 4.0:
            return 15.0
        elif combat_strength_ratio > 3.0:
            return 12.0
        elif combat_strength_ratio > 2.0:
            return 10.0
        elif combat_strength_ratio > 1.8:
            return 8.0
        elif combat_strength_ratio > 1.6:
            return 6.0
        elif combat_strength_ratio > 1.4:
            return 4.0
        elif combat_strength_ratio > 1.2:
            return 2.0
        elif combat_strength_ratio > 0.8:
            return -5.0
        elif combat_strength_ratio > 0.6:
            return -10.0
        elif combat_strength_ratio > 0.4:
            return -20.0
        else:
            return -40.0

    @staticmethod
    def _has_no_units_that_can_attack_city_without_dying(civ_info: Civilization, their_city: City) -> bool:
        """Check if we have any units that can attack a city without dying."""
        return not any(
            BattleDamage.calculate_damage_to_attacker(
                MapUnitCombatant(unit),
                CityCombatant(their_city)
            ) < 100
            for unit in civ_info.units.get_civ_units()
            if unit.is_military()
        )

    @staticmethod
    def _get_attack_paths_modifier(civ_info: Civilization, other_civ: Civilization, 
                                 target_cities_with_our_city: List[Tuple[City, City]]) -> float:
        """Calculate motivation based on available attack paths.
        
        The more routes of attack and shorter the path the higher a motivation will be returned.
        Sea attack routes are less valuable.
        
        Returns:
            float: The motivation ranging from -30 to around +10
        """
        def is_tile_can_move_through(tile: Tile) -> bool:
            owner = tile.get_owner()
            return (not tile.is_impassible() and 
                   (owner == other_civ or owner is None or 
                    civ_info.diplomacy_functions.can_pass_through_tiles(owner)))

        def is_land_tile_can_move_through(tile: Tile) -> bool:
            return tile.is_land and is_tile_can_move_through(tile)

        attack_paths: List[List[Tile]] = []
        attack_path_modifiers = -3.0

        # For each city, we want to calculate if there is an attack path to the enemy
        for city_to_attack_from, cities_to_attack in target_cities_with_our_city:
            city_attack_value = 0.0

            # We only want to calculate the best attack path and use its value
            # Land routes are clearly better than sea routes
            for city_to_attack in cities_to_attack:
                land_attack_path = MapPathing.get_connection(
                    civ_info, city_to_attack_from.get_center_tile(), 
                    city_to_attack.get_center_tile(), is_land_tile_can_move_through
                )
                if land_attack_path and len(land_attack_path) < 16:
                    attack_paths.append(land_attack_path)
                    city_attack_value = 3.0
                    break

                if city_attack_value > 0:
                    continue

                land_and_sea_attack_path = MapPathing.get_connection(
                    civ_info, city_to_attack_from.get_center_tile(), 
                    city_to_attack.get_center_tile(), is_tile_can_move_through
                )
                if land_and_sea_attack_path and len(land_and_sea_attack_path) < 16:
                    attack_paths.append(land_and_sea_attack_path)
                    city_attack_value += 1.0

            attack_path_modifiers += city_attack_value

        if not attack_paths:
            # Do an expensive BFS to find any possible attack path
            capital = civ_info.get_capital(True)
            if not capital:
                return -50.0  # Can't even reach the enemy city, no point in war

            reachable_enemy_cities_bfs = BFS(capital.get_center_tile(), is_tile_can_move_through)
            reachable_enemy_cities_bfs.step_to_end()
            reachable_enemy_cities = [
                city for city in other_civ.cities 
                if reachable_enemy_cities_bfs.has_reached_tile(city.get_center_tile())
            ]
            if not reachable_enemy_cities:
                return -50.0  # Can't even reach the enemy city, no point in war

            min_attack_distance = min(
                len(reachable_enemy_cities_bfs.get_path_to(city.get_center_tile()))
                for city in reachable_enemy_cities
            )

            # Longer attack paths are worse, but if the attack path is too far away we shouldn't completely discard the possibility
            attack_path_modifiers -= max(0, min(30, min_attack_distance - 10))

        return attack_path_modifiers 