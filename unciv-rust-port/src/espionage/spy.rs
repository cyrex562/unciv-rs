use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::f32;
use std::f64;

use crate::game_info::GameInfo;
use crate::civilization::Civilization;
use crate::city::City;
use crate::unique::{Unique, UniqueType, StateForConditionals};
use crate::constants::SPY_HIDEOUT;
use crate::diplomacy::DiplomaticModifiers;
use crate::notification::{NotificationCategory, NotificationIcon, NotificationAction};
use crate::espionage_manager::EspionageManager;
use crate::civ_flags::CivFlags;

/// Enum representing the different actions a spy can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpyAction {
    None,
    Moving,
    EstablishNetwork,
    Surveillance,
    StealingTech,
    RiggingElections,
    Coup,
    CounterIntelligence,
    Dead,
}

impl SpyAction {
    /// Returns the display string for the action
    pub fn display_string(&self) -> &'static str {
        match self {
            SpyAction::None => "None",
            SpyAction::Moving => "Moving",
            SpyAction::EstablishNetwork => "Establishing Network",
            SpyAction::Surveillance => "Observing City",
            SpyAction::StealingTech => "Stealing Tech",
            SpyAction::RiggingElections => "Rigging Elections",
            SpyAction::Coup => "Coup",
            SpyAction::CounterIntelligence => "Counter-intelligence",
            SpyAction::Dead => "Dead",
        }
    }

    /// Returns whether the action has a countdown of turns
    pub fn has_countdown_turns(&self) -> bool {
        match self {
            SpyAction::Moving | SpyAction::EstablishNetwork | SpyAction::Dead => true,
            _ => false,
        }
    }

    /// Returns whether the action should show turns
    pub fn show_turns(&self) -> bool {
        match self {
            SpyAction::Moving | SpyAction::EstablishNetwork | SpyAction::Dead => true,
            _ => false,
        }
    }

    /// Returns whether the spy is set up in the city
    pub fn is_set_up(&self) -> bool {
        match self {
            SpyAction::Surveillance | SpyAction::StealingTech | SpyAction::RiggingElections |
            SpyAction::Coup | SpyAction::CounterIntelligence => true,
            _ => false,
        }
    }

    /// Returns whether the spy is doing work
    pub fn is_doing_work(&self, spy: &Spy) -> bool {
        match self {
            SpyAction::Moving | SpyAction::EstablishNetwork | SpyAction::StealingTech |
            SpyAction::Coup => true,
            SpyAction::RiggingElections => !spy.civ_info.is_at_war_with(&spy.get_city().civ),
            SpyAction::CounterIntelligence => spy.turns_remaining_for_action > 0,
            _ => false,
        }
    }
}

/// Represents a spy in the game
#[derive(Clone, Serialize, Deserialize)]
pub struct Spy {
    name: String,
    rank: i32,
    location: Option<(i32, i32)>, // Using (x, y) tuple instead of Vector2
    action: SpyAction,
    turns_remaining_for_action: i32,
    progress_towards_stealing_tech: i32,

    #[serde(skip)]
    civ_info: Option<Civilization>,

    #[serde(skip)]
    espionage_manager: Option<EspionageManager>,

    #[serde(skip)]
    city: Option<City>,
}

impl Spy {
    /// Creates a new spy with the given name and rank
    pub fn new(name: String, rank: i32) -> Self {
        Spy {
            name,
            rank,
            location: None,
            action: SpyAction::None,
            turns_remaining_for_action: 0,
            progress_towards_stealing_tech: 0,
            civ_info: None,
            espionage_manager: None,
            city: None,
        }
    }

    /// Clones the spy
    pub fn clone(&self) -> Self {
        let mut to_return = Spy::new(self.name.clone(), self.rank);
        to_return.location = self.location;
        to_return.action = self.action;
        to_return.turns_remaining_for_action = self.turns_remaining_for_action;
        to_return.progress_towards_stealing_tech = self.progress_towards_stealing_tech;
        to_return
    }

    /// Sets the transients for the spy
    pub fn set_transients(&mut self, civ_info: Civilization) {
        self.civ_info = Some(civ_info.clone());
        self.espionage_manager = Some(civ_info.espionage_manager.clone());
    }

    /// Sets the action for the spy
    pub fn set_action(&mut self, new_action: SpyAction, turns: i32) {
        assert!(!new_action.has_countdown_turns() || turns > 0);
        self.action = new_action;
        self.turns_remaining_for_action = turns;
    }

