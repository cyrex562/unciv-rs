use crate::map::bfs::BFS;
use crate::ruleset::building::Building;
use crate::map::unit::BaseUnit;
use crate::civilization::civilization::Civilization;
use crate::city::city::City;
use crate::city::city_focus::CityFocus;
use crate::city::city_stats::CityStats;
use crate::ruleset::construction::{INonPerpetualConstruction, PerpetualConstruction};
use crate::ai::personality::PersonalityValue;
use crate::ruleset::tile::resource_type::ResourceType;
use crate::ruleset::tile::tile_improvement::TileImprovement;
use crate::stats::stats::Stats;
use crate::unique::state_for_conditionals::StateForConditionals;
use crate::unique::unique::LocalUniqueCache;
use crate::unique::UniqueType;
use crate::ruleset::victory::Victory;
use crate::stats::stat::{Stat};
use crate::map::tile_map::TileMap;
use crate::tile::tile::Tile;
use crate::map::MapUnit;
use crate::ranking_type::RankingType;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

pub struct Automation;

impl Automation {
    pub fn rank_tile_for_city_work(tile: &Tile, city: &City, local_unique_cache: &LocalUniqueCache) -> f32 {
        let stats = tile.stats.get_tile_stats(city, &city.civ, local_unique_cache);
        Self::rank_stats_for_city_work(&stats, city, false, local_unique_cache)
    }

    pub fn rank_specialist(specialist: &str, city: &City, local_unique_cache: &LocalUniqueCache) -> f32 {
        let stats = city.city_stats.get_stats_of_specialist(specialist, local_unique_cache);
        let mut rank = Self::rank_stats_for_city_work(&stats, city, true, local_unique_cache);

        // Derive GPP score
        let mut gpp = 0.0;
        if let Some(specialist_info) = city.get_ruleset().specialists.get(specialist) {
            gpp = specialist_info.great_person_points.sum_values() as f32;
        }
        gpp = gpp * (100.0 + city.current_gpp_bonus) / 100.0;
        rank += gpp * 3.0; // GPP weight
        rank
    }

    fn get_food_mod_weight(city: &City, surplus_food: f32) -> f32 {
        let speed = city.civ.game_info.speed.modifier;

        // Zero out Growth if close to Unhappiness limit
        if city.civ.get_happiness() < -8 {
            return 0.0;
        }

        if city.civ.is_ai() {
            // When Happy, 2 production is better than 1 growth,
            // but setting such by default worsens AI civ citizen assignment
            if city.population.population < 5 {
                return 2.0;
            }
            return 1.5;
        }

        // Human weights
        if city.get_city_focus() == CityFocus::NoFocus {
            if city.population.population < 5 {
                return 2.0;
            }
            if surplus_food > city.population.get_food_to_next_population() / (10.0 * speed) {
                return 0.75; // get Growth just under Production
            }
        }

        1.0
    }

