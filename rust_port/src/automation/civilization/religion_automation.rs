use crate::models::civilization::Civilization;
use crate::models::city::City;
use crate::models::stats::Stat;
use crate::models::religion::{ReligionState, Religion};
use crate::models::unique::UniqueType;
use std::cmp::min;
use rand::Rng;
use crate::models::belief::{Belief, BeliefType};
use crate::models::counter::Counter;
use crate::models::tile::Tile;
use std::collections::HashSet;

/// Contains logic for automating religious decisions and actions
pub struct ReligionAutomation;

impl ReligionAutomation {
    /// Spends faith on religious items and units
    pub fn spend_faith_on_religion(civ_info: &mut Civilization) {
        if civ_info.cities.is_empty() {
            return;
        }

        // Save for great prophet
        if civ_info.religion_manager.religion_state != ReligionState::EnhancedReligion
            && (civ_info.religion_manager.remaining_foundable_religions() != 0
                || civ_info.religion_manager.religion_state > ReligionState::Pantheon)
        {
            Self::buy_great_prophet_in_any_city(civ_info);
            return;
        }

        if civ_info.religion_manager.remaining_foundable_religions() == 0 {
            Self::buy_great_person(civ_info);
            Self::try_buy_any_religious_building(civ_info);
            return;
        }

        // If we don't have majority in all our own cities, build missionaries and inquisitors to solve this
        let cities_without_our_religion = civ_info.cities.iter()
            .filter(|city| city.religion.get_majority_religion() != civ_info.religion_manager.religion)
            .collect::<Vec<_>>();

        let religious_units = civ_info.units.get_civ_units().iter()
            .filter(|unit| unit.has_unique(UniqueType::CanSpreadReligion)
                || unit.has_unique(UniqueType::CanRemoveHeresy))
            .count();

        if cities_without_our_religion.len() > 4 * religious_units {
            let (city, pressure_difference) = cities_without_our_religion.iter()
                .map(|city| (*city, city.religion.get_pressure_deficit(
                    civ_info.religion_manager.religion.as_ref().map(|r| r.name.clone()))))
                .max_by_key(|(_, pressure)| *pressure)
                .unwrap();

            if pressure_difference >= 60 { // AI_PREFER_INQUISITOR_OVER_MISSIONARY_PRESSURE_DIFFERENCE
                Self::buy_inquisitor_near(civ_info, city);
            }
            Self::buy_missionary_in_any_city(civ_info);
            return;
        }

        // Get an inquisitor to defend our holy city
        if let Some(holy_city) = civ_info.religion_manager.get_holy_city() {
            if civ_info.cities.contains(&holy_city)
                && civ_info.units.get_civ_units().iter()
                    .filter(|unit| unit.has_unique(UniqueType::PreventSpreadingReligion))
                    .count() == 0
                && !holy_city.religion.is_protected_by_inquisitor()
            {
                Self::buy_inquisitor_near(civ_info, &holy_city);
                return;
            }
        }

        // Just buy missionaries to spread our religion outside of our civ
        if civ_info.units.get_civ_units().iter()
            .filter(|unit| unit.has_unique(UniqueType::CanSpreadReligion))
            .count() < 4
        {
            Self::buy_missionary_in_any_city(civ_info);
            return;
        }
    }

    fn try_buy_any_religious_building(civ_info: &mut Civilization) {
        for city in &mut civ_info.cities {
            if city.religion.get_majority_religion().is_none() {
                continue;
            }

            let buildings = city.religion.get_majority_religion().unwrap()
                .buildings_purchasable_by_beliefs;

            let building_to_purchase = buildings.iter()
                .map(|name| civ_info.get_equivalent_building(name))
                .filter(|building| building.is_purchasable(&city.city_constructions))
                .filter(|building| {
                    if let Some(cost) = building.get_stat_buy_cost(&city, Stat::Faith) {
                        cost <= civ_info.religion_manager.stored_faith
                    } else {
                        false
                    }
                })
                .min_by_key(|building| building.get_stat_buy_cost(&city, Stat::Faith).unwrap());

            if let Some(building) = building_to_purchase {
                city.city_constructions.purchase_construction(building, -1, true, Stat::Faith);
                return;
            }
        }
    }