    /// Ends the turn for the spy
    pub fn end_turn(&mut self) {
        if self.action.has_countdown_turns() && {
            self.turns_remaining_for_action -= 1;
            self.turns_remaining_for_action > 0
        } {
            return;
        }

        match self.action {
            SpyAction::None => return,
            SpyAction::Moving => {
                if self.get_city().civ == self.civ_info.as_ref().unwrap() {
                    // Your own cities are certainly familiar surroundings, so skip establishing a network
                    self.set_action(SpyAction::CounterIntelligence, 10);
                } else {
                    // Should depend on cultural familiarity level if that is ever implemented inter-civ
                    self.set_action(SpyAction::EstablishNetwork, 3);
                }
            },
            SpyAction::EstablishNetwork => {
                let city = self.get_city(); // This should never throw an exception, as going to the hideout sets your action to None.
                if city.civ.is_city_state() {
                    let turns = city.civ.flags_countdown.get(&CivFlags::TurnsTillCityStateElection.to_string())
                        .unwrap_or(&1) - 1;
                    self.set_action(SpyAction::RiggingElections, turns);
                } else if city.civ == self.civ_info.as_ref().unwrap() {
                    self.set_action(SpyAction::CounterIntelligence, 10);
                } else {
                    self.start_stealing_tech();
                }
            },
            SpyAction::Surveillance => {
                if !self.get_city().civ.is_major_civ() {
                    return;
                }

                let stealable_techs = self.espionage_manager.as_ref().unwrap()
                    .get_techs_to_steal(&self.get_city().civ);

                if stealable_techs.is_empty() || self.get_turns_remaining_to_steal_tech() < 0 {
                    return;
                }

                self.set_action(SpyAction::StealingTech); // There are new techs to steal!
            },
            SpyAction::StealingTech => {
                self.turns_remaining_for_action = self.get_turns_remaining_to_steal_tech();

                if self.turns_remaining_for_action < 0 {
                    // Either we have no technologies to steam (-1) or the city produces no science (-1)
                    self.set_action(SpyAction::Surveillance);
                    if self.turns_remaining_for_action == -1 {
                        self.add_notification(&format!(
                            "Your spy [{}] cannot steal any more techs from [{}] as we've already researched all the technology they know!",
                            self.name, self.get_city().civ.civ_name
                        ));
                    }
                } else if self.turns_remaining_for_action == 0 {
                    self.steal_tech();
                }
            },
            SpyAction::RiggingElections => {
                // No action done here
                // Handled in CityStateFunctions.nextTurnElections()
                // TODO: Once we remove support for the old flag system we can remove the null check
                // Our spies might update before the flag is created in the city-state
                let turns = self.get_city().civ.flags_countdown.get(&CivFlags::TurnsTillCityStateElection.to_string())
                    .unwrap_or(&0) - 1;
                self.turns_remaining_for_action = turns;
            },
            SpyAction::Coup => {
                self.initiate_coup();
            },
            SpyAction::Dead => {
                let old_spy_name = self.name.clone();
                self.name = self.espionage_manager.as_ref().unwrap().get_spy_name();
                self.set_action(SpyAction::None);
                self.rank = self.espionage_manager.as_ref().unwrap().get_starting_spy_rank();
                self.add_notification(&format!(
                    "We have recruited a new spy name [{}] after [{}] was killed.",
                    self.name, old_spy_name
                ));
            },
            SpyAction::CounterIntelligence => {
                // Counter intelligence spies don't do anything here
                // However the AI will want to keep track of how long a spy has been doing counter intelligence for
                // Once turnsRemainingForAction is <= 0 the spy won't be considered to be doing work any more
                self.turns_remaining_for_action -= 1;
                return;
            },
        }
    }

    /// Starts stealing tech
    fn start_stealing_tech(&mut self) {
        self.progress_towards_stealing_tech = 0;
        self.set_action(SpyAction::StealingTech);
    }