    pub fn rank_stats_for_city_work(
        stats: &Stats,
        city: &City,
        are_we_ranking_specialist: bool,
        local_unique_cache: &LocalUniqueCache
    ) -> f32 {
        let city_ai_focus = city.get_city_focus();
        let mut yield_stats = stats.clone();
        let civ_personality = city.civ.get_personality();
        let city_stats_obj = &city.city_stats;
        let civ_info = &city.civ;
        let all_techs_are_researched = civ_info.tech.all_techs_are_researched();

        if are_we_ranking_specialist {
            // Handle Food Bonus for specialists
            for unique in local_unique_cache.for_city_get_matching_uniques(
                city,
                UniqueType::FoodConsumptionBySpecialists,
                &StateForConditionals::new()
            ) {
                if city.matches_filter(&unique.params[1], &city.civ, city) {
                    yield_stats.food -= (unique.params[0].parse::<f32>().unwrap() / 100.0) * 2.0;
                }
            }

            // Handle Specialist Happiness
            for unique in local_unique_cache.for_city_get_matching_uniques(
                city,
                UniqueType::UnhappinessFromPopulationTypePercentageChange,
                &StateForConditionals::new()
            ) {
                if unique.params[1] == "Specialists" && city.matches_filter(&unique.params[2], &city.civ, city) {
                    yield_stats.happiness -= unique.params[0].parse::<f32>().unwrap() / 100.0;
                }
            }
        }

        let surplus_food = city.city_stats.get_current_city_stats()[Stat::Food];
        let starving = surplus_food < 0.0;

        // Handle Food to Production conversion
        if city_stats_obj.can_convert_food_to_production(surplus_food, &city.city_constructions.get_current_construction()) {
            yield_stats.production += city_stats_obj.get_production_from_excessive_food(surplus_food + yield_stats.food)
                - city_stats_obj.get_production_from_excessive_food(surplus_food);
            yield_stats.food = 0.0;
        }

        // Split Food Yield
        let mut feed_food = 0.0;
        if starving {
            feed_food = (yield_stats.food).min(-surplus_food).max(Some(0.0));
        }
        let mut growth_food = yield_stats.food - feed_food;

        if city.avoid_growth {
            growth_food = 0.0;
        }
        yield_stats.food = 1.0;

        // Apply base weights
        yield_stats.apply_ranking_weights();
        let food_base_weight = yield_stats.food;

        // Scale feed food
        yield_stats.food = feed_food * (food_base_weight * 8.0);

        // Handle growth food
        if let None = city.get_matching_uniques(UniqueType::NullifiesGrowth, Some(&city.civ), city).first() {
            let mut new_growth_food = growth_food;
            let city_stats = CityStats::new(city);
            let growth_bonuses = city_stats.get_growth_bonus(growth_food);

            for growth_bonus in growth_bonuses {
                new_growth_food += growth_bonus.value.food;
            }

            if city.is_we_love_the_king_day_active() && city.civ.get_happiness() >= 0 {
                new_growth_food += growth_food / 4.0;
            }

            new_growth_food = new_growth_food.max(0.0);
            yield_stats.food += new_growth_food * food_base_weight * Self::get_food_mod_weight(city, surplus_food);
        }

        // Apply various modifiers
        if city.population.population < 10 {
            yield_stats.science /= 2.0;
        }

        if city.civ.stats.stats_for_next_turn.gold < 0 {
            yield_stats.gold *= 2.0;
        }

        if city.civ.get_happiness() < 0 {
            yield_stats.happiness *= 2.0;
        }

        if all_techs_are_researched {
            yield_stats.science *= 0.0;
        }

        if city.city_constructions.get_current_construction().is::<PerpetualConstruction>() {
            yield_stats.production /= 6.0;
        }

        // Apply personality and focus weights
        for stat in Stat::entries() {
            if city.civ.wants_to_focus_on(stat) {
                yield_stats[stat] *= 2.0;
            }

            let scaled_focus = civ_personality.scaled_focus(PersonalityValue[stat]);
            if scaled_focus != 1.0 {
                yield_stats[stat] *= scaled_focus;
            }
        }

        city_ai_focus.apply_weight_to(&mut yield_stats);
        yield_stats.values.sum()
    }

    pub fn try_train_military_unit(city: &mut City) {
        if city.is_puppet {
            return;
        }
        if let Some(current) = city.city_constructions.get_current_construction().downcast_ref::<BaseUnit>() {
            if current.is_military {
                return; // already training a military unit
            }
        }

        if let Some(chosen_unit_name) = Self::choose_military_unit(city, city.civ.game_info.ruleset.units.values()) {
            city.city_constructions.current_construction_from_queue = chosen_unit_name;
        }
    }

    fn provides_unneeded_carrying_slots(base_unit: &BaseUnit, civ_info: &Civilization) -> bool {
        // Simplified, will not work for crazy mods with more than one carrying filter for a unit
        let carry_unique = base_unit.get_matching_uniques(UniqueType::CarryAirUnits).first()
            .expect("Unit should have CarryAirUnits unique");
        let carry_filter = &carry_unique.params[1];

        let get_carry_amount = |map_unit: &MapUnit| -> i32 {
            let map_unit_carry_unique = map_unit.get_matching_uniques(UniqueType::CarryAirUnits).first();
            if map_unit_carry_unique.params[1] != *carry_filter {
                return 0; // Carries a different type of unit
            }
            map_unit_carry_unique.params[0].parse::<i32>().unwrap() +
                map_unit.get_matching_uniques(UniqueType::CarryExtraAirUnits)
                    .filter(|it| it.params[1] == *carry_filter)
                    .map(|it| it.params[0].parse::<i32>().unwrap())
                    .sum::<i32>()
        };

        let total_carriable_units = civ_info.units.get_civ_units()
            .filter(|it| it.matches_filter(carry_filter))
            .count();
        let total_carrying_slots = civ_info.units.get_civ_units()
            .map(&get_carry_amount)
            .sum::<i32>();

        total_carriable_units < total_carrying_slots as usize
    }

