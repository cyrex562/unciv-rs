use crate::civilization::{Civilization, NotificationCategory, NotificationIcon, PlayerType, Proximity};
use crate::civilization::diplomacy::{CityStatePersonality, DiplomacyFlags, DiplomaticStatus};
use crate::models::ruleset::{Quest, QuestName};
use crate::models::ruleset::tile::{ResourceType, TileResource};
use crate::models::ruleset::unit::BaseUnit;
use crate::utils::extensions::{FillPlaceholders, GetPlaceholderParameters, ToPercent};
use crate::utils::random::RandomWeighted;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use std::f64;

/// Constants for quest management
pub const UNSET: i32 = -1;
pub const GLOBAL_QUEST_FIRST_POSSIBLE_TURN: i32 = 30;
pub const INDIVIDUAL_QUEST_FIRST_POSSIBLE_TURN: i32 = 30;
pub const GLOBAL_QUEST_FIRST_POSSIBLE_TURN_RAND: i32 = 20;
pub const INDIVIDUAL_QUEST_FIRST_POSSIBLE_TURN_RAND: i32 = 20;
pub const GLOBAL_QUEST_MIN_TURNS_BETWEEN: i32 = 40;
pub const INDIVIDUAL_QUEST_MIN_TURNS_BETWEEN: i32 = 20;
pub const GLOBAL_QUEST_RAND_TURNS_BETWEEN: i32 = 25;
pub const INDIVIDUAL_QUEST_RAND_TURNS_BETWEEN: i32 = 25;
pub const GLOBAL_QUEST_MAX_ACTIVE: i32 = 1;
pub const INDIVIDUAL_QUEST_MAX_ACTIVE: i32 = 2;

/// Manages quests for a civilization
#[derive(Clone, Serialize, Deserialize)]
pub struct QuestManager {
    /// Reference to the civilization this manager belongs to
    #[serde(skip)]
    pub civ: Option<Arc<Civilization>>,

    /// List of active quests, both global and individual ones
    pub assigned_quests: Vec<AssignedQuest>,

    /// Number of turns left before starting new global quest
    pub global_quest_countdown: i32,

    /// Number of turns left before this city state can start a new individual quest
    /// Key is major civ name, value is turns to quest
    pub individual_quest_countdown: HashMap<String, i32>,

    /// Target number of units to kill for this war, for war with major pseudo-quest
    pub units_to_kill_for_civ: HashMap<String, i32>,

    /// For this attacker, number of units killed by each civ
    pub units_killed_from_civ: HashMap<String, HashMap<String, i32>>,
}

/// Represents an assigned quest
#[derive(Clone, Serialize, Deserialize)]
pub struct AssignedQuest {
    /// Name of the quest
    pub quest_name: String,

    /// Name of the civilization that assigned the quest
    pub assigner: String,

    /// Name of the civilization that received the quest
    pub assignee: String,

    /// Turn when the quest was assigned
    pub assigned_on_turn: i32,

    /// First data field for quest-specific information
    pub data1: String,

    /// Second data field for quest-specific information
    pub data2: String,

    /// Reference to the game info
    #[serde(skip)]
    pub game_info: Option<Arc<GameInfo>>,

    /// Reference to the quest object
    #[serde(skip)]
    pub quest_object: Option<Arc<Quest>>,
}

impl QuestManager {
    /// Creates a new QuestManager
    pub fn new() -> Self {
        Self {
            civ: None,
            assigned_quests: Vec::new(),
            global_quest_countdown: UNSET,
            individual_quest_countdown: HashMap::new(),
            units_to_kill_for_civ: HashMap::new(),
            units_killed_from_civ: HashMap::new(),
        }
    }

    /// Sets the transient references to the civilization
    pub fn set_transients(&mut self, civ: Arc<Civilization>) {
        this.civ = Some(civ.clone());
        for quest in &mut this.assigned_quests {
            quest.set_transients(civ.game_info.clone());
        }
    }

    /// Gets the ruleset through the civilization
    fn get_ruleset(&self) -> &Ruleset {
        let civ = this.civ.as_ref().expect("Civ not set");
        &civ.game_info.ruleset
    }

