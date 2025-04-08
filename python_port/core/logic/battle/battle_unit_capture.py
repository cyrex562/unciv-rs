from typing import Optional
import random
from com.badlogic.gdx.math import Vector2
from com.unciv import Constants
from com.unciv.logic.civilization import (
    AlertType, Civilization, MapUnitAction, NotificationCategory,
    NotificationIcon, PlayerType, PopupAlert
)
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant
from com.unciv.logic.battle.i_combatant import ICombatant
from com.unciv.logic.battle.battle import Battle

class BattleUnitCapture:
    @staticmethod
    def try_capture_military_unit(
        attacker: ICombatant,
        defender: ICombatant,
        attacked_tile: Tile
    ) -> bool:
        # https://forums.civfanatics.com/threads/prize-ships-for-land-units.650196/
        # https://civilization.fandom.com/wiki/Module:Data/Civ5/GK/Defines
        # There are 3 ways of capturing a unit, we separate them for cleaner code but we also need to ensure a unit isn't captured twice

        if not isinstance(defender, MapUnitCombatant) or not isinstance(attacker, MapUnitCombatant):
            return False
        if defender.has_unique(UniqueType.Uncapturable, StateForConditionals(
                unit=defender.unit,
                our_combatant=defender,
                their_combatant=attacker,
                attacked_tile=attacked_tile)):
            return False

        if not defender.is_defeated() or defender.unit.is_civilian():
            return False

        # Due to the way OR operators short-circuit, calling just A() || B() means B isn't called if A is true.
        # Therefore we run all functions before checking if one is true.
        was_unit_captured = any([
            BattleUnitCapture.unit_captured_prize_ships_unique(attacker, defender),
            BattleUnitCapture.unit_captured_from_encampment(attacker, defender, attacked_tile),
            BattleUnitCapture.unit_gain_from_defeating_unit(attacker, defender)
        ])

        if not was_unit_captured:
            return False

        # This is called after takeDamage and so the defeated defender is already destroyed and
        # thus removed from the tile - but MapUnit.destroy() will not clear the unit's currentTile.
        # Therefore placeUnitNearTile _will_ place the new unit exactly where the defender was
        return BattleUnitCapture.spawn_captured_unit(defender, attacker)

    @staticmethod
    def unit_captured_prize_ships_unique(
        attacker: MapUnitCombatant,
        defender: MapUnitCombatant
    ) -> bool:
        if not any(defender.matches_filter(unique.params[0])
                  for unique in attacker.unit.get_matching_uniques(UniqueType.KillUnitCapture)):
            return False

        capture_chance = min(
            0.8,
            0.1 + attacker.get_attacking_strength() / defender.get_defending_strength() * 0.4
        )
        # Between 0 and 1. Defaults to turn and location-based random to avoid save scumming
        random_seed = (attacker.get_civ_info().game_info.turns *
                      defender.get_tile().position.hash_code())
        return random.Random(random_seed).random() <= capture_chance

    @staticmethod
    def unit_gain_from_defeating_unit(
        attacker: MapUnitCombatant,
        defender: MapUnitCombatant
    ) -> bool:
        if not attacker.is_melee():
            return False
        unit_captured = False
        state = StateForConditionals(
            attacker.get_civ_info(),
            our_combatant=attacker,
            their_combatant=defender
        )
        for unique in attacker.get_matching_uniques(UniqueType.GainFromDefeatingUnit, state, True):
            if defender.unit.matches_filter(unique.params[0]):
                attacker.get_civ_info().add_gold(int(unique.params[1]))
                unit_captured = True
        return unit_captured

    @staticmethod
    def unit_captured_from_encampment(
        attacker: MapUnitCombatant,
        defender: MapUnitCombatant,
        attacked_tile: Tile
    ) -> bool:
        if not defender.get_civ_info().is_barbarian:
            return False
        if attacked_tile.improvement != Constants.barbarian_encampment:
            return False

        unit_captured = False
        # German unique - needs to be checked before we try to move to the enemy tile, since the encampment disappears after we move in
        for unique in attacker.get_civ_info().get_matching_uniques(UniqueType.GainFromEncampment):
            attacker.get_civ_info().add_gold(int(unique.params[0]))
            unit_captured = True
        return unit_captured

    @staticmethod
    def spawn_captured_unit(
        defender: MapUnitCombatant,
        attacker: MapUnitCombatant
    ) -> bool:
        defender_tile = defender.get_tile()
        added_unit = attacker.get_civ_info().units.place_unit_near_tile(
            defender_tile.position, defender.get_name())
        if not added_unit:
            return False

        added_unit.current_movement = 0.0
        added_unit.health = 50
        attacker.get_civ_info().add_notification(
            f"An enemy [{defender.get_name()}] has joined us!",
            MapUnitAction(added_unit),
            NotificationCategory.War,
            defender.get_name()
        )

        defender.get_civ_info().add_notification(
            f"An enemy [{attacker.get_name()}] has captured our [{defender.get_name()}]",
            defender.get_tile().position,
            NotificationCategory.War,
            attacker.get_name(),
            NotificationIcon.War,
            defender.get_name()
        )

        civilian_unit = defender_tile.civilian_unit
        # placeUnitNearTile might not have spawned the unit in exactly this tile, in which case no capture would have happened on this tile. So we need to do that here.
        if added_unit.get_tile() != defender_tile and civilian_unit:
            BattleUnitCapture.capture_civilian_unit(attacker, MapUnitCombatant(civilian_unit))
        return True

    @staticmethod
    def capture_civilian_unit(
        attacker: ICombatant,
        defender: MapUnitCombatant,
        check_defeat: bool = True
    ) -> None:
        if attacker.get_civ_info() == defender.get_civ_info():
            raise ValueError("Can't capture our own unit!")

        # need to save this because if the unit is captured its owner will be overwritten
        defender_civ = defender.get_civ_info()

        captured_unit = defender.unit
        # Stop current action
        captured_unit.action = None
        captured_unit.automated = False

        captured_unit_tile = captured_unit.get_tile()
        original_owner = (captured_unit.civ.game_info.get_civilization(captured_unit.original_owner)
                         if captured_unit.original_owner else None)

        was_destroyed_instead = False
        if defender.unit.has_unique(UniqueType.Uncapturable):
            # Uncapturable units are destroyed
            captured_unit.destroy()
            was_destroyed_instead = True
        elif (captured_unit.has_unique(UniqueType.FoundCity) and
              attacker.get_civ_info().is_city_state):
            # City states can never capture settlers at all
            captured_unit.destroy()
            was_destroyed_instead = True
        elif attacker.get_civ_info() == original_owner:
            # Then it is recaptured without converting settlers to workers
            captured_unit.captured_by(attacker.get_civ_info())
        elif (defender.get_civ_info().is_barbarian
              and original_owner is not None
              and not original_owner.is_barbarian
              and attacker.get_civ_info() != original_owner
              and attacker.get_civ_info().knows(original_owner)
              and original_owner.is_alive()
              and not attacker.get_civ_info().is_at_war_with(original_owner)
              and attacker.get_civ_info().player_type == PlayerType.Human):  # Only humans get the choice
            captured_unit.captured_by(attacker.get_civ_info())
            attacker.get_civ_info().popup_alerts.append(
                PopupAlert(
                    AlertType.RecapturedCivilian,
                    captured_unit.current_tile.position.to_string()
                )
            )
        else:
            if not BattleUnitCapture.capture_or_convert_to_worker(captured_unit, attacker.get_civ_info()):
                was_destroyed_instead = True

        if not was_destroyed_instead:
            defender_civ.add_notification(
                f"An enemy [{attacker.get_name()}] has captured our [{defender.get_name()}]",
                defender.get_tile().position,
                NotificationCategory.War,
                attacker.get_name(),
                NotificationIcon.War,
                defender.get_name()
            )
        else:
            defender_civ.add_notification(
                f"An enemy [{attacker.get_name()}] has destroyed our [{defender.get_name()}]",
                defender.get_tile().position,
                NotificationCategory.War,
                attacker.get_name(),
                NotificationIcon.War,
                defender.get_name()
            )
            Battle.trigger_defeat_uniques(defender, attacker, captured_unit_tile)

        if check_defeat:
            Battle.destroy_if_defeated(defender_civ, attacker.get_civ_info())
        captured_unit.update_visible_tiles()

    @staticmethod
    def capture_or_convert_to_worker(
        captured_unit: MapUnit,
        capturing_civ: Civilization
    ) -> Optional[Vector2]:
        # Captured settlers are converted to workers unless captured by barbarians (so they can be returned later).
        if not captured_unit.has_unique(UniqueType.FoundCity) or capturing_civ.is_barbarian:
            captured_unit.captured_by(capturing_civ)
            return captured_unit.current_tile.position  # if capturedBy has moved the unit, this is updated

        captured_unit.destroy()
        # This is so that future checks which check if a unit has been captured are caught give the right answer
        #  For example, in postBattleMoveToAttackedTile
        captured_unit.civ = capturing_civ
        captured_unit.cache.state = StateForConditionals(captured_unit)

        worker_type_unit = next(
            (unit for unit in capturing_civ.game_info.ruleset.units.values()
             if unit.is_civilian() and any(
                unique.params[0] == "Land"
                for unique in unit.get_matching_uniques(UniqueType.BuildImprovements)
             )),
            None
        )
        if not worker_type_unit:
            return None

        placed_unit = capturing_civ.units.place_unit_near_tile(
            captured_unit.current_tile.position,
            worker_type_unit,
            captured_unit.id
        )
        return placed_unit.current_tile.position if placed_unit else None