from math import pow
from com.unciv import Constants
from com.unciv.logic import MultiFilter
from com.unciv.logic.city import City
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map.tile import Tile
from com.unciv.models import UncivSound
from com.unciv.models.ruleset.unique import StateForConditionals, UniqueType
from com.unciv.models.ruleset.unit import UnitType
from com.unciv.ui.components.extensions import to_percent
from com.unciv.logic.battle.i_combatant import ICombatant
from com.unciv.logic.battle.combat_action import CombatAction

class CityCombatant(ICombatant):
    def __init__(self, city: City):
        self.city = city

    def get_max_health(self) -> int:
        return self.city.get_max_health()

    def get_health(self) -> int:
        return self.city.health

    def get_civ_info(self) -> Civilization:
        return self.city.civ

    def get_tile(self) -> Tile:
        return self.city.get_center_tile()

    def get_name(self) -> str:
        return self.city.name

    def is_defeated(self) -> bool:
        return self.city.health == 1

    def is_invisible(self, to: Civilization) -> bool:
        return False

    def can_attack(self) -> bool:
        return self.city.can_bombard()

    def matches_filter(self, filter: str, multi_filter: bool = False) -> bool:
        if multi_filter:
            return MultiFilter.multi_filter(
                filter,
                lambda x: (x == "City" or x in Constants.all or
                          self.city.matches_filter(x, multi_filter=False))
            )
        return (filter == "City" or filter in Constants.all or
                self.city.matches_filter(filter, multi_filter=False))

    def get_attack_sound(self) -> UncivSound:
        return UncivSound.Bombard

    def take_damage(self, damage: int) -> None:
        self.city.health -= damage
        if self.city.health < 1:
            self.city.health = 1  # min health is 1

    def get_unit_type(self) -> UnitType:
        return UnitType.City

    def get_attacking_strength(self) -> int:
        return int(self.get_city_strength(CombatAction.Attack) * 0.75)

    def get_defending_strength(self, attacked_by_ranged: bool) -> int:
        if self.is_defeated():
            return 1
        return self.get_city_strength()

    def get_city_strength(self, combat_action: CombatAction = CombatAction.Defend) -> int:
        # Civ fanatics forum, from a modder who went through the original code
        mod_constants = self.get_civ_info().game_info.ruleset.mod_options.constants
        strength = mod_constants.city_strength_base
        strength += (self.city.population.population * mod_constants.city_strength_per_pop)  # Each 5 pop gives 2 defence

        city_tile = self.city.get_center_tile()
        for unique in [unique for terrain in city_tile.all_terrains
                      for unique in terrain.get_matching_uniques(UniqueType.GrantsCityStrength)]:
            strength += int(unique.params[0])

        # as tech progresses so does city strength
        tech_count = self.get_civ_info().game_info.ruleset.technologies.size
        techs_percent_known = (self.city.civ.tech.techs_researched.size / tech_count
                             if tech_count > 0 else 0.5)  # for mods with no tech
        strength += (pow(techs_percent_known * mod_constants.city_strength_from_techs_multiplier,
                        mod_constants.city_strength_from_techs_exponent) *
                    mod_constants.city_strength_from_techs_full_multiplier)

        # The way all of this adds up...
        # All ancient techs - 0.5 extra, Classical - 2.7, Medieval - 8, Renaissance - 17.5,
        # Industrial - 32.4, Modern - 51, Atomic - 72.5, All - 118.3

        # Garrisoned unit gives up to 20% of strength to city, health-dependant
        if city_tile.military_unit:
            strength += (city_tile.military_unit.base_unit.strength *
                        (city_tile.military_unit.health / 100.0) *
                        mod_constants.city_strength_from_garrison)

        buildings_strength = self.city.get_strength()
        state_for_conditionals = StateForConditionals(
            self.get_civ_info(),
            city=self.city,
            our_combatant=self,
            combat_action=combat_action
        )

        for unique in self.get_civ_info().get_matching_uniques(
            UniqueType.BetterDefensiveBuildings,
            state_for_conditionals
        ):
            buildings_strength *= to_percent(unique.params[0])
        strength += buildings_strength

        return int(strength)

    def __str__(self) -> str:
        return self.city.name  # for debug