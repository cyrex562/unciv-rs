use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::models::{
    civilization::{Civilization, CivFlags},
    city::{City, CityConstructions},
    map::{MapUnit, Tile},
    ruleset::{
        unique::{Unique, UniqueType, StateForConditionals, UniqueMap, UniqueTarget},
        tech::Technology,
        Ruleset, RulesetObject, RejectionReason, RejectionReasonType,
        INonPerpetualConstruction,
    },
    stats::{Stat, Stats},
    translations::{fill_placeholders, has_placeholder_parameters},
    constants::CONSTANTS,
};
use crate::utils::{add_to_map_of_sets, random_weighted};
use crate::game::UncivGame;
use crate::automation::NextTurnAutomation;
use crate::map_generator::{NaturalWonderGenerator, RiverGenerator};
use crate::unit::UnitPromotions;
use crate::unit::UnitMovementType;

/// Represents the basic information of a unit as specified in Units.json,
/// in contrast to MapUnit which represents a specific unit on the map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseUnit {
    pub name: String,
    pub cost: i32,
    pub hurry_cost_modifier: i32,
    pub movement: i32,
    pub strength: i32,
    pub ranged_strength: i32,
    pub religious_strength: i32,
    pub range: i32,
    pub intercept_range: i32,
    pub unit_type: String,
    pub required_tech: Option<String>,
    pub required_resource: Option<String>,
    pub replacement_text_for_uniques: String,
    pub promotions: HashSet<String>,
    pub obsolete_tech: Option<String>,
    pub upgrades_to: Option<String>,
    pub replaces: Option<String>,
    pub unique_to: Option<String>,
    pub attack_sound: Option<String>,

    #[serde(skip)]
    cached_force_evaluation: i32,

    #[serde(skip)]
    ruleset: Option<Arc<Ruleset>>,

    #[serde(skip)]
    ruleset_unique_objects: Vec<Unique>,

    #[serde(skip)]
    ruleset_unique_map: UniqueMap,

    #[serde(skip)]
    cached_matches_filter_result: HashMap<String, bool>,
}