    fn buy_missionary_in_any_city(civ_info: &mut Civilization) {
        if civ_info.religion_manager.religion_state < ReligionState::Religion {
            return;
        }

        let missionaries = civ_info.game_info.ruleset.units.values()
            .filter(|unit| unit.has_unique(UniqueType::CanSpreadReligion))
            .map(|unit| civ_info.get_equivalent_unit(unit))
            .collect::<Vec<_>>();

        let (missionary_construction, _) = missionaries.iter()
            .map(|unit| (unit, civ_info.cities.iter()
                .filter(|city| unit.is_purchasable(&city.city_constructions)
                    && unit.can_be_purchased_with_stat(city, Stat::Faith))
                .collect::<Vec<_>>()))
            .filter(|(_, cities)| !cities.is_empty())
            .min_by_key(|(unit, cities)|
                cities.iter()
                    .map(|city| unit.get_stat_buy_cost(city, Stat::Faith).unwrap())
                    .min()
                    .unwrap())
            .map(|(unit, _)| unit)
            .ok_or(())
            .unwrap();

        let has_unique_to_take_civ_religion = missionary_construction
            .has_unique(UniqueType::TakeReligionOverBirthCity);

        let valid_cities_to_buy = civ_info.cities.iter()
            .filter(|city| {
                (has_unique_to_take_civ_religion
                    || city.religion.get_majority_religion() == civ_info.religion_manager.religion)
                    && missionary_construction.get_stat_buy_cost(city, Stat::Faith)
                        .map_or(false, |cost| cost <= civ_info.religion_manager.stored_faith)
                    && missionary_construction.is_purchasable(&city.city_constructions)
                    && missionary_construction.can_be_purchased_with_stat(city, Stat::Faith)
            })
            .collect::<Vec<_>>();

        if valid_cities_to_buy.is_empty() {
            return;
        }

        let cities_with_bonus_charges = valid_cities_to_buy.iter()
            .filter(|city| {
                city.get_matching_uniques(UniqueType::UnitStartingPromotions)
                    .iter()
                    .any(|unique| {
                        let promotion_name = &unique.params[2];
                        city.get_ruleset().unit_promotions.get(promotion_name)
                            .map_or(false, |promotion|
                                promotion.has_unique(UniqueType::CanSpreadReligion))
                    })
            })
            .collect::<Vec<_>>();

        let holy_city = valid_cities_to_buy.iter()
            .find(|city| city.is_holy_city_of(civ_info.religion_manager.religion.as_ref().unwrap().name.clone()));

        let city_to_buy_missionary = if !cities_with_bonus_charges.is_empty() {
            cities_with_bonus_charges[0]
        } else if let Some(city) = holy_city {
            city
        } else {
            valid_cities_to_buy[0]
        };

        city_to_buy_missionary.city_constructions
            .purchase_construction(missionary_construction, -1, true, Stat::Faith);
    }

    fn buy_great_prophet_in_any_city(civ_info: &mut Civilization) {
        if civ_info.religion_manager.religion_state < ReligionState::Religion {
            return;
        }

        let Some(great_prophet_unit) = civ_info.religion_manager.get_great_prophet_equivalent() else {
            return;
        };
        let great_prophet_unit = civ_info.get_equivalent_unit(&great_prophet_unit);

        let city_to_buy_great_prophet = civ_info.cities.iter_mut()
            .filter(|city| great_prophet_unit.is_purchasable(&city.city_constructions))
            .filter(|city| great_prophet_unit.can_be_purchased_with_stat(city, Stat::Faith))
            .filter(|city| {
                great_prophet_unit.get_stat_buy_cost(city, Stat::Faith)
                    .map_or(false, |cost| cost <= civ_info.religion_manager.stored_faith)
            })
            .min_by_key(|city| great_prophet_unit.get_stat_buy_cost(city, Stat::Faith).unwrap());

        if let Some(city) = city_to_buy_great_prophet {
            city.city_constructions.purchase_construction(&great_prophet_unit, -1, true, Stat::Faith);
        }
    }

