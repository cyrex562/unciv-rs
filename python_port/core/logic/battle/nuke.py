import random
from typing import List, Tuple, Sequence
from com.badlogic.gdx.math import Vector2
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization, CivilopediaAction, LocationAction, Notification, NotificationCategory, NotificationIcon
from com.unciv.logic.civilization.diplomacy import DiplomaticModifiers, DiplomaticStatus
from com.unciv.logic.map.tile import RoadStatus, Tile
from com.unciv.models.ruleset.unique import UniqueType
from com.unciv.ui.components.extensions import to_percent
from com.unciv.ui.screens.worldscreen.bottombar import BattleTable
from com.unciv.logic.battle.map_unit_combatant import MapUnitCombatant
from com.unciv.logic.battle.city_combatant import CityCombatant
from com.unciv.logic.battle.air_interception import AirInterception
from com.unciv.logic.battle.battle import Battle

class Nuke:
    @staticmethod
    def may_use_nuke(nuke: MapUnitCombatant, target_tile: Tile) -> bool:
        """
        Checks whether nuke is allowed to nuke target_tile
        - Not if we would need to declare war on someone we can't.
        - Disallow nuking the tile the nuke is in, as per Civ5 (but not nuking your own tiles/units otherwise)

        Both BattleTable.simulateNuke and AirUnitAutomation.automateNukes check range, so that check is omitted here.
        """
        if nuke.get_tile() == target_tile:
            return False
        # Can only nuke visible Tiles
        if not target_tile.is_visible(nuke.get_civ_info()):
            return False

        can_nuke = True
        attacker_civ = nuke.get_civ_info()

        def check_defender_civ(defender_civ: Civilization | None) -> None:
            nonlocal can_nuke
            if defender_civ is None:
                return
            # Allow nuking yourself! (Civ5 source: CvUnit::isNukeVictim)
            if defender_civ == attacker_civ or defender_civ.is_defeated():
                return
            if defender_civ.is_barbarian:
                return
            # Gleaned from Civ5 source - this disallows nuking unknown civs even in invisible tiles
            # https://github.com/Gedemon/Civ5-DLL/blob/master/CvGameCoreDLL_Expansion1/CvUnit.cpp#L5056
            # https://github.com/Gedemon/Civ5-DLL/blob/master/CvGameCoreDLL_Expansion1/CvTeam.cpp#L986
            if attacker_civ.get_diplomacy_manager(defender_civ)?.can_attack() == True:
                return
            can_nuke = False

        blast_radius = nuke.unit.get_nuke_blast_radius()
        for tile in target_tile.get_tiles_in_distance(blast_radius):
            check_defender_civ(tile.get_owner())
            check_defender_civ(Battle.get_map_combatant_of_tile(tile)?.get_civ_info())
        return can_nuke

    @staticmethod
    def NUKE(attacker: MapUnitCombatant, target_tile: Tile) -> None:
        """Main nuke execution function"""
        attacking_civ = attacker.get_civ_info()
        nuke_strength = next(
            (int(unique.params[0]) for unique in attacker.unit.get_matching_uniques(UniqueType.NuclearWeapon)),
            None
        )
        if nuke_strength is None:
            return

        blast_radius = next(
            (int(unique.params[0]) for unique in attacker.unit.get_matching_uniques(UniqueType.BlastRadius)),
            2
        )

        hit_tiles = target_tile.get_tiles_in_distance(blast_radius)

        hit_civs_territory, notify_declared_war_civs = Nuke._declare_war_on_hit_civs(
            attacking_civ, hit_tiles, attacker, target_tile
        )

        Nuke._add_nuke_notifications(target_tile, attacker, notify_declared_war_civs, attacking_civ, hit_civs_territory)

        if attacker.is_defeated():
            return

        attacker.unit.attacks_since_turn_start.append(Vector2(target_tile.position))

        for tile in hit_tiles:
            # Handle complicated effects
            Nuke._do_nuke_explosion_for_tile(attacker, tile, nuke_strength, target_tile == tile)

        # Instead of postBattleAction() just destroy the unit, all other functions are not relevant
        if attacker.unit.has_unique(UniqueType.SelfDestructs):
            attacker.unit.destroy()

        # It's unclear whether using nukes results in a penalty with all civs, or only affected civs.
        # For now I'll make it give a diplomatic penalty to all known civs, but some testing for this would be appreciated
        for civ in attacking_civ.get_known_civs():
            civ.get_diplomacy_manager(attacking_civ)!.set_modifier(DiplomaticModifiers.UsedNuclearWeapons, -50.0)

        if not attacker.is_defeated():
            attacker.unit.attacks_this_turn += 1

    @staticmethod
    def _add_nuke_notifications(
        target_tile: Tile,
        attacker: MapUnitCombatant,
        notify_declared_war_civs: List[Civilization],
        attacking_civ: Civilization,
        hit_civs_territory: List[Civilization]
    ) -> None:
        nuke_notification_action = [LocationAction(target_tile.position), CivilopediaAction("Units/" + attacker.get_name())]

        # If the nuke has been intercepted and destroyed then it fails to detonate
        if attacker.is_defeated():
            # Notify attacker that they are now at war for the attempt
            for defending_civ in notify_declared_war_civs:
                attacking_civ.add_notification(
                    f"After an attempted attack by our [{attacker.get_name()}], [{defending_civ}] has declared war on us!",
                    nuke_notification_action,
                    NotificationCategory.Diplomacy,
                    defending_civ.civ_name,
                    NotificationIcon.War,
                    attacker.get_name()
                )
            return

        # Notify attacker that they are now at war
        for defending_civ in notify_declared_war_civs:
            attacking_civ.add_notification(
                f"After being hit by our [{attacker.get_name()}], [{defending_civ}] has declared war on us!",
                nuke_notification_action,
                NotificationCategory.Diplomacy,
                defending_civ.civ_name,
                NotificationIcon.War,
                attacker.get_name()
            )

        # Message all other civs
        for other_civ in attacking_civ.game_info.civilizations:
            if not other_civ.is_alive() or other_civ == attacking_civ:
                continue
            if other_civ in hit_civs_territory:
                other_civ.add_notification(
                    f"A(n) [{attacker.get_name()}] from [{attacking_civ.civ_name}] has exploded in our territory!",
                    nuke_notification_action,
                    NotificationCategory.War,
                    attacking_civ.civ_name,
                    NotificationIcon.War,
                    attacker.get_name()
                )
            elif other_civ.knows(attacking_civ):
                other_civ.add_notification(
                    f"A(n) [{attacker.get_name()}] has been detonated by [{attacking_civ.civ_name}]!",
                    nuke_notification_action,
                    NotificationCategory.War,
                    attacking_civ.civ_name,
                    NotificationIcon.War,
                    attacker.get_name()
                )
            else:
                other_civ.add_notification(
                    f"A(n) [{attacker.get_name()}] has been detonated by an unknown civilization!",
                    nuke_notification_action,
                    NotificationCategory.War,
                    NotificationIcon.War,
                    attacker.get_name()
                )

    @staticmethod
    def _declare_war_on_hit_civs(
        attacking_civ: Civilization,
        hit_tiles: Sequence[Tile],
        attacker: MapUnitCombatant,
        target_tile: Tile
    ) -> Tuple[List[Civilization], List[Civilization]]:
        # Declare war on the owners of all hit tiles
        notify_declared_war_civs: List[Civilization] = []

        def try_declare_war(civ_suffered: Civilization) -> None:
            if (civ_suffered != attacking_civ
                and civ_suffered.knows(attacking_civ)
                and attacking_civ.get_diplomacy_manager(civ_suffered)!.diplomatic_status != DiplomaticStatus.War):
                attacking_civ.get_diplomacy_manager(civ_suffered)!.declare_war()
                if civ_suffered not in notify_declared_war_civs:
                    notify_declared_war_civs.append(civ_suffered)

        hit_civs_territory: List[Civilization] = []
        for hit_civ in {tile.get_owner() for tile in hit_tiles if tile.get_owner() is not None}:
            hit_civs_territory.append(hit_civ)
            try_declare_war(hit_civ)

        # Declare war on all potentially hit units. They'll try to intercept the nuke before it drops
        for civ_whose_unit_was_attacked in {unit.civ for tile in hit_tiles for unit in tile.get_units() if unit.civ != attacking_civ}:
            try_declare_war(civ_whose_unit_was_attacked)
            if attacker.unit.base_unit.is_air_unit() and not attacker.is_defeated():
                AirInterception.try_intercept_air_attack(
                    attacker,
                    target_tile,
                    civ_whose_unit_was_attacked,
                    None
                )
        return hit_civs_territory, notify_declared_war_civs

    @staticmethod
    def _do_nuke_explosion_for_tile(
        attacker: MapUnitCombatant,
        tile: Tile,
        nuke_strength: int,
        is_ground_zero: bool
    ) -> None:
        # https://forums.civfanatics.com/resources/unit-guide-modern-future-units-g-k.25628/
        # https://www.carlsguides.com/strategy/civilization5/units/aircraft-nukes.ph
        # Testing done by Ravignir
        # original source code: GenerateNuclearExplosionDamage(), ApplyNuclearExplosionDamage()

        damage_modifier_from_missing_resource = 1.0
        civ_resources = attacker.get_civ_info().get_civ_resources_by_name()
        for resource in attacker.unit.get_resource_requirements_per_turn().keys():
            if civ_resources[resource] < 0 and not attacker.get_civ_info().is_barbarian:
                damage_modifier_from_missing_resource *= 0.5  # I could not find a source for this number, but this felt about right
                # - Original Civ5 does *not* reduce damage from missing resource, from source inspection

        building_modifier = 1.0  # Strange, but in Civ5 a bunker mitigates damage to garrison, even if the city is destroyed by the nuke

        # Damage city and reduce its population
        city = tile.get_city()
        if city is not None and tile.position == city.location:
            building_modifier = city.get_aggregate_modifier(UniqueType.GarrisonDamageFromNukes)
            Nuke._do_nuke_explosion_damage_to_city(city, nuke_strength, damage_modifier_from_missing_resource)
            Battle.post_battle_notifications(attacker, CityCombatant(city), city.get_center_tile())
            Battle.destroy_if_defeated(city.civ, attacker.get_civ_info(), city.location)

        # Damage and/or destroy units on the tile
        for unit in list(tile.get_units()):  # toList so if it's destroyed there's no concurrent modification
            damage = int((Nuke._get_nuke_damage(nuke_strength, is_ground_zero) * building_modifier * damage_modifier_from_missing_resource + 1.0))
            defender = MapUnitCombatant(unit)
            if unit.is_civilian():
                if unit.health - damage <= 40:
                    unit.destroy()  # Civ5: NUKE_NON_COMBAT_DEATH_THRESHOLD = 60
            else:
                defender.take_damage(damage)
            Battle.post_battle_notifications(attacker, defender, defender.get_tile())
            Battle.destroy_if_defeated(defender.get_civ_info(), attacker.get_civ_info())

        # Pillage improvements, pillage roads, add fallout
        if tile.is_city_center():
            return  # Never touch city centers - if they survived

        if tile.terrain_has_unique(UniqueType.DestroyableByNukesChance):
            # Note: Safe from concurrent modification exceptions only because removeTerrainFeature
            # *replaces* terrainFeatureObjects and the loop will continue on the old one
            for terrain_feature in tile.terrain_feature_objects:
                for unique in terrain_feature.get_matching_uniques(UniqueType.DestroyableByNukesChance):
                    chance = float(unique.params[0]) / 100.0
                    if not (chance > 0.0 and is_ground_zero) and random.random() >= chance:
                        continue
                    tile.remove_terrain_feature(terrain_feature.name)
                    Nuke._apply_pillage_and_fallout(tile)
        elif is_ground_zero or random.random() < 0.5:  # Civ5: NUKE_FALLOUT_PROB
            Nuke._apply_pillage_and_fallout(tile)

    @staticmethod
    def _get_nuke_damage(nuke_strength: int, is_ground_zero: bool) -> float:
        if is_ground_zero or nuke_strength >= 2:
            return 100.0
        # The following constants are NUKE_UNIT_DAMAGE_BASE / NUKE_UNIT_DAMAGE_RAND_1 / NUKE_UNIT_DAMAGE_RAND_2 in Civ5
        if nuke_strength == 1:
            return 30.0 + random.randint(0, 39) + random.randint(0, 39)
        # Level 0 does not exist in Civ5 (it treats units same as level 2)
        return 20.0 + random.randint(0, 29)

    @staticmethod
    def _apply_pillage_and_fallout(tile: Tile) -> None:
        if tile.get_unpillaged_improvement() is not None and not tile.get_tile_improvement()!.has_unique(UniqueType.Irremovable):
            if tile.get_tile_improvement()!.has_unique(UniqueType.Unpillagable):
                tile.remove_improvement()
            else:
                tile.set_pillaged()
        if tile.get_unpillaged_road() != RoadStatus.None:
            tile.set_pillaged()
        if tile.is_water or tile.is_impassible() or "Fallout" in tile.terrain_features:
            return
        tile.add_terrain_feature("Fallout")

    @staticmethod
    def _do_nuke_explosion_damage_to_city(targeted_city: City, nuke_strength: int, damage_modifier_from_missing_resource: float) -> None:
        # Original Capitals must be protected, `can_be_destroyed` is responsible for that check.
        # The `just_captured = true` parameter is what allows other Capitals to suffer normally.
        if ((nuke_strength > 2 or nuke_strength > 1 and targeted_city.population.population < 5)
            and targeted_city.can_be_destroyed(True)):
            targeted_city.destroy_city()
            return

        city_combatant = CityCombatant(targeted_city)
        city_combatant.take_damage(int(city_combatant.get_health() * 0.5 * damage_modifier_from_missing_resource))

        # Difference to original: Civ5 rounds population loss down twice - before and after bomb shelters
        population_loss = int(
            targeted_city.population.population *
            targeted_city.get_aggregate_modifier(UniqueType.PopulationLossFromNukes) *
            {
                0: 0.0,
                1: (30 + random.randint(0, 19) + random.randint(0, 19)) / 100.0,
                2: (60 + random.randint(0, 9) + random.randint(0, 9)) / 100.0,
            }.get(nuke_strength, 1.0)  # hypothetical nukeStrength 3 -> always to 1 pop
        )
        targeted_city.population.add_population(-population_loss)

    @staticmethod
    def get_aggregate_modifier(city: City, unique_type: UniqueType) -> float:
        modifier = 1.0
        for unique in city.get_matching_uniques(unique_type):
            if not city.matches_filter(unique.params[1]):
                continue
            modifier *= to_percent(unique.params[0])
        return modifier