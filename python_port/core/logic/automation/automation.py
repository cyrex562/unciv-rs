from typing import Dict, Optional, Sequence, Set
from com.unciv.logic.city import City, CityFocus, CityStats
from com.unciv.logic.civilization import Civilization
from com.unciv.logic.map import BFS, TileMap
from com.unciv.logic.map.mapunit import MapUnit
from com.unciv.logic.map.tile import Tile
from com.unciv.models.ruleset import Building, INonPerpetualConstruction, PerpetualConstruction, Victory
from com.unciv.models.ruleset.nation import PersonalityValue
from com.unciv.models.ruleset.tile import ResourceType, TileImprovement
from com.unciv.models.ruleset.unique import LocalUniqueCache, StateForConditionals, UniqueType
from com.unciv.models.ruleset.unit import BaseUnit
from com.unciv.models.stats import Stat, Stats
from com.unciv.ui.screens.victoryscreen import RankingType

class Automation:
    """Contains automation logic for various game aspects."""
    
    @staticmethod
    def rank_tile_for_city_work(tile: Tile, city: City, local_unique_cache: LocalUniqueCache = LocalUniqueCache(False)) -> float:
        """Rank a tile for city work.
        
        Args:
            tile: The tile to rank
            city: The city to rank for
            local_unique_cache: Cache for unique checks
            
        Returns:
            The ranking value for the tile
        """
        stats = tile.stats.get_tile_stats(city, city.civ, local_unique_cache)
        return Automation.rank_stats_for_city_work(stats, city, False, local_unique_cache)

    @staticmethod
    def rank_specialist(specialist: str, city: City, local_unique_cache: LocalUniqueCache) -> float:
        """Rank a specialist for a city.
        
        Args:
            specialist: The specialist type to rank
            city: The city to rank for
            local_unique_cache: Cache for unique checks
            
        Returns:
            The ranking value for the specialist
        """
        stats = city.city_stats.get_stats_of_specialist(specialist, local_unique_cache)
        rank = Automation.rank_stats_for_city_work(stats, city, True, local_unique_cache)
        
        # Derive GPP score
        gpp = 0.0
        if specialist in city.get_ruleset().specialists:
            specialist_info = city.get_ruleset().specialists[specialist]
            gpp = specialist_info.great_person_points.sum_values()
            
        gpp = gpp * (100 + city.current_gpp_bonus) / 100
        rank += gpp * 3  # GPP weight
        return rank

    @staticmethod
    def _get_food_mod_weight(city: City, surplus_food: float) -> float:
        """Calculate food modification weight for city work ranking.
        
        Args:
            city: The city to calculate for
            surplus_food: Current food surplus
            
        Returns:
            The food modification weight
        """
        speed = city.civ.game_info.speed.modifier
        
        # Zero out Growth if close to Unhappiness limit
        if city.civ.get_happiness() < -8:
            return 0.0
            
        if city.civ.is_ai():
            # When Happy, 2 production is better than 1 growth,
            # but setting such by default worsens AI civ citizen assignment,
            # probably due to badly configured personalities not properly weighing food vs non-food yields
            if city.population.population < 5:
                return 2.0
            return 1.5
            
        # Human weights. May be different since AI Happiness is always "easier"
        # Only apply these for Default to not interfere with Focus weights
        if city.get_city_focus() == CityFocus.NoFocus:
            if city.population.population < 5:
                return 2.0
            if surplus_food > city.population.get_food_to_next_population() / (10 * speed):
                return 0.75  # get Growth just under Production
                
        return 1.0

    @staticmethod
    def rank_stats_for_city_work(stats: Stats, city: City, are_we_ranking_specialist: bool, 
                                local_unique_cache: LocalUniqueCache) -> float:
        """Rank stats for city work.
        
        Args:
            stats: The stats to rank
            city: The city to rank for
            are_we_ranking_specialist: Whether we're ranking a specialist
            local_unique_cache: Cache for unique checks
            
        Returns:
            The ranking value for the stats
        """
        city_ai_focus = city.get_city_focus()
        yield_stats = stats.clone()
        civ_personality = city.civ.get_personality()
        city_stats_obj = city.city_stats
        civ_info = city.civ
        all_techs_are_researched = civ_info.tech.all_techs_are_researched()

        if are_we_ranking_specialist:
            # If you have the Food Bonus, count as 1 extra food production (base is 2food)
            for unique in local_unique_cache.for_city_get_matching_uniques(city, UniqueType.FoodConsumptionBySpecialists):
                if city.matches_filter(unique.params[1]):
                    yield_stats.food -= (float(unique.params[0]) / 100.0) * 2.0  # base 2 food per Pop
            # Specialist Happiness Percentage Change 0f-1f
            for unique in local_unique_cache.for_city_get_matching_uniques(city, UniqueType.UnhappinessFromPopulationTypePercentageChange):
                if unique.params[1] == "Specialists" and city.matches_filter(unique.params[2]):
                    yield_stats.happiness -= (float(unique.params[0]) / 100.0)  # relative val is negative, make positive

        surplus_food = city.city_stats.current_city_stats[Stat.Food]
        starving = surplus_food < 0
        
        # If current Production converts Food into Production, then calculate increased Production Yield
        if city_stats_obj.can_convert_food_to_production(surplus_food, city.city_constructions.get_current_construction()):
            # calculate delta increase of food->prod. This isn't linear
            yield_stats.production += (city_stats_obj.get_production_from_excessive_food(surplus_food + yield_stats.food) - 
                                     city_stats_obj.get_production_from_excessive_food(surplus_food))
            yield_stats.food = 0.0  # all food goes to 0

        # Split Food Yield into feedFood, amount needed to not Starve
        # and growthFood, any amount above that
        feed_food = 0.0
        if starving:
            feed_food = max(min(yield_stats.food, -surplus_food), 0.0)
        growth_food = yield_stats.food - feed_food  # how much extra Food we yield
        
        # Avoid Growth, only count Food that gets you not-starving, but no more
        if city.avoid_growth:
            growth_food = 0.0
            
        yield_stats.food = 1.0
        
        # Apply base weights
        yield_stats.apply_ranking_weights()

        food_base_weight = yield_stats.food

        # If starving, need Food, so feedFood > 0
        # scale feedFood by 14(base weight)*8(super important)
        # By only scaling what we need to reach Not Starving by x8, we can pick a tile that gives
        # exactly as much Food as we need to Not Starve that also has other good yields instead of
        # always picking the Highest Food tile until Not Starving
        yield_stats.food = feed_food * (food_base_weight * 8)
        # growthFood is any additional food not required to meet Starvation

        # Growth is penalized when Unhappy, see GlobalUniques.json
        # No Growth if <-10, 1/4 if <0
        # Reusing food growth code from CityStats.updateFinalStatList()
        growth_nullifying_unique = city.get_matching_uniques(UniqueType.NullifiesGrowth).first_or_none()
        
        if growth_nullifying_unique is None:  # if not nullified
            new_growth_food = growth_food  # running count of growthFood
            city_stats = CityStats(city)
            growth_bonuses = city_stats.get_growth_bonus(growth_food)
            for growth_bonus in growth_bonuses:
                new_growth_food += growth_bonus.value.food
            if city.is_we_love_the_king_day_active() and city.civ.get_happiness() >= 0:
                new_growth_food += growth_food / 4
            new_growth_food = max(new_growth_food, 0.0)  # floor to 0 for safety
            
            yield_stats.food += new_growth_food * food_base_weight * Automation._get_food_mod_weight(city, surplus_food)

        if city.population.population < 10:
            # "small city" - we care more about food and less about global problems like gold science and culture
            # Food already handled above. Gold/Culture have low weights in Stats already
            yield_stats.science /= 2

        if city.civ.stats.stats_for_next_turn.gold < 0:
            # We have a global problem, we need to deal with it before it leads to science loss
            yield_stats.gold *= 2

        if city.civ.get_happiness() < 0:
            yield_stats.happiness *= 2

        if all_techs_are_researched:
            # Science is useless at this point
            yield_stats.science *= 0

        if isinstance(city.city_constructions.get_current_construction(), PerpetualConstruction):
            # With 4:1 conversion of production to science, production is overvalued by a factor (12*4)/7 = 6.9
            yield_stats.production /= 6

        for stat in Stat:
            if city.civ.wants_to_focus_on(stat):
                yield_stats[stat] *= 2.0

            scaled_focus = civ_personality.scaled_focus(PersonalityValue[stat])
            if scaled_focus != 1.0:
                yield_stats[stat] *= scaled_focus

        # Apply City focus
        city_ai_focus.apply_weight_to(yield_stats)

        return sum(yield_stats.values) 

    @staticmethod
    def try_train_military_unit(city: City) -> None:
        """Try to train a military unit in the city.
        
        Args:
            city: The city to train the unit in
        """
        if city.is_puppet:
            return
        if (isinstance(city.city_constructions.get_current_construction(), BaseUnit) 
            and city.city_constructions.get_current_construction().is_military):
            return  # already training a military unit
            
        chosen_unit_name = Automation.choose_military_unit(city, city.civ.game_info.ruleset.units.values())
        if chosen_unit_name is not None:
            city.city_constructions.current_construction_from_queue = chosen_unit_name

    @staticmethod
    def provides_unneeded_carrying_slots(base_unit: BaseUnit, civ_info: Civilization) -> bool:
        """Check if a unit provides unneeded carrying slots.
        
        Args:
            base_unit: The unit to check
            civ_info: The civilization to check for
            
        Returns:
            True if the unit provides unneeded carrying slots
        """
        # Simplified, will not work for crazy mods with more than one carrying filter for a unit
        carry_unique = base_unit.get_matching_uniques(UniqueType.CarryAirUnits).first()
        carry_filter = carry_unique.params[1]

        def get_carry_amount(map_unit: MapUnit) -> int:
            map_unit_carry_unique = map_unit.get_matching_uniques(UniqueType.CarryAirUnits).first_or_none()
            if map_unit_carry_unique is None:
                return 0
            if map_unit_carry_unique.params[1] != carry_filter:
                return 0  # Carries a different type of unit
            return (int(map_unit_carry_unique.params[0]) +
                   sum(int(it.params[0]) for it in map_unit.get_matching_uniques(UniqueType.CarryExtraAirUnits)
                       if it.params[1] == carry_filter))

        total_carriable_units = sum(1 for unit in civ_info.units.get_civ_units() 
                                  if unit.matches_filter(carry_filter))
        total_carrying_slots = sum(get_carry_amount(unit) 
                                 for unit in civ_info.units.get_civ_units())
                
        return total_carriable_units < total_carrying_slots

    @staticmethod
    def choose_military_unit(city: City, available_units: Sequence[BaseUnit]) -> Optional[str]:
        """Choose a military unit to train in the city.
        
        Args:
            city: The city to train the unit in
            available_units: Sequence of available units to choose from
            
        Returns:
            The name of the chosen unit, or None if no suitable unit was found
        """
        current_choice = city.city_constructions.get_current_construction()
        if (isinstance(current_choice, BaseUnit) and not current_choice.is_civilian()):
            return city.city_constructions.current_construction_from_queue

        # if not coastal, removeShips == true so don't even consider ships
        remove_ships = True
        is_missing_naval_units_for_city_defence = False

        def is_naval_melee_unit(unit: BaseUnit) -> bool:
            return unit.is_melee() and unit.type.is_water_unit()

        if city.is_coastal():
            # in the future this could be simplified by assigning every distinct non-lake body of
            # water their own ID like a continent ID
            find_water_connected_cities_and_enemies = BFS(city.get_center_tile(), 
                                                        lambda t: t.is_water or t.is_city_center())
            find_water_connected_cities_and_enemies.step_to_end()

            number_of_our_connected_cities = sum(1 for t in find_water_connected_cities_and_enemies.get_reached_tiles()
                                               if t.is_city_center() and t.get_owner() == city.civ)
            number_of_our_naval_melee_units = sum(
                sum(1 for unit in t.get_units() if is_naval_melee_unit(unit.base_unit))
                for t in find_water_connected_cities_and_enemies.get_reached_tiles()
            )
                
            is_missing_naval_units_for_city_defence = number_of_our_connected_cities > number_of_our_naval_melee_units

            remove_ships = not any(
                (t.is_city_center() and t.get_owner() != city.civ)
                or (t.military_unit is not None and t.military_unit.civ != city.civ)
                for t in find_water_connected_cities_and_enemies.get_reached_tiles()
            )  # there is absolutely no reason for you to make water units on this body of water.

        military_units = [
            unit for unit in available_units
            if (unit.is_military
                and (not remove_ships or not unit.is_water_unit)
                and Automation.allow_spending_resource(city.civ, unit)
                and not (unit.has_unique(UniqueType.CarryAirUnits)
                        and Automation.provides_unneeded_carrying_slots(unit, city.civ))
                and unit.is_buildable(city.city_constructions))
        ]

        if not military_units:
            return None

        chosen_unit: BaseUnit
        if (not city.civ.is_at_war()
                and any(t.get_center_tile().military_unit is None for t in city.civ.cities)
                and any(u.is_ranged() for u in military_units)):  # this is for city defence so get a ranged unit if we can
            chosen_unit = max((u for u in military_units if u.is_ranged()), key=lambda u: u.cost)
        elif is_missing_naval_units_for_city_defence and any(is_naval_melee_unit(u) for u in military_units):
            chosen_unit = max((u for u in military_units if is_naval_melee_unit(u)), key=lambda u: u.cost)
        else:  # randomize type of unit and take the most expensive of its kind
            best_units_for_type = {}
            for unit in military_units:
                if (unit.unit_type not in best_units_for_type 
                    or best_units_for_type[unit.unit_type].cost < unit.cost):
                    best_units_for_type[unit.unit_type] = unit
                    
            # Check the maximum force evaluation for the shortlist so we can prune useless ones (ie scouts)
            best_force = max((u.get_force_evaluation() for u in best_units_for_type.values()), default=0)
            eligible_units = [u for u in best_units_for_type.values()
                            if u.unique_to is not None or u.get_force_evaluation() > best_force / 3]
            if not eligible_units:
                return None
            chosen_unit = eligible_units[0]  # random.choice(eligible_units)  # TODO: Implement random choice
            
        return chosen_unit.name

    @staticmethod
    def afraid_of_barbarians(civ_info: Civilization) -> bool:
        """Determine if a civilization should be afraid of barbarians.
        
        Args:
            civ_info: The civilization to check
            
        Returns:
            True if the civilization should be afraid of barbarians
        """
        if civ_info.is_city_state or civ_info.is_barbarian:
            return False

        if civ_info.game_info.game_parameters.no_barbarians:
            return False  # If there are no barbarians we are not afraid

        speed = civ_info.game_info.speed
        if civ_info.game_info.turns > 200 * speed.barbarian_modifier:
            return False  # Very late in the game we are not afraid

        multiplier = 1.3 if civ_info.game_info.game_parameters.raging_barbarians else 1.0  # We're slightly more afraid of raging barbs

        # Past the early game we are less afraid
        if civ_info.game_info.turns > 120 * speed.barbarian_modifier * multiplier:
            multiplier /= 2

        # If we have no cities or a lot of units we are not afraid
        if not civ_info.cities or len(civ_info.units.get_civ_units()) >= 4 * multiplier:
            return False

        # If we have vision of our entire starting continent (ish) we are not afraid
        civ_info.game_info.tile_map.assign_continents(TileMap.AssignContinentsMode.Ensure)
        starting_continent = civ_info.get_capital().get_center_tile().get_continent()
        starting_continent_size = civ_info.game_info.tile_map.continent_sizes.get(starting_continent)
        if starting_continent_size is not None and starting_continent_size < len(civ_info.viewable_tiles) * multiplier:
            return False

        # Otherwise we're afraid
        return True

    @staticmethod
    def allow_automated_construction(civ_info: Civilization, city: City, 
                                   construction: INonPerpetualConstruction) -> bool:
        """Check if automated construction is allowed.
        
        Args:
            civ_info: The civilization to check for
            city: The city to check in
            construction: The construction to check
            
        Returns:
            True if automated construction is allowed
        """
        return (Automation.allow_create_improvement_buildings(civ_info, city, construction)
                and Automation.allow_spending_resource(civ_info, construction, city))

    @staticmethod
    def allow_create_improvement_buildings(civ_info: Civilization, city: City, 
                                         construction: INonPerpetualConstruction) -> bool:
        """Check if creating improvement buildings is allowed.
        
        Args:
            civ_info: The civilization to check for
            city: The city to check in
            construction: The construction to check
            
        Returns:
            True if creating improvement buildings is allowed
        """
        if not isinstance(construction, Building):
            return True
        if not construction.has_create_one_improvement_unique():
            return True  # redundant but faster???
            
        improvement = construction.get_improvement_to_create(city.get_ruleset(), civ_info)
        if improvement is None:
            return True
            
        return any(t.improvement_functions.can_build_improvement(improvement, civ_info)
                  for t in city.get_tiles())

    @staticmethod
    def allow_spending_resource(civ_info: Civilization, construction: INonPerpetualConstruction, 
                               city_info: Optional[City] = None) -> bool:
        """Check if spending resources on a construction is allowed.
        
        Args:
            civ_info: The civilization to check for
            construction: The construction to check
            city_info: Optional city to check in
            
        Returns:
            True if spending resources is allowed
        """
        # City states do whatever they want
        if civ_info.is_city_state:
            return True

        # Spaceships are always allowed
        if construction.name in civ_info.game_info.space_resources:
            return True

        required_resources = (construction.get_resource_requirements_per_turn(civ_info.state)
                            if isinstance(construction, BaseUnit)
                            else construction.get_resource_requirements_per_turn(city_info.state if city_info else civ_info.state))
                            
        # Does it even require any resources?
        if not required_resources:
            return True

        civ_resources = civ_info.get_civ_resources_by_name()

        # Rule of thumb: reserve 2-3 for spaceship, then reserve half each for buildings and units
        # Assume that no buildings provide any resources
        for resource, amount in required_resources.items():
            # Also count things under construction
            future_for_units = 0
            future_for_buildings = 0

            for city in civ_info.cities:
                other_construction = city.city_constructions.get_current_construction()
                if isinstance(other_construction, Building):
                    future_for_buildings += other_construction.get_resource_requirements_per_turn(city.state).get(resource, 0)
                else:
                    future_for_units += other_construction.get_resource_requirements_per_turn(civ_info.state).get(resource, 0)

            # Make sure we have some for space
            if (resource in civ_info.game_info.space_resources 
                and civ_resources[resource] - amount - future_for_buildings - future_for_units
                < Automation.get_reserved_space_resource_amount(civ_info)):
                return False

            # Assume buildings remain useful
            needed_for_building = civ_info.cache.last_era_resource_used_for_building.get(resource) is not None
            # Don't care about old units
            needed_for_units = (civ_info.cache.last_era_resource_used_for_unit.get(resource) is not None
                              and civ_info.cache.last_era_resource_used_for_unit[resource] >= civ_info.get_era_number())

            # No need to save for both
            if not needed_for_building or not needed_for_units:
                continue

            used_for_units = sum(-r.amount for r in civ_info.detailed_civ_resources 
                               if r.resource.name == resource and r.origin == "Units")
            used_for_buildings = sum(-r.amount for r in civ_info.detailed_civ_resources 
                                   if r.resource.name == resource and r.origin == "Buildings")

            if isinstance(construction, Building):
                # Will more than half the total resources be used for buildings after this construction?
                if (civ_resources[resource] + used_for_units 
                    < used_for_buildings + amount + future_for_buildings):
                    return False
            else:
                # Will more than half the total resources be used for units after this construction?
                if (civ_resources[resource] + used_for_buildings 
                    < used_for_units + amount + future_for_units):
                    return False
                    
        return True

    @staticmethod
    def get_reserved_space_resource_amount(civ_info: Civilization) -> int:
        """Get the amount of space resources to reserve.
        
        Args:
            civ_info: The civilization to check for
            
        Returns:
            The amount of space resources to reserve
        """
        return 3 if civ_info.wants_to_focus_on(Victory.Focus.Science) else 2 

    @staticmethod
    def threat_assessment(assessor: Civilization, assessed: Civilization) -> 'ThreatLevel':
        """Assess the threat level of one civilization to another.
        
        Args:
            assessor: The civilization doing the assessment
            assessed: The civilization being assessed
            
        Returns:
            The threat level of the assessed civilization
        """
        power_level_comparison = (assessed.get_stat_for_ranking(RankingType.Force) / 
                                assessor.get_stat_for_ranking(RankingType.Force))
                                
        if power_level_comparison > 2:
            return ThreatLevel.VeryHigh
        elif power_level_comparison > 1.5:
            return ThreatLevel.High
        elif power_level_comparison < 0.5:
            return ThreatLevel.VeryLow
        elif power_level_comparison < (1 / 1.5):
            return ThreatLevel.Low
        else:
            return ThreatLevel.Medium

    @staticmethod
    def get_tile_for_construction_improvement(city: City, improvement: TileImprovement) -> Optional[Tile]:
        """Get the best tile for a construction improvement.
        
        Args:
            city: The city to check
            improvement: The improvement to place
            
        Returns:
            The best tile for the improvement, or None if no suitable tile was found
        """
        local_unique_cache = LocalUniqueCache()
        return max(
            (tile for tile in city.get_tiles()
             if (tile.get_tile_improvement() is None or 
                 not tile.get_tile_improvement().has_unique(UniqueType.AutomatedUnitsWillNotReplace,
                                                          StateForConditionals(city.civ, city, tile=tile)))
             and tile.improvement_functions.can_build_improvement(improvement, city.civ)),
            key=lambda t: Automation.rank_tile_for_city_work(t, city, local_unique_cache),
            default=None
        )

    @staticmethod
    def rank_tile(tile: Optional[Tile], civ_info: Civilization, 
                  local_unique_cache: LocalUniqueCache) -> float:
        """Rank a tile for any purpose except the expansion algorithm of cities.
        
        Args:
            tile: The tile to rank
            civ_info: The civilization to rank for
            local_unique_cache: Cache for unique checks
            
        Returns:
            The ranking value for the tile
        """
        if tile is None:
            return 0.0
            
        tile_owner = tile.get_owner()
        if tile_owner is not None and tile_owner != civ_info:
            return 0.0  # Already belongs to another civilization, useless to us
            
        stats = tile.stats.get_tile_stats(None, civ_info, local_unique_cache)
        rank = Automation.rank_stats_value(stats, civ_info)
        
        if tile.improvement is None:
            rank += 0.5  # improvement potential!
        if tile.is_pillaged():
            rank += 0.6
        if tile.has_viewable_resource(civ_info):
            resource = tile.tile_resource
            if resource.resource_type != ResourceType.Bonus:
                rank += 1.0  # for usage
            if tile.improvement is None:
                rank += 1.0  # improvement potential - resources give lots when improved!
            if tile.is_pillaged():
                rank += 1.1  # even better, repair is faster
                
        return rank

    @staticmethod
    def rank_tile_for_expansion(tile: Tile, city: City, 
                               local_unique_cache: LocalUniqueCache) -> int:
        """Rank a tile for the expansion algorithm of cities.
        
        Args:
            tile: The tile to rank
            city: The city to rank for
            local_unique_cache: Cache for unique checks
            
        Returns:
            The ranking value for the tile (lower is better)
        """
        # https://github.com/Gedemon/Civ5-DLL/blob/aa29e80751f541ae04858b6d2a2c7dcca454201e/CvGameCoreDLL_Expansion1/CvCity.cpp#L10301
        # Apparently this is not the full calculation. The exact tiles are also
        # dependent on which tiles are between the chosen tile and the city center
        # Exact details are not implemented, but can be found in CvAStar.cpp:2119,
        # function `InfluenceCost()`.
        # Implementing these will require an additional variable for each terrainType
        distance = tile.aerial_distance_to(city.get_center_tile())

        # Higher score means tile is less likely to be picked
        score = distance * 100

        # Resources are good: less points
        if tile.has_viewable_resource(city.civ):
            if tile.tile_resource.resource_type != ResourceType.Bonus:
                score -= 105
            elif distance <= city.get_work_range():
                score -= 104
        else:
            # Water tiles without resources aren't great
            if tile.is_water:
                score += 25
            # Can't work it anyways
            if distance > city.get_work_range():
                score += 100

        if tile.natural_wonder is not None:
            score -= 105

        # Straight up take the sum of all yields
        score -= int(sum(tile.stats.get_tile_stats(city, city.civ, local_unique_cache).values()))

        # Check if we get access to better tiles from this tile
        adjacent_natural_wonder = False

        for adjacent_tile in tile.neighbors:
            if adjacent_tile.get_owner() is None:
                adjacent_distance = city.get_center_tile().aerial_distance_to(adjacent_tile)
                if (adjacent_tile.has_viewable_resource(city.civ)
                    and (adjacent_distance < city.get_work_range()
                         or adjacent_tile.tile_resource.resource_type != ResourceType.Bonus)):
                    score -= 1
                if adjacent_tile.natural_wonder is not None:
                    if adjacent_distance < city.get_work_range():
                        adjacent_natural_wonder = True
                    score -= 1
                    
        if adjacent_natural_wonder:
            score -= 1

        # Tiles not adjacent to owned land are very hard to acquire
        if not any(t.get_city() is not None and t.get_city().id == city.id 
                  for t in tile.neighbors):
            score += 1000

        return score

    @staticmethod
    def rank_stats_value(stats: Stats, civ_info: Civilization) -> float:
        """Rank the value of stats for a civilization.
        
        Args:
            stats: The stats to rank
            civ_info: The civilization to rank for
            
        Returns:
            The ranking value for the stats
        """
        rank = 0.0
        rank += stats.food * 1.2  # food get more value to keep city growing

        if civ_info.gold < 0 and civ_info.stats.stats_for_next_turn.gold <= 0:
            rank += stats.gold  # build more gold infrastructure if in serious gold problems
        else:
            rank += stats.gold / 3  # Gold is valued less than is the case for citizen assignment,
            # otherwise the AI would replace tiles with trade posts upon entering a golden age,
            # and replace the trade post again when the golden age ends.
            # We need a way to take golden age gold into account before the GA actually takes place
            
        rank += stats.happiness
        rank += stats.production
        rank += stats.science
        rank += stats.culture
        rank += stats.faith
        return rank


class ThreatLevel:
    """Enumeration of threat levels."""
    VeryLow = "VeryLow"
    Low = "Low"
    Medium = "Medium"
    High = "High"
    VeryHigh = "VeryHigh" 