    fn buy_inquisitor_near(civ_info: &mut Civilization, city: &City) {
        if civ_info.religion_manager.religion_state < ReligionState::Religion {
            return;
        }

        let inquisitors = civ_info.game_info.ruleset.units.values()
            .filter(|unit|
                unit.has_unique(UniqueType::CanRemoveHeresy)
                || unit.has_unique(UniqueType::PreventSpreadingReligion))
            .map(|unit| civ_info.get_equivalent_unit(unit))
            .collect::<Vec<_>>();

        let (inquisitor_construction, _) = inquisitors.iter()
            .map(|unit| (unit, civ_info.cities.iter()
                .filter(|city| unit.is_purchasable(&city.city_constructions)
                    && unit.can_be_purchased_with_stat(city, Stat::Faith))
                .collect::<Vec<_>>()))
            .filter(|(_, cities)| !cities.is_empty())
            .min_by_key(|(unit, cities)|
                cities.iter()
                    .map(|city| unit.get_stat_buy_cost(city, Stat::Faith).unwrap())
                    .min()
                    .unwrap())
            .map(|(unit, _)| unit)
            .ok_or(())
            .unwrap();

        let has_unique_to_take_civ_religion = inquisitor_construction
            .has_unique(UniqueType::TakeReligionOverBirthCity);

        let valid_cities_to_buy = civ_info.cities.iter()
            .filter(|buy_city| {
                (has_unique_to_take_civ_religion
                    || buy_city.religion.get_majority_religion() == civ_info.religion_manager.religion)
                    && inquisitor_construction.get_stat_buy_cost(buy_city, Stat::Faith)
                        .map_or(false, |cost| cost <= civ_info.religion_manager.stored_faith)
                    && inquisitor_construction.is_purchasable(&buy_city.city_constructions)
                    && inquisitor_construction.can_be_purchased_with_stat(buy_city, Stat::Faith)
            });

        let city_to_buy = valid_cities_to_buy
            .min_by_key(|buy_city|
                buy_city.get_center_tile().aerial_distance_to(city.get_center_tile()));

        if let Some(buy_city) = city_to_buy {
            buy_city.city_constructions
                .purchase_construction(inquisitor_construction, -1, true, Stat::Faith);
        }
    }

    fn buy_great_person(civ_info: &mut Civilization) {
        let great_person_units = civ_info.game_info.ruleset.units.values()
            .filter(|unit|
                unit.has_unique(UniqueType::GreatPerson)
                && !unit.has_unique(UniqueType::MayFoundReligion))
            .collect::<Vec<_>>();

        let (great_person_construction, _) = great_person_units.iter()
            .map(|unit| (unit, civ_info.cities.iter()
                .filter(|city| unit.is_purchasable(&city.city_constructions)
                    && unit.can_be_purchased_with_stat(city, Stat::Faith))
                .collect::<Vec<_>>()))
            .filter(|(_, cities)| !cities.is_empty())
            .min_by_key(|(unit, cities)|
                cities.iter()
                    .map(|city| unit.get_stat_buy_cost(city, Stat::Faith).unwrap())
                    .min()
                    .unwrap())
            .map(|(unit, _)| unit)
            .ok_or(())
            .unwrap();

        let valid_cities_to_buy = civ_info.cities.iter()
            .filter(|city|
                great_person_construction.get_stat_buy_cost(city, Stat::Faith)
                    .map_or(false, |cost| cost <= civ_info.religion_manager.stored_faith));

        if let Some(city) = valid_cities_to_buy.next() {
            city.city_constructions
                .purchase_construction(great_person_construction, -1, true, Stat::Faith);
        }
    }