    /// Gets the number of turns remaining to steal tech
    ///
    /// Returns:
    /// - The number of turns left to steal the technology, note that this is a guess and may change.
    /// - A 0 means that we are ready to steal the technology.
    /// - A -1 means we have no technologies to steal.
    /// - A -2 means we the city produces no science
    fn get_turns_remaining_to_steal_tech(&self) -> i32 {
        let stealable_techs = self.espionage_manager.as_ref().unwrap()
            .get_techs_to_steal(&self.get_city().civ);

        if stealable_techs.is_empty() {
            return -1;
        }

        let mut tech_steal_cost = stealable_techs.iter()
            .map(|tech_name| {
                self.civ_info.as_ref().unwrap().game_info.ruleset.technologies.get(tech_name)
                    .unwrap().cost as f32
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let tech_steal_cost_modifier = self.civ_info.as_ref().unwrap().game_info.ruleset.mod_options.constants.spy_tech_steal_cost_modifier;
        let tech_speed_modifier = self.civ_info.as_ref().unwrap().game_info.speed.science_cost_modifier;

        tech_steal_cost *= tech_steal_cost_modifier * tech_speed_modifier;

        let mut progress_this_turn = self.get_city().city_stats.current_city_stats.science;

        if progress_this_turn <= 0.0 {
            return -2; // The city has no science
        }

        // 25% spy bonus for each level
        let rank_tech_steal_modifier = self.rank as f32 * self.civ_info.as_ref().unwrap().game_info.ruleset.mod_options.constants.spy_rank_steal_percent_bonus;
        progress_this_turn *= (rank_tech_steal_modifier + 75.0) / 100.0;
        progress_this_turn *= self.get_efficiency_modifier() as f32;

        self.progress_towards_stealing_tech += progress_this_turn as i32;

        if self.progress_towards_stealing_tech >= tech_steal_cost as i32 {
            return 0;
        } else {
            return (tech_steal_cost - self.progress_towards_stealing_tech as f32) / progress_this_turn.ceil() as i32;
        }
    }

    /// Steals tech from the city
    fn steal_tech(&mut self) {
        let city = self.get_city();
        let other_civ = &city.civ;
        let random_seed = self.random_seed();

        let stealable_techs = self.espionage_manager.as_ref().unwrap()
            .get_techs_to_steal(&city.civ);

        // Get a random tech to steal
        let stolen_tech = if !stealable_techs.is_empty() {
            let index = (random_seed % stealable_techs.len() as i32) as usize;
            Some(stealable_techs.iter().nth(index).unwrap().clone())
        } else {
            None
        };

        // Lower is better
        let mut spy_result = (random_seed % 300) as i32;
        // Add our spies experience
        spy_result -= self.get_skill_modifier_percent();
        // Subtract the experience of the counter intelligence spies
        let defending_spy = city.civ.espionage_manager.get_spy_assigned_to_city(&city);
        spy_result += defending_spy.as_ref().map_or(0, |spy| spy.get_skill_modifier_percent());

        let detection_string = if spy_result >= 200 {
            // The spy was killed in the attempt
            if defending_spy.is_none() {
                Some(format!(
                    "A spy from [{}] was found and killed trying to steal Technology in [{}]!",
                    self.civ_info.as_ref().unwrap().civ_name, city.name
                ))
            } else {
                Some(format!(
                    "A spy from [{}] was found and killed by [{}] trying to steal Technology in [{}]!",
                    self.civ_info.as_ref().unwrap().civ_name, defending_spy.unwrap().name, city.name
                ))
            }
        } else if stolen_tech.is_none() {
            None // Nothing to steal
        } else if spy_result < 0 {
            None // Not detected
        } else if spy_result < 100 {
            Some(format!(
                "An unidentified spy stole the Technology [{}] from [{}]!",
                stolen_tech.as_ref().unwrap(), city.name
            ))
        } else {
            Some(format!(
                "A spy from [{}] stole the Technology [{}] from [{}]!",
                self.civ_info.as_ref().unwrap().civ_name, stolen_tech.as_ref().unwrap(), city.name
            ))
        };

        if let Some(detection_string) = detection_string {
            // Not using Spy.addNotification, shouldn't open the espionage screen
            other_civ.add_notification(
                &detection_string,
                city.location,
                NotificationCategory::Espionage,
                NotificationIcon::Spy
            );
        }

        if spy_result < 200 && stolen_tech.is_some() {
            self.civ_info.as_ref().unwrap().tech.add_technology(&stolen_tech.unwrap());
            self.add_notification(&format!(
                "Your spy [{}] stole the Technology [{}] from [{}]!",
                self.name, stolen_tech.unwrap(), city.name
            ));
            self.level_up_spy(1);
        }

        if spy_result >= 200 {
            self.add_notification(&format!(
                "Your spy [{}] was killed trying to steal Technology in [{}]!",
                self.name, city.name
            ));
            if let Some(defending_spy) = defending_spy {
                defending_spy.level_up_spy(1);
            }
            self.kill_spy();
        } else {
            self.start_stealing_tech(); // reset progress
        }

        if spy_result >= 100 {
            if let Some(diplomacy_manager) = other_civ.get_diplomacy_manager(&self.civ_info.as_ref().unwrap()) {
                diplomacy_manager.add_modifier(DiplomaticModifiers::SpiedOnUs, -15.0);
            }
        }
    }

    /// Checks if the spy can do a coup
    pub fn can_do_coup(&self) -> bool {
        self.get_city_or_null().is_some() &&
        self.get_city().civ.is_city_state &&
        self.is_set_up() &&
        self.get_city().civ.get_ally_civ() != Some(self.civ_info.as_ref().unwrap().civ_name.clone())
    }

    /// Initiates a coup
    fn initiate_coup(&mut self) {
        if !self.can_do_coup() {
            // Maybe we are the new ally of the city-state
            // However we know that we are still in the city and it hasn't been conquered
            self.set_action(SpyAction::RiggingElections, 10);
            return;
        }

        let city_state = &self.get_city().civ;
        let ally_civ_name = city_state.get_ally_civ();
        let ally_civ = ally_civ_name.as_ref().map(|name| {
            self.civ_info.as_ref().unwrap().game_info.get_civilization(name)
        });

        let success_chance = self.get_coup_chance_of_success(true);
        let random_value = (self.random_seed() as f32) / (i32::MAX as f32);

        if random_value <= success_chance {
            // Success
            let previous_influence = if let Some(ally_civ) = &ally_civ {
                city_state.get_diplomacy_manager(ally_civ).unwrap().get_influence()
            } else {
                80.0
            };

            city_state.get_diplomacy_manager(&self.civ_info.as_ref().unwrap()).unwrap()
                .set_influence(previous_influence);

            self.civ_info.as_ref().unwrap().add_notification(
                &format!(
                    "Your spy [{}] successfully staged a coup in [{}]!",
                    self.name, city_state.civ_name
                ),
                self.get_city().location,
                NotificationCategory::Espionage,
                NotificationIcon::Spy,
                &city_state.civ_name
            );

            if let Some(ally_civ) = &ally_civ {
                city_state.get_diplomacy_manager_or_meet(ally_civ).reduce_influence(20.0);
                ally_civ.add_notification(
                    &format!(
                        "A spy from [{}] successfully staged a coup in our former ally [{}]!",
                        self.civ_info.as_ref().unwrap().civ_name, city_state.civ_name
                    ),
                    self.get_city().location,
                    NotificationCategory::Espionage,
                    &self.civ_info.as_ref().unwrap().civ_name,
                    NotificationIcon::Spy,
                    &city_state.civ_name
                );

                ally_civ.get_diplomacy_manager_or_meet(&self.civ_info.as_ref().unwrap())
                    .add_modifier(DiplomaticModifiers::SpiedOnUs, -15.0);
            }

            for civ in city_state.get_known_civs_with_spectators() {
                if Some(&civ) == ally_civ.as_ref() || Some(&civ) == self.civ_info.as_ref() {
                    continue;
                }

                civ.add_notification(
                    &format!(
                        "A spy from [{}] successfully staged a coup in [{}]!",
                        self.civ_info.as_ref().unwrap().civ_name, city_state.civ_name
                    ),
                    self.get_city().location,
                    NotificationCategory::Espionage,
                    &self.civ_info.as_ref().unwrap().civ_name,
                    NotificationIcon::Spy,
                    &city_state.civ_name
                );

                if civ.is_spectator() {
                    continue;
                }

                city_state.get_diplomacy_manager(&civ).unwrap().reduce_influence(10.0); // Guess
            }

            self.set_action(SpyAction::RiggingElections, 10);
            city_state.city_state_functions.update_ally_civ_for_city_state();
        } else {
            // Failure
            let spy = ally_civ.as_ref().and_then(|civ| {
                civ.espionage_manager.get_spy_assigned_to_city(&self.get_city())
            });

            city_state.get_diplomacy_manager(&self.civ_info.as_ref().unwrap()).unwrap()
                .add_influence(-20.0);

            if let Some(ally_civ) = &ally_civ {
                ally_civ.add_notification(
                    &format!(
                        "A spy from [{}] failed to stag a coup in our ally [{}] and was killed!",
                        self.civ_info.as_ref().unwrap().civ_name, city_state.civ_name
                    ),
                    self.get_city().location,
                    NotificationCategory::Espionage,
                    &self.civ_info.as_ref().unwrap().civ_name,
                    NotificationIcon::Spy,
                    &city_state.civ_name
                );

                ally_civ.get_diplomacy_manager_or_meet(&self.civ_info.as_ref().unwrap())
                    .add_modifier(DiplomaticModifiers::SpiedOnUs, -10.0);
            }

            self.civ_info.as_ref().unwrap().add_notification(
                &format!(
                    "Our spy [{}] failed to stag a coup in [{}] and was killed!",
                    self.name, city_state.civ_name
                ),
                self.get_city().location,
                NotificationCategory::Espionage,
                &self.civ_info.as_ref().unwrap().civ_name,
                NotificationIcon::Spy,
                &city_state.civ_name
            );

            self.kill_spy();

            if let Some(spy) = spy {
                spy.level_up_spy(1); // Technically not in Civ V, but it's like the same thing as with counter-intelligence
            }
        }
    }

    /// Calculates the success chance of a coup in this city state
    pub fn get_coup_chance_of_success(&self, include_unknown_factors: bool) -> f32 {
        let city_state = &self.get_city().civ;
        let mut success_percentage = 50.0;

        // Influence difference should always be a positive value
        let mut influence_difference = if let Some(ally_civ_name) = city_state.get_ally_civ() {
            city_state.get_diplomacy_manager_by_name(ally_civ_name).unwrap().get_influence()
        } else {
            60.0
        };

        influence_difference -= city_state.get_diplomacy_manager(&self.civ_info.as_ref().unwrap()).unwrap().get_influence();
        success_percentage -= influence_difference / 2.0;

        // If we are viewing the success chance we don't want to reveal that there is a defending spy
        let defending_spy = if include_unknown_factors {
            city_state.get_ally_civ().and_then(|ally_civ_name| {
                self.civ_info.as_ref().unwrap().game_info.get_civilization(&ally_civ_name)
            }).and_then(|ally_civ| {
                ally_civ.espionage_manager.get_spy_assigned_to_city(&self.get_city())
            })
        } else {
            None
        };

        let spy_ranks = self.get_skill_modifier_percent() - defending_spy.as_ref().map_or(0, |spy| spy.get_skill_modifier_percent());
        success_percentage += spy_ranks as f32 / 2.0; // Each rank counts for 15%

        success_percentage = success_percentage.max(0.0).min(85.0);
        success_percentage / 100.0
    }

    /// Moves the spy to a city
    pub fn move_to(&mut self, city: Option<&City>) {
        if city.is_none() { // Moving to spy hideout
            self.location = None;
            self.city = None;
            self.set_action(SpyAction::None);
            return;
        }

        let city = city.unwrap();
        self.location = Some(city.location);
        self.city = Some(city.clone());
        self.set_action(SpyAction::Moving, 1);
    }

    /// Checks if the spy can move to a city
    pub fn can_move_to(&self, city: &City) -> bool {
        if self.get_city_or_null().map_or(false, |c| c == city) {
            return true;
        }

        if !city.get_center_tile().is_explored(&self.civ_info.as_ref().unwrap()) {
            return false;
        }

        self.espionage_manager.as_ref().unwrap().get_spy_assigned_to_city(city).is_none()
    }

    /// Checks if the spy is set up
    pub fn is_set_up(&self) -> bool {
        self.action.is_set_up()
    }

    /// Checks if the spy is idle
    pub fn is_idle(&self) -> bool {
        self.action == SpyAction::None
    }

    /// Checks if the spy is doing work
    pub fn is_doing_work(&self) -> bool {
        self.action.is_doing_work(this)
    }

    /// Returns the City this Spy is in, or None if it is in the hideout
    pub fn get_city_or_null(&self) -> Option<&City> {
        if self.location.is_none() {
            return None;
        }

        if self.city.is_none() {
            let location = self.location.unwrap();
            let city = self.civ_info.as_ref().unwrap().game_info.tile_map.get(&location)
                .and_then(|tile| tile.get_city());

            if let Some(city) = city {
                // This is a bit of a hack since we can't modify self here
                // In a real implementation, you might want to use interior mutability
                unsafe {
                    let this = self as *const Spy as *mut Spy;
                    (*this).city = Some(city.clone());
                }
                return Some(city);
            }
        }

        self.city.as_ref()
    }

    /// Returns the City this Spy is in
    pub fn get_city(&self) -> &City {
        self.get_city_or_null().unwrap()
    }

    /// Gets the location name
    pub fn get_location_name(&self) -> String {
        self.get_city_or_null().map_or(SPY_HIDEOUT.to_string(), |city| city.name.clone())
    }

    /// Levels up the spy
    pub fn level_up_spy(&mut self, amount: i32) {
        let max_rank = self.civ_info.as_ref().unwrap().game_info.ruleset.mod_options.constants.max_spy_rank;

        if self.rank >= max_rank {
            return;
        }

        let ranks_to_level_up = (amount).min(max_rank - self.rank);

        if ranks_to_level_up == 1 {
            self.add_notification(&format!("Your spy [{}] has leveled up!", self.name));
        } else {
            self.add_notification(&format!("Your spy [{}] has leveled up [{}] times!", self.name, ranks_to_level_up));
        }

        self.rank += ranks_to_level_up;
    }

    /// Gets the skill modifier percent
    pub fn get_skill_modifier_percent(&self) -> i32 {
        self.rank * self.civ_info.as_ref().unwrap().game_info.ruleset.mod_options.constants.spy_rank_skill_percent_bonus
    }

    /// Gets the efficiency modifier
    pub fn get_efficiency_modifier(&self) -> f64 {
        let city = self.get_city_or_null();

        let (friendly_uniques, enemy_uniques) = match city {
            None => {
                // Spy is in hideout - effectiveness won't matter
                (
                    self.civ_info.as_ref().unwrap().get_matching_uniques(UniqueType::SpyEffectiveness),
                    Vec::new()
                )
            },
            Some(city) if city.civ == *self.civ_info.as_ref().unwrap() => {
                // Spy is in our own city
                (
                    city.get_matching_uniques(UniqueType::SpyEffectiveness, city.state.clone(), true),
                    Vec::new()
                )
            },
            Some(city) => {
                // Spy is active in a foreign city
                (
                    self.civ_info.as_ref().unwrap().get_matching_uniques(UniqueType::SpyEffectiveness),
                    city.get_matching_uniques(UniqueType::EnemySpyEffectiveness, city.state.clone(), true)
                )
            }
        };

        let mut total_efficiency = 1.0;

        let friendly_bonus = friendly_uniques.iter()
            .map(|unique| unique.params[0].parse::<i32>().unwrap_or(0))
            .sum::<i32>();

        let enemy_bonus = enemy_uniques.iter()
            .map(|unique| unique.params[0].parse::<i32>().unwrap_or(0))
            .sum::<i32>();

        total_efficiency *= (100.0 + friendly_bonus as f64) / 100.0;
        total_efficiency *= (100.0 + enemy_bonus as f64) / 100.0;

        total_efficiency.max(0.0)
    }

    /// Kills the spy
    fn kill_spy(&mut self) {
        // We don't actually remove this spy object, we set them as dead and let them revive
        self.move_to(None);
        self.set_action(SpyAction::Dead, 5);
        self.rank = 1;
    }

    /// Checks if the spy is alive
    pub fn is_alive(&self) -> bool {
        self.action != SpyAction::Dead
    }

    /// Adds a notification
    pub fn add_notification(&self, text: &str) {
        self.civ_info.as_ref().unwrap().add_notification(
            text,
            NotificationAction::with_location(self.location),
            NotificationCategory::Espionage,
            NotificationIcon::Spy
        );
    }

    /// Gets a random seed for the spy
    fn random_seed(&self) -> i32 {
        let city = self.get_city();
        let location = city.location;
        let turns = self.civ_info.as_ref().unwrap().game_info.turns;

        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);

        ((location.0 * location.1) as f32 + 123.0 * turns as f32) as i32 + (hasher.finish() as i32)
    }
}

impl fmt::Display for Spy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Debug for Spy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Spy {{ name: {}, rank: {}, action: {:?} }}", self.name, self.rank, self.action)
    }
}

impl Hash for Spy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.rank.hash(state);
        self.location.hash(state);
        self.action.hash(state);
    }
}

impl PartialEq for Spy {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
        self.rank == other.rank &&
        self.location == other.location &&
        self.action == other.action
    }
}

impl Eq for Spy {}