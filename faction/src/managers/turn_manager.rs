use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use rand::Rng;
use crate::civilization::{Civilization, CivFlags, AlertType, NotificationCategory, NotificationIcon, PlayerType, PopupAlert};
use crate::city::managers::CityTurnManager;
use crate::map::mapunit::UnitTurnManager;
use crate::map::tile::Tile;
use crate::ui::screens::worldscreen::status::NextTurnProgress;
use crate::ui::components::MayaCalendar;
use crate::models::ruleset::unique::{UniqueTriggerActivation, UniqueType};
use crate::models::stats::Stats;
use crate::automation::civilization::NextTurnAutomation;
use crate::trade::TradeEvaluation;
use crate::victory::VictoryData;
use crate::utils::Log;

/// Handles turn-based operations for civilizations
#[derive(Clone, Serialize, Deserialize)]
pub struct TurnManager {
    #[serde(skip)]
    pub civ_info: Option<Arc<Civilization>>,
}

impl TurnManager {
    pub fn new(civ_info: Arc<Civilization>) -> Self {
        Self {
            civ_info: Some(civ_info),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            civ_info: self.civ_info.clone(),
        }
    }

    pub fn set_transients(&mut self, civ_info: Arc<Civilization>) {
        self.civ_info = Some(civ_info);
    }

    /// Starts the turn for the civilization
    pub fn start_turn(&mut self, progress_bar: Option<&mut NextTurnProgress>) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if civ_info.is_spectator() {
            return;
        }

        civ_info.threat_manager.clear();
        if civ_info.is_major_civ() && civ_info.is_alive() {
            civ_info.stats_history.record_ranking_stats(civ_info);
        }

        if !civ_info.cities.is_empty() && !civ_info.game_info.ruleset.technologies.is_empty() {
            civ_info.tech.update_research_progress();
        }

        civ_info.cache.update_civ_resources(); // If you offered a trade last turn, this turn it will have been accepted/declined
        for stockpiled_resource in civ_info.get_civ_resource_supply().iter().filter(|r| r.resource.is_stockpiled) {
            civ_info.gain_stockpiled_resource(&stockpiled_resource.resource, stockpiled_resource.amount);
        }

        civ_info.civ_constructions.start_turn();
        civ_info.attacks_since_turn_start.clear();
        civ_info.update_stats_for_next_turn(); // for things that change when turn passes e.g. golden age, city state influence

        // Do this after update_stats_for_next_turn but before cities.start_turn
        if civ_info.player_type == PlayerType::AI && civ_info.game_info.ruleset.mod_options.has_unique(UniqueType::ConvertGoldToScience) {
            NextTurnAutomation::automate_gold_to_science_percentage(civ_info);
        }

        // Generate great people at the start of the turn,
        // so they won't be generated out in the open and vulnerable to enemy attacks before you can control them
        if !civ_info.cities.is_empty() { //if no city available, add_great_person will throw exception
            let mut great_person = civ_info.great_people.get_new_great_person();
            while let Some(person) = great_person {
                if civ_info.game_info.ruleset.units.contains_key(&person) {
                    civ_info.units.add_unit(person);
                }
                great_person = civ_info.great_people.get_new_great_person();
            }
            civ_info.religion_manager.start_turn();
            if civ_info.is_long_count_active() {
                MayaCalendar::start_turn_for_maya(civ_info);
            }
        }

        civ_info.cache.update_viewable_tiles(); // adds explored tiles so that the units will be able to perform automated actions better
        civ_info.cache.update_cities_connected_to_capital();
        self.start_turn_flags();
        self.update_revolts();

        for unique in civ_info.get_triggered_uniques(UniqueType::TriggerUponTurnStart, &civ_info.state) {
            UniqueTriggerActivation::trigger_unique(unique, civ_info);
        }

        for city in &civ_info.cities {
            if let Some(progress_bar) = progress_bar {
                progress_bar.increment();
            }
            CityTurnManager::new(city.clone()).start_turn(); // Most expensive part of start_turn
        }

        for unit in civ_info.units.get_civ_units() {
            UnitTurnManager::new(unit.clone()).start_turn();
        }

        if civ_info.player_type == PlayerType::Human && civ_info.game_info.settings.automated_units_move_on_turn_start {
            civ_info.has_moved_automated_units = true;
            for unit in civ_info.units.get_civ_units() {
                unit.do_action();
            }
        } else {
            civ_info.has_moved_automated_units = false;
        }

