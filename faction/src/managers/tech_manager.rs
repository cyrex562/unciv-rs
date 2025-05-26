use crate::city::City;
use crate::civilization::{
    AlertType, Civilization, LocationAction, NotificationCategory, NotificationIcon, PlayerType,
    PopupAlert, TechAction,
};
use crate::constants::Constants;
use crate::map::tile::RoadStatus;
use crate::models::ruleset::{
    unique::{StateForConditionals, UniqueMap, UniqueTriggerActivation, UniqueType},
    unit::BaseUnit,
    Era, INonPerpetualConstruction, Technology, TileResource,
};
use crate::utils::random::Random;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Manages technology research and related functionality for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct TechManager {
    #[serde(skip)]
    pub era: Era,

    #[serde(skip)]
    pub civ: Option<Arc<Civilization>>,

    #[serde(skip)]
    pub researched_technologies: Vec<Technology>,

    #[serde(skip)]
    pub tech_uniques: UniqueMap,

    // MapUnit.canPassThrough is the most called function in the game, and having these extremely specific booleans is one way of improving the time cost
    #[serde(skip)]
    pub units_can_embark: bool,

    #[serde(skip)]
    pub embarked_units_can_enter_ocean: bool,

    #[serde(skip)]
    pub all_units_can_enter_ocean: bool,

    #[serde(skip)]
    pub specific_units_can_enter_ocean: bool,

    // UnitMovementAlgorithms.getMovementCostBetweenAdjacentTiles is a close second =)
    #[serde(skip)]
    pub movement_speed_on_roads: f32,

    #[serde(skip)]
    pub roads_connect_across_rivers: bool,

    #[serde(skip)]
    pub all_techs_are_researched: bool,

    pub free_techs: i32,
    // For calculating score
    pub repeating_techs_researched: i32,

    /** For calculating Great Scientist yields - see https://civilization.fandom.com/wiki/Great_Scientist_(Civ5)  */
    pub science_of_last_8_turns: Vec<i32>,
    pub science_from_research_agreements: i32,
    /** This is the list of strings, which is serialized */
    pub techs_researched: HashSet<String>,

    /** When moving towards a certain tech, the user doesn't have to manually pick every one. */
    pub techs_to_research: Vec<String>,
    pub overflow_science: i32,
    pub techs_in_progress: HashMap<String, i32>,

    /** In civ IV, you can auto-convert a certain percentage of gold in cities to science */
    pub gold_percent_converted_to_science: f32,
}

