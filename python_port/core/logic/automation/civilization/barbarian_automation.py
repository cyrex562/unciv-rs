from typing import List, Optional

from com.unciv.Constants import Constants
from com.unciv.logic.automation.unit import BattleHelper, UnitAutomation
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map.mapunit import MapUnit

class BarbarianAutomation:
    """Handles automation of barbarian units in the game."""
    
    def __init__(self, civ_info: Civilization):
        self.civ_info = civ_info
    
    def automate(self):
        """Automate all barbarian units in a specific order."""
        # Ranged units go first, then melee, then everyone else
        civ_units = self.civ_info.units.get_civ_units()
        
        # Process ranged units
        for unit in (unit for unit in civ_units if unit.base_unit.is_ranged()):
            self._automate_unit(unit)
            
        # Process melee units
        for unit in (unit for unit in civ_units if unit.base_unit.is_melee()):
            self._automate_unit(unit)
            
        # Process other units
        for unit in (unit for unit in civ_units 
                    if not unit.base_unit.is_ranged() and not unit.base_unit.is_melee()):
            self._automate_unit(unit)
            
        # Clear popup alerts to reduce save size and ease debugging
        self.civ_info.popup_alerts.clear()
    
    def _automate_unit(self, unit: MapUnit):
        """Automate a single unit based on its type and current state."""
        if unit.is_civilian():
            self._automate_captured_civilian(unit)
        elif unit.current_tile.improvement == Constants.barbarian_encampment:
            self._automate_unit_on_encampment(unit)
        else:
            self._automate_combat_unit(unit)
    
    def _automate_captured_civilian(self, unit: MapUnit):
        """Handle automation of captured civilian units."""
        # 1. Stay on current encampment if already there
        if unit.current_tile.improvement == Constants.barbarian_encampment:
            return
            
        # 2. Find and move to nearest available encampment
        camp_tiles = [
            self.civ_info.game_info.tile_map[camp.position]
            for camp in self.civ_info.game_info.barbarians.encampments
        ]
        camp_tiles.sort(key=lambda tile: unit.current_tile.aerial_distance_to(tile))
        
        best_camp = next(
            (tile for tile in camp_tiles 
             if tile.civilian_unit is None and unit.movement.can_reach(tile)),
            None
        )
        
        if best_camp:
            unit.movement.head_towards(best_camp)
        else:
            # 3. Wander aimlessly if no reachable encampment found
            UnitAutomation.wander(unit)
    
    def _automate_unit_on_encampment(self, unit: MapUnit):
        """Handle automation of units stationed at encampments."""
        # 1. Try to upgrade
        if UnitAutomation.try_upgrade_unit(unit):
            return
            
        # 2. Try to attack without leaving encampment
        if BattleHelper.try_attack_nearby_enemy(unit, stay_on_tile=True):
            return
            
        # 3. Fortify if possible
        unit.fortify_if_can()
    
    def _automate_combat_unit(self, unit: MapUnit):
        """Handle automation of combat units."""
        # 1. Try pillaging to restore health if low
        if unit.health < 50 and UnitAutomation.try_pillage_improvement(unit, True):
            if not unit.has_movement():
                return
                
        # 2. Try to upgrade
        if UnitAutomation.try_upgrade_unit(unit):
            return
            
        # 3. Try to attack enemy
        # If an embarked melee unit can land and attack next turn, do not attack from water
        if BattleHelper.try_disembark_unit_to_attack_position(unit):
            return
            
        if not unit.is_civilian() and BattleHelper.try_attack_nearby_enemy(unit):
            return
            
        # 4. Try to pillage tile or route
        while UnitAutomation.try_pillage_improvement(unit):
            if not unit.has_movement():
                return
                
        # 5. Wander if no other actions possible
        UnitAutomation.wander(unit) 