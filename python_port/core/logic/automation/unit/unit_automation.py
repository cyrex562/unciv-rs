from typing import Dict, List, Optional, Set, Tuple
from com.unciv import Constants
from com.unciv import UncivGame
from com.unciv.logic.automation import Automation
from com.unciv.logic.battle import Battle, BattleDamage, CityCombatant, ICombatant, MapUnitCombatant, TargetHelper
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization, MapUnitAction, NotificationCategory
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags, DiplomaticStatus
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UpgradeUnitAction
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.models.ruleset.unit import BaseUnit
from com.unciv.ui.screens.worldscreen.unit.actions import UnitActionsPillage, UnitActionsUpgrade
from com.unciv.utils import random_weighted

class UnitAutomation:
    """Handles automation for all unit types."""
    
    CLOSE_ENEMY_TILES_AWAY_LIMIT = 5
    CLOSE_ENEMY_TURNS_AWAY_LIMIT = 3.0
    
    @staticmethod
    def is_good_tile_to_explore(unit: MapUnit, tile: Tile) -> bool:
        """Check if a tile is good for exploration.
        
        Args:
            unit: The unit to check for
            tile: The tile to evaluate
            
        Returns:
            True if the tile is good for exploration
        """
        if not unit.movement.can_move_to(tile):
            return False
            
        if not unit.movement.can_reach(tile):
            return False
            
        if unit.get_damage_from_terrain(tile) > 0:
            return False
            
        if unit.civ.has_explored(tile):
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(tile, 6, False) < 4:
            return False
            
        return True
                
    @staticmethod
    def try_explore(unit: MapUnit) -> bool:
        """Try to explore with the unit.
        
        Args:
            unit: The unit to try to explore with
            
        Returns:
            True if unit explored
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        unexplored_tiles = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if not unit.civ.has_explored(tile)
            and unit.movement.can_reach(tile)
        ]
        
        if not unexplored_tiles:
            return False
            
        closest_unexplored = min(
            unexplored_tiles,
            key=lambda t: t.aerial_distance_to(unit.get_tile())
        )
        
        unit.movement.head_towards(closest_unexplored)
        return True
        
    @staticmethod
    def try_go_to_ruin_and_encampment(unit: MapUnit) -> bool:
        """Try to go to a ruin or encampment.
        
        Args:
            unit: The unit to try to go to a ruin or encampment with
            
        Returns:
            True if unit went to a ruin or encampment
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        ruins_and_encampments = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if ((tile.improvement == Constants.ancient_ruins
                 or tile.improvement == Constants.barbarian_encampment)
                and unit.civ.has_explored(tile)
                and unit.movement.can_reach(tile))
        ]
        
        if not ruins_and_encampments:
            return False
            
        closest = min(
            ruins_and_encampments,
            key=lambda t: t.aerial_distance_to(unit.get_tile())
        )
        
        unit.movement.head_towards(closest)
        return True
        
    @staticmethod
    def try_fog_bust(unit: MapUnit) -> bool:
        """Try to fog bust with the unit.
        
        Args:
            unit: The unit to try to fog bust with
            
        Returns:
            True if unit fog busted
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        fog_tiles = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if not unit.civ.has_explored(tile)
            and unit.movement.can_reach(tile)
        ]
        
        if not fog_tiles:
            return False
            
        closest_fog = min(
            fog_tiles,
            key=lambda t: t.aerial_distance_to(unit.get_tile())
        )
        
        unit.movement.head_towards(closest_fog)
        return True
        
    @staticmethod
    def wander(unit: MapUnit, stay_in_territory: bool = False, avoid_tiles: Optional[Set[Tile]] = None) -> None:
        """Make the unit wander around.
        
        Args:
            unit: The unit to make wander
            stay_in_territory: Whether to stay in own territory
            avoid_tiles: Tiles to avoid
        """
        if unit.base_unit.is_air_unit():
            return
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return
            
        if unit.health < 50:
            return
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return
            
        available_tiles = [
            tile for tile in unit.current_tile.neighbors
            if unit.movement.can_move_to(tile)
            and (not stay_in_territory or tile.owner == unit.civ)
            and (not avoid_tiles or tile not in avoid_tiles)
        ]
        
        if not available_tiles:
            return
            
        unit.movement.move_to_tile(random.choice(available_tiles))
        
    @staticmethod
    def try_upgrade_unit(unit: MapUnit) -> bool:
        """Try to upgrade the unit.
        
        Args:
            unit: The unit to try to upgrade
            
        Returns:
            True if unit was upgraded
        """
        if (unit.civ.is_human() 
            and not UncivGame.Current.settings.automated_units_can_upgrade
            and not UncivGame.Current.world_screen.auto_play.is_auto_playing_and_full_auto_play_ai()):
            return False
            
        upgrade_units = list(UnitAutomation.get_units_to_upgrade_to(unit))
        if not upgrade_units:
            return False  # for resource reasons, usually
            
        upgraded_unit = min(upgrade_units, key=lambda u: u.cost)
        
        if any(not unit.requires_resource(r) 
               for r in upgraded_unit.get_resource_requirements_per_turn(unit.cache.state)):
            # The upgrade requires new resource types, so check if we are willing to invest them
            if not Automation.allow_spending_resource(unit.civ, upgraded_unit):
                return False
                
        upgrade_actions = UnitActionsUpgrade.get_upgrade_actions(unit)
        
        upgrade_action = next(
            (action for action in upgrade_actions
             if isinstance(action, UpgradeUnitAction) 
             and action.unit_to_upgrade_to == upgraded_unit),
            None
        )
        
        if not upgrade_action or not upgrade_action.action:
            return False
            
        upgrade_action.action()
        return unit.is_destroyed  # a successful upgrade action will destroy this unit
        
    @staticmethod
    def get_units_to_upgrade_to(unit: MapUnit) -> List[BaseUnit]:
        """Get the base units this map unit could upgrade to.
        
        Args:
            unit: The unit to check upgrades for
            
        Returns:
            List of possible upgrade units
        """
        def is_invalid_upgrade_destination(base_unit: BaseUnit) -> bool:
            if not unit.civ.tech.is_researched(base_unit):
                return True
            if unit.civ.is_barbarian and base_unit.has_unique(UniqueType.CannotBeBarbarian):
                return True
            return any(not u.conditionals_apply(unit.cache.state)
                      for u in base_unit.get_matching_uniques(UniqueType.OnlyAvailable, StateForConditionals.IgnoreConditionals))
                      
        return [
            unit.civ.get_equivalent_unit(u)
            for u in unit.base_unit.get_ruleset_upgrade_units(unit.cache.state)
            if not is_invalid_upgrade_destination(unit.civ.get_equivalent_unit(u))
            and unit.upgrade.can_upgrade(unit.civ.get_equivalent_unit(u))
        ] 

    @staticmethod
    def automate_unit_moves(unit: MapUnit) -> None:
        """Automate unit movement and actions.
        
        Args:
            unit: The unit to automate
        """
        if unit.civ.is_barbarian:
            raise ValueError("Barbarians is not allowed here.")
            
        # Might die next turn - move!
        if unit.health <= unit.get_damage_from_terrain() and UnitAutomation.try_heal_unit(unit):
            return
            
        if unit.is_civilian():
            CivilianUnitAutomation.automate_civilian_unit(unit, UnitAutomation.get_dangerous_tiles(unit))
            return
            
        while (unit.promotions.can_be_promoted()
               and (UncivGame.Current.settings.automated_units_choose_promotions or unit.civ.is_ai())):
            available_promotions = unit.promotions.get_available_promotions()
            if (unit.health < 60 
                and not (unit.base_unit.is_air_unit() or unit.base_unit.has_unique(UniqueType.CanMoveAfterAttacking))
                and any(p.has_unique(UniqueType.OneTimeUnitHeal) for p in available_promotions)):
                available_promotions = [p for p in available_promotions if p.has_unique(UniqueType.OneTimeUnitHeal)]
            else:
                available_promotions = [p for p in available_promotions if not p.has_unique(UniqueType.SkipPromotion)]
                
            if not available_promotions:
                break
                
            free_promotions = [p for p in available_promotions if p.has_unique(UniqueType.FreePromotion)]
            state_for_conditionals = unit.cache.state
            
            if free_promotions:
                chosen_promotion = random_weighted(
                    free_promotions,
                    lambda p: p.get_weight_for_ai_decision(state_for_conditionals)
                )
            else:
                chosen_promotion = random_weighted(
                    available_promotions,
                    lambda p: p.get_weight_for_ai_decision(state_for_conditionals)
                )
                
            unit.promotions.add_promotion(chosen_promotion.name)
            
        # Handle units with civilian abilities in peace time
        if ((unit.has_unique(UniqueType.BuildImprovements) 
             or unit.has_unique(UniqueType.FoundCity)
             or unit.has_unique(UniqueType.ReligiousUnit)
             or unit.has_unique(UniqueType.CreateWaterImprovements))
            and not unit.civ.is_at_war()):
            CivilianUnitAutomation.automate_civilian_unit(unit, UnitAutomation.get_dangerous_tiles(unit))
            return
            
        # Handle nuclear weapons
        if unit.is_nuclear_weapon():
            return AirUnitAutomation.automate_nukes(unit)
            
        # Handle air units
        if unit.base_unit.is_air_unit():
            if unit.can_intercept():
                return AirUnitAutomation.automate_fighter(unit)
            if unit.has_unique(UniqueType.SelfDestructs):
                return AirUnitAutomation.automate_missile(unit)
            return AirUnitAutomation.automate_bomber(unit)
            
        # Accompany settlers
        if UnitAutomation.try_accompany_settler_or_great_person(unit):
            return
            
        if UnitAutomation.try_go_to_ruin_and_encampment(unit) and not unit.has_movement():
            return
            
        if unit.health < 50 and (UnitAutomation.try_retreat(unit) or UnitAutomation.try_heal_unit(unit)):
            return  # do nothing but heal
            
        # If there are no enemies nearby and we can heal here, wait until we are at full health
        if unit.health < 100 and UnitAutomation.can_unit_heal_in_turns_on_current_tile(unit, 2, 4):
            return
            
        if UnitAutomation.try_head_towards_our_sieged_city(unit):
            return
            
        # If an embarked melee unit can land and attack next turn, do not attack from water
        if BattleHelper.try_disembark_unit_to_attack_position(unit):
            return
            
        # If there is an attackable unit in the vicinity, attack!
        if UnitAutomation.try_attacking(unit):
            return
            
        if UnitAutomation.try_take_back_captured_city(unit):
            return
            
        # Focus all units without a specific target on the enemy city closest to one of our cities
        if HeadTowardsEnemyCityAutomation.try_head_towards_enemy_city(unit):
            return
            
        if UnitAutomation.try_garrisoning_ranged_land_unit(unit):
            return
            
        if UnitAutomation.try_stationing_melee_naval_unit(unit):
            return
            
        if unit.health < 80 and UnitAutomation.try_heal_unit(unit):
            return
            
        # Move towards the closest reasonably attackable enemy unit within 3 turns of movement
        if UnitAutomation.try_advance_towards_close_enemy(unit):
            return
            
        if UnitAutomation.try_head_towards_encampment(unit):
            return
            
        if unit.health < 100 and UnitAutomation.try_heal_unit(unit):
            return
            
        if UnitAutomation.try_prepare(unit):
            return
            
        # Try to go to unreached tiles
        if UnitAutomation.try_explore(unit):
            return
            
        if UnitAutomation.try_fog_bust(unit):
            return
            
        # Idle CS units should wander so they don't obstruct players so much
        if unit.civ.is_city_state:
            UnitAutomation.wander(unit, stay_in_territory=True)
            
    @staticmethod
    def try_attacking(unit: MapUnit) -> bool:
        """Try to attack with the unit.
        
        Args:
            unit: The unit to try attacking with
            
        Returns:
            True if unit has 0 movement left
        """
        for _ in range(unit.max_attacks_per_turn() - unit.attacks_this_turn):
            if BattleHelper.try_attack_nearby_enemy(unit):
                return True
            # Cavalry style tactic, attack and then retreat
            if unit.health < 50 and UnitAutomation.try_retreat(unit):
                return True
        return False
        
    @staticmethod
    def try_head_towards_encampment(unit: MapUnit) -> bool:
        """Try to move unit towards an encampment.
        
        Args:
            unit: The unit to move
            
        Returns:
            True if unit was moved towards an encampment
        """
        if unit.has_unique(UniqueType.SelfDestructs):
            return False  # don't use single-use units against barbarians
            
        known_encampments = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if tile.improvement == Constants.barbarian_encampment 
            and unit.civ.has_explored(tile)
        ]
        
        cities = unit.civ.cities
        encampments_close_to_cities = [
            encampment for encampment in known_encampments
            if any(city.get_center_tile().aerial_distance_to(encampment) < 6 
                  for city in cities)
        ]
        
        encampment_to_head_towards = next(
            (encampment for encampment in sorted(
                encampments_close_to_cities,
                key=lambda e: e.aerial_distance_to(unit.current_tile)
            ) if unit.movement.can_reach(encampment)),
            None
        )
        
        if encampment_to_head_towards:
            unit.movement.head_towards(encampment_to_head_towards)
            return True
            
        return False
        
    @staticmethod
    def try_retreat(unit: MapUnit) -> bool:
        """Try to retreat the unit.
        
        Args:
            unit: The unit to try to retreat
            
        Returns:
            True if unit retreated
        """
        # Precondition: This must be a military unit
        if unit.is_civilian():
            return False
        if unit.base_unit.is_air_unit():
            return False
        # Better to do a more healing oriented move then
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, True) > 4:
            return False
            
        unit_distance_to_tiles = unit.movement.get_distance_to_tiles()
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if c.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        # Finding the distance to the closest enemy is expensive, so sort tiles using a cheaper function
        sorted_tiles_to_retreat_to = sorted(
            unit_distance_to_tiles.keys(),
            key=lambda t: (t.aerial_distance_to(closest_city.get_center_tile()) 
                         if closest_city 
                         else -unit.civ.threat_manager.get_distance_to_closest_enemy_unit(t, 3, False))
        )
        
        our_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False)
        
        # Check all tiles and swap with the first one
        for retreat_tile in sorted_tiles_to_retreat_to:
            tile_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(retreat_tile, 6, False)
            if our_distance_to_closest_enemy >= tile_distance_to_closest_enemy:
                continue
                
            other_unit = retreat_tile.military_unit
            if other_unit is None:
                # See if we can retreat to the tile
                if not unit.movement.can_move_to(retreat_tile):
                    continue
                unit.movement.move_to_tile(retreat_tile)
                return True
            elif other_unit.civ == unit.civ:
                # The tile is taken, see if we want to swap retreat to it
                if other_unit.health <= 80:
                    continue
                if other_unit.base_unit.is_ranged():
                    # Don't swap ranged units closer than they have to be
                    range_ = other_unit.base_unit.range
                    if our_distance_to_closest_enemy < range_:
                        continue
                if unit.movement.can_unit_swap_to(retreat_tile):
                    unit.movement.head_towards(retreat_tile)  # we need to move through the intermediate tiles
                    # if nothing changed
                    if (unit.current_tile.neighbors.contains(other_unit.current_tile) 
                        and unit.movement.can_unit_swap_to(retreat_tile)):
                        unit.movement.swap_move_to_tile(retreat_tile)
                    return True
                    
        return False 

    @staticmethod
    def try_heal_unit(unit: MapUnit) -> bool:
        """Try to heal the unit.
        
        Args:
            unit: The unit to try to heal
            
        Returns:
            True if unit was healed
        """
        if unit.health >= 100:
            return False
            
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.has_unique(UniqueType.OneTimeUnitHeal):
            unit.promotions.add_promotion(Constants.healPromotion)
            return True
            
        if unit.has_unique(UniqueType.HealOutsideFriendlyTerritory):
            return True
            
        if unit.current_tile.is_city_center():
            return True
            
        if unit.current_tile.owner == unit.civ:
            return True
            
        return False
        
    @staticmethod
    def try_prepare(unit: MapUnit) -> bool:
        """Try to prepare the unit for combat.
        
        Args:
            unit: The unit to try to prepare
            
        Returns:
            True if unit was prepared
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.has_unique(UniqueType.OneTimeUnitHeal):
            return False
            
        if unit.has_unique(UniqueType.HealOutsideFriendlyTerritory):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner == unit.civ:
            return False
            
        if unit.health < 100:
            return False
            
        if unit.civ.is_at_war():
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        return True
        
    @staticmethod
    def try_garrisoning_ranged_land_unit(unit: MapUnit) -> bool:
        """Try to garrison a ranged land unit.
        
        Args:
            unit: The unit to try to garrison
            
        Returns:
            True if unit was garrisoned
        """
        if not unit.base_unit.is_ranged():
            return False
            
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner != unit.civ:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_city:
            unit.movement.head_towards(closest_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_stationing_melee_naval_unit(unit: MapUnit) -> bool:
        """Try to station a melee naval unit.
        
        Args:
            unit: The unit to try to station
            
        Returns:
            True if unit was stationed
        """
        if not unit.base_unit.is_melee():
            return False
            
        if not unit.base_unit.is_water_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner != unit.civ:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_city:
            unit.movement.head_towards(closest_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_advance_towards_close_enemy(unit: MapUnit) -> bool:
        """Try to advance towards a close enemy.
        
        Args:
            unit: The unit to try to advance
            
        Returns:
            True if unit advanced
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_enemy = next(
            (enemy for enemy in unit.civ.game_info.civilizations
             if enemy != unit.civ and enemy.is_at_war_with(unit.civ)),
            None
        )
        
        if not closest_enemy:
            return False
            
        closest_enemy_city = next(
            (city for city in sorted(
                closest_enemy.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_enemy_city:
            unit.movement.head_towards(closest_enemy_city.get_center_tile())
            return True
            
        return False 

    @staticmethod
    def try_accompany_settler_or_great_person(unit: MapUnit) -> bool:
        """Try to accompany a settler or great person.
        
        Args:
            unit: The unit to try to accompany with
            
        Returns:
            True if unit accompanied
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        settler_or_great_person = next(
            (other for other in unit.civ.units.get_civ_units()
             if other != unit
             and (other.base_unit.has_unique(UniqueType.FoundCity)
                  or other.base_unit.has_unique(UniqueType.GreatPerson))
             and other.movement.can_reach(unit.get_tile())),
            None
        )
        
        if settler_or_great_person:
            unit.movement.head_towards(settler_or_great_person.get_tile())
            return True
            
        return False
        
    @staticmethod
    def try_head_towards_our_sieged_city(unit: MapUnit) -> bool:
        """Try to head towards a sieged city.
        
        Args:
            unit: The unit to try to head towards a sieged city with
            
        Returns:
            True if unit headed towards a sieged city
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        sieged_city = next(
            (city for city in unit.civ.cities
             if city.is_under_siege()
             and unit.movement.can_reach(city.get_center_tile())),
            None
        )
        
        if sieged_city:
            unit.movement.head_towards(sieged_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_take_back_captured_city(unit: MapUnit) -> bool:
        """Try to take back a captured city.
        
        Args:
            unit: The unit to try to take back a captured city with
            
        Returns:
            True if unit headed towards a captured city
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        captured_city = next(
            (city for city in unit.civ.cities
             if city.is_captured()
             and unit.movement.can_reach(city.get_center_tile())),
            None
        )
        
        if captured_city:
            unit.movement.head_towards(captured_city.get_center_tile())
            return True
            
        return False 

    @staticmethod
    def get_dangerous_tiles(unit: MapUnit) -> Set[Tile]:
        """Get tiles that are dangerous for the unit.
        
        Args:
            unit: The unit to get dangerous tiles for
            
        Returns:
            Set of dangerous tiles
        """
        dangerous_tiles = set()
        
        for enemy_civ in unit.civ.game_info.civilizations:
            if enemy_civ == unit.civ or not enemy_civ.is_at_war_with(unit.civ):
                continue
                
            for enemy_unit in enemy_civ.units.get_civ_units():
                if enemy_unit.base_unit.is_air_unit():
                    continue
                    
                if enemy_unit.has_unique(UniqueType.SelfDestructs):
                    continue
                    
                if enemy_unit.health < 50:
                    continue
                    
                dangerous_tiles.add(enemy_unit.get_tile())
                dangerous_tiles.update(enemy_unit.get_tile().neighbors)
                
        return dangerous_tiles
        
    @staticmethod
    def can_unit_heal_in_turns_on_current_tile(unit: MapUnit, min_turns: int, max_turns: int) -> bool:
        """Check if unit can heal in a certain number of turns on its current tile.
        
        Args:
            unit: The unit to check
            min_turns: Minimum number of turns to heal
            max_turns: Maximum number of turns to heal
            
        Returns:
            True if unit can heal in the specified number of turns
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health >= 100:
            return False
            
        if unit.get_damage_from_terrain() > 0:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        healing_per_turn = unit.get_healing_per_turn()
        if healing_per_turn <= 0:
            return False
            
        turns_to_heal = (100 - unit.health) / healing_per_turn
        return min_turns <= turns_to_heal <= max_turns
        
    @staticmethod
    def try_go_to_ruin_and_encampment(unit: MapUnit) -> bool:
        """Try to go to a ruin or encampment.
        
        Args:
            unit: The unit to try to go to a ruin or encampment with
            
        Returns:
            True if unit went to a ruin or encampment
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        ruins_and_encampments = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if ((tile.improvement == Constants.ancient_ruins
                 or tile.improvement == Constants.barbarian_encampment)
                and unit.civ.has_explored(tile)
                and unit.movement.can_reach(tile))
        ]
        
        if not ruins_and_encampments:
            return False
            
        closest = min(
            ruins_and_encampments,
            key=lambda t: t.aerial_distance_to(unit.get_tile())
        )
        
        unit.movement.head_towards(closest)
        return True
        
    @staticmethod
    def try_fog_bust(unit: MapUnit) -> bool:
        """Try to fog bust with the unit.
        
        Args:
            unit: The unit to try to fog bust with
            
        Returns:
            True if unit fog busted
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        fog_tiles = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if not unit.civ.has_explored(tile)
            and unit.movement.can_reach(tile)
        ]
        
        if not fog_tiles:
            return False
            
        closest_fog = min(
            fog_tiles,
            key=lambda t: t.aerial_distance_to(unit.get_tile())
        )
        
        unit.movement.head_towards(closest_fog)
        return True
        
    @staticmethod
    def wander(unit: MapUnit, stay_in_territory: bool = False, avoid_tiles: Optional[Set[Tile]] = None) -> None:
        """Make the unit wander around.
        
        Args:
            unit: The unit to make wander
            stay_in_territory: Whether to stay in own territory
            avoid_tiles: Tiles to avoid
        """
        if unit.base_unit.is_air_unit():
            return
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return
            
        if unit.health < 50:
            return
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return
            
        available_tiles = [
            tile for tile in unit.current_tile.neighbors
            if unit.movement.can_move_to(tile)
            and (not stay_in_territory or tile.owner == unit.civ)
            and (not avoid_tiles or tile not in avoid_tiles)
        ]
        
        if not available_tiles:
            return
            
        unit.movement.move_to_tile(random.choice(available_tiles))
        
    @staticmethod
    def try_upgrade_unit(unit: MapUnit) -> bool:
        """Try to upgrade the unit.
        
        Args:
            unit: The unit to try to upgrade
            
        Returns:
            True if unit was upgraded
        """
        if (unit.civ.is_human() 
            and not UncivGame.Current.settings.automated_units_can_upgrade
            and not UncivGame.Current.world_screen.auto_play.is_auto_playing_and_full_auto_play_ai()):
            return False
            
        upgrade_units = list(UnitAutomation.get_units_to_upgrade_to(unit))
        if not upgrade_units:
            return False  # for resource reasons, usually
            
        upgraded_unit = min(upgrade_units, key=lambda u: u.cost)
        
        if any(not unit.requires_resource(r) 
               for r in upgraded_unit.get_resource_requirements_per_turn(unit.cache.state)):
            # The upgrade requires new resource types, so check if we are willing to invest them
            if not Automation.allow_spending_resource(unit.civ, upgraded_unit):
                return False
                
        upgrade_actions = UnitActionsUpgrade.get_upgrade_actions(unit)
        
        upgrade_action = next(
            (action for action in upgrade_actions
             if isinstance(action, UpgradeUnitAction) 
             and action.unit_to_upgrade_to == upgraded_unit),
            None
        )
        
        if not upgrade_action or not upgrade_action.action:
            return False
            
        upgrade_action.action()
        return unit.is_destroyed  # a successful upgrade action will destroy this unit
        
    @staticmethod
    def get_units_to_upgrade_to(unit: MapUnit) -> List[BaseUnit]:
        """Get the base units this map unit could upgrade to.
        
        Args:
            unit: The unit to check upgrades for
            
        Returns:
            List of possible upgrade units
        """
        def is_invalid_upgrade_destination(base_unit: BaseUnit) -> bool:
            if not unit.civ.tech.is_researched(base_unit):
                return True
            if unit.civ.is_barbarian and base_unit.has_unique(UniqueType.CannotBeBarbarian):
                return True
            return any(not u.conditionals_apply(unit.cache.state)
                      for u in base_unit.get_matching_uniques(UniqueType.OnlyAvailable, StateForConditionals.IgnoreConditionals))
                      
        return [
            unit.civ.get_equivalent_unit(u)
            for u in unit.base_unit.get_ruleset_upgrade_units(unit.cache.state)
            if not is_invalid_upgrade_destination(unit.civ.get_equivalent_unit(u))
            and unit.upgrade.can_upgrade(unit.civ.get_equivalent_unit(u))
        ] 

    @staticmethod
    def automate_unit_moves(unit: MapUnit) -> None:
        """Automate unit movement and actions.
        
        Args:
            unit: The unit to automate
        """
        if unit.civ.is_barbarian:
            raise ValueError("Barbarians is not allowed here.")
            
        # Might die next turn - move!
        if unit.health <= unit.get_damage_from_terrain() and UnitAutomation.try_heal_unit(unit):
            return
            
        if unit.is_civilian():
            CivilianUnitAutomation.automate_civilian_unit(unit, UnitAutomation.get_dangerous_tiles(unit))
            return
            
        while (unit.promotions.can_be_promoted()
               and (UncivGame.Current.settings.automated_units_choose_promotions or unit.civ.is_ai())):
            available_promotions = unit.promotions.get_available_promotions()
            if (unit.health < 60 
                and not (unit.base_unit.is_air_unit() or unit.base_unit.has_unique(UniqueType.CanMoveAfterAttacking))
                and any(p.has_unique(UniqueType.OneTimeUnitHeal) for p in available_promotions)):
                available_promotions = [p for p in available_promotions if p.has_unique(UniqueType.OneTimeUnitHeal)]
            else:
                available_promotions = [p for p in available_promotions if not p.has_unique(UniqueType.SkipPromotion)]
                
            if not available_promotions:
                break
                
            free_promotions = [p for p in available_promotions if p.has_unique(UniqueType.FreePromotion)]
            state_for_conditionals = unit.cache.state
            
            if free_promotions:
                chosen_promotion = random_weighted(
                    free_promotions,
                    lambda p: p.get_weight_for_ai_decision(state_for_conditionals)
                )
            else:
                chosen_promotion = random_weighted(
                    available_promotions,
                    lambda p: p.get_weight_for_ai_decision(state_for_conditionals)
                )
                
            unit.promotions.add_promotion(chosen_promotion.name)
            
        # Handle units with civilian abilities in peace time
        if ((unit.has_unique(UniqueType.BuildImprovements) 
             or unit.has_unique(UniqueType.FoundCity)
             or unit.has_unique(UniqueType.ReligiousUnit)
             or unit.has_unique(UniqueType.CreateWaterImprovements))
            and not unit.civ.is_at_war()):
            CivilianUnitAutomation.automate_civilian_unit(unit, UnitAutomation.get_dangerous_tiles(unit))
            return
            
        # Handle nuclear weapons
        if unit.is_nuclear_weapon():
            return AirUnitAutomation.automate_nukes(unit)
            
        # Handle air units
        if unit.base_unit.is_air_unit():
            if unit.can_intercept():
                return AirUnitAutomation.automate_fighter(unit)
            if unit.has_unique(UniqueType.SelfDestructs):
                return AirUnitAutomation.automate_missile(unit)
            return AirUnitAutomation.automate_bomber(unit)
            
        # Accompany settlers
        if UnitAutomation.try_accompany_settler_or_great_person(unit):
            return
            
        if UnitAutomation.try_go_to_ruin_and_encampment(unit) and not unit.has_movement():
            return
            
        if unit.health < 50 and (UnitAutomation.try_retreat(unit) or UnitAutomation.try_heal_unit(unit)):
            return  # do nothing but heal
            
        # If there are no enemies nearby and we can heal here, wait until we are at full health
        if unit.health < 100 and UnitAutomation.can_unit_heal_in_turns_on_current_tile(unit, 2, 4):
            return
            
        if UnitAutomation.try_head_towards_our_sieged_city(unit):
            return
            
        # If an embarked melee unit can land and attack next turn, do not attack from water
        if BattleHelper.try_disembark_unit_to_attack_position(unit):
            return
            
        # If there is an attackable unit in the vicinity, attack!
        if UnitAutomation.try_attacking(unit):
            return
            
        if UnitAutomation.try_take_back_captured_city(unit):
            return
            
        # Focus all units without a specific target on the enemy city closest to one of our cities
        if HeadTowardsEnemyCityAutomation.try_head_towards_enemy_city(unit):
            return
            
        if UnitAutomation.try_garrisoning_ranged_land_unit(unit):
            return
            
        if UnitAutomation.try_stationing_melee_naval_unit(unit):
            return
            
        if unit.health < 80 and UnitAutomation.try_heal_unit(unit):
            return
            
        # Move towards the closest reasonably attackable enemy unit within 3 turns of movement
        if UnitAutomation.try_advance_towards_close_enemy(unit):
            return
            
        if UnitAutomation.try_head_towards_encampment(unit):
            return
            
        if unit.health < 100 and UnitAutomation.try_heal_unit(unit):
            return
            
        if UnitAutomation.try_prepare(unit):
            return
            
        # Try to go to unreached tiles
        if UnitAutomation.try_explore(unit):
            return
            
        if UnitAutomation.try_fog_bust(unit):
            return
            
        # Idle CS units should wander so they don't obstruct players so much
        if unit.civ.is_city_state:
            UnitAutomation.wander(unit, stay_in_territory=True)
            
    @staticmethod
    def try_attacking(unit: MapUnit) -> bool:
        """Try to attack with the unit.
        
        Args:
            unit: The unit to try attacking with
            
        Returns:
            True if unit has 0 movement left
        """
        for _ in range(unit.max_attacks_per_turn() - unit.attacks_this_turn):
            if BattleHelper.try_attack_nearby_enemy(unit):
                return True
            # Cavalry style tactic, attack and then retreat
            if unit.health < 50 and UnitAutomation.try_retreat(unit):
                return True
        return False
        
    @staticmethod
    def try_head_towards_encampment(unit: MapUnit) -> bool:
        """Try to move unit towards an encampment.
        
        Args:
            unit: The unit to move
            
        Returns:
            True if unit was moved towards an encampment
        """
        if unit.has_unique(UniqueType.SelfDestructs):
            return False  # don't use single-use units against barbarians
            
        known_encampments = [
            tile for tile in unit.civ.game_info.tile_map.values()
            if tile.improvement == Constants.barbarian_encampment 
            and unit.civ.has_explored(tile)
        ]
        
        cities = unit.civ.cities
        encampments_close_to_cities = [
            encampment for encampment in known_encampments
            if any(city.get_center_tile().aerial_distance_to(encampment) < 6 
                  for city in cities)
        ]
        
        encampment_to_head_towards = next(
            (encampment for encampment in sorted(
                encampments_close_to_cities,
                key=lambda e: e.aerial_distance_to(unit.current_tile)
            ) if unit.movement.can_reach(encampment)),
            None
        )
        
        if encampment_to_head_towards:
            unit.movement.head_towards(encampment_to_head_towards)
            return True
            
        return False
        
    @staticmethod
    def try_retreat(unit: MapUnit) -> bool:
        """Try to retreat the unit.
        
        Args:
            unit: The unit to try to retreat
            
        Returns:
            True if unit retreated
        """
        # Precondition: This must be a military unit
        if unit.is_civilian():
            return False
        if unit.base_unit.is_air_unit():
            return False
        # Better to do a more healing oriented move then
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, True) > 4:
            return False
            
        unit_distance_to_tiles = unit.movement.get_distance_to_tiles()
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if c.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        # Finding the distance to the closest enemy is expensive, so sort tiles using a cheaper function
        sorted_tiles_to_retreat_to = sorted(
            unit_distance_to_tiles.keys(),
            key=lambda t: (t.aerial_distance_to(closest_city.get_center_tile()) 
                         if closest_city 
                         else -unit.civ.threat_manager.get_distance_to_closest_enemy_unit(t, 3, False))
        )
        
        our_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False)
        
        # Check all tiles and swap with the first one
        for retreat_tile in sorted_tiles_to_retreat_to:
            tile_distance_to_closest_enemy = unit.civ.threat_manager.get_distance_to_closest_enemy_unit(retreat_tile, 6, False)
            if our_distance_to_closest_enemy >= tile_distance_to_closest_enemy:
                continue
                
            other_unit = retreat_tile.military_unit
            if other_unit is None:
                # See if we can retreat to the tile
                if not unit.movement.can_move_to(retreat_tile):
                    continue
                unit.movement.move_to_tile(retreat_tile)
                return True
            elif other_unit.civ == unit.civ:
                # The tile is taken, see if we want to swap retreat to it
                if other_unit.health <= 80:
                    continue
                if other_unit.base_unit.is_ranged():
                    # Don't swap ranged units closer than they have to be
                    range_ = other_unit.base_unit.range
                    if our_distance_to_closest_enemy < range_:
                        continue
                if unit.movement.can_unit_swap_to(retreat_tile):
                    unit.movement.head_towards(retreat_tile)  # we need to move through the intermediate tiles
                    # if nothing changed
                    if (unit.current_tile.neighbors.contains(other_unit.current_tile) 
                        and unit.movement.can_unit_swap_to(retreat_tile)):
                        unit.movement.swap_move_to_tile(retreat_tile)
                    return True
                    
        return False 

    @staticmethod
    def try_heal_unit(unit: MapUnit) -> bool:
        """Try to heal the unit.
        
        Args:
            unit: The unit to try to heal
            
        Returns:
            True if unit was healed
        """
        if unit.health >= 100:
            return False
            
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.has_unique(UniqueType.OneTimeUnitHeal):
            unit.promotions.add_promotion(Constants.healPromotion)
            return True
            
        if unit.has_unique(UniqueType.HealOutsideFriendlyTerritory):
            return True
            
        if unit.current_tile.is_city_center():
            return True
            
        if unit.current_tile.owner == unit.civ:
            return True
            
        return False
        
    @staticmethod
    def try_prepare(unit: MapUnit) -> bool:
        """Try to prepare the unit for combat.
        
        Args:
            unit: The unit to try to prepare
            
        Returns:
            True if unit was prepared
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.has_unique(UniqueType.OneTimeUnitHeal):
            return False
            
        if unit.has_unique(UniqueType.HealOutsideFriendlyTerritory):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner == unit.civ:
            return False
            
        if unit.health < 100:
            return False
            
        if unit.civ.is_at_war():
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        return True
        
    @staticmethod
    def try_garrisoning_ranged_land_unit(unit: MapUnit) -> bool:
        """Try to garrison a ranged land unit.
        
        Args:
            unit: The unit to try to garrison
            
        Returns:
            True if unit was garrisoned
        """
        if not unit.base_unit.is_ranged():
            return False
            
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner != unit.civ:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_city:
            unit.movement.head_towards(closest_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_stationing_melee_naval_unit(unit: MapUnit) -> bool:
        """Try to station a melee naval unit.
        
        Args:
            unit: The unit to try to station
            
        Returns:
            True if unit was stationed
        """
        if not unit.base_unit.is_melee():
            return False
            
        if not unit.base_unit.is_water_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.current_tile.is_city_center():
            return False
            
        if unit.current_tile.owner != unit.civ:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_city = next(
            (city for city in sorted(
                unit.civ.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_city:
            unit.movement.head_towards(closest_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_advance_towards_close_enemy(unit: MapUnit) -> bool:
        """Try to advance towards a close enemy.
        
        Args:
            unit: The unit to try to advance
            
        Returns:
            True if unit advanced
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        closest_enemy = next(
            (enemy for enemy in unit.civ.game_info.civilizations
             if enemy != unit.civ and enemy.is_at_war_with(unit.civ)),
            None
        )
        
        if not closest_enemy:
            return False
            
        closest_enemy_city = next(
            (city for city in sorted(
                closest_enemy.cities,
                key=lambda c: c.get_center_tile().aerial_distance_to(unit.get_tile())
            ) if city.get_center_tile().aerial_distance_to(unit.get_tile()) < 20),
            None
        )
        
        if closest_enemy_city:
            unit.movement.head_towards(closest_enemy_city.get_center_tile())
            return True
            
        return False 

    @staticmethod
    def try_accompany_settler_or_great_person(unit: MapUnit) -> bool:
        """Try to accompany a settler or great person.
        
        Args:
            unit: The unit to try to accompany with
            
        Returns:
            True if unit accompanied
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        settler_or_great_person = next(
            (other for other in unit.civ.units.get_civ_units()
             if other != unit
             and (other.base_unit.has_unique(UniqueType.FoundCity)
                  or other.base_unit.has_unique(UniqueType.GreatPerson))
             and other.movement.can_reach(unit.get_tile())),
            None
        )
        
        if settler_or_great_person:
            unit.movement.head_towards(settler_or_great_person.get_tile())
            return True
            
        return False
        
    @staticmethod
    def try_head_towards_our_sieged_city(unit: MapUnit) -> bool:
        """Try to head towards a sieged city.
        
        Args:
            unit: The unit to try to head towards a sieged city with
            
        Returns:
            True if unit headed towards a sieged city
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        sieged_city = next(
            (city for city in unit.civ.cities
             if city.is_under_siege()
             and unit.movement.can_reach(city.get_center_tile())),
            None
        )
        
        if sieged_city:
            unit.movement.head_towards(sieged_city.get_center_tile())
            return True
            
        return False
        
    @staticmethod
    def try_take_back_captured_city(unit: MapUnit) -> bool:
        """Try to take back a captured city.
        
        Args:
            unit: The unit to try to take back a captured city with
            
        Returns:
            True if unit headed towards a captured city
        """
        if unit.base_unit.is_air_unit():
            return False
            
        if unit.has_unique(UniqueType.SelfDestructs):
            return False
            
        if unit.health < 50:
            return False
            
        if unit.civ.threat_manager.get_distance_to_closest_enemy_unit(unit.get_tile(), 6, False) < 4:
            return False
            
        captured_city = next(
            (city for city in unit.civ.cities
             if city.is_captured()
             and unit.movement.can_reach(city.get_center_tile())),
            None
        )
        
        if captured_city:
            unit.movement.head_towards(captured_city.get_center_tile())
            return True
            
        return False 