impl TechManager {
    pub fn new() -> Self {
        Self {
            era: Era::new(),
            civ: None,
            researched_technologies: Vec::new(),
            tech_uniques: UniqueMap::new(),
            units_can_embark: false,
            embarked_units_can_enter_ocean: false,
            all_units_can_enter_ocean: false,
            specific_units_can_enter_ocean: false,
            movement_speed_on_roads: 1.0,
            roads_connect_across_rivers: false,
            all_techs_are_researched: false,
            free_techs: 0,
            repeating_techs_researched: 0,
            science_of_last_8_turns: vec![0; 8],
            science_from_research_agreements: 0,
            techs_researched: HashSet::new(),
            techs_to_research: Vec::new(),
            overflow_science: 0,
            techs_in_progress: HashMap::new(),
            gold_percent_converted_to_science: 0.6,
        }
    }

    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.techs_researched = self.techs_researched.clone();
        to_return.free_techs = self.free_techs;
        to_return.repeating_techs_researched = self.repeating_techs_researched;
        to_return.techs_in_progress = self.techs_in_progress.clone();
        to_return.techs_to_research = self.techs_to_research.clone();
        to_return.science_of_last_8_turns = self.science_of_last_8_turns.clone();
        to_return.science_from_research_agreements = self.science_from_research_agreements;
        to_return.overflow_science = self.overflow_science;
        to_return.gold_percent_converted_to_science = self.gold_percent_converted_to_science;
        to_return
    }

    pub fn get_number_of_techs_researched(&self) -> usize {
        self.techs_researched.len()
    }

    pub fn get_overflow_science(&self) -> i32 {
        self.overflow_science
    }

    fn get_science_modifier(&self, tech_name: &str) -> f32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let number_of_civs_researched_this_tech = civ
            .get_known_civs()
            .iter()
            .filter(|it| it.is_major_civ() && it.tech.is_researched(tech_name))
            .count();
        let number_of_civs_remaining = civ
            .game_info
            .civilizations
            .iter()
            .filter(|it| it.is_major_civ() && !it.is_defeated())
            .count();
        1.0 + (number_of_civs_researched_this_tech as f32) / (number_of_civs_remaining as f32) * 0.3
    }

    fn get_ruleset(&self) -> &crate::models::ruleset::Ruleset {
        let civ = self.civ.as_ref().expect("Civ not set");
        &civ.game_info.ruleset
    }

    pub fn cost_of_tech(&self, tech_name: &str) -> i32 {
        let civ = self.civ.as_ref().expect("Civ not set");
        let ruleset = self.get_ruleset();
        let tech = ruleset.technologies.get(tech_name).expect("Tech not found");

        let mut tech_cost = tech.cost as f32;

        if civ.is_human() {
            tech_cost *= civ.get_difficulty().research_cost_modifier;
        }

        tech_cost *= civ.game_info.speed.science_cost_modifier;
        tech_cost /= self.get_science_modifier(tech_name);

        let map_size_predef = civ
            .game_info
            .tile_map
            .map_parameters
            .map_size
            .get_predefined_or_next_smaller();
        tech_cost *= map_size_predef.tech_cost_multiplier;

        let mut city_modifier = (civ.cities.iter().filter(|it| !it.is_puppet()).count() - 1) as f32
            * map_size_predef.tech_cost_per_city_modifier;

        for unique in civ.get_matching_uniques(UniqueType::LessTechCostFromCities) {
            city_modifier *= 1.0 - unique.params[0].parse::<f32>().unwrap() / 100.0;
        }

        for unique in civ.get_matching_uniques(UniqueType::LessTechCost) {
            tech_cost *= unique.params[0].parse::<f32>().unwrap() / 100.0;
        }

        tech_cost *= 1.0 + city_modifier;
        tech_cost as i32
    }

    pub fn current_technology(&self) -> Option<&Technology> {
        let current_technology_name = self.current_technology_name()?;
        Some(
            self.get_ruleset()
                .technologies
                .get(current_technology_name)?,
        )
    }

    pub fn current_technology_name(&self) -> Option<&str> {
        if self.techs_to_research.is_empty() {
            None
        } else {
            Some(&self.techs_to_research[0])
        }
    }

    pub fn research_of_tech(&self, tech_name: Option<&str>) -> i32 {
        match tech_name {
            Some(name) => *self.techs_in_progress.get(name).unwrap_or(&0),
            None => 0,
        }
    }

    pub fn remaining_science_to_tech(&self, tech_name: &str) -> i32 {
        let spare_science = if self.can_be_researched(tech_name) {
            self.get_overflow_science()
        } else {
            0
        };
        self.cost_of_tech(tech_name) - self.research_of_tech(Some(tech_name)) - spare_science
    }

    pub fn turns_to_tech(&self, tech_name: &str) -> String {
        let civ = self.civ.as_ref().expect("Civ not set");
        let remaining_cost = self.remaining_science_to_tech(tech_name) as f64;

        if remaining_cost <= 0.0 {
            "0".to_string()
        } else if civ.stats.stats_for_next_turn.science <= 0.0 {
            "∞".to_string()
        } else {
            let turns = (remaining_cost / civ.stats.stats_for_next_turn.science)
                .ceil()
                .max(1.0) as i32;
            turns.to_string()
        }
    }

    pub fn is_researched(&self, tech_name: &str) -> bool {
        self.techs_researched.contains(tech_name)
    }

    pub fn is_researched_construction(&self, construction: &dyn INonPerpetualConstruction) -> bool {
        construction
            .required_techs()
            .iter()
            .all(|required_tech| self.is_researched(required_tech))
    }

    /** resources which need no research count as researched */
    pub fn is_revealed(&self, resource: &TileResource) -> bool {
        match resource.revealed_by {
            Some(ref revealed_by) => self.is_researched(revealed_by),
            None => true,
        }
    }

    pub fn is_obsolete(&self, unit: &BaseUnit) -> bool {
        unit.techs_that_obsolete_this()
            .iter()
            .any(|obsolete_tech| self.is_researched(obsolete_tech))
    }

    pub fn is_unresearchable(&self, tech: &Technology) -> bool {
        let civ = self.civ.as_ref().expect("Civ not set");

        if tech
            .get_matching_uniques(
                UniqueType::OnlyAvailable,
                &StateForConditionals::ignore_conditionals(),
            )
            .iter()
            .any(|unique| !unique.conditionals_apply(&civ.state))
        {
            return true;
        }

        if tech.has_unique(UniqueType::Unavailable, &civ.state) {
            return true;
        }

        false
    }

    pub fn can_be_researched(&self, tech_name: &str) -> bool {
        let tech = self
            .get_ruleset()
            .technologies
            .get(tech_name)
            .expect("Tech not found");

        if self.is_unresearchable(tech) {
            return false;
        }

        if self.is_researched(tech.name) && !tech.is_continually_researchable() {
            return false;
        }

        tech.prerequisites
            .iter()
            .all(|prereq| self.is_researched(prereq))
    }

    pub fn all_techs_are_researched(&self) -> bool {
        self.all_techs_are_researched
    }

    /** Returns empty list if no path exists */
    pub fn get_required_techs_to_destination(
        &self,
        destination_tech: &Technology,
    ) -> Vec<&Technology> {
        let mut prerequisites = Vec::new();
        let mut check_prerequisites = VecDeque::new();

        if self.is_unresearchable(destination_tech) {
            return Vec::new();
        }

        check_prerequisites.push_back(destination_tech);

        while !check_prerequisites.is_empty() {
            let tech_to_check = check_prerequisites.pop_front().unwrap();

            if self.is_unresearchable(tech_to_check) {
                return Vec::new();
            }

            // future tech can have been researched even when we're researching it,
            // so...if we skip it we'll end up with 0 techs in the "required techs", which will mean that we don't have anything to research. Yeah.
            if !tech_to_check.is_continually_researchable()
                && (self.is_researched(&tech_to_check.name)
                    || prerequisites.contains(&tech_to_check))
            {
                continue; //no need to add or check prerequisites
            }

            for prerequisite in &tech_to_check.prerequisites {
                check_prerequisites
                    .push_back(self.get_ruleset().technologies.get(prerequisite).unwrap());
            }

            prerequisites.push(tech_to_check);
        }

        // Sort by column number
        prerequisites.sort_by(|a, b| {
            let a_col = a.column.as_ref().map(|col| col.column_number).unwrap_or(0);
            let b_col = b.column.as_ref().map(|col| col.column_number).unwrap_or(0);
            a_col.cmp(&b_col)
        });

        prerequisites
    }

    pub fn get_science_from_great_scientist(&self) -> i32 {
        // https://civilization.fandom.com/wiki/Great_Scientist_(Civ5)
        let civ = self.civ.as_ref().expect("Civ not set");
        (self.science_of_last_8_turns.iter().sum::<i32>() as f32
            * civ.game_info.speed.science_cost_modifier) as i32
    }

    fn add_current_science_to_science_of_last_8_turns(&mut self, science: i32) {
        let civ = self.civ.as_ref().expect("Civ not set");
        self.science_of_last_8_turns[civ.game_info.turns % 8] = science;
    }

    fn limit_overflow_science(&self, overflow_science: i32) -> i32 {
        // http://www.civclub.net/bbs/forum.php?mod=viewthread&tid=123976
        // Apparently yes, we care about the absolute tech cost, not the actual calculated-for-this-player tech cost,
        //  so don't change to costOfTech()
        let civ = self.civ.as_ref().expect("Civ not set");
        let current_tech_name = self
            .current_technology_name()
            .expect("No current technology");
        let current_tech = self
            .get_ruleset()
            .technologies
            .get(current_tech_name)
            .expect("Tech not found");

        let max_overflow =
            (civ.stats.stats_for_next_turn.science as i32 * 5).max(current_tech.cost);
        overflow_science.min(max_overflow)
    }

    fn science_from_research_agreements(&self) -> i32 {
        // https://forums.civfanatics.com/resources/research-agreements-bnw.25568/
        let civ = self.civ.as_ref().expect("Civ not set");
        let mut research_agreement_modifier = 0.5;

        for unique in civ.get_matching_uniques(UniqueType::ScienceFromResearchAgreements) {
            research_agreement_modifier += unique.params[0].parse::<f32>().unwrap() / 200.0;
        }

        (self.science_from_research_agreements as f32 / 3.0 * research_agreement_modifier) as i32
    }

    pub fn end_turn(&mut self, science_for_new_turn: i32) {
        self.add_current_science_to_science_of_last_8_turns(science_for_new_turn);

        if self.current_technology_name().is_none() {
            return;
        }

        let mut final_science_to_add = science_for_new_turn;

        if self.science_from_research_agreements != 0 {
            let science_boost = self.science_from_research_agreements();
            final_science_to_add += science_boost;
            self.science_from_research_agreements = 0;

            let civ = self.civ.as_ref().expect("Civ not set");
            civ.add_notification(
                format!(
                    "We gained [{}] Science from Research Agreement",
                    science_boost
                ),
                NotificationCategory::General,
                NotificationIcon::Science,
            );
        }

        if self.overflow_science != 0 {
            final_science_to_add += self.get_overflow_science();
            self.overflow_science = 0;
        }

        self.add_science(final_science_to_add);
    }

    pub fn add_science(&mut self, science_get: i32) {
        let current_technology = match self.current_technology_name() {
            Some(name) => name,
            None => return,
        };

        let current_progress = self.research_of_tech(Some(current_technology));
        self.techs_in_progress.insert(
            current_technology.to_string(),
            current_progress + science_get,
        );

        if self.techs_in_progress.get(current_technology).unwrap()
            < &self.cost_of_tech(current_technology)
        {
            return;
        }

        // We finished it!
        // http://www.civclub.net/bbs/forum.php?mod=viewthread&tid=123976
        let extra_science_left_over = self.techs_in_progress.get(current_technology).unwrap()
            - self.cost_of_tech(current_technology);
        self.overflow_science += self.limit_overflow_science(extra_science_left_over);
        self.add_technology(current_technology);
    }

    /**
     * Checks whether the research on the current technology can be completed
     * and, if so, completes the research.
     */
    pub fn update_research_progress(&mut self) {
        let current_technology = match self.current_technology_name() {
            Some(name) => name,
            None => return,
        };

        let real_overflow = self.get_overflow_science();
        let science_spent = self.research_of_tech(Some(current_technology)) + real_overflow;

        if science_spent >= self.cost_of_tech(current_technology) {
            self.overflow_science = 0;
            if real_overflow != 0 {
                self.add_science(real_overflow);
            }
        }
    }

    pub fn get_free_technology(&mut self, tech_name: &str) {
        self.free_techs -= 1;
        self.add_technology(tech_name);
    }

    pub fn add_technology(&mut self, tech_name: &str, show_notification: bool) {
        let civ = self.civ.as_ref().expect("Civ not set");
        let is_new_tech = self.techs_researched.insert(tech_name.to_string());

        // this is to avoid concurrent modification problems
        let new_tech = self
            .get_ruleset()
            .technologies
            .get(tech_name)
            .expect("Tech not found");

        if !new_tech.is_continually_researchable() {
            self.techs_to_research.retain(|t| t != tech_name);
        } else {
            self.repeating_techs_researched += 1;
        }

        self.techs_in_progress.remove(tech_name);
        self.researched_technologies.push(new_tech.clone());
        self.add_tech_to_transients(new_tech);

        self.move_to_new_era(show_notification);

        if !civ.is_spectator() && show_notification {
            civ.add_notification(
                format!("Research of [{}] has completed!", tech_name),
                TechAction::new(tech_name),
                NotificationCategory::General,
                NotificationIcon::Science,
            );
        }

        if is_new_tech {
            civ.popup_alerts
                .push(PopupAlert::new(AlertType::TechResearched, tech_name));
        }

        let trigger_notification_text = format!("due to researching [{}]", tech_name);

        for unique in &new_tech.unique_objects {
            if !unique.has_trigger_conditional() && unique.conditionals_apply(&civ.state) {
                UniqueTriggerActivation::trigger_unique(
                    unique.clone(),
                    civ.clone(),
                    Some(trigger_notification_text.clone()),
                );
            }
        }

        for unique in civ
            .get_triggered_uniques(UniqueType::TriggerUponResearch)
            .iter()
            .filter(|unique| new_tech.matches_filter(&unique.params[0], &civ.state))
        {
            UniqueTriggerActivation::trigger_unique(
                unique.clone(),
                civ.clone(),
                Some(trigger_notification_text.clone()),
            );
        }

        let revealed_resources: Vec<&TileResource> = self
            .get_ruleset()
            .tile_resources
            .values()
            .filter(|resource| Some(tech_name) == resource.revealed_by.as_deref())
            .collect();

        if civ.player_type == PlayerType::Human {
            for revealed_resource in revealed_resources {
                civ.game_info.notify_explored_resources(
                    civ.clone(),
                    revealed_resource.name.clone(),
                    5,
                );
            }
        }

        self.update_transient_booleans();

        // In the case of a player hurrying research, this civ's resource availability may now be out of date
        // - e.g. when an owned tile by luck already has an appropriate improvement or when a tech provides a resource.
        // That can be seen on WorldScreenTopBar, so better update.
        civ.cache.update_civ_resources();

        for city in &civ.cities {
            city.reassign_population_deferred();
        }

        self.obsolete_old_units(tech_name);

        for unique in civ.get_matching_uniques(UniqueType::MayanGainGreatPerson) {
            if unique.params[1] != tech_name {
                continue;
            }
            civ.add_notification(
                "You have unlocked [The Long Count]!".to_string(),
                crate::civilization::MayaLongCountAction::new(),
                NotificationCategory::General,
                crate::ui::components::MayaCalendar::notification_icon(),
            );
        }

        self.update_research_progress();
    }

    /** A variant of kotlin's [associateBy] that omits null values */
    fn associate_by_not_null<T, K, V, F, G>(
        iter: impl Iterator<Item = T>,
        key_selector: F,
        value_transform: G,
    ) -> HashMap<K, V>
    where
        F: Fn(&T) -> K,
        G: Fn(&T) -> Option<V>,
        K: std::hash::Hash + Eq,
    {
        let mut destination = HashMap::new();
        for element in iter {
            if let Some(value) = value_transform(&element) {
                destination.insert(key_selector(&element), value);
            }
        }
        destination
    }

    fn obsolete_old_units(&mut self, tech_name: &str) {
        let civ = self.civ.as_ref().expect("Civ not set");

        // First build a map with obsoleted units to their (nation-specific) upgrade
        fn get_equivalent_upgrade_or_null(
            unit: &BaseUnit,
            tech_name: &str,
            civ: &Civilization,
        ) -> Option<BaseUnit> {
            let unit_upgrades_to =
                unit.automatically_upgraded_in_production_to_unit_by_tech(tech_name)?;
            Some(civ.get_equivalent_unit(unit_upgrades_to))
        }

        let obsolete_units = Self::associate_by_not_null(
            self.get_ruleset().units.entries.iter(),
            |entry| entry.key.clone(),
            |entry| get_equivalent_upgrade_or_null(entry.value, tech_name, civ),
        );

        if obsolete_units.is_empty() {
            return;
        }

        // Apply each to all cities - and remember which cities had which obsoleted unit
        //  in their construction queues in this Map<String, MutableSet<City>>:
        let mut unit_upgrades: HashMap<String, HashSet<&City>> = HashMap::new();
        for unit_name in obsolete_units.keys() {
            unit_upgrades.insert(unit_name.clone(), HashSet::new());
        }

        fn transform_construction(
            old: &str,
            city: &City,
            unit_upgrades: &mut HashMap<String, HashSet<&City>>,
            obsolete_units: &HashMap<String, BaseUnit>,
        ) -> Option<String> {
            let entry = unit_upgrades.get_mut(old)?;
            entry.insert(city);
            obsolete_units.get(old).map(|u| u.name.clone())
        }

        for city in &civ.cities {
            // Replace queue - the sequence iteration and finalization happens before the result
            // is reassigned, therefore no concurrent modification worries
            let mut new_queue = Vec::new();
            for construction in &city.city_constructions.construction_queue {
                if let Some(new_construction) =
                    transform_construction(construction, city, &mut unit_upgrades, &obsolete_units)
                {
                    new_queue.push(new_construction);
                }
            }
            city.city_constructions.construction_queue = new_queue;
        }

        // Add notifications for obsolete units/constructions
        for (unit, cities) in unit_upgrades {
            if cities.is_empty() {
                continue;
            }

            //The validation check happens again while processing start and end of turn,
            //but for mid-turn free tech picks like Oxford University, it should happen immediately
            //so the hammers from the obsolete unit are guaranteed to go to the upgraded unit
            //and players don't think they lost all their production mid turn
            for city in &cities {
                city.city_constructions.validate_in_progress_constructions();
            }

            let location_action =
                LocationAction::new(cities.iter().map(|city| city.location).collect());
            let city_text = if cities.len() == 1 {
                format!("[{}]", cities.iter().next().unwrap().name)
            } else {
                format!("[{}] cities", cities.len())
            };

            let new_unit = obsolete_units.get(&unit).map(|u| u.name.clone());
            let text = if new_unit.is_none() {
                format!(
                    "[{}] has become obsolete and was removed from the queue in {}!",
                    unit, city_text
                )
            } else {
                format!(
                    "{} changed production from [{}] to [{}]",
                    city_text,
                    unit,
                    new_unit.unwrap()
                )
            };

            let icons = if new_unit.is_none() {
                vec![NotificationIcon::Construction]
            } else {
                vec![
                    NotificationIcon::from_str(&unit).unwrap(),
                    NotificationIcon::Construction,
                    NotificationIcon::from_str(&new_unit.unwrap()).unwrap(),
                ]
            };

            civ.add_notification(
                text,
                location_action,
                NotificationCategory::Production,
                icons,
            );
        }
    }

    fn move_to_new_era(&mut self, show_notification: bool) {
        let civ = self.civ.as_ref().expect("Civ not set");
        let previous_era = civ.get_era();
        self.update_era();
        let current_era = civ.get_era();

        if previous_era == current_era {
            return;
        }

        if show_notification {
            if !civ.is_spectator() {
                civ.add_notification(
                    format!("You have entered the [{}]!", current_era.name),
                    NotificationCategory::General,
                    NotificationIcon::Science,
                );
            }

            if civ.is_major_civ() {
                for known_civ in civ.get_known_civs_with_spectators() {
                    known_civ.add_notification(
                        format!("[{}] has entered the [{}]!", civ.civ_name, current_era.name),
                        NotificationCategory::General,
                        civ.civ_name.clone(),
                        NotificationIcon::Science,
                    );
                }
            }

            for policy_branch in self
                .get_ruleset()
                .policy_branches
                .values()
                .filter(|branch| {
                    branch.era == current_era.name && civ.policies.is_adoptable(branch)
                })
            {
                if !civ.is_spectator() {
                    civ.add_notification(
                        format!("[{}] policy branch unlocked!", policy_branch.name),
                        crate::civilization::PolicyAction::new(policy_branch.name.clone()),
                        NotificationCategory::General,
                        NotificationIcon::Culture,
                    );
                }
            }
        }

        let eras_passed: Vec<&Era> = self
            .get_ruleset()
            .eras
            .values()
            .filter(|era| {
                era.era_number > previous_era.era_number && era.era_number <= current_era.era_number
            })
            .collect();

        // Sort by era number
        let mut eras_passed = eras_passed;
        eras_passed.sort_by(|a, b| a.era_number.cmp(&b.era_number));

        for era in eras_passed {
            for unique in &era.unique_objects {
                if !unique.has_trigger_conditional() && unique.conditionals_apply(&civ.state) {
                    UniqueTriggerActivation::trigger_unique(
                        unique.clone(),
                        civ.clone(),
                        Some(format!("due to entering the [{}]", era.name)),
                    );
                }
            }
        }

        let era_names: HashSet<String> = eras_passed.iter().map(|era| era.name.clone()).collect();

        for unique in civ.get_triggered_uniques(UniqueType::TriggerUponEnteringEra) {
            for era_name in &era_names {
                if unique
                    .get_modifiers(UniqueType::TriggerUponEnteringEra)
                    .iter()
                    .any(|modifier| modifier.params[0] == *era_name)
                {
                    UniqueTriggerActivation::trigger_unique(
                        unique.clone(),
                        civ.clone(),
                        Some(format!("due to entering the [{}]", era_name)),
                    );
                }
            }
        }

        // The unfiltered version
        for unique in civ.get_triggered_uniques(UniqueType::TriggerUponEnteringEraUnfiltered) {
            UniqueTriggerActivation::trigger_unique(
                unique.clone(),
                civ.clone(),
                Some(format!("due to entering the [{}]", current_era.name)),
            );
        }
    }

    fn update_era(&mut self) {
        let civ = self.civ.as_ref().expect("Civ not set");
        let ruleset = self.get_ruleset();

        if ruleset.technologies.is_empty() || self.researched_technologies.is_empty() {
            return;
        }

        let max_era_of_researched_techs = self
            .researched_technologies
            .iter()
            .map(|tech| tech.column.as_ref().unwrap())
            .max_by_key(|col| col.column_number)
            .unwrap()
            .era;

        let max_era = ruleset.eras.get(&max_era_of_researched_techs).unwrap();

        let min_era_of_non_researched_techs = ruleset
            .technologies
            .values
            .iter()
            .filter(|tech| !self.researched_technologies.contains(tech))
            .map(|tech| tech.column.as_ref().unwrap())
            .min_by_key(|col| col.column_number)
            .map(|col| col.era);

        if min_era_of_non_researched_techs.is_none() {
            self.era = max_era.clone();
            return;
        }

        let min_era = ruleset
            .eras
            .get(&min_era_of_non_researched_techs.unwrap())
            .unwrap();

        self.era = if min_era.era_number <= max_era.era_number {
            max_era.clone()
        } else {
            min_era.clone()
        };
    }

    fn add_tech_to_transients(&mut self, tech: &Technology) {
        self.tech_uniques.add_uniques(&tech.unique_objects);
    }

    pub fn set_transients(&mut self, civ: Arc<Civilization>) {
        self.civ = Some(civ.clone());
        self.researched_technologies = self
            .techs_researched
            .iter()
            .map(|tech_name| {
                self.get_ruleset()
                    .technologies
                    .get(tech_name)
                    .unwrap()
                    .clone()
            })
            .collect();

        for tech in &self.researched_technologies {
            self.add_tech_to_transients(tech);
        }

        self.update_era(); // before updateTransientBooleans so era-based conditionals can work
        self.update_transient_booleans();
    }

    fn update_transient_booleans(&mut self) {
        let civ = self.civ.as_ref().expect("Civ not set");

        self.units_can_embark = civ.has_unique(UniqueType::LandUnitEmbarkation);

        let enter_ocean_uniques = civ.get_matching_uniques(UniqueType::UnitsMayEnterOcean);
        self.all_units_can_enter_ocean = enter_ocean_uniques
            .iter()
            .any(|unique| unique.params[0] == Constants::ALL);
        self.embarked_units_can_enter_ocean = self.all_units_can_enter_ocean
            || enter_ocean_uniques
                .iter()
                .any(|unique| unique.params[0] == Constants::EMBARKED);
        self.specific_units_can_enter_ocean = enter_ocean_uniques.iter().any(|unique| {
            unique.params[0] != Constants::ALL && unique.params[0] != Constants::EMBARKED
        });

        self.movement_speed_on_roads = if civ.has_unique(UniqueType::RoadMovementSpeed) {
            RoadStatus::Road.movement_improved
        } else {
            RoadStatus::Road.movement
        };

        self.roads_connect_across_rivers = civ.has_unique(UniqueType::RoadsConnectAcrossRivers);

        self.all_techs_are_researched = civ
            .game_info
            .ruleset
            .technologies
            .values
            .iter()
            .all(|tech| self.is_researched(&tech.name) || !self.can_be_researched(&tech.name));
    }

    pub fn get_best_road_available(&self) -> RoadStatus {
        let railroad_improvement = self.get_ruleset().railroad_improvement.clone(); // May not exist in mods
        if let Some(railroad) = railroad_improvement {
            if railroad.tech_required.is_none()
                || self.is_researched(railroad.tech_required.unwrap())
            {
                return RoadStatus::Railroad;
            }
        }

        let road_improvement = self.get_ruleset().road_improvement.clone();
        if let Some(road) = road_improvement {
            if road.tech_required.is_none() || self.is_researched(road.tech_required.unwrap()) {
                return RoadStatus::Road;
            }
        }

        RoadStatus::None
    }

    pub fn can_research_tech(&self) -> bool {
        self.get_ruleset()
            .technologies
            .values
            .iter()
            .any(|tech| self.can_be_researched(&tech.name))
    }
}