    pub fn choose_military_unit(city: &City, available_units: impl Iterator<Item = &BaseUnit>) -> Option<String> {
        let current_choice = city.city_constructions.get_current_construction();
        if let Some(unit) = current_choice.downcast_ref::<BaseUnit>() {
            if !unit.is_civilian() {
                return Some(city.city_constructions.current_construction_from_queue.clone());
            }
        }

        let mut remove_ships = true;
        let mut is_missing_naval_units_for_city_defence = false;

        fn is_naval_melee_unit(unit: &BaseUnit) -> bool {
            unit.is_melee() && unit.unit_type.is_water_unit()
        }

        if city.is_coastal() {
            let find_water_connected_cities_and_enemies = BFS::new(city.get_center_tile(), |it| it.is_water || it.is_city_center());
            find_water_connected_cities_and_enemies.step_to_end();

            let number_of_our_connected_cities = find_water_connected_cities_and_enemies.get_reached_tiles()
                .filter(|it| it.is_city_center() && it.get_owner() == Some(&city.civ))
                .count();
            let number_of_our_naval_melee_units = find_water_connected_cities_and_enemies.get_reached_tiles()
                .map(|it| it.get_units().filter(|u| is_naval_melee_unit(&u.base_unit)).count())
                .sum();

            is_missing_naval_units_for_city_defence = number_of_our_connected_cities > number_of_our_naval_melee_units;

            remove_ships = find_water_connected_cities_and_enemies.get_reached_tiles().none(|it| {
                (it.is_city_center() && it.get_owner() != Some(&city.civ))
                    || (it.military_unit.is_some() && it.military_unit.as_ref().unwrap().civ != city.civ)
            });
        }

        let military_units: Vec<&BaseUnit> = available_units
            .filter(|it| it.is_military)
            .filter(|it| !remove_ships || !it.is_water_unit)
            .filter(|it| Self::allow_spending_resource(&city.civ, *it, None))
            .filter(|it| {
                !(it.has_unique(UniqueType::CarryAirUnits)
                    && Self::provides_unneeded_carrying_slots(it, &city.civ))
            })
            .filter(|it| it.is_buildable(&city.city_constructions))
            .collect();

        let chosen_unit = if !city.civ.is_at_war()
            && city.civ.cities.iter().any(|it| it.get_center_tile().military_unit.is_none())
            && military_units.iter().any(|it| it.is_ranged())
        {
            military_units.iter()
                .filter(|it| it.is_ranged())
                .max_by_key(|it| it.cost)?
        } else if is_missing_naval_units_for_city_defence
            && military_units.iter().any(|it| is_naval_melee_unit(it))
        {
            military_units.iter()
                .filter(|it| is_naval_melee_unit(it))
                .max_by_key(|it| it.cost)?
        } else {
            let mut best_units_for_type = HashMap::new();
            for unit in military_units {
                if !best_units_for_type.contains_key(&unit.unit_type)
                    || best_units_for_type[&unit.unit_type].cost < unit.cost
                {
                    best_units_for_type.insert(unit.unit_type.clone(), unit);
                }
            }

            let best_force = best_units_for_type.values()
                .map(|it| it.get_force_evaluation())
                .max()?;

            best_units_for_type.values()
                .filter(|it| it.unique_to.is_some() || it.get_force_evaluation() > best_force / 3)
                .collect::<Vec<_>>()
                .choose(&mut rand::thread_rng())?
        };

        Some(chosen_unit.name.clone())
    }

    pub fn afraid_of_barbarians(civ_info: &Civilization) -> bool {
        if civ_info.is_city_state || civ_info.is_barbarian {
            return false;
        }

        if civ_info.game_info.game_parameters.no_barbarians {
            return false;
        }

        let speed = civ_info.game_info.speed;
        if civ_info.game_info.turns > 200 * speed.barbarian_modifier {
            return false;
        }

        let mut multiplier = if civ_info.game_info.game_parameters.raging_barbarians {
            1.3
        } else {
            1.0
        };

        if civ_info.game_info.turns > 120 * speed.barbarian_modifier * multiplier {
            multiplier /= 2.0;
        }

        if civ_info.cities.is_empty() || civ_info.units.get_civ_units().count() >= (4.0 * multiplier) as usize {
            return false;
        }

        civ_info.game_info.tile_map.assign_continents(TileMap::AssignContinentsMode::Ensure);
        let starting_continent = civ_info.get_capital().unwrap().get_center_tile().get_continent();
        let starting_continent_size = civ_info.game_info.tile_map.continent_sizes[&starting_continent];

        if let Some(size) = starting_continent_size {
            if size < civ_info.viewable_tiles.len() * multiplier as usize {
                return false;
            }
        }

        true
    }

