from typing import List, Optional, Set
from dataclasses import dataclass
import random
import math

from com.badlogic.gdx.math import Vector2
from com.unciv.Constants import Constants
from com.unciv.logic import GameInfo, IsPartOfGameInfoSerialization
from com.unciv.logic.civilization import NotificationCategory, NotificationIcon
from com.unciv.logic.map import TileMap
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.models.ruleset.unit import BaseUnit
from com.unciv.utils import random_weighted

@dataclass
class Encampment(IsPartOfGameInfoSerialization):
    """Represents a barbarian encampment in the game."""
    position: Vector2
    countdown: int = 0
    spawned_units: int = -1
    destroyed: bool = False  # destroyed encampments haunt the vicinity for 15 turns preventing new spawns
    game_info: Optional[GameInfo] = None

    def clone(self) -> 'Encampment':
        """Create a deep copy of the encampment."""
        return Encampment(
            position=Vector2(self.position.x, self.position.y),
            countdown=self.countdown,
            spawned_units=self.spawned_units,
            destroyed=self.destroyed
        )

    def update(self):
        """Update the encampment state and possibly spawn a barbarian."""
        if self.countdown > 0:
            self.countdown -= 1
        elif not self.destroyed and self._spawn_barbarian():
            self.spawned_units += 1
            self._reset_countdown()

    def was_attacked(self):
        """Handle encampment being attacked by speeding up next spawn."""
        if not self.destroyed:
            self.countdown //= 2

    def was_destroyed(self):
        """Handle encampment being destroyed."""
        if not self.destroyed:
            self.countdown = 15
            self.destroyed = True

    def _spawn_barbarian(self) -> bool:
        """Attempt to spawn a barbarian from this encampment."""
        tile = self.game_info.tile_map[self.position]

        # Empty camp - spawn a defender
        if tile.military_unit is None:
            return self._spawn_unit(False)

        # Don't spawn wandering barbs too early
        if self.game_info.turns < 10:
            return False

        # Too many barbarians around already?
        barbarian_civ = self.game_info.get_barbarian_civilization()
        if sum(1 for t in tile.get_tiles_in_distance(4) 
               if t.military_unit and t.military_unit.civ == barbarian_civ) > 2:
            return False

        can_spawn_boats = self.game_info.turns > 30
        valid_tiles = [
            neighbor for neighbor in tile.neighbors
            if (not neighbor.is_impassible() and
                not neighbor.is_city_center() and
                neighbor.get_first_unit() is None and
                (not neighbor.is_water or can_spawn_boats) and
                not (neighbor.terrain_has_unique(UniqueType.FreshWater) and neighbor.is_water))
        ]
        
        if not valid_tiles:
            return False

        return self._spawn_unit(random.choice(valid_tiles).is_water)

    def _spawn_unit(self, naval: bool) -> bool:
        """Attempt to spawn a barbarian unit."""
        unit_to_spawn = self._choose_barbarian_unit(naval)
        if not unit_to_spawn:
            return False
            
        spawned_unit = self.game_info.tile_map.place_unit_near_tile(
            self.position, unit_to_spawn, self.game_info.get_barbarian_civilization()
        )
        return spawned_unit is not None

    def _choose_barbarian_unit(self, naval: bool) -> Optional[BaseUnit]:
        """Choose a barbarian unit to spawn based on current game state."""
        # Get all researched techs from non-barbarian civilizations
        all_researched_techs = list(self.game_info.ruleset.technologies.keys())
        for civ in self.game_info.civilizations:
            if not civ.is_barbarian and not civ.is_defeated():
                all_researched_techs = [tech for tech in all_researched_techs 
                                     if tech in civ.tech.techs_researched]

        # Set barbarian tech to match researched techs
        barbarian_civ = self.game_info.get_barbarian_civilization()
        barbarian_civ.tech.techs_researched = set(all_researched_techs)

        # Filter available units
        unit_list = [
            unit for unit in self.game_info.ruleset.units.values
            if (unit.is_military and
                not unit.has_unique(UniqueType.CannotAttack) and
                not unit.has_unique(UniqueType.CannotBeBarbarian) and
                (naval == unit.is_water_unit) and
                unit.is_buildable(barbarian_civ))
        ]

        if not unit_list:
            return None

        # Weight units by their force evaluation
        weightings = [unit.get_force_evaluation() for unit in unit_list]
        return random_weighted(unit_list, weightings)

    def _reset_countdown(self):
        """Reset the spawn countdown with appropriate modifiers."""
        # Base 8-12 turns
        self.countdown = 8 + random.randint(0, 4)
        
        # Quicker on Raging Barbarians
        if self.game_info.game_parameters.raging_barbarians:
            self.countdown //= 2
            
        # Higher on low difficulties
        self.countdown += self.game_info.ruleset.difficulties[
            self.game_info.game_parameters.difficulty
        ].barbarian_spawn_delay
        
        # Quicker if this camp has already spawned units
        self.countdown -= min(3, self.spawned_units)

        self.countdown = int(self.countdown * self.game_info.speed.barbarian_modifier)