    pub fn rate_belief(civ_info: &Civilization, belief: &Belief) -> f32 {
        let mut score = 0.0; // Roughly equivalent to the sum of stats gained across all cities

        for city in &civ_info.cities {
            for tile in city.get_center_tile().get_tiles_in_distance(city.get_work_range()) {
                let tile_score = Self::belief_bonus_for_tile(belief, tile, city);
                score += tile_score * match () {
                    _ if city.worked_tiles.contains(&tile.position) => 1.0, // worked
                    _ if tile.get_city() == Some(city) => 0.7, // workable
                    _ => 0.5, // unavailable - for now
                } * (rand::thread_rng().gen_range(0.975..1.025));
            }

            score += Self::belief_bonus_for_city(civ_info, belief, city)
                * rand::thread_rng().gen_range(0.95..1.05);
        }

        score += Self::belief_bonus_for_player(civ_info, belief)
            * rand::thread_rng().gen_range(0.85..1.15);

        if belief.belief_type == BeliefType::Pantheon {
            score *= 0.9;
        }

        score
    }

    fn belief_bonus_for_tile(belief: &Belief, tile: &Tile, city: &City) -> f32 {
        let mut bonus_yield = 0.0;
        for unique in &belief.unique_objects {
            match unique.unique_type {
                UniqueType::StatsFromObject => {
                    if (tile.matches_filter(&unique.params[1])
                        && !(tile.last_terrain.has_unique(UniqueType::ProductionBonusWhenRemoved)
                            && tile.last_terrain.matches_filter(&unique.params[1])))
                        || (tile.resource.is_some()
                            && (tile.tile_resource.matches_filter(&unique.params[1])
                                || tile.tile_resource.is_improved_by(&unique.params[1])))
                    {
                        bonus_yield += unique.stats.values().sum::<f32>();
                    }
                }
                UniqueType::StatsFromTilesWithout => {
                    if city.matches_filter(&unique.params[3])
                        && tile.matches_filter(&unique.params[1])
                        && !tile.matches_filter(&unique.params[2])
                    {
                        bonus_yield += unique.stats.values().sum::<f32>();
                    }
                }
                _ => {}
            }
        }
        bonus_yield
    }

    fn belief_bonus_for_city(civ_info: &Civilization, belief: &Belief, city: &City) -> f32 {
        let mut score = 0.0;
        let ruleset = &civ_info.game_info.ruleset;

        for unique in &belief.unique_objects {
            let modifier = 0.5f32.powi(unique.modifiers.len() as i32);

            score += modifier * match unique.unique_type {
                UniqueType::GrowthPercentBonus =>
                    unique.params[0].parse::<f32>().unwrap() / 3.0,
                UniqueType::BorderGrowthPercentage =>
                    -unique.params[0].parse::<f32>().unwrap() / 10.0,
                UniqueType::StrengthForCities =>
                    unique.params[0].parse::<f32>().unwrap() / 10.0,
                UniqueType::CityHealingUnits =>
                    unique.params[1].parse::<f32>().unwrap() / 10.0,
                UniqueType::PercentProductionBuildings =>
                    unique.params[0].parse::<f32>().unwrap() / 3.0,
                UniqueType::PercentProductionWonders =>
                    unique.params[0].parse::<f32>().unwrap() / 3.0,
                UniqueType::PercentProductionUnits =>
                    unique.params[0].parse::<f32>().unwrap() / 3.0,
                UniqueType::StatsFromCitiesOnSpecificTiles => {
                    if city.get_center_tile().matches_filter(&unique.params[1]) {
                        unique.stats.values().sum::<f32>()
                    } else {
                        0.0
                    }
                }
                UniqueType::StatsFromObject => {
                    match () {
                        _ if ruleset.buildings.contains_key(&unique.params[1]) => {
                            unique.stats.values().sum::<f32>() *
                                if ruleset.buildings[&unique.params[1]].is_national_wonder {
                                    0.25 // at most 1 copy in empire
                                } else {
                                    1.0
                                }
                        }
                        _ if ruleset.specialists.contains_key(&unique.params[1]) => {
                            unique.stats.values().sum::<f32>() *
                                if city.population.population > 8 {
                                    2.0
                                } else {
                                    1.0
                                }
                        }
                        _ => 0.0 // yields from wonders and great improvements
                    }
                }
                UniqueType::StatsFromTradeRoute => {
                    unique.stats.values().sum::<f32>() *
                        if city.is_connected_to_capital() {
                            1.0
                        } else {
                            0.0
                        }
                }
                UniqueType::StatPercentFromReligionFollowers => {
                    f32::min(
                        unique.params[0].parse::<f32>().unwrap() * city.population.population as f32,
                        unique.params[2].parse::<f32>().unwrap()
                    )
                }
                UniqueType::StatsPerCity => {
                    if city.matches_filter(&unique.params[1]) {
                        unique.stats.values().sum::<f32>()
                    } else {
                        0.0
                    }
                }
                _ => 0.0
            };
        }

        score
    }