    pub fn allow_automated_construction(
        civ_info: &Civilization,
        city: &City,
        construction: &dyn INonPerpetualConstruction
    ) -> bool {
        Self::allow_create_improvement_buildings(civ_info, city, construction)
            && Self::allow_spending_resource(civ_info, construction, Some(city))
    }

    fn allow_create_improvement_buildings(
        civ_info: &Civilization,
        city: &City,
        construction: &dyn INonPerpetualConstruction
    ) -> bool {
        if let Some(building) = construction.downcast_ref::<Building>() {
            if !building.has_create_one_improvement_unique() {
                return true;
            }
            if let Some(improvement) = building.get_improvement_to_create(city.get_ruleset(), civ_info) {
                return city.get_tiles().iter().any(|it| {
                    it.improvement_functions.can_build_improvement(&improvement, civ_info)
                });
            }
        }
        true
    }

    pub fn allow_spending_resource(
        civ_info: &Civilization,
        construction: &dyn INonPerpetualConstruction,
        city_info: Option<&City>
    ) -> bool {
        if civ_info.is_city_state {
            return true;
        }

        if civ_info.game_info.space_resources.contains(&construction.name()) {
            return true;
        }

        let required_resources = if let Some(unit) = construction.downcast_ref::<BaseUnit>() {
            unit.get_resource_requirements_per_turn(&civ_info.state)
        } else {
            construction.get_resource_requirements_per_turn(city_info.map_or(Some(&civ_info.state), |c| Some(&c.state)))
        };

        if required_resources.is_empty() {
            return true;
        }

        let civ_resources = civ_info.get_civ_resources_by_name();

        for (resource, amount) in required_resources {
            let mut future_for_units = 0;
            let mut future_for_buildings = 0;

            for city in &civ_info.cities {
                let other_construction = city.city_constructions.get_current_construction();
                if let Some(building) = other_construction.downcast_ref::<Building>() {
                    future_for_buildings += building.get_resource_requirements_per_turn(&city.state)[&resource];
                } else {
                    future_for_units += other_construction.get_resource_requirements_per_turn(&civ_info.state)[&resource];
                }
            }

            if civ_info.game_info.space_resources.contains(&resource)
                && civ_resources[&resource].unwrap() - amount - future_for_buildings - future_for_units
                < Self::get_reserved_space_resource_amount(civ_info)
            {
                return false;
            }

            let needed_for_building = civ_info.cache.last_era_resource_used_for_building[&resource].is_some();
            let needed_for_units = civ_info.cache.last_era_resource_used_for_unit[&resource]
                .map_or(false, |era| era >= civ_info.get_era_number());

            if !needed_for_building || !needed_for_units {
                continue;
            }

            let used_for_units = civ_info.detailed_civ_resources.iter()
                .filter(|it| it.resource.name == resource && it.origin == "Units")
                .map(|it| -it.amount)
                .sum::<i32>();
            let used_for_buildings = civ_info.detailed_civ_resources.iter()
                .filter(|it| it.resource.name == resource && it.origin == "Buildings")
                .map(|it| -it.amount)
                .sum::<i32>();

            if construction.downcast_ref::<Building>().is_some() {
                if civ_resources[&resource].unwrap() + used_for_units < used_for_buildings + amount + future_for_buildings {
                    return false;
                }
            } else {
                if civ_resources[&resource].unwrap() + used_for_buildings < used_for_units + amount + future_for_units {
                    return false;
                }
            }
        }
        true
    }

    pub fn get_reserved_space_resource_amount(civ_info: &Civilization) -> i32 {
        if civ_info.wants_to_focus_on(Victory::Focus::Science) {
            3
        } else {
            2
        }
    }

