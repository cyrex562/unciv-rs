use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::civilization::Civilization;
use crate::models::{Counter, Religion, Belief, BeliefType};
use crate::models::ruleset::unique::{UniqueTriggerActivation, UniqueType};
use crate::models::ruleset::unit::BaseUnit;
use crate::utils::extensions::{ToPercent, FillPlaceholders};
use crate::utils::random::Random;
use crate::city::City;
use crate::map::mapunit::MapUnit;
use crate::map::tile::Tile;
use crate::ui::screens::worldscreen::unit::actions::UnitActionModifiers;

/// Represents the state of a civilization's religion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReligionState {
    None,
    Pantheon,
    FoundingReligion, // Great prophet used, but religion has not yet been founded
    Religion,
    EnhancingReligion, // Great prophet used, but religion has not yet been enhanced
    EnhancedReligion,
}

impl ReligionState {
    pub fn to_string(&self) -> String {
        match self {
            ReligionState::None => "None".to_string(),
            ReligionState::Pantheon => "Pantheon".to_string(),
            ReligionState::FoundingReligion => "Founding Religion".to_string(),
            ReligionState::Religion => "Religion".to_string(),
            ReligionState::EnhancingReligion => "Enhancing Religion".to_string(),
            ReligionState::EnhancedReligion => "Enhanced Religion".to_string(),
        }
    }
}

/// Manages religion-related functionality for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct ReligionManager {
    #[serde(skip)]
    pub civ: Option<Arc<Civilization>>,

    pub stored_faith: i32,

    #[serde(skip)]
    pub religion: Option<Religion>,

    pub religion_state: ReligionState,

    // Counter containing the number of free beliefs types that this civ can add to its religion this turn
    // Uses String instead of BeliefType enum for serialization reasons
    pub free_beliefs: Counter<String>,

    // These cannot be transient, as saving and loading after using a great prophet but before
    // founding a religion would break :(
    founding_city_id: Option<String>,
    // Only used for keeping track of the city a prophet was used when founding a religion

    should_choose_pantheon_belief: bool,
}

impl ReligionManager {
    pub fn new() -> Self {
        Self {
            civ: None,
            stored_faith: 0,
            religion: None,
            religion_state: ReligionState::None,
            free_beliefs: Counter::new(),
            founding_city_id: None,
            should_choose_pantheon_belief: false,
        }
    }

    pub fn clone(&self) -> Self {
        let mut clone = Self::new();
        clone.founding_city_id = self.founding_city_id.clone();
        clone.should_choose_pantheon_belief = self.should_choose_pantheon_belief;
        clone.stored_faith = self.stored_faith;
        clone.religion_state = self.religion_state;
        clone.free_beliefs = self.free_beliefs.clone();
        clone
    }

    pub fn set_transients(&mut self, civ: Arc<Civilization>) {
        self.civ = Some(civ.clone());
        // Find our religion from the map of founded religions.
        // First check if there is any major religion
        self.religion = civ.game_info.religions.values()
            .find(|r| r.founding_civ_name == civ.civ_name && r.is_major_religion())
            .cloned();
        // If there isn't, check for just pantheons.
        if self.religion.is_none() {
            self.religion = civ.game_info.religions.values()
                .find(|r| r.founding_civ_name == civ.civ_name)
                .cloned();
        }
    }

    pub fn start_turn(&mut self) {
        if self.can_generate_prophet() {
            self.generate_prophet();
        }
    }

    pub fn end_turn(&mut self, faith_from_new_turn: i32) {
        self.stored_faith += faith_from_new_turn;
    }