        for trade_request in civ_info.trade_requests.to_vec() { // remove trade requests where one of the sides can no longer supply
            let offering_civ = civ_info.game_info.get_civilization(&trade_request.requesting_civ);
            if offering_civ.is_defeated() || !TradeEvaluation::new().is_trade_valid(&trade_request.trade, civ_info, &offering_civ) {
                civ_info.trade_requests.retain(|r| r != &trade_request);
                // Yes, this is the right direction. I checked.
                offering_civ.add_notification("Our proposed trade is no longer relevant!", NotificationCategory::Trade, NotificationIcon::Trade);
            }
        }

        self.update_winning_civ();
    }

    /// Handles turn start flags
    fn start_turn_flags(&mut self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        for flag in civ_info.flags_countdown.keys().cloned().collect::<Vec<_>>() {
            // In case we remove flags while iterating
            if !civ_info.flags_countdown.contains_key(&flag) {
                continue;
            }

            if flag == CivFlags::CityStateGreatPersonGift.to_string() {
                let city_state_allies: Vec<Arc<Civilization>> = civ_info.get_known_civs()
                    .iter()
                    .filter(|civ| civ.is_city_state && civ.get_ally_civ() == civ_info.civ_name)
                    .cloned()
                    .collect();

                let giving_city_state = city_state_allies.iter()
                    .filter(|civ| !civ.cities.is_empty())
                    .choose(&mut rand::thread_rng());

                if !city_state_allies.is_empty() {
                    *civ_info.flags_countdown.get_mut(&flag).unwrap() -= 1;
                }

                if civ_info.flags_countdown[&flag] < city_state_allies.len().min(10) && !civ_info.cities.is_empty() && giving_city_state.is_some() {
                    giving_city_state.unwrap().city_state_functions.give_great_person_to_patron(civ_info);
                    *civ_info.flags_countdown.get_mut(&flag).unwrap() = civ_info.city_state_functions.turns_for_great_person_from_city_state();
                }

                continue;
            }

            if civ_info.flags_countdown[&flag] > 0 {
                *civ_info.flags_countdown.get_mut(&flag).unwrap() -= 1;
            }

            if civ_info.flags_countdown[&flag] != 0 {
                continue;
            }

            match flag.as_str() {
                s if s == CivFlags::RevoltSpawning.to_string() => self.do_revolt_spawn(),
                s if s == CivFlags::TurnsTillCityStateElection.to_string() => civ_info.city_state_functions.hold_elections(),
                _ => {}
            }
        }

        self.handle_diplomatic_victory_flags();
    }

    /// Handles diplomatic victory flags
    fn handle_diplomatic_victory_flags(&mut self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if civ_info.flags_countdown.get(&CivFlags::ShouldResetDiplomaticVotes.to_string()) == Some(&0) {
            civ_info.game_info.diplomatic_victory_votes_cast.clear();
            civ_info.remove_flag(CivFlags::ShowDiplomaticVotingResults);
            civ_info.remove_flag(CivFlags::ShouldResetDiplomaticVotes);
        }

        if civ_info.flags_countdown.get(&CivFlags::ShowDiplomaticVotingResults.to_string()) == Some(&0) {
            civ_info.game_info.process_diplomatic_victory();
            if civ_info.game_info.civilizations.iter().any(|civ| civ.victory_manager.has_won()) {
                civ_info.remove_flag(CivFlags::TurnsTillNextDiplomaticVote);
            } else {
                civ_info.add_flag(CivFlags::ShouldResetDiplomaticVotes, 1);
                civ_info.add_flag(CivFlags::TurnsTillNextDiplomaticVote, civ_info.get_turns_between_diplomatic_votes());
            }
        }

        if civ_info.flags_countdown.get(&CivFlags::TurnsTillNextDiplomaticVote.to_string()) == Some(&0) {
            civ_info.add_flag(CivFlags::ShowDiplomaticVotingResults, 1);
        }
    }

    /// Updates revolt status
    fn update_revolts(&mut self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if !civ_info.game_info.civilizations.iter().any(|civ| civ.is_barbarian) {
            // Can't spawn revolts without barbarians ¯\_(ツ)_/¯
            return;
        }

        if !civ_info.has_unique(UniqueType::SpawnRebels) {
            civ_info.remove_flag(CivFlags::RevoltSpawning);
            return;
        }

        if !civ_info.has_flag(CivFlags::RevoltSpawning) {
            civ_info.add_flag(CivFlags::RevoltSpawning, self.get_turns_before_revolt().max(1));
            return;
        }
    }

    /// Spawns revolt units
    fn do_revolt_spawn(&mut self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        let barbarians = match civ_info.game_info.get_barbarian_civilization() {
            Ok(barb) => barb,
            Err(_) => {
                Log::error("Barbarian civilization not found");
                civ_info.remove_flag(CivFlags::RevoltSpawning);
                return;
            }
        };

        let mut rng = rand::thread_rng();
        let rebel_count = 1 + rng.gen_range(0..100 + 20 * (civ_info.cities.len() - 1)) / 100;

        let spawn_city = match civ_info.cities.iter()
            .max_by_key(|city| rng.gen_range(0..city.population.population + 10)) {
            Some(city) => city,
            None => return,
        };

        let spawn_tile = match spawn_city.get_tiles().iter()
            .max_by_key(|tile| self.rate_tile_for_revolt_spawn(tile)) {
            Some(tile) => tile,
            None => return,
        };

        let unit_to_spawn = match civ_info.game_info.ruleset.units.values()
            .filter(|unit| unit.unique_to.is_none() && unit.is_melee() && unit.is_land_unit
                && !unit.has_unique(UniqueType::CannotAttack) && unit.is_buildable(civ_info))
            .max_by_key(|_| rng.gen_range(0..1000)) {
            Some(unit) => unit,
            None => return,
        };

        for _ in 0..rebel_count {
            civ_info.game_info.tile_map.place_unit_near_tile(
                spawn_tile.position,
                unit_to_spawn.clone(),
                barbarians.clone()
            );
        }

        // Will be automatically added again as long as unhappiness is still low enough
        civ_info.remove_flag(CivFlags::RevoltSpawning);

        civ_info.add_notification(
            "Your citizens are revolting due to very high unhappiness!",
            spawn_tile.position,
            NotificationCategory::General,
            unit_to_spawn.name.clone(),
            "StatIcons/Malcontent"
        );
    }

    /// Rates a tile for revolt spawn (higher is better)
    fn rate_tile_for_revolt_spawn(&self, tile: &Tile) -> i32 {
        if tile.is_water || tile.military_unit.is_some() || tile.civilian_unit.is_some() || tile.is_city_center() || tile.is_impassible() {
            return -1;
        }

        let mut score = 10;
        if tile.improvement.is_none() {
            score += 4;
            if tile.resource.is_some() {
                score += 3;
            }
        }
        if tile.get_defensive_bonus() > 0 {
            score += 4;
        }
        score
    }

    /// Gets the number of turns before a revolt
    fn get_turns_before_revolt(&self) -> i32 {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");
        let mut rng = rand::thread_rng();

        ((civ_info.game_info.ruleset.mod_options.constants.base_turns_until_revolt + rng.gen_range(0..3))
            * civ_info.game_info.speed.modifier.max(1.0)) as i32
    }

    /// Ends the turn for the civilization
    pub fn end_turn(&mut self, progress_bar: Option<&mut NextTurnProgress>) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if civ_info.game_info.settings.cities_auto_bombard_at_end_of_turn {
            NextTurnAutomation::automate_city_bombardment(civ_info); // Bombard with all cities that haven't, maybe you missed one
        }

        for unique in civ_info.get_triggered_uniques(UniqueType::TriggerUponTurnEnd, &civ_info.state) {
            UniqueTriggerActivation::trigger_unique(unique, civ_info);
        }

        let notifications_log = &mut civ_info.notifications_log;
        let notifications_this_turn = Civilization::NotificationsLog::new(civ_info.game_info.turns);
        notifications_this_turn.notifications.extend(civ_info.notifications.clone());

        while notifications_log.len() >= civ_info.game_info.settings.notifications_log_max_turns {
            notifications_log.remove(0);
        }

        if !notifications_this_turn.notifications.is_empty() {
            notifications_log.push(notifications_this_turn);
        }

        civ_info.notifications.clear();

        if civ_info.is_defeated() || civ_info.is_spectator() {
            return; // yes they do call this, best not update any further stuff
        }

        let mut next_turn_stats = if civ_info.is_barbarian {
            Stats::new()
        } else {
            civ_info.update_stats_for_next_turn();
            civ_info.stats.stats_for_next_turn.clone()
        };

        civ_info.policies.end_turn(next_turn_stats.culture as i32);
        civ_info.total_culture_for_contests += next_turn_stats.culture as i32;

        if civ_info.is_city_state {
            civ_info.quest_manager.end_turn();

            // Set turns to elections to a random number so not every city-state has the same election date
            // May be called at game start or when migrating a game from an older version
            if civ_info.game_info.is_espionage_enabled() && !civ_info.has_flag(CivFlags::TurnsTillCityStateElection) {
                let mut rng = rand::thread_rng();
                civ_info.add_flag(
                    CivFlags::TurnsTillCityStateElection,
                    rng.gen_range(0..=civ_info.game_info.ruleset.mod_options.constants.city_state_election_turns)
                );
            }
        }

        // disband units until there are none left OR the gold values are normal
        if !civ_info.is_barbarian && civ_info.gold <= -200 && next_turn_stats.gold < 0.0 {
            loop {
                let military_units: Vec<_> = civ_info.units.get_civ_units()
                    .iter()
                    .filter(|unit| unit.is_military())
                    .collect();

                let unit_to_disband = military_units.iter()
                    .min_by_key(|unit| unit.base_unit.cost);

                match unit_to_disband {
                    Some(unit) => {
                        unit.disband();
                        let unit_name = unit.short_display_name();
                        civ_info.add_notification(
                            format!("Cannot provide unit upkeep for {} - unit has been disbanded!", unit_name),
                            NotificationCategory::Units,
                            unit_name.clone(),
                            NotificationIcon::Death
                        );
                        // No need to recalculate unit upkeep, disband did that in UnitManager.remove_unit
                        next_turn_stats = civ_info.stats.stats_for_next_turn.clone();

                        if civ_info.gold > -200 || next_turn_stats.gold >= 0.0 {
                            break;
                        }
                    },
                    None => break,
                }
            }
        }

        civ_info.add_gold(next_turn_stats.gold as i32);

        if !civ_info.cities.is_empty() && !civ_info.game_info.ruleset.technologies.is_empty() {
            civ_info.tech.end_turn(next_turn_stats.science as i32);
        }

        civ_info.religion_manager.end_turn(next_turn_stats.faith as i32);
        civ_info.total_faith_for_contests += next_turn_stats.faith as i32;

        civ_info.espionage_manager.end_turn();

        if civ_info.is_major_civ() { // City-states don't get great people!
            civ_info.great_people.add_great_person_points();
        }

        // To handle tile's owner issue (#8246), we need to run cities being razed first.
        // a city can be removed while iterating (if it's being razed) so we need to iterate over a copy - sorting does one
        let cities_to_process: Vec<_> = civ_info.cities.iter()
            .sorted_by(|a, b| b.is_being_razed.cmp(&a.is_being_razed))
            .cloned()
            .collect();

        for city in cities_to_process {
            if let Some(progress_bar) = progress_bar {
                progress_bar.increment();
            }
            CityTurnManager::new(city).end_turn();
        }

        civ_info.temporary_uniques.end_turn();

        civ_info.golden_ages.end_turn(civ_info.get_happiness());

        for unit in civ_info.units.get_civ_units() {
            UnitTurnManager::new(unit.clone()).end_turn(); // This is the most expensive part of end_turn
        }

        for diplomacy in civ_info.diplomacy.values().cloned().collect::<Vec<_>>() {
            diplomacy.next_turn(); // we copy the diplomacy values so if it changes in-loop we won't crash
        }

        civ_info.cache.update_has_active_enemy_movement_penalty();
        civ_info.cached_military_might = -1; // Reset so we don't use a value from a previous turn

        self.update_winning_civ(); // Maybe we did something this turn to win
    }

    /// Updates the winning civilization
    pub fn update_winning_civ(&self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        if civ_info.game_info.victory_data.is_some() {
            return; // Game already won
        }

        if let Some(victory_type) = civ_info.victory_manager.get_victory_type_achieved() {
            civ_info.game_info.victory_data = Some(
                VictoryData::new(civ_info.civ_name.clone(), victory_type, civ_info.game_info.turns)
            );

            // Notify other human players about this civ's victory
            for other_civ in &civ_info.game_info.civilizations {
                // Skip winner, displaying VictoryScreen is handled separately in WorldScreen.update
                // by checking `viewing_civ.is_defeated() || game_info.check_for_victory()`
                if other_civ.player_type != PlayerType::Human || other_civ == civ_info {
                    continue;
                }
                other_civ.popup_alerts.push(PopupAlert::new(AlertType::GameHasBeenWon, "".to_string()));
            }
        }
    }

    /// Automates the turn for AI civilizations
    pub fn automate_turn(&self) {
        let civ_info = self.civ_info.as_ref().expect("Civ not set");

        // Defeated civs do nothing
        if civ_info.is_defeated() {
            return;
        }

        // Do stuff
        NextTurnAutomation::automate_civ_moves(civ_info);

        // Update barbarian camps
        if civ_info.is_barbarian && !civ_info.game_info.game_parameters.no_barbarians {
            civ_info.game_info.barbarians.update_encampments();
        }
    }
}