    pub fn threat_assessment(assessor: &Civilization, assessed: &Civilization) -> ThreatLevel {
        let power_level_comparison = assessed.get_stat_for_ranking(RankingType::Force)
            / assessor.get_stat_for_ranking(RankingType::Force) as f32;

        match power_level_comparison {
            x if x > 2.0 => ThreatLevel::VeryHigh,
            x if x > 1.5 => ThreatLevel::High,
            x if x < 0.5 => ThreatLevel::VeryLow,
            x if x < 1.0/1.5 => ThreatLevel::Low,
            _ => ThreatLevel::Medium,
        }
    }

    pub fn get_tile_for_construction_improvement<'a>(city: &'a City, improvement: &TileImprovement) -> Option<&'a Tile> {
        let local_unique_cache = LocalUniqueCache::new(city.get_ruleset());
        city.get_tiles().iter()
            .filter(|it| {
                !it.get_tile_improvement()
                    .map_or(false, |imp| imp.has_unique(
                        UniqueType::AutomatedUnitsWillNotReplace,
                        StateForConditionals::new()
                    ))
                && it.improvement_functions.can_build_improvement(improvement, &city.civ)
            })
            .max_by(|a, b| {
                let a_tile: &Tile = &***a;
                let b_tile: &Tile = &***b;
                Self::rank_tile_for_city_work(a_tile, city, &local_unique_cache)
                    .partial_cmp(&Self::rank_tile_for_city_work(b_tile, city, &local_unique_cache))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|it| &***it)
    }

    pub fn rank_tile(tile: Option<&Tile>, civ_info: &Civilization, local_unique_cache: &LocalUniqueCache) -> f32 {
        let tile = match tile {
            Some(t) => t,
            None => return 0.0,
        };

        if let Some(tile_owner) = tile.get_owner() {
            if tile_owner != civ_info {
                return 0.0;
            }
        }

        let stats = tile.stats.get_tile_stats(None, civ_info, local_unique_cache);
        let mut rank = Self::rank_stats_value(&stats, civ_info);

        if tile.improvement.is_none() {
            rank += 0.5;
        }
        if tile.is_pillaged() {
            rank += 0.6;
        }
        if tile.has_viewable_resource(civ_info) {
            let resource = tile.tile_resource;
            if resource.resource_type != ResourceType::Bonus {
                rank += 1.0;
            }
            if tile.improvement.is_none() {
                rank += 1.0;
            }
            if tile.is_pillaged() {
                rank += 1.1;
            }
        }
        rank
    }

    pub fn rank_tile_for_expansion(tile: &Tile, city: &City, local_unique_cache: &LocalUniqueCache) -> i32 {
        let distance = tile.aerial_distance_to(&*city.get_center_tile());
        let mut score = distance * 100;

        if tile.has_viewable_resource(&city.civ) {
            if tile.tile_resource.resource_type != ResourceType::Bonus {
                score -= 105;
            } else if distance <= city.get_work_range() {
                score -= 104;
            }
        } else {
            if tile.is_water {
                score += 25;
            }
            if distance > city.get_work_range() {
                score += 100;
            }
        }

        if tile.natural_wonder.is_some() {
            score -= 105;
        }

        score -= tile.stats.get_tile_stats(city, &city.civ, local_unique_cache)
            .values.sum() as i32;

        let mut adjacent_natural_wonder = false;

        for adjacent_tile in tile.neighbors.iter().filter(|it| it.get_owner().is_none()) {
            let adjacent_distance = city.get_center_tile().aerial_distance_to(adjacent_tile);
            if adjacent_tile.has_viewable_resource(&city.civ)
                && (adjacent_distance < city.get_work_range()
                    || adjacent_tile.tile_resource.resource_type != ResourceType::Bonus)
            {
                score -= 1;
            }
            if adjacent_tile.natural_wonder.is_some() {
                if adjacent_distance < city.get_work_range() {
                    adjacent_natural_wonder = true;
                }
                score -= 1;
            }
        }
        if adjacent_natural_wonder {
            score -= 1;
        }

        if tile.neighbors.iter().none(|it| it.get_city().map_or(false, |c| c.id == city.id)) {
            score += 1000;
        }

        score
    }

    pub fn rank_stats_value(stats: &Stats, civ_info: &Civilization) -> f32 {
        let mut rank = 0.0;
        rank += stats.food * 1.2;

        rank += if civ_info.gold < 0 && civ_info.stats.stats_for_next_turn.gold <= 0 {
            stats.gold
        } else {
            stats.gold / 3.0
        };

        rank += stats.happiness;
        rank += stats.production;
        rank += stats.science;
        rank += stats.culture;
        rank += stats.faith;
        rank
    }
}