    pub fn is_majority_religion_for_civ(&self, religion: &Religion) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.cities.iter()
            .filter(|city| city.religion.get_majority_religion().map_or(false, |r| r == *religion))
            .count() > civ.cities.len() / 2
    }

    /// This helper function makes it easy to interface the Counter<String> free_beliefs with functions
    /// that use Counter<BeliefType>
    pub fn free_beliefs_as_enums(&self) -> Counter<BeliefType> {
        let mut to_return = Counter::new();
        for (key, value) in self.free_beliefs.entries() {
            to_return.add(BeliefType::from_str(key).unwrap(), value);
        }
        to_return
    }

    pub fn has_free_beliefs(&self) -> bool {
        self.free_beliefs.sum_values() > 0
    }

    pub fn using_free_beliefs(&self) -> bool {
        (self.religion_state == ReligionState::None && self.stored_faith < self.faith_for_pantheon()) // first pantheon is free
            || self.religion_state == ReligionState::Pantheon // any subsequent pantheons before founding a religion
            || (self.religion_state == ReligionState::Religion || self.religion_state == ReligionState::EnhancedReligion) // any belief adding outside of great prophet use
    }

    pub fn faith_for_pantheon(&self, additional_civs: i32) -> i32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let game_info = &civ.game_info;
        let num_civs = additional_civs + game_info.civilizations.iter()
            .filter(|c| c.is_major_civ() && c.religion_manager.religion.is_some())
            .count() as i32;
        let cost = game_info.ruleset.mod_options.constants.pantheon_base +
            num_civs * game_info.ruleset.mod_options.constants.pantheon_growth;
        (cost as f32 * game_info.speed.faith_cost_modifier).round() as i32
    }

    pub fn can_found_or_expand_pantheon(&self) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !civ.game_info.is_religion_enabled() { return false; }
        if self.religion_state > ReligionState::Pantheon { return false; }
        if !civ.is_major_civ() { return false; }
        if self.number_of_beliefs_available(BeliefType::Pantheon) == 0 {
            return false; // no more available pantheons
        }
        if civ.game_info.civilizations.iter().any(|c| c.religion_manager.religion_state == ReligionState::EnhancedReligion)
            && civ.game_info.civilizations.iter()
                .filter(|c| c.religion_manager.religion_state >= ReligionState::Pantheon)
                .count() >= self.max_number_of_religions() {
            return false;
        }
        (self.religion_state == ReligionState::None && self.stored_faith >= self.faith_for_pantheon(0)) // earned pantheon
            || self.free_beliefs.get(&BeliefType::Pantheon.to_string()) > 0 // free pantheon belief
    }

    fn found_pantheon(&mut self, belief_name: String, use_free_belief: bool) {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !use_free_belief {
            // paid for the initial pantheon using faith
            self.stored_faith -= self.faith_for_pantheon(0);
        }
        let mut religion = Religion::new(belief_name.clone(), civ.game_info.clone(), civ.civ_name.clone());
        civ.game_info.religions.insert(belief_name.clone(), religion.clone());
        for city in &civ.cities {
            city.religion.add_pressure(belief_name.clone(), 200 * city.population.population);
        }
        self.religion = Some(religion);
    }

    pub fn great_prophets_earned(&self) -> i32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let prophet_name = self.get_great_prophet_equivalent()
            .map(|u| u.name.clone())
            .unwrap_or_default();
        civ.civ_constructions.bought_items_with_increasing_price.get(&prophet_name)
    }

    pub fn faith_for_next_great_prophet(&self) -> i32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let great_prophets_earned = this.great_prophets_earned();

        let mut faith_cost = (200.0 + 100.0 * great_prophets_earned as f32 * (great_prophets_earned + 1) as f32 / 2.0) *
            civ.game_info.speed.faith_cost_modifier;

        for unique in civ.get_matching_uniques(UniqueType::FaithCostOfGreatProphetChange) {
            faith_cost *= unique.params[0].to_percent();
        }

        faith_cost as i32
    }

    pub fn can_generate_prophet(&self, ignore_faith_amount: bool) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !civ.game_info.is_religion_enabled() { return false; } // No religion, no prophets
        if self.religion.is_none() || self.religion_state == ReligionState::None { return false; } // First get a pantheon, then we'll talk about a real religion
        if self.get_great_prophet_equivalent().is_none() { return false; }
        if !ignore_faith_amount && self.stored_faith < this.faith_for_next_great_prophet() { return false; }
        if !civ.is_major_civ() { return false; }
        if civ.has_unique(UniqueType::MayNotGenerateGreatProphet) { return false; }
        if self.religion_state == ReligionState::Pantheon && this.remaining_foundable_religions() == 0 { return false; } // too many have been founded
        true
    }

    pub fn get_great_prophet_equivalent(&self) -> Option<BaseUnit> {
        let civ = self.civ.as_ref().expect("Civ not set");
        let base_unit = civ.game_info.ruleset.units.values()
            .find(|u| u.has_unique(UniqueType::MayFoundReligion))
            .cloned();
        base_unit.map(|u| civ.get_equivalent_unit(&u))
    }

    fn generate_prophet(&mut self) {
        let civ = self.civ.as_ref().expect("Civ not set");
        let prophet_unit = self.get_great_prophet_equivalent()?;

        let prophet_spawn_change = (5.0 + self.stored_faith as f32 - this.faith_for_next_great_prophet() as f32) / 100.0;

        if Random::new(civ.game_info.turns).next_f32() < prophet_spawn_change {
            let birth_city = if self.religion_state <= ReligionState::Pantheon {
                civ.get_capital()
            } else {
                self.get_holy_city()
            }?;
            let mut prophet = civ.units.add_unit(prophet_unit, birth_city)?;
            prophet.religion = self.religion.as_ref()?.name.clone();
            self.stored_faith -= this.faith_for_next_great_prophet();
            civ.civ_constructions.bought_items_with_increasing_price.add(prophet_unit.name.clone(), 1);
        }
    }

    fn max_number_of_religions(&self) -> usize {
        let civ = self.civ.as_ref().expect("Civ not set");
        let game_info = &civ.game_info;
        let ruleset = &game_info.ruleset;
        let multiplier = ruleset.mod_options.constants.religion_limit_multiplier;
        let base = ruleset.mod_options.constants.religion_limit_base;
        let civ_count = game_info.civilizations.iter()
            .filter(|c| c.is_major_civ())
            .count();
        std::cmp::min(ruleset.religions.len(), base + (civ_count as f32 * multiplier) as usize)
    }

    /// Calculates the number of religions that are already founded
    fn founded_religions_count(&self) -> usize {
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.game_info.civilizations.iter()
            .filter(|c| c.religion_manager.religion.is_some() &&
                   c.religion_manager.religion_state >= ReligionState::Religion)
            .count()
    }

    /// Calculates the amount of religions that can still be founded
    pub fn remaining_foundable_religions(&self) -> i32 {
        // count the number of foundable religions left given defined ruleset religions and number of civs in game
        let max_number_of_additional_religions = this.max_number_of_religions() - this.founded_religions_count();

        let available_beliefs_to_found = std::cmp::min(
            this.number_of_beliefs_available(BeliefType::Follower),
            this.number_of_beliefs_available(BeliefType::Founder)
        );

        std::cmp::min(max_number_of_additional_religions as i32, available_beliefs_to_found)
    }

    /// Get info breaking down the reasons behind the result of remaining_foundable_religions
    pub fn remaining_foundable_religions_breakdown(&self) -> Vec<(String, i32)> {
        let civ = self.civ.as_ref().expect("Civ not set");
        let game_info = &civ.game_info;
        let ruleset = &game_info.ruleset;
        let mut breakdown = Vec::new();

        breakdown.push(("Available religion symbols".to_string(), ruleset.religions.len() as i32));

        let multiplier = ruleset.mod_options.constants.religion_limit_multiplier;
        let base = ruleset.mod_options.constants.religion_limit_base;
        let civ_count = game_info.civilizations.iter()
            .filter(|c| c.is_major_civ())
            .count();
        let hide_civ_count = civ.hide_civ_count();
        if hide_civ_count {
            let known_civs = 1 + civ.get_known_civs().iter()
                .filter(|c| c.is_major_civ())
                .count();
            let estimated_civ_count = (
                game_info.game_parameters.min_number_of_players.max(known_civs) +
                game_info.game_parameters.max_number_of_players - 1
            ) / 2 + 1;
            let civs_and_base = base + (estimated_civ_count as f32 * multiplier) as i32;
            breakdown.push((format!("Estimated number of civilizations * [{}] + [{}]", multiplier, base), civs_and_base));
        } else {
            let civs_and_base = base + (civ_count as f32 * multiplier) as i32;
            breakdown.push((format!("Number of civilizations * [{}] + [{}]", multiplier, base), civs_and_base));
        }

        breakdown.push(("Religions already founded".to_string(), this.founded_religions_count() as i32));
        breakdown.push(("Available founder beliefs".to_string(), this.number_of_beliefs_available(BeliefType::Founder)));
        breakdown.push(("Available follower beliefs".to_string(), this.number_of_beliefs_available(BeliefType::Follower)));

        breakdown
    }

    pub fn number_of_beliefs_available(&self, belief_type: BeliefType) -> i32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let game_info = &civ.game_info;
        let number_of_beliefs = if belief_type == BeliefType::Any {
            game_info.ruleset.beliefs.len() as i32
        } else {
            game_info.ruleset.beliefs.values()
                .filter(|b| b.belief_type == belief_type)
                .count() as i32
        };
        number_of_beliefs - game_info.religions.values()
            .flat_map(|r| r.get_beliefs(belief_type))
            .collect::<std::collections::HashSet<_>>()
            .len() as i32
    }

    pub fn get_religion_with_belief(&self, belief: &Belief) -> Option<Religion> {
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.game_info.religions.values()
            .find(|r| r.has_belief(&belief.name))
            .cloned()
    }

    pub fn may_found_religion_at_all(&self) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !civ.game_info.is_religion_enabled() { return false; } // No religion
        if this.religion_state >= ReligionState::Religion { return false; } // Already created a major religion
        if !civ.is_major_civ() { return false; } // Only major civs may use religion
        if this.remaining_foundable_religions() == 0 { return false; } // Too bad, too many religions have already been founded
        true
    }

    pub fn may_found_religion_here(&self, tile: &Tile) -> bool {
        if !this.may_found_religion_at_all() { return false; }
        if !tile.is_city_center() { return false; }
        if tile.get_city().map_or(false, |c| c.is_holy_city()) { return false; }
        // No double holy cities. Not sure if these were allowed in the base game
        true
    }

    pub fn found_religion(&mut self, prophet: &MapUnit) {
        if !this.may_found_religion_here(prophet.get_tile()) { return; } // How did you do this?
        if this.religion_state == ReligionState::None {
            self.should_choose_pantheon_belief = true;
        }
        self.religion_state = ReligionState::FoundingReligion;
        self.founding_city_id = Some(prophet.get_tile().get_city()?.id.clone());
    }

    pub fn may_enhance_religion_at_all(&self) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !civ.game_info.is_religion_enabled() { return false; }
        if self.religion.is_none() { return false; } // First found a pantheon
        if this.religion_state != ReligionState::Religion { return false; } // First found an actual religion
        if !civ.is_major_civ() { return false; } // Only major civs
        if this.number_of_beliefs_available(BeliefType::Follower) == 0 { return false; } // Mod maker did not provide enough follower beliefs
        if this.number_of_beliefs_available(BeliefType::Enhancer) == 0 { return false; } // Mod maker did not provide enough enhancer beliefs
        true
    }

    pub fn may_enhance_religion_here(&self, tile: &Tile) -> bool {
        if !this.may_enhance_religion_at_all() { return false; }
        if !tile.is_city_center() { return false; }
        true
    }

    pub fn use_prophet_for_enhancing_religion(&mut self, prophet: &MapUnit) {
        if !this.may_enhance_religion_here(prophet.get_tile()) { return; } // How did you do this?
        self.religion_state = ReligionState::EnhancingReligion;
    }

    /// Unifies the selection of what beliefs are available for when a great prophet is expended
    fn get_beliefs_to_choose_at_prophet_use(&self, enhancing_religion: bool) -> Counter<BeliefType> {
        let civ = self.civ.as_ref().expect("Civ not set");
        let action = if enhancing_religion { "enhancing" } else { "founding" };
        let mut beliefs_to_choose = Counter::new();

        // Counter of the number of available beliefs of each type
        let mut available_beliefs = Counter::new();
        for belief_type in BeliefType::iter() {
            if belief_type == BeliefType::None { continue; }
            available_beliefs.add(belief_type, this.number_of_beliefs_available(belief_type));
        }

        // function to help with bookkeeping
        let mut choose_belief_to_add = |belief_type: BeliefType, number: i32| {
            let number_to_add = std::cmp::min(number, available_beliefs.get(&belief_type));
            beliefs_to_choose.add(belief_type, number_to_add);
            available_beliefs.add(belief_type, -number_to_add);
            if belief_type != BeliefType::Any {
                // deduct from BeliefType::Any as well
                available_beliefs.add(BeliefType::Any, -number_to_add);
            }
        };

        if enhancing_religion {
            choose_belief_to_add(BeliefType::Enhancer, 1);
        } else {
            choose_belief_to_add(BeliefType::Founder, 1);
            if self.should_choose_pantheon_belief {
                choose_belief_to_add(BeliefType::Pantheon, 1);
            }
        }
        choose_belief_to_add(BeliefType::Follower, 1);

        for unique in civ.get_matching_uniques(UniqueType::FreeExtraBeliefs) {
            if unique.params[2] != action { continue; }
            let belief_type = BeliefType::from_str(&unique.params[1]).unwrap();
            choose_belief_to_add(belief_type, unique.params[0].parse().unwrap());
        }
        for unique in civ.get_matching_uniques(UniqueType::FreeExtraAnyBeliefs) {
            if unique.params[1] != action { continue; }
            choose_belief_to_add(BeliefType::Any, unique.params[0].parse().unwrap());
        }

        for (belief_type, count) in this.free_beliefs_as_enums().entries() {
            choose_belief_to_add(belief_type, count);
        }

        beliefs_to_choose
    }

    pub fn get_beliefs_to_choose_at_founding(&self) -> Counter<BeliefType> {
        this.get_beliefs_to_choose_at_prophet_use(false)
    }

    pub fn get_beliefs_to_choose_at_enhancing(&self) -> Counter<BeliefType> {
        this.get_beliefs_to_choose_at_prophet_use(true)
    }

    pub fn choose_beliefs(&mut self, beliefs: Vec<Belief>, use_free_beliefs: bool) {
        let civ = self.civ.as_ref().expect("Civ not set");
        // Remove the free beliefs in case we had them
        // Must be done first in case when gain more later
        self.free_beliefs.clear();

        if this.religion_state == ReligionState::None {
            this.found_pantheon(beliefs[0].name.clone(), use_free_beliefs);  // makes religion non-null
        }
        // add beliefs (religion exists at this point)
        self.religion.as_mut().unwrap().add_beliefs(beliefs.clone());

        match this.religion_state {
            ReligionState::None => {
                self.religion_state = ReligionState::Pantheon;
                for unique in civ.get_triggered_uniques(UniqueType::TriggerUponFoundingPantheon) {
                    UniqueTriggerActivation::trigger_unique(unique, civ);
                }
            }
            ReligionState::FoundingReligion => {
                self.religion_state = ReligionState::Religion;
                for unique in civ.get_triggered_uniques(UniqueType::TriggerUponFoundingReligion) {
                    UniqueTriggerActivation::trigger_unique(unique, civ);
                }
            }
            ReligionState::EnhancingReligion => {
                self.religion_state = ReligionState::EnhancedReligion;
                for unique in civ.get_triggered_uniques(UniqueType::TriggerUponEnhancingReligion) {
                    UniqueTriggerActivation::trigger_unique(unique, civ);
                }
            }
            _ => {}
        }

        for unique in civ.get_triggered_uniques(UniqueType::TriggerUponAdoptingPolicyOrBelief) {
            for belief in &beliefs {
                if unique.get_modifiers(UniqueType::TriggerUponAdoptingPolicyOrBelief)
                    .iter()
                    .any(|m| m.params[0] == belief.name) {
                    UniqueTriggerActivation::trigger_unique(
                        unique,
                        civ,
                        Some(format!("due to adopting [{}]", belief.name))
                    );
                }
            }
        }

        for belief in &beliefs {
            for unique in belief.unique_objects.iter()
                .filter(|u| !u.has_trigger_conditional() && u.conditionals_apply(&civ.state)) {
                UniqueTriggerActivation::trigger_unique(unique, civ);
            }
        }

        civ.update_stats_for_next_turn();  // a belief can have an immediate effect on stats
    }

    pub fn found_religion_with_name(&mut self, display_name: String, name: String) {
        let civ = self.civ.as_ref().expect("Civ not set");
        let mut new_religion = Religion::new(name.clone(), civ.game_info.clone(), civ.civ_name.clone());
        new_religion.display_name = display_name;
        if let Some(ref old_religion) = self.religion {
            new_religion.add_beliefs(old_religion.get_all_beliefs_ordered());
        }

        self.religion = Some(new_religion.clone());
        civ.game_info.religions.insert(name.clone(), new_religion);

        if let Some(holy_city) = civ.cities.iter().find(|c| c.id == self.founding_city_id) {
            holy_city.religion.religion_this_is_the_holy_city_of = Some(name.clone());
            holy_city.religion.add_pressure(name, holy_city.population.population * 500);
        }

        self.founding_city_id = None;
        self.should_choose_pantheon_belief = false;

        for unit in civ.units.get_civ_units() {
            if unit.has_unique(UniqueType::ReligiousUnit) && unit.has_unique(UniqueType::TakeReligionOverBirthCity) {
                unit.religion = Some(name.clone());
            }
        }
    }

    pub fn may_spread_religion_at_all(&self, missionary: &MapUnit) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");
        if !civ.is_major_civ() { return false; } // Only major civs
        if !civ.game_info.is_religion_enabled() { return false; } // No religion, no spreading

        let religion = missionary.civ.game_info.religions.get(&missionary.religion?)?;
        if religion.is_pantheon() { return false; }
        if UnitActionModifiers::get_usable_unit_action_uniques(missionary, UniqueType::CanSpreadReligion).is_empty() { return false; }
        true
    }

    pub fn may_spread_religion_now(&self, missionary: &MapUnit) -> bool {
        if !this.may_spread_religion_at_all(missionary) { return false; }
        if missionary.get_tile().get_owner().is_none() { return false; }
        if missionary.current_tile.owning_city.as_ref()
            .and_then(|c| c.religion.get_majority_religion())
            .map_or(false, |r| r.name == missionary.religion?) {
            return false;
        }
        if missionary.get_tile().get_city()
            .map_or(false, |c| c.religion.is_protected_by_inquisitor(&missionary.religion?)) {
            return false;
        }
        true
    }

    pub fn number_of_cities_following_this_religion(&self) -> i32 {
        if self.religion.is_none() { return 0; }
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.game_info.get_cities()
            .iter()
            .filter(|c| c.religion.get_majority_religion().map_or(false, |r| r == *self.religion.as_ref().unwrap()))
            .count() as i32
    }

    pub fn number_of_followers_following_this_religion(&self, city_filter: &str) -> i32 {
        if self.religion.is_none() { return 0; }
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.game_info.get_cities()
            .iter()
            .filter(|c| c.matches_filter(city_filter, civ))
            .map(|c| c.religion.get_followers_of(self.religion.as_ref().unwrap().name.clone()))
            .sum()
    }

    pub fn get_holy_city(&self) -> Option<City> {
        if self.religion.is_none() { return None; }
        let civ = self.civ.as_ref().expect("Civ not set");
        civ.game_info.get_cities()
            .into_iter()
            .find(|c| c.is_holy_city_of(&self.religion.as_ref().unwrap().name))
    }

    pub fn get_majority_religion(&self) -> Option<Religion> {
        let civ = self.civ.as_ref().expect("Civ not set");
        // let's count for each religion (among those actually presents in civ's cities)
        let mut religion_counter = Counter::new();
        for city in &civ.cities {
            // if city's majority Religion is null, let's just continue to next loop iteration
            let city_majority_religion = city.religion.get_majority_religion()?;
            // if not yet continued to next iteration from previous line, let's add the Religion to religion_counter
            religion_counter.add(city_majority_religion.clone(), 1);
        }
        // let's get the max-counted Religion if there is one, null otherwise; if null, return null
        let max_religion_counter_entry = religion_counter.entries()
            .max_by_key(|(_, count)| *count)?;
        // if not returned null from prev. line, check if the maxReligion is in most of the cities
        if max_religion_counter_entry.1 > civ.cities.len() / 2 {
            // if maxReligionCounterEntry > half-cities-count we return the Religion of maxReligionCounterEntry
            Some(max_religion_counter_entry.0.clone())
        } else {
            // if maxReligionCounterEntry <= half-cities-count we just return null
            None
        }
    }
}