    fn belief_bonus_for_player(civ_info: &Civilization, belief: &Belief) -> f32 {
        let mut score = 0.0;
        let number_of_founded_religions = civ_info.game_info.civilizations.iter()
            .filter(|civ| civ.religion_manager.religion.is_some()
                && civ.religion_manager.religion_state >= ReligionState::Religion)
            .count();
        let max_number_of_religions = number_of_founded_religions
            + civ_info.religion_manager.remaining_foundable_religions();

        // Adjusts scores of certain beliefs as game evolves
        let game_time_scaling_percent = match civ_info.religion_manager.religion_state {
            ReligionState::FoundingReligion => {
                100 - ((number_of_founded_religions * 100) / max_number_of_religions)
            }
            ReligionState::EnhancingReligion => {
                let amount_of_enhanced_religions = civ_info.game_info.civilizations.iter()
                    .filter(|civ| civ.religion_manager.religion.is_some()
                        && civ.religion_manager.religion_state == ReligionState::EnhancedReligion)
                    .count();
                100 - ((amount_of_enhanced_religions * 100) / max_number_of_religions)
            }
            _ => 100
        };

        let good_early_modifier = match game_time_scaling_percent {
            0..=32 => 1.0,
            33..=65 => 2.0,
            _ => 4.0
        };

        let good_late_modifier = match game_time_scaling_percent {
            0..=32 => 2.0,
            33..=65 => 1.0,
            _ => 0.5
        };

        for unique in &belief.unique_objects {
            let modifier = if unique.get_modifiers(UniqueType::ConditionalOurUnit)
                .iter()
                .any(|m| m.params[0] == civ_info.religion_manager
                    .get_great_prophet_equivalent()
                    .map(|u| u.name.clone())
                    .unwrap_or_default())
            {
                0.5
            } else {
                1.0
            };

            score += modifier * match unique.unique_type {
                UniqueType::KillUnitPlunderNearCity => {
                    unique.params[0].parse::<f32>().unwrap() *
                        if civ_info.wants_to_focus_on(Victory::Focus::Military) {
                            0.5
                        } else {
                            0.25
                        }
                }
                UniqueType::BuyUnitsForAmountStat | UniqueType::BuyBuildingsForAmountStat => {
                    if civ_info.religion_manager.religion
                        .as_ref()
                        .map_or(false, |r| r.follower_belief_unique_map
                            .get_uniques(unique.unique_type)
                            .next()
                            .is_some())
                    {
                        0.0
                    } else {
                        civ_info.stats.stats_for_next_turn[Stat::from_str(&unique.params[2]).unwrap()]
                            * 300.0 / unique.params[1].parse::<f32>().unwrap()
                    }
                }
                // ... more unique type matches ...
                _ => 0.0
            };
        }

        score
    }

    pub fn choose_religious_beliefs(civ_info: &mut Civilization) {
        Self::choose_pantheon(civ_info);
        Self::found_religion(civ_info);
        Self::enhance_religion(civ_info);
        Self::choose_free_beliefs(civ_info);
    }