class BarbarianManager(IsPartOfGameInfoSerialization):
    """Manages barbarian encampments and their spawning behavior."""
    
    def __init__(self):
        self.encampments: List[Encampment] = []
        self.game_info: Optional[GameInfo] = None
        self.tile_map: Optional[TileMap] = None

    def clone(self) -> 'BarbarianManager':
        """Create a deep copy of the barbarian manager."""
        to_return = BarbarianManager()
        to_return.encampments = [camp.clone() for camp in self.encampments]
        return to_return

    def set_transients(self, game_info: GameInfo):
        """Set up transient references and initialize encampments."""
        self.game_info = game_info
        self.tile_map = game_info.tile_map

        # Add any preexisting camps as Encampment objects
        existing_encampment_locations = {camp.position for camp in self.encampments}

        for tile in self.tile_map.values:
            if (tile.improvement == Constants.barbarian_encampment and
                tile.position not in existing_encampment_locations):
                self.encampments.append(Encampment(tile.position))

        for camp in self.encampments:
            camp.game_info = game_info

    def update_encampments(self):
        """Update all encampments and handle spawning."""
        # Check if camps were destroyed
        for encampment in self.encampments[:]:  # Create a copy to avoid concurrent modification
            if self.tile_map[encampment.position].improvement != Constants.barbarian_encampment:
                encampment.was_destroyed()
            # Check if the ghosts are ready to depart
            if encampment.destroyed and encampment.countdown == 0:
                self.encampments.remove(encampment)

        # Possibly place a new encampment
        self._place_barbarian_encampment()

        for encampment in self.encampments:
            encampment.update()

    def camp_attacked(self, position: Vector2):
        """Handle an encampment being attacked."""
        for encampment in self.encampments:
            if encampment.position == position:
                encampment.was_attacked()
                break

    def _place_barbarian_encampment(self, for_testing: bool = False):
        """Place new barbarian encampments based on game state."""
        # Early return if we don't want to place a camp
        if not for_testing and self.game_info.turns > 1 and random.random() < 0.5:
            return

        # Get all viewable tiles
        all_viewable_tiles = {
            tile for civ in self.game_info.civilizations
            if not civ.is_barbarian and not civ.is_spectator()
            for tile in civ.viewable_tiles
        }
        
        # Get fog tiles
        fog_tiles = [
            tile for tile in self.tile_map.values
            if tile.is_land and tile not in all_viewable_tiles
        ]

        # Calculate number of camps based on map size
        fog_tiles_per_camp = int(math.pow(len(self.tile_map), 0.4))
        camps_to_add = (len(fog_tiles) // fog_tiles_per_camp) - sum(1 for camp in self.encampments if not camp.destroyed)

        # First turn of the game add 1/3 of all possible camps
        if self.game_info.turns == 1:
            camps_to_add //= 3
            camps_to_add = max(camps_to_add, 1)  # At least 1 on first turn
        elif camps_to_add > 0:
            camps_to_add = 1

        if camps_to_add <= 0:
            return

        # Get tiles that are too close to capitals or other camps
        too_close_to_capitals = {
            tile for civ in self.game_info.civilizations
            if not (civ.is_barbarian or civ.is_spectator() or not civ.cities or 
                   civ.is_city_state or not civ.get_capital())
            for tile in civ.get_capital().get_center_tile().get_tiles_in_distance(4)
        }

        too_close_to_camps = {
            tile for camp in self.encampments
            for tile in self.tile_map[camp.position].get_tiles_in_distance(
                4 if camp.destroyed else 7
            )
        }

        # Get viable tiles for new camps
        viable_tiles = [
            tile for tile in fog_tiles
            if (not tile.is_impassible() and
                tile.resource is None and
                not any(feature.has_unique(UniqueType.RestrictedBuildableImprovements)
                       for feature in tile.terrain_feature_objects) and
                any(neighbor.is_land for neighbor in tile.neighbors) and
                tile not in too_close_to_capitals and
                tile not in too_close_to_camps)
        ]

        added_camps = 0
        bias_coast = random.randint(0, 5) == 0

        # Add the camps
        while added_camps < camps_to_add and viable_tiles:
            # If we're biasing for coast, get a coast tile if possible
            if bias_coast:
                coastal_tiles = [tile for tile in viable_tiles if tile.is_coastal_tile()]
                tile = random.choice(coastal_tiles) if coastal_tiles else random.choice(viable_tiles)
            else:
                tile = random.choice(viable_tiles)

            tile.set_improvement(Constants.barbarian_encampment)
            new_camp = Encampment(tile.position)
            new_camp.game_info = self.game_info
            self.encampments.append(new_camp)
            self._notify_civs_of_barbarian_encampment(tile)
            added_camps += 1

            # Remove newly non-viable tiles
            if added_camps < camps_to_add:
                viable_tiles = [t for t in viable_tiles 
                              if t not in tile.get_tiles_in_distance(7)]
                bias_coast = random.randint(0, 5) == 0

    def _notify_civs_of_barbarian_encampment(self, tile: Tile):
        """Notify civilizations about new barbarian encampments."""
        for civ in self.game_info.civilizations:
            if (civ.has_unique(UniqueType.NotifiedOfBarbarianEncampments) and
                civ.has_explored(tile)):
                civ.add_notification(
                    "A new barbarian encampment has spawned!",
                    tile.position,
                    NotificationCategory.War,
                    NotificationIcon.War
                )
                civ.set_last_seen_improvement(tile.position, Constants.barbarian_encampment) 