    /// Returns true if the civilization has active quests for the challenger
    pub fn have_quests_for(&self, challenger: &Civilization) -> bool {
        self.get_assigned_quests_for(&challenger.civ_name).next().is_some()
    }

    /// Gets all assigned quests for a civilization
    pub fn get_assigned_quests_for<'a>(&'a self, civ_name: &str) -> impl Iterator<Item = &'a AssignedQuest> {
        this.assigned_quests.iter().filter(move |q| q.assignee == *civ_name)
    }

    /// Gets all assigned quests of a specific type
    fn get_assigned_quests_of_name<'a>(&'a self, quest_name: QuestName) -> impl Iterator<Item = &'a AssignedQuest> {
        this.assigned_quests.iter().filter(move |q| q.quest_name_instance() == quest_name)
    }

    /// Returns true if the civilization has asked anyone to conquer the target
    pub fn wants_dead(&self, target: &str) -> bool {
        self.get_assigned_quests_of_name(QuestName::ConquerCityState)
            .any(|q| q.data1 == *target)
    }

    /// Returns the influence multiplier for a donor from an Investment quest
    pub fn get_investment_multiplier(&self, donor: &str) -> f64 {
        let investment_quest = self.get_assigned_quests_of_name(QuestName::Invest)
            .find(|q| q.assignee == *donor)
            .map(|q| q.data1.parse::<f64>().unwrap_or(0.0) / 100.0)
            .unwrap_or(1.0);

        investment_quest
    }

    /// Creates a clone of this QuestManager
    pub fn clone(&self) -> Self {
        let mut to_return = Self::new();
        to_return.global_quest_countdown = this.global_quest_countdown;
        to_return.individual_quest_countdown = this.individual_quest_countdown.clone();
        to_return.assigned_quests = this.assigned_quests.clone();
        to_return.units_to_kill_for_civ = this.units_to_kill_for_civ.clone();

        for (attacker, units_killed) in &this.units_killed_from_civ {
            to_return.units_killed_from_civ.insert(attacker.clone(), units_killed.clone());
        }

        to_return
    }

    /// Processes end-of-turn actions
    pub fn end_turn(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        if civ.is_defeated() {
            this.assigned_quests.clear();
            this.individual_quest_countdown.clear();
            this.global_quest_countdown = UNSET;
            return;
        }

        if civ.cities.is_empty() {
            return; // don't assign quests until we have a city
        }

        self.seed_global_quest_countdown();
        self.seed_individual_quests_countdowns();

        self.decrement_quest_countdowns();

        self.handle_global_quests();
        self.handle_individual_quests();

        self.try_start_new_global_quest();
        self.try_start_new_individual_quests();

        self.try_barbarian_invasion();
        self.try_end_war_with_major_quests();
    }

    /// Decrements quest countdowns
    fn decrement_quest_countdowns(&mut self) {
        if this.global_quest_countdown > 0 {
            this.global_quest_countdown -= 1;
        }

        for (_, countdown) in this.individual_quest_countdown.iter_mut() {
            if *countdown > 0 {
                *countdown -= 1;
            }
        }
    }

    /// Seeds the global quest countdown
    fn seed_global_quest_countdown(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        if civ.game_info.turns < GLOBAL_QUEST_FIRST_POSSIBLE_TURN {
            return;
        }

        if this.global_quest_countdown != UNSET {
            return;
        }

        let countdown = if civ.game_info.turns == GLOBAL_QUEST_FIRST_POSSIBLE_TURN {
            rand::random::<i32>() % GLOBAL_QUEST_FIRST_POSSIBLE_TURN_RAND
        } else {
            GLOBAL_QUEST_MIN_TURNS_BETWEEN + (rand::random::<i32>() % GLOBAL_QUEST_RAND_TURNS_BETWEEN)
        };

        this.global_quest_countdown = (countdown as f64 * civ.game_info.speed.modifier) as i32;
    }

    /// Seeds individual quests countdowns
    fn seed_individual_quests_countdowns(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        if civ.game_info.turns < INDIVIDUAL_QUEST_FIRST_POSSIBLE_TURN {
            return;
        }

        let major_civs = civ.game_info.get_alive_major_civs();
        for major_civ in major_civs {
            if !this.individual_quest_countdown.contains_key(&major_civ.civ_name) ||
               this.individual_quest_countdown[&major_civ.civ_name] == UNSET {
                self.seed_individual_quests_countdown(&major_civ);
            }
        }
    }

    /// Seeds individual quests countdown for a challenger
    fn seed_individual_quests_countdown(&mut self, challenger: &Civilization) {
        let civ = this.civ.as_ref().expect("Civ not set");

        let countdown = if civ.game_info.turns == INDIVIDUAL_QUEST_FIRST_POSSIBLE_TURN {
            rand::random::<i32>() % INDIVIDUAL_QUEST_FIRST_POSSIBLE_TURN_RAND
        } else {
            INDIVIDUAL_QUEST_MIN_TURNS_BETWEEN + (rand::random::<i32>() % INDIVIDUAL_QUEST_RAND_TURNS_BETWEEN)
        };

        this.individual_quest_countdown.insert(
            challenger.civ_name.clone(),
            (countdown as f64 * civ.game_info.speed.modifier) as i32
        );
    }

    /// Gets quests matching a predicate
    fn get_quests<F>(&self, predicate: F) -> Vec<&Quest>
    where
        F: Fn(&Quest) -> bool,
    {
        self.get_ruleset().quests.values()
            .filter(|q| predicate(q))
            .collect()
    }

    /// Tries to start a new global quest
    fn try_start_new_global_quest(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        if this.global_quest_countdown != 0 {
            return;
        }

        if this.assigned_quests.iter().filter(|q| q.is_global()).count() >= GLOBAL_QUEST_MAX_ACTIVE as usize {
            return;
        }

        let major_civs = civ.get_known_civs()
            .filter(|c| c.is_major_civ() && !c.is_at_war_with(civ));

        let assignable_quests = self.get_quests(|q| {
            q.is_global() && major_civs.clone()
                .filter(|c| self.is_quest_valid(q, c))
                .count() >= q.minimum_civs as usize
        });

        if !assignable_quests.is_empty() {
            let quest = assignable_quests.random_weighted(|q| self.get_quest_weight(&q.name));
            let assignees = civ.game_info.get_alive_major_civs()
                .filter(|c| !c.is_at_war_with(civ) && self.is_quest_valid(quest, c));

            self.assign_new_quest(quest, assignees);
            this.global_quest_countdown = UNSET;
        }
    }

    /// Tries to start new individual quests
    fn try_start_new_individual_quests(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        for (challenger_name, countdown) in &this.individual_quest_countdown {
            let challenger = civ.game_info.get_civilization(challenger_name);

            if *countdown != 0 {
                continue;
            }

            if self.get_assigned_quests_for(&challenger.civ_name)
                .filter(|q| q.is_individual())
                .count() >= INDIVIDUAL_QUEST_MAX_ACTIVE as usize {
                continue;
            }

            let assignable_quests = self.get_quests(|q| q.is_individual() && self.is_quest_valid(q, &challenger));

            if !assignable_quests.is_empty() {
                let quest = assignable_quests.random_weighted(|q| self.get_quest_weight(&q.name));
                let assignees = vec![challenger.clone()];

                self.assign_new_quest(quest, assignees);
            }
        }
    }

    /// Tries to trigger a barbarian invasion
    fn try_barbarian_invasion(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        if (civ.get_turns_till_call_for_barb_help().is_none() || civ.get_turns_till_call_for_barb_help() == 0)
            && civ.city_state_functions.get_num_threatening_barbarians() >= 2 {

            for other_civ in civ.get_known_civs().filter(|c| {
                c.is_major_civ()
                && c.is_alive()
                && !c.is_at_war_with(civ)
                && c.get_proximity(civ) <= Proximity::Far
            }) {
                other_civ.add_notification(
                    &format!("[{}] is being invaded by Barbarians! Destroy Barbarians near their territory to earn Influence.", civ.civ_name),
                    civ.get_capital().map(|c| c.location),
                    NotificationCategory::Diplomacy,
                    &civ.civ_name,
                    NotificationIcon::War
                );
            }

            civ.add_flag(CivFlags::TurnsTillCallForBarbHelp.name(), 30);
        }
    }

    /// Handles global quests
    fn handle_global_quests(&mut self) {
        let civ = this.civ.as_ref().expect("Civ not set");

        // Remove any participants that are no longer valid because of being dead or at war with the CS
        this.assigned_quests.retain(|q| {
            !q.is_global() || self.can_assign_a_quest_to(&civ.game_info.get_civilization(&q.assignee))
        });

        let global_quests_expired = this.assigned_quests.iter()
            .filter(|q| q.is_global() && q.is_expired())
            .map(|q| q.quest_name_instance())
            .collect::<HashSet<_>>();

        for global_quest_name in global_quests_expired {
            self.handle_global_quest(global_quest_name);
        }
    }

    /// Handles a global quest
    fn handle_global_quest(&mut self, quest_name: QuestName) {
        let winners_and_losers = WinnersAndLosers::new(self, quest_name);

        for winner in &winners_and_losers.winners {
            self.give_reward(winner);
        }

        for loser in &winners_and_losers.losers {
            self.notify_expired(loser, &winners_and_losers.winners);
        }

        this.assigned_quests.retain(|q| q.quest_name_instance() != quest_name);
    }

    /// Handles individual quests
    fn handle_individual_quests(&mut self) {
        this.assigned_quests.retain(|q| {
            !q.is_individual() || !self.handle_individual_quest(q)
        });
    }

    /// Handles an individual quest
    fn handle_individual_quest(&self, assigned_quest: &AssignedQuest) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");
        let assignee = civ.game_info.get_civilization(&assigned_quest.assignee);

        // One of the civs is defeated, or they started a war: remove quest
        if !self.can_assign_a_quest_to(&assignee) {
            return true;
        }

        if self.is_complete(assigned_quest) {
            self.give_reward(assigned_quest);
            return true;
        }

        if self.is_obsolete(assigned_quest) {
            self.notify_expired(assigned_quest, &[]);
            return true;
        }

        if assigned_quest.is_expired() {
            self.notify_expired(assigned_quest, &[]);
            return true;
        }

        false
    }

    /// Assigns a new quest to assignees
    fn assign_new_quest(&mut self, quest: &Quest, assignees: Vec<Arc<Civilization>>) {
        let civ = this.civ.as_ref().expect("Civ not set");
        let turn = civ.game_info.turns;

        for assignee in assignees {
            let mut data1 = String::new();
            let mut data2 = String::new();
            let mut notification_actions = vec![NotificationAction::Diplomacy(civ.civ_name.clone())];

            match quest.quest_name_instance {
                QuestName::ClearBarbarianCamp => {
                    let camp = self.get_barbarian_encampment_for_quest().expect("No barbarian camp found");
                    data1 = camp.position.x.to_string();
                    data2 = camp.position.y.to_string();
                    notification_actions = vec![
                        NotificationAction::Location(camp.position),
                        notification_actions[0].clone(),
                    ];
                }
                QuestName::ConnectResource => {
                    data1 = self.get_resource_for_quest(&assignee).expect("No resource found").name;
                }
                QuestName::ConstructWonder => {
                    data1 = self.get_wonder_to_build_for_quest(&assignee).expect("No wonder found").name;
                }
                QuestName::GreatPerson => {
                    data1 = self.get_great_person_for_quest(&assignee).expect("No great person found").name;
                }
                QuestName::FindPlayer => {
                    data1 = self.get_civilization_to_find_for_quest(&assignee).expect("No civilization found").civ_name;
                }
                QuestName::FindNaturalWonder => {
                    data1 = self.get_natural_wonder_to_find_for_quest(&assignee).expect("No natural wonder found");
                }
                QuestName::ConquerCityState => {
                    data1 = self.get_city_state_target(&assignee).expect("No city state target found").civ_name;
                }
                QuestName::BullyCityState => {
                    data1 = self.get_city_state_target(&assignee).expect("No city state target found").civ_name;
                }
                QuestName::PledgeToProtect => {
                    data1 = self.get_most_recent_bully().expect("No recent bully found");
                }
                QuestName::GiveGold => {
                    data1 = self.get_most_recent_bully().expect("No recent bully found");
                }
                QuestName::DenounceCiv => {
                    data1 = self.get_most_recent_bully().expect("No recent bully found");
                }
                QuestName::SpreadReligion => {
                    let player_religion = civ.game_info.religions.values
                        .iter()
                        .find(|r| r.founding_civ_name == assignee.civ_name && r.is_major_religion())
                        .expect("No player religion found");
                    data1 = player_religion.get_religion_display_name();
                    data2 = player_religion.name.clone();
                }
                QuestName::ContestCulture => {
                    data1 = assignee.total_culture_for_contests.to_string();
                }
                QuestName::ContestFaith => {
                    data1 = assignee.total_faith_for_contests.to_string();
                }
                QuestName::ContestTech => {
                    data1 = assignee.tech.get_number_of_techs_researched().to_string();
                }
                QuestName::Invest => {
                    data1 = quest.description.get_placeholder_parameters()[0].clone();
                }
                _ => {}
            }

            let mut new_quest = AssignedQuest {
                quest_name: quest.name.clone(),
                assigner: civ.civ_name.clone(),
                assignee: assignee.civ_name.clone(),
                assigned_on_turn: turn,
                data1,
                data2,
                game_info: None,
                quest_object: None,
            };

            new_quest.set_transients(civ.game_info.clone(), Some(quest.clone()));

            this.assigned_quests.push(new_quest);

            if quest.is_individual() {
                this.individual_quest_countdown.insert(assignee.civ_name.clone(), UNSET);
            }

            assignee.add_notification(
                &format!("[{}] assigned you a new quest: [{}].", civ.civ_name, quest.name),
                notification_actions,
                NotificationCategory::Diplomacy,
                &civ.civ_name,
                "OtherIcons/Quest"
            );
        }
    }

    /// Returns true if a quest can be assigned to a challenger
    fn can_assign_a_quest_to(&self, challenger: &Civilization) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");

        !challenger.is_defeated() && challenger.is_major_civ() &&
        civ.knows(challenger) && !civ.is_at_war_with(challenger)
    }

    /// Returns true if a quest can be assigned to a challenger
    fn is_quest_valid(&self, quest: &Quest, challenger: &Civilization) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");

        if !self.can_assign_a_quest_to(challenger) {
            return false;
        }

        if self.get_assigned_quests_of_name(quest.quest_name_instance)
            .any(|q| q.assignee == challenger.civ_name) {
            return false;
        }

        if quest.is_individual() && civ.get_diplomacy_manager(&challenger.civ_name)
            .map_or(false, |dm| dm.has_flag(DiplomacyFlags::Bullied)) {
            return false;
        }

        match quest.quest_name_instance {
            QuestName::ClearBarbarianCamp => self.get_barbarian_encampment_for_quest().is_some(),
            QuestName::Route => self.is_route_quest_valid(challenger),
            QuestName::ConnectResource => self.get_resource_for_quest(challenger).is_some(),
            QuestName::ConstructWonder => self.get_wonder_to_build_for_quest(challenger).is_some(),
            QuestName::GreatPerson => self.get_great_person_for_quest(challenger).is_some(),
            QuestName::FindPlayer => self.get_civilization_to_find_for_quest(challenger).is_some(),
            QuestName::FindNaturalWonder => self.get_natural_wonder_to_find_for_quest(challenger).is_some(),
            QuestName::PledgeToProtect => {
                self.get_most_recent_bully().is_some() &&
                !civ.city_state_functions.get_protector_civs().contains(challenger)
            }
            QuestName::GiveGold => self.get_most_recent_bully().is_some(),
            QuestName::DenounceCiv => {
                self.is_denounce_civ_quest_valid(challenger, self.get_most_recent_bully())
            }
            QuestName::SpreadReligion => {
                let player_religion = civ.game_info.religions.values
                    .iter()
                    .find(|r| r.founding_civ_name == challenger.civ_name && r.is_major_religion())
                    .map(|r| r.name.clone());

                player_religion.is_some() &&
                civ.get_capital()
                    .and_then(|c| c.religion.get_majority_religion())
                    .map_or(false, |r| r.name != player_religion.unwrap())
            }
            QuestName::ConquerCityState => {
                self.get_city_state_target(challenger).is_some() &&
                civ.city_state_personality != CityStatePersonality::Friendly
            }
            QuestName::BullyCityState => self.get_city_state_target(challenger).is_some(),
            QuestName::ContestFaith => civ.game_info.is_religion_enabled(),
            _ => true
        }
    }

    /// Returns true if a route quest is valid for a challenger
    fn is_route_quest_valid(&self, challenger: &Civilization) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");

        if challenger.cities.is_empty() {
            return false;
        }

        if challenger.is_capital_connected_to_city(civ.get_capital().unwrap()) {
            return false;
        }

        let capital = civ.get_capital()?;
        let capital_tile = capital.get_center_tile();

        challenger.cities.iter().any(|city| {
            let city_tile = city.get_center_tile();
            city_tile.get_continent() == capital_tile.get_continent() &&
            city_tile.aerial_distance_to(&capital_tile) <= 7
        })
    }

    /// Returns true if a denounce civ quest is valid for a challenger
    fn is_denounce_civ_quest_valid(&self, challenger: &Civilization, most_recent_bully: Option<&str>) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");

        if let Some(bully_name) = most_recent_bully {
            challenger.knows(bully_name) &&
            !challenger.get_diplomacy_manager(bully_name)
                .map_or(false, |dm| dm.has_flag(DiplomacyFlags::Denunciation)) &&
            challenger.get_diplomacy_manager(bully_name)
                .map_or(false, |dm| dm.diplomatic_status != DiplomaticStatus::War) &&
            !(challenger.player_type == PlayerType::Human &&
              civ.game_info.get_civilization(bully_name).player_type == PlayerType::Human)
        } else {
            false
        }
    }

    /// Checks if a quest is completed
    fn is_quest_completed(&self, quest: &AssignedQuest) -> bool {
        let civ = this.civ.as_ref().expect("Civ not set");
        let assignee = civ.game_info.get_civilization(&quest.assignee)?;

        match quest.quest_name_instance {
            QuestName::ClearBarbarianCamp => {
                let x = quest.data1.parse::<i32>().ok()?;
                let y = quest.data2.parse::<i32>().ok()?;
                let position = TilePosition { x, y };

                !civ.game_info.get_tile(&position).map_or(false, |t| t.is_barbarian_camp())
            }
            QuestName::Route => {
                assignee.is_capital_connected_to_city(civ.get_capital()?)
            }
            QuestName::ConnectResource => {
                let resource_name = &quest.data1;
                civ.get_capital()?.get_tile().get_resource().map_or(false, |r| r.name == *resource_name)
            }
            QuestName::ConstructWonder => {
                let wonder_name = &quest.data1;
                civ.get_capital()?.get_built_wonders().contains(wonder_name)
            }
            QuestName::GreatPerson => {
                let great_person_name = &quest.data1;
                civ.get_great_people().iter().any(|gp| gp.name == *great_person_name)
            }
            QuestName::FindPlayer => {
                let target_name = &quest.data1;
                civ.knows(target_name)
            }
            QuestName::FindNaturalWonder => {
                let wonder_name = &quest.data1;
                civ.get_explored_tiles().iter().any(|t| t.is_natural_wonder() && t.get_natural_wonder_name() == *wonder_name)
            }
            QuestName::PledgeToProtect => {
                let target_name = &quest.data1;
                civ.city_state_functions.get_protector_civs().contains(&assignee.civ_name)
            }
            QuestName::GiveGold => {
                let target_name = &quest.data1;
                assignee.gold >= 250
            }
            QuestName::DenounceCiv => {
                let target_name = &quest.data1;
                assignee.get_diplomacy_manager(target_name)
                    .map_or(false, |dm| dm.has_flag(DiplomacyFlags::Denunciation))
            }
            QuestName::SpreadReligion => {
                let religion_name = &quest.data2;
                civ.get_capital()?.religion.get_majority_religion()
                    .map_or(false, |r| r.name == *religion_name)
            }
            QuestName::ConquerCityState => {
                let target_name = &quest.data1;
                !civ.game_info.get_civilization(target_name).map_or(false, |cs| cs.is_city_state())
            }
            QuestName::BullyCityState => {
                let target_name = &quest.data1;
                civ.get_diplomacy_manager(target_name)
                    .map_or(false, |dm| dm.has_flag(DiplomacyFlags::Bullied))
            }
            QuestName::ContestCulture => {
                let target_culture = quest.data1.parse::<i32>().ok()?;
                assignee.total_culture_for_contests >= target_culture
            }
            QuestName::ContestFaith => {
                let target_faith = quest.data1.parse::<i32>().ok()?;
                assignee.total_faith_for_contests >= target_faith
            }
            QuestName::ContestTech => {
                let target_techs = quest.data1.parse::<i32>().ok()?;
                assignee.tech.get_number_of_techs_researched() >= target_techs
            }
            QuestName::Invest => {
                let target_name = &quest.data1;
                assignee.gold >= 500
            }
            _ => false
        }
    }

    /// Awards a quest completion
    fn award_quest_completion(&mut self, quest: &AssignedQuest) {
        let civ = this.civ.as_ref().expect("Civ not set");
        let assignee = civ.game_info.get_civilization(&quest.assignee)?;

        let reward = match quest.quest_name_instance {
            QuestName::ClearBarbarianCamp => 30,
            QuestName::Route => 20,
            QuestName::ConnectResource => 25,
            QuestName::ConstructWonder => 40,
            QuestName::GreatPerson => 35,
            QuestName::FindPlayer => 15,
            QuestName::FindNaturalWonder => 20,
            QuestName::PledgeToProtect => 25,
            QuestName::GiveGold => 30,
            QuestName::DenounceCiv => 35,
            QuestName::SpreadReligion => 30,
            QuestName::ConquerCityState => 40,
            QuestName::BullyCityState => 25,
            QuestName::ContestCulture => 30,
            QuestName::ContestFaith => 30,
            QuestName::ContestTech => 30,
            QuestName::Invest => 35,
            _ => 0
        };

        if reward > 0 {
            assignee.add_influence(reward);

            assignee.add_notification(
                &format!("You completed a quest for [{}] and received {} influence!", civ.civ_name, reward),
                vec![NotificationAction::Diplomacy(civ.civ_name.clone())],
                NotificationCategory::Diplomacy,
                &civ.civ_name,
                "OtherIcons/Quest"
            );

            civ.add_notification(
                &format!("[{}] completed your quest and received {} influence!", assignee.civ_name, reward),
                vec![NotificationAction::Diplomacy(assignee.civ_name.clone())],
                NotificationCategory::Diplomacy,
                &assignee.civ_name,
                "OtherIcons/Quest"
            );
        }
    }

    /// Handles quest expiration
    fn handle_quest_expiration(&mut self, quest: &AssignedQuest) {
        let civ = this.civ.as_ref().expect("Civ not set");
        let assignee = civ.game_info.get_civilization(&quest.assignee)?;

        assignee.add_notification(
            &format!("You failed to complete a quest for [{}] in time.", civ.civ_name),
            vec![NotificationAction::Diplomacy(civ.civ_name.clone())],
            NotificationCategory::Diplomacy,
            &civ.civ_name,
            "OtherIcons/Quest"
        );

        civ.add_notification(
            &format!("[{}] failed to complete your quest in time.", assignee.civ_name),
            vec![NotificationAction::Diplomacy(assignee.civ_name.clone())],
            NotificationCategory::Diplomacy,
            &assignee.civ_name,
            "OtherIcons/Quest"
        );
    }
}