    fn choose_pantheon(civ_info: &mut Civilization) {
        if !civ_info.religion_manager.can_found_or_expand_pantheon() {
            return;
        }

        if let Some(chosen_pantheon) = Self::choose_belief_of_type(civ_info, BeliefType::Pantheon) {
            civ_info.religion_manager.choose_beliefs(
                vec![chosen_pantheon],
                civ_info.religion_manager.using_free_beliefs()
            );
        }
    }

    fn found_religion(civ_info: &mut Civilization) {
        if civ_info.religion_manager.religion_state != ReligionState::FoundingReligion {
            return;
        }

        let available_religion_icons = civ_info.game_info.ruleset.religions.iter()
            .filter(|name| !civ_info.game_info.religions.values()
                .any(|religion| religion.name == **name))
            .collect::<Vec<_>>();

        let religion_icon = if let Some(favored) = &civ_info.nation.favored_religion {
            if available_religion_icons.contains(&favored) && rand::thread_rng().gen_range(0..10) < 5 {
                favored
            } else {
                available_religion_icons.choose(&mut rand::thread_rng())
                    .unwrap_or_else(|| return)
            }
        } else {
            available_religion_icons.choose(&mut rand::thread_rng())
                .unwrap_or_else(|| return)
        };

        civ_info.religion_manager.found_religion(religion_icon, religion_icon);

        let chosen_beliefs = Self::choose_beliefs(
            civ_info,
            civ_info.religion_manager.get_beliefs_to_choose_at_founding()
        );
        civ_info.religion_manager.choose_beliefs(chosen_beliefs.into_iter().collect());
    }

    fn enhance_religion(civ_info: &mut Civilization) {
        if civ_info.religion_manager.religion_state != ReligionState::EnhancingReligion {
            return;
        }

        let chosen_beliefs = Self::choose_beliefs(
            civ_info,
            civ_info.religion_manager.get_beliefs_to_choose_at_enhancing()
        );
        civ_info.religion_manager.choose_beliefs(chosen_beliefs.into_iter().collect());
    }

    fn choose_free_beliefs(civ_info: &mut Civilization) {
        if !civ_info.religion_manager.has_free_beliefs() {
            return;
        }

        let chosen_beliefs = Self::choose_beliefs(
            civ_info,
            civ_info.religion_manager.free_beliefs_as_enums()
        );
        civ_info.religion_manager.choose_beliefs(
            chosen_beliefs.into_iter().collect(),
            true
        );
    }

    fn choose_beliefs(civ_info: &Civilization, beliefs_to_choose: Counter<BeliefType>) -> HashSet<Belief> {
        let mut chosen_beliefs = HashSet::new();

        for belief_type in BeliefType::iter() {
            if belief_type == BeliefType::None {
                continue;
            }
            for _ in 0..beliefs_to_choose[belief_type] {
                if let Some(belief) = Self::choose_belief_of_type(
                    civ_info,
                    belief_type,
                    &chosen_beliefs
                ) {
                    chosen_beliefs.insert(belief);
                }
            }
        }

        chosen_beliefs
    }

    fn choose_belief_of_type(
        civ_info: &Civilization,
        belief_type: BeliefType,
        additional_beliefs_to_exclude: &HashSet<Belief>
    ) -> Option<Belief> {
        civ_info.game_info.ruleset.beliefs.values()
            .filter(|belief| {
                (belief.belief_type == belief_type || belief_type == BeliefType::Any)
                    && !additional_beliefs_to_exclude.contains(belief)
                    && civ_info.religion_manager.get_religion_with_belief(belief).is_none()
                    && belief.get_matching_uniques(UniqueType::OnlyAvailable)
                        .iter()
                        .all(|unique| unique.conditionals_apply(&civ_info.state))
            })
            .max_by(|a, b| {
                Self::rate_belief(civ_info, a)
                    .partial_cmp(&Self::rate_belief(civ_info, b))
                    .unwrap()
            })
            .cloned()
    }
}