impl BaseUnit {
    /// Creates a new BaseUnit with default values
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cost: -1,
            hurry_cost_modifier: 0,
            movement: 0,
            strength: 0,
            ranged_strength: 0,
            religious_strength: 0,
            range: 2,
            intercept_range: 0,
            unit_type: String::new(),
            required_tech: None,
            required_resource: None,
            replacement_text_for_uniques: String::new(),
            promotions: HashSet::new(),
            obsolete_tech: None,
            upgrades_to: None,
            replaces: None,
            unique_to: None,
            attack_sound: None,
            cached_force_evaluation: -1,
            ruleset: None,
            ruleset_unique_objects: Vec::new(),
            ruleset_unique_map: UniqueMap::new(),
            cached_matches_filter_result: HashMap::new(),
        }
    }

    /// Sets the ruleset for this unit
    pub fn set_ruleset(&mut self, ruleset: Arc<Ruleset>) {
        self.ruleset = Some(ruleset.clone());

        let mut list = self.uniques().to_vec();
        list.extend(ruleset.global_uniques.unit_uniques.iter().cloned());
        list.extend(self.get_type().uniques.iter().cloned());

        self.ruleset_unique_objects = list;
        self.ruleset_unique_map = UniqueMap::from_unique_objects(&self.ruleset_unique_objects);
    }

    /// Gets the unit type from the ruleset
    pub fn get_type(&self) -> &UnitType {
        if let Some(ruleset) = &self.ruleset {
            ruleset.unit_types.get(&self.unit_type)
                .expect(&format!("Unit {} has unit type {} which is not present in ruleset!",
                    self.name, self.unit_type))
        } else {
            panic!("Ruleset not set for unit {}", self.name)
        }
    }

    /// Gets the unique target for this unit
    pub fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Unit
    }

    /// Gets the short description of this unit
    pub fn get_short_description<F>(&self, unique_exclusion_filter: F) -> String
    where
        F: Fn(&Unique) -> bool,
    {
        // Implementation would go here
        format!("{} unit", self.name)
    }

    /// Gets the description of this unit for a city
    pub fn get_description(&self, city: &City) -> String {
        // Implementation would go here
        format!("{} unit", self.name)
    }

    /// Makes a link to this unit in the civilopedia
    pub fn make_link(&self) -> String {
        format!("Unit/{}", self.name)
    }

    /// Gets the civilopedia text lines for this unit
    pub fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        // Implementation would go here
        vec![]
    }

    /// Checks if this unit is unavailable by game settings
    pub fn is_unavailable_by_settings(&self, game_info: &GameInfo) -> bool {
        if !game_info.game_parameters.nuclear_weapons_enabled && self.is_nuclear_weapon() {
            return true;
        }
        false
    }

    /// Gets the upgrade units for this unit
    pub fn get_upgrade_units(&self, state_for_conditionals: &StateForConditionals) -> Vec<String> {
        let mut result = Vec::new();

        if let Some(upgrades_to) = &this.upgrades_to {
            result.push(upgrades_to.clone());
        }

        for unique in this.get_matching_uniques(UniqueType::CanUpgrade, state_for_conditionals) {
            result.push(unique.params[0].clone());
        }

        result
    }

    /// Gets the ruleset upgrade units for this unit
    pub fn get_ruleset_upgrade_units(&self, state_for_conditionals: &StateForConditionals) -> Vec<&BaseUnit> {
        let mut result = Vec::new();

        for unit_name in this.get_upgrade_units(state_for_conditionals) {
            if let Some(ruleset) = &this.ruleset {
                if let Some(unit) = ruleset.units.get(&unit_name) {
                    result.push(unit);
                }
            }
        }

        result
    }

    /// Gets a MapUnit for this unit
    pub fn get_map_unit(&self, civ_info: &Civilization, unit_id: Option<i32>) -> MapUnit {
        let mut unit = MapUnit::new();
        unit.name = self.name.clone();
        unit.civ = civ_info.clone();
        unit.owner = civ_info.civ_name.clone();
        unit.id = unit_id.unwrap_or_else(|| {
            civ_info.game_info.last_unit_id += 1;
            civ_info.game_info.last_unit_id
        });

        // Must be after setting name & civInfo because it sets the baseUnit according to the name
        // and the civInfo is required for using `has_unique` when determining its movement options
        unit.set_transients(&civ_info.game_info.ruleset);

        unit
    }

    /// Checks if this unit has a unique of the given type
    pub fn has_unique(&self, unique_type: UniqueType, state: Option<&StateForConditionals>) -> bool {
        let state_for_conditionals = state.unwrap_or(&StateForConditionals::empty_state());

        if let Some(_) = &this.ruleset {
            this.ruleset_unique_map.has_unique(unique_type, state_for_conditionals)
        } else {
            // Fallback to base implementation
            self.uniques().iter().any(|u| u.type_ == unique_type &&
                u.conditionals_apply(state_for_conditionals))
        }
    }

    /// Checks if this unit has a unique with the given tag
    pub fn has_unique_tag(&self, unique_tag: &str, state: Option<&StateForConditionals>) -> bool {
        let state_for_conditionals = state.unwrap_or(&StateForConditionals::empty_state());

        if let Some(_) = &this.ruleset {
            this.ruleset_unique_map.has_unique_tag(unique_tag, state_for_conditionals)
        } else {
            // Fallback to base implementation
            self.uniques().iter().any(|u| u.tag == unique_tag &&
                u.conditionals_apply(state_for_conditionals))
        }
    }

    /// Gets matching uniques of the given type
    pub fn get_matching_uniques(&self, unique_type: UniqueType, state: &StateForConditionals) -> Vec<&Unique> {
        if let Some(_) = &this.ruleset {
            this.ruleset_unique_map.get_matching_uniques(unique_type, state)
        } else {
            // Fallback to base implementation
            self.uniques().iter()
                .filter(|u| u.type_ == unique_type && u.conditionals_apply(state))
                .collect()
        }
    }

    /// Gets matching uniques with the given tag
    pub fn get_matching_uniques_by_tag(&self, unique_tag: &str, state: &StateForConditionals) -> Vec<&Unique> {
        if let Some(_) = &this.ruleset {
            this.ruleset_unique_map.get_matching_uniques_by_tag(unique_tag, state)
        } else {
            // Fallback to base implementation
            self.uniques().iter()
                .filter(|u| u.tag == unique_tag && u.conditionals_apply(state))
                .collect()
        }
    }

    /// Gets the production cost for this unit
    pub fn get_production_cost(&self, civ_info: &Civilization, city: Option<&City>) -> i32 {
        // Implementation would go here
        self.cost
    }

    /// Checks if this unit can be purchased with the given stat
    pub fn can_be_purchased_with_stat(&self, city: Option<&City>, stat: Stat) -> bool {
        if city.is_none() {
            return true; // Base implementation
        }

        let city = city.unwrap();

        if self.has_unique(UniqueType::CannotBePurchased, None) {
            return false;
        }

        if !self.get_rejection_reasons(city.city_constructions).iter()
            .any(|r| r.type_ != RejectionReasonType::Unbuildable) {
            return false;
        }

        // Implementation would continue here
        true
    }

    /// Gets the base buy cost for this unit
    pub fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        // Implementation would go here
        Some(self.cost as f32)
    }

    /// Gets the stat buy cost for this unit
    pub fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32> {
        // Implementation would go here
        Some(self.cost)
    }

    /// Gets the disband gold for this unit
    pub fn get_disband_gold(&self, civ_info: &Civilization) -> i32 {
        self.get_base_gold_cost(civ_info, None) / 20
    }

    /// Checks if this unit should be displayed in the city constructions
    pub fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        let rejection_reasons = self.get_rejection_reasons(city_constructions);

        if self.has_unique(UniqueType::ShowsWhenUnbuilable, Some(&city_constructions.city.state)) &&
            !rejection_reasons.iter().any(|r| r.is_never_visible()) {
            return true;
        }

        if !rejection_reasons.iter().any(|r| !r.should_show) {
            return true;
        }

        if self.can_be_purchased_with_any_stat(city_constructions.city) &&
            rejection_reasons.iter().all(|r| r.type_ == RejectionReasonType::Unbuildable) {
            return true;
        }

        false
    }

    /// Gets rejection reasons for this unit
    pub fn get_rejection_reasons(&self, city_constructions: &CityConstructions) -> Vec<RejectionReason> {
        self.get_rejection_reasons_for_civ_and_city(
            &city_constructions.city.civ,
            Some(&city_constructions.city),
            &Counter::new()
        )
    }

    /// Gets rejection reasons for this unit with civ and city
    pub fn get_rejection_reasons_for_civ_and_city(
        &self,
        civ: &Civilization,
        city: Option<&City>,
        additional_resources: &Counter<String>
    ) -> Vec<RejectionReason> {
        let mut reasons = Vec::new();

        let state_for_conditionals = city.map(|c| c.state.clone())
            .unwrap_or_else(|| civ.state.clone());

        if let Some(city) = city {
            if self.is_water_unit && !city.is_coastal() {
                reasons.push(RejectionReason::new(
                    RejectionReasonType::WaterUnitsInCoastalCities,
                    None
                ));
            }
        }

        for unique in self.get_matching_uniques(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals()) {
            reasons.extend(self.not_met_rejections(unique, civ, city, false));
        }

        for unique in self.get_matching_uniques(UniqueType::CanOnlyBeBuiltWhen, &StateForConditionals::ignore_conditionals()) {
            reasons.extend(self.not_met_rejections(unique, civ, city, true));
        }

        for unique in self.get_matching_uniques(UniqueType::Unavailable, &state_for_conditionals) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::ShouldNotBeDisplayed,
                None
            ));
        }

        if let Some(city) = city {
            for unique in self.get_matching_uniques(UniqueType::RequiresPopulation, &state_for_conditionals) {
                let required_pop = unique.params[0].parse::<i32>().unwrap();
                if required_pop > city.population.population {
                    reasons.push(RejectionReason::new(
                        RejectionReasonType::PopulationRequirement,
                        Some(unique.get_display_text())
                    ));
                }
            }
        }

        for required_tech in self.required_techs() {
            if !civ.tech.is_researched(&required_tech) {
                reasons.push(RejectionReason::new(
                    RejectionReasonType::RequiresTech,
                    Some(format!("{} not researched", required_tech))
                ));
            }
        }

        for obsolete_tech in self.techs_at_which_no_longer_available() {
            if civ.tech.is_researched(&obsolete_tech) {
                reasons.push(RejectionReason::new(
                    RejectionReasonType::Obsoleted,
                    Some(format!("Obsolete by {}", obsolete_tech))
                ));
            }
        }

        if let Some(unique_to) = &this.unique_to {
            if !civ.matches_filter(unique_to, &state_for_conditionals) {
                reasons.push(RejectionReason::new(
                    RejectionReasonType::UniqueToOtherNation,
                    Some(format!("Unique to {}", unique_to))
                ));
            }
        }

        if civ.cache.unique_units.iter().any(|u| u.replaces == Some(this.name.clone())) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::ReplacedByOurUnique,
                Some("Our unique unit replaces this".to_string())
            ));
        }

        if self.is_unavailable_by_settings(&civ.game_info) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::DisabledBySetting,
                None
            ));
        }

        if this.has_unique(UniqueType::Unbuildable, Some(&state_for_conditionals)) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::Unbuildable,
                None
            ));
        }

        if (civ.is_city_state || civ.is_one_city_challenger()) &&
            this.has_unique(UniqueType::FoundCity, Some(&state_for_conditionals)) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::NoSettlerForOneCityPlayers,
                None
            ));
        }

        if this.get_matching_uniques(UniqueType::MaxNumberBuildable, &state_for_conditionals).iter()
            .any(|u| {
                let max = u.params[0].parse::<i32>().unwrap();
                civ.civ_constructions.count_constructed_objects(this) >= max
            }) {
            reasons.push(RejectionReason::new(
                RejectionReasonType::MaxNumberBuildable,
                None
            ));
        }

        if !civ.is_barbarian {
            let mut civ_resources = Counter::from(civ.get_civ_resources_by_name());
            civ_resources.add(additional_resources);

            for (resource, required_amount) in this.get_resource_requirements_per_turn(Some(&state_for_conditionals)) {
                let available_amount = civ_resources.get(&resource);
                if available_amount < required_amount {
                    let message = resource.get_need_more_amount_string(required_amount - available_amount);
                    reasons.push(RejectionReason::new(
                        RejectionReasonType::ConsumesResources,
                        Some(message)
                    ));
                }
            }

            if city.is_none() || city.unwrap().city_constructions.get_work_done(&this.name) == 0 {
                for (resource_name, amount) in this.get_stockpiled_resource_requirements(Some(&state_for_conditionals)) {
                    let available_resources = city.map(|c| c.get_available_resource_amount(&resource_name))
                        .unwrap_or_else(|| civ.get_resource_amount(&resource_name));

                    if available_resources < amount {
                        let message = resource_name.get_need_more_amount_string(amount - available_resources);
                        reasons.push(RejectionReason::new(
                            RejectionReasonType::ConsumesResources,
                            Some(message)
                        ));
                    }
                }
            }
        }

        for unique in civ.get_matching_uniques(UniqueType::CannotBuildUnits, &state_for_conditionals) {
            if this.matches_filter(&unique.params[0], Some(&state_for_conditionals)) {
                let has_happiness_condition = unique.has_modifier(UniqueType::ConditionalBelowHappiness) ||
                    unique.has_modifier(UniqueType::ConditionalBetweenHappiness);

                if has_happiness_condition {
                    reasons.push(RejectionReason::new(
                        RejectionReasonType::CannotBeBuiltUnhappiness,
                        Some(unique.get_display_text())
                    ));
                } else {
                    reasons.push(RejectionReason::new(
                        RejectionReasonType::CannotBeBuilt,
                        None
                    ));
                }
            }
        }

        if let Some(city) = city {
            if this.is_air_unit() {
                let fake_unit = this.get_map_unit(civ, Some(CONSTANTS.NO_ID));
                let can_unit_enter_tile = fake_unit.movement.can_move_to(city.get_center_tile());

                if !can_unit_enter_tile {
                    reasons.push(RejectionReason::new(
                        RejectionReasonType::NoPlaceToPutUnit,
                        None
                    ));
                }
            }
        }

        reasons
    }

    /// Gets rejection reasons for not met conditions
    fn not_met_rejections(
        &self,
        unique: &Unique,
        civ: &Civilization,
        city: Option<&City>,
        built: bool
    ) -> Vec<RejectionReason> {
        let mut reasons = Vec::new();

        for conditional in &unique.modifiers {
            // We yield a rejection only when conditionals are NOT met
            if Conditionals::conditional_applies(unique, conditional, city.map(|c| &c.state).unwrap_or(&civ.state)) {
                continue;
            }

            match conditional.type_ {
                UniqueType::ConditionalBuildingBuiltAmount => {
                    let building = civ.get_equivalent_building(&conditional.params[0]).name;
                    let amount = conditional.params[1].parse::<i32>().unwrap();
                    let city_filter = &conditional.params[2];

                    let number_of_cities = civ.cities.iter().filter(|c| {
                        c.city_constructions.contains_building_or_equivalent(&building) &&
                        c.matches_filter(city_filter)
                    }).count();

                    if number_of_cities < amount as usize {
                        reasons.push(RejectionReason::new(
                            RejectionReasonType::RequiresBuildingInSomeCities,
                            Some(format!("Requires a [{}] in at least [{}] cities ({}/{})",
                                building, amount, number_of_cities, number_of_cities))
                        ));
                    }
                },
                UniqueType::ConditionalBuildingBuiltAll => {
                    let building = civ.get_equivalent_building(&conditional.params[0]).name;
                    let city_filter = &conditional.params[1];

                    if civ.cities.iter().any(|c| {
                        c.matches_filter(city_filter) &&
                        !c.is_puppet &&
                        !c.city_constructions.contains_building_or_equivalent(&building)
                    }) {
                        reasons.push(RejectionReason::new(
                            RejectionReasonType::RequiresBuildingInAllCities,
                            Some(format!("Requires a [{}] in all cities", building))
                        ));
                    }
                },
                _ => {
                    if built {
                        reasons.push(RejectionReason::new(
                            RejectionReasonType::CanOnlyBeBuiltInSpecificCities,
                            Some(unique.get_display_text())
                        ));
                    } else {
                        reasons.push(RejectionReason::new(
                            RejectionReasonType::ShouldNotBeDisplayed,
                            None
                        ));
                    }
                }
            }
        }

        reasons
    }

    /// Checks if this unit is buildable for a civilization
    pub fn is_buildable_for_civ(&self, civ_info: &Civilization) -> bool {
        this.get_rejection_reasons_for_civ_and_city(civ_info, None, &Counter::new()).is_empty()
    }

    /// Checks if this unit is buildable for a city constructions
    pub fn is_buildable_for_city_constructions(&self, city_constructions: &CityConstructions) -> bool {
        this.get_rejection_reasons(city_constructions).is_empty()
    }

    /// Constructs this unit in a city
    pub fn construct(&self, city_constructions: &CityConstructions, bought_with: Option<Stat>) -> Option<MapUnit> {
        let civ_info = &city_constructions.city.civ;
        let unit = civ_info.units.add_unit(this, &city_constructions.city)?;

        // Movement penalty
        if bought_with.is_some() && !civ_info.game_info.game_parameters.god_mode &&
            !unit.has_unique(UniqueType::CanMoveImmediatelyOnceBought, None) {
            unit.current_movement = 0.0;
        }

        this.add_construction_bonuses(&mut unit, city_constructions);

        Some(unit)
    }

    /// Gets the unit this unit automatically upgrades to at a given tech
    pub fn automatically_upgraded_in_production_to_unit_by_tech(&self, tech_name: &str) -> Option<String> {
        for obsolete_tech in this.techs_at_which_auto_upgrade_in_production() {
            if obsolete_tech == tech_name {
                return this.upgrades_to.clone();
            }
        }
        None
    }

    /// Adds construction bonuses to a unit
    pub fn add_construction_bonuses(&self, unit: &mut MapUnit, city_constructions: &CityConstructions) {
        let civ_info = &city_constructions.city.civ;

        let mut xp = 0;

        for unique in city_constructions.city.get_matching_uniques(UniqueType::UnitStartingExperience, &city_constructions.city.state)
            .iter()
            .chain(city_constructions.city.get_matching_uniques(UniqueType::UnitStartingExperienceOld, &city_constructions.city.state).iter())
            .filter(|u| city_constructions.city.matches_filter(&u.params[2])) {
            if unit.matches_filter(&unique.params[0]) {
                xp += unique.params[1].parse::<i32>().unwrap();
            }
        }

        unit.promotions.xp = xp;

        for unique in city_constructions.city.get_matching_uniques(UniqueType::UnitStartingPromotions, &city_constructions.city.state)
            .iter()
            .filter(|u| city_constructions.city.matches_filter(&u.params[1])) {
            let filter = &unique.params[0];
            let promotion = &unique.params.last().unwrap();

            let is_relevant_promotion = filter == "relevant" &&
                civ_info.game_info.ruleset.unit_promotions.values.iter()
                    .any(|p| p.name == *promotion && unit.type_.name == p.unit_types);

            if is_relevant_promotion || unit.matches_filter(filter) {
                unit.promotions.add_promotion(promotion, true);
            }
        }
    }

    /// Gets the replaced unit from the ruleset
    pub fn get_replaced_unit(&self, ruleset: &Ruleset) -> &BaseUnit {
        if let Some(replaces) = &this.replaces {
            ruleset.units.get(replaces).unwrap()
        } else {
            this
        }
    }

    /// Checks if this unit matches a filter
    pub fn matches_filter(&self, filter: &str, state: Option<&StateForConditionals>, multi_filter: bool) -> bool {
        if multi_filter {
            MultiFilter::multi_filter(filter, |f| {
                *this.cached_matches_filter_result.entry(f.to_string()).or_insert_with(|| {
                    this.matches_single_filter(f) ||
                    state.is_some() && this.has_unique_tag(f, state) ||
                    state.is_none() && this.has_tag_unique(f)
                })
            })
        } else {
            *this.cached_matches_filter_result.entry(filter.to_string()).or_insert_with(|| {
                this.matches_single_filter(filter) ||
                state.is_some() && this.has_unique_tag(filter, state) ||
                state.is_none() && this.has_tag_unique(filter)
            })
        }
    }

    /// Checks if this unit matches a single filter
    fn matches_single_filter(&self, filter: &str) -> bool {
        match filter {
            "all" | "All" => true,
            "Melee" => this.is_melee(),
            "Ranged" => this.is_ranged(),
            "Civilian" => this.is_civilian(),
            "Military" => this.is_military,
            "Land" => this.is_land_unit,
            "Water" => this.is_water_unit,
            "Air" => this.is_air_unit(),
            "non-air" => !this.moves_like_air_units,
            "Nuclear Weapon" => this.is_nuclear_weapon(),
            "Great Person" => this.is_great_person,
            "Religious" => this.has_unique(UniqueType::ReligiousUnit, None),
            _ => {
                if filter == this.unit_type {
                    return true;
                } else if filter == this.name {
                    return true;
                } else if let Some(replaces) = &this.replaces {
                    if filter == replaces {
                        return true;
                    }
                }

                for required_tech in this.required_techs() {
                    if let Some(ruleset) = &this.ruleset {
                        if let Some(tech) = ruleset.technologies.get(&required_tech) {
                            if tech.matches_filter(filter, false) {
                                return true;
                            }
                        }
                    }
                }

                if filter.ends_with(" units") {
                    let filter_without_suffix = filter.strip_suffix(" units").unwrap();
                    let filter_capitalized = filter_without_suffix.chars().next().unwrap().to_uppercase().collect::<String>() +
                        &filter_without_suffix[1..];

                    return this.matches_filter(&filter_capitalized, None, false);
                }

                false
            }
        }
    }

    /// Checks if this unit is a city founder
    pub fn is_city_founder(&self) -> bool {
        this.has_unique(UniqueType::FoundCity, Some(&StateForConditionals::ignore_conditionals()))
    }

    /// Checks if this unit is a great person
    pub fn is_great_person(&self) -> bool {
        this.get_matching_uniques(UniqueType::GreatPerson, &StateForConditionals::empty_state()).is_empty()
    }

    /// Checks if this unit is a great person of a specific type
    pub fn is_great_person_of_type(&self, type_: &str) -> bool {
        this.get_matching_uniques(UniqueType::GreatPerson, &StateForConditionals::empty_state())
            .iter()
            .any(|u| u.params[0] == type_)
    }

    /// Checks if this unit is a nuclear weapon
    pub fn is_nuclear_weapon(&self) -> bool {
        this.has_unique(UniqueType::NuclearWeapon, Some(&StateForConditionals::ignore_conditionals()))
    }

    /// Checks if this unit moves like air units
    pub fn moves_like_air_units(&self) -> bool {
        this.get_type().get_movement_type() == UnitMovementType::Air
    }

    /// Gets the resource requirements per turn for this unit
    pub fn get_resource_requirements_per_turn(&self, state: Option<&StateForConditionals>) -> Counter<String> {
        let mut resource_requirements = Counter::new();

        if let Some(required_resource) = &this.required_resource {
            resource_requirements.insert(required_resource.clone(), 1);
        }

        let state = state.unwrap_or(&StateForConditionals::empty_state());

        for unique in this.get_matching_uniques(UniqueType::ConsumesResources, state) {
            let amount = unique.params[0].parse::<i32>().unwrap();
            let resource = unique.params[1].clone();
            resource_requirements.add(&resource, amount);
        }

        resource_requirements
    }

    /// Checks if this unit is ranged
    pub fn is_ranged(&self) -> bool {
        this.ranged_strength > 0
    }

    /// Checks if this unit is melee
    pub fn is_melee(&self) -> bool {
        !this.is_ranged() && this.strength > 0
    }

    /// Checks if this unit is military
    pub fn is_military(&self) -> bool {
        this.is_ranged() || this.is_melee()
    }

    /// Checks if this unit is civilian
    pub fn is_civilian(&self) -> bool {
        !this.is_military()
    }

    /// Checks if this unit is a land unit
    pub fn is_land_unit(&self) -> bool {
        this.get_type().is_land_unit()
    }

    /// Checks if this unit is a water unit
    pub fn is_water_unit(&self) -> bool {
        this.get_type().is_water_unit()
    }

    /// Checks if this unit is an air unit
    pub fn is_air_unit(&self) -> bool {
        this.get_type().is_air_unit()
    }

    /// Checks if this unit is probably a siege unit
    pub fn is_probably_siege_unit(&self) -> bool {
        this.is_ranged() &&
            this.get_matching_uniques(UniqueType::Strength, &StateForConditionals::ignore_conditionals())
                .iter()
                .any(|u| {
                    u.params[0].parse::<i32>().unwrap() > 0 &&
                    u.has_modifier(UniqueType::ConditionalVsCity)
                })
    }

    /// Gets the force evaluation for this unit
    pub fn get_force_evaluation(&self) -> i32 {
        if this.cached_force_evaluation < 0 {
            this.evaluate_force();
        }
        this.cached_force_evaluation
    }

    /// Evaluates the force of this unit
    fn evaluate_force(&mut self) {
        if this.strength == 0 && this.ranged_strength == 0 {
            this.cached_force_evaluation = 0;
            return;
        }

        let mut power = (this.strength as f32).powf(1.5);
        let mut ranged_power = (this.ranged_strength as f32).powf(1.45);

        // Value ranged naval units less
        if this.is_water_unit {
            ranged_power /= 2.0;
        }

        if ranged_power > 0.0 {
            power = ranged_power;
        }

        // Replicates the formula from civ V, which is a lower multiplier than probably intended, because math
        // They did fix it in BNW so it was completely bugged and always 1, again math
        power *= (this.movement as f32).powf(0.3);

        if this.has_unique(UniqueType::SelfDestructs, None) {
            power /= 2.0;
        }

        if this.is_nuclear_weapon() {
            power += 4000.0;
        }

        // Uniques
        let mut all_uniques = this.ruleset_unique_objects.iter().collect::<Vec<_>>();

        for promotion_name in &this.promotions {
            if let Some(ruleset) = &this.ruleset {
                if let Some(promotion) = ruleset.unit_promotions.get(promotion_name) {
                    all_uniques.extend(promotion.unique_objects.iter());
                }
            }
        }

        for unique in all_uniques {
            match unique.type_ {
                UniqueType::Strength => {
                    let strength_bonus = unique.params[0].parse::<i32>().unwrap();
                    if strength_bonus <= 0 {
                        continue;
                    }

                    if unique.has_modifier(UniqueType::ConditionalVsUnits) {
                        // Bonus vs some units - a quarter of the bonus
                        power *= (strength_bonus as f32 / 4.0).to_percent();
                    } else if unique.modifiers.iter().any(|m| {
                        m.type_ == UniqueType::ConditionalVsCity || // City Attack - half the bonus
                        m.type_ == UniqueType::ConditionalAttacking || // Attack - half the bonus
                        m.type_ == UniqueType::ConditionalDefending || // Defense - half the bonus
                        m.type_ == UniqueType::ConditionalFightingInTiles // Bonus in terrain or feature - half the bonus
                    }) {
                        power *= (strength_bonus as f32 / 2.0).to_percent();
                    } else {
                        power *= strength_bonus.to_percent(); // Static bonus
                    }
                },
                UniqueType::StrengthNearCapital => {
                    let strength_bonus = unique.params[0].parse::<i32>().unwrap();
                    if strength_bonus > 0 {
                        // Bonus decreasing with distance from capital - not worth much most of the map???
                        power *= (strength_bonus as f32 / 4.0).to_percent();
                    }
                },
                UniqueType::MayParadrop => {
                    // Paradrop - 25% bonus
                    power += power / 4.0;
                },
                UniqueType::MustSetUp => {
                    // Must set up - 20% penalty
                    power -= power / 5.0;
                },
                UniqueType::AdditionalAttacks => {
                    // Extra attacks - 20% bonus per extra attack
                    let extra_attacks = unique.params[0].parse::<i32>().unwrap();
                    power += (power * extra_attacks as f32) / 5.0;
                },
                _ => {}
            }
        }

        this.cached_force_evaluation = power as i32;
    }

    /// Gets the required techs for this unit
    pub fn required_techs(&self) -> Vec<String> {
        if let Some(required_tech) = &this.required_tech {
            vec![required_tech.clone()]
        } else {
            vec![]
        }
    }

    /// Gets the techs that obsolete this unit
    pub fn techs_that_obsolete_this(&self) -> Vec<String> {
        if let Some(obsolete_tech) = &this.obsolete_tech {
            vec![obsolete_tech.clone()]
        } else {
            vec![]
        }
    }

    /// Gets the techs at which this unit auto-upgrades in production
    pub fn techs_at_which_auto_upgrade_in_production(&self) -> Vec<String> {
        this.techs_that_obsolete_this()
    }

    /// Gets the techs at which this unit is no longer available
    pub fn techs_at_which_no_longer_available(&self) -> Vec<String> {
        this.techs_that_obsolete_this()
    }

    /// Checks if this unit is obsoleted by a tech
    pub fn is_obsoleted_by(&self, tech_name: &str) -> bool {
        this.techs_that_obsolete_this().contains(&tech_name.to_string())
    }

    /// Gets the stockpiled resource requirements for this unit
    pub fn get_stockpiled_resource_requirements(&self, state: Option<&StateForConditionals>) -> Counter<String> {
        let mut resource_requirements = Counter::new();

        let state = state.unwrap_or(&StateForConditionals::empty_state());

        for unique in this.get_matching_uniques(UniqueType::ConsumesResources, state) {
            if unique.has_modifier(UniqueType::Stockpiled) {
                let amount = unique.params[0].parse::<i32>().unwrap();
                let resource = unique.params[1].clone();
                resource_requirements.insert(resource, amount);
            }
        }

        resource_requirements
    }
}

impl RulesetObject for BaseUnit {
    fn name(&self) -> &str {
        &this.name
    }

    fn uniques(&self) -> &[Unique] {
        &[]
    }

    fn unique_objects(&self) -> &[Unique] {
        &this.ruleset_unique_objects
    }

    fn unique_map(&self) -> &UniqueMap {
        &this.ruleset_unique_map
    }

    fn get_unique_target(&self) -> UniqueTarget {
        UniqueTarget::Unit
    }

    fn make_link(&self) -> String {
        format!("Unit/{}", this.name)
    }

    fn get_civilopedia_text_lines(&self, ruleset: &Ruleset) -> Vec<FormattedLine> {
        // Implementation would go here
        vec![]
    }
}

impl INonPerpetualConstruction for BaseUnit {
    fn cost(&self) -> i32 {
        this.cost
    }

    fn hurry_cost_modifier(&self) -> i32 {
        this.hurry_cost_modifier
    }

    fn required_tech(&self) -> Option<&str> {
        this.required_tech.as_deref()
    }

    fn get_production_cost(&self, civ_info: &Civilization, city: Option<&City>) -> i32 {
        this.get_production_cost(civ_info, city)
    }

    fn can_be_purchased_with_stat(&self, city: Option<&City>, stat: Stat) -> bool {
        this.can_be_purchased_with_stat(city, stat)
    }

    fn get_base_buy_cost(&self, city: &City, stat: Stat) -> Option<f32> {
        this.get_base_buy_cost(city, stat)
    }

    fn get_stat_buy_cost(&self, city: &City, stat: Stat) -> Option<i32> {
        this.get_stat_buy_cost(city, stat)
    }

    fn should_be_displayed(&self, city_constructions: &CityConstructions) -> bool {
        this.should_be_displayed(city_constructions)
    }

    fn get_rejection_reasons(&self, city_constructions: &CityConstructions) -> Vec<RejectionReason> {
        this.get_rejection_reasons(city_constructions)
    }

    fn is_buildable(&self, city_constructions: &CityConstructions) -> bool {
        this.is_buildable_for_city_constructions(city_constructions)
    }

    fn construct(&self, city_constructions: &CityConstructions, bought_with: Option<Stat>) -> Option<MapUnit> {
        this.construct(city_constructions, bought_with)
    }

    fn get_resource_requirements_per_turn(&self, state: Option<&StateForConditionals>) -> Counter<String> {
        this.get_resource_requirements_per_turn(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_unit_creation() {
        let unit = BaseUnit::new();
        assert_eq!(unit.cost, -1);
        assert_eq!(unit.movement, 0);
        assert_eq!(unit.strength, 0);
    }

    #[test]
    fn test_is_ranged() {
        let mut unit = BaseUnit::new();
        unit.ranged_strength = 5;
        assert!(unit.is_ranged());

        unit.ranged_strength = 0;
        assert!(!unit.is_ranged());
    }

    #[test]
    fn test_is_melee() {
        let mut unit = BaseUnit::new();
        unit.strength = 5;
        assert!(unit.is_melee());

        unit.ranged_strength = 5;
        assert!(!unit.is_melee());

        unit.ranged_strength = 0;
        unit.strength = 0;
        assert!(!unit.is_melee());
    }
}