use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::io::{self, Write};
use std::f64::consts::PI;

use crate::constants::Constants;
use crate::game::unciv_game::UncivGame;
use crate::game::game_info::GameInfo;
use crate::game::game_starter::GameStarter;
use crate::models::metadata::GameSetupInfo;
use crate::simulation::mutable_int::MutableInt;
use crate::simulation::simulation_step::SimulationStep;

/// Statistics collected during simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    /// Sum of values
    Sum,
    /// Number of values
    Num,
}

/// A simulation of multiple games
pub struct Simulation {
    /// The new game info
    new_game_info: GameInfo,
    /// The number of simulations per thread
    simulations_per_thread: usize,
    /// The number of threads
    threads_number: usize,
    /// The maximum number of turns
    max_turns: usize,
    /// The turns to collect statistics for
    stat_turns: Vec<i32>,
    /// The maximum number of simulations
    max_simulations: usize,
    /// The civilizations in the game
    civilizations: Vec<String>,
    /// The number of major civilizations
    major_civs: usize,
    /// The start time
    start_time: Instant,
    /// The simulation steps
    steps: Arc<Mutex<Vec<SimulationStep>>>,
    /// The number of wins per civilization
    num_wins: Arc<Mutex<HashMap<String, MutableInt>>>,
    /// The summary statistics
    summary_stats: Arc<Mutex<HashMap<String, HashMap<i32, HashMap<Stat, MutableInt>>>>>,
    /// The win rate by victory type
    win_rate_by_victory: Arc<Mutex<HashMap<String, HashMap<String, MutableInt>>>>,
    /// The win turn by victory type
    win_turn_by_victory: Arc<Mutex<HashMap<String, HashMap<String, MutableInt>>>>,
    /// The average speed
    avg_speed: f32,
    /// The average duration
    avg_duration: Duration,
    /// The total number of turns
    total_turns: usize,
    /// The total duration
    total_duration: Duration,
    /// The step counter
    step_counter: Arc<AtomicUsize>,
}

impl Simulation {
    /// Create a new Simulation
    ///
    /// # Parameters
    ///
    /// * `new_game_info` - The new game info
    /// * `simulations_per_thread` - The number of simulations per thread
    /// * `threads_number` - The number of threads
    /// * `max_turns` - The maximum number of turns
    /// * `stat_turns` - The turns to collect statistics for
    ///
    /// # Returns
    ///
    /// A new Simulation
    pub fn new(
        new_game_info: GameInfo,
        simulations_per_thread: usize,
        threads_number: usize,
        max_turns: usize,
        stat_turns: Vec<i32>,
    ) -> Self {
        let max_simulations = threads_number * simulations_per_thread;
        let civilizations: Vec<String> = new_game_info
            .civilizations
            .iter()
            .filter(|c| c.civ_name != Constants::spectator)
            .map(|c| c.civ_name.clone())
            .collect();
        let major_civs = new_game_info
            .civilizations
            .iter()
            .filter(|c| c.civ_name != Constants::spectator)
            .filter(|c| c.is_major_civ())
            .count();

        let mut num_wins = HashMap::new();
        let mut summary_stats = HashMap::new();
        let mut win_rate_by_victory = HashMap::new();
        let mut win_turn_by_victory = HashMap::new();

        for civ in &civilizations {
            num_wins.insert(civ.clone(), MutableInt::new(0));
            let mut civ_stats = HashMap::new();
            for &turn in &stat_turns {
                let mut turn_stats = HashMap::new();
                turn_stats.insert(Stat::Sum, MutableInt::new(0));
                turn_stats.insert(Stat::Num, MutableInt::new(0));
                civ_stats.insert(turn, turn_stats);
            }
            // End of game stats
            let mut end_stats = HashMap::new();
            end_stats.insert(Stat::Sum, MutableInt::new(0));
            end_stats.insert(Stat::Num, MutableInt::new(0));
            civ_stats.insert(-1, end_stats);
            summary_stats.insert(civ.clone(), civ_stats);

            let mut civ_win_rate = HashMap::new();
            let mut civ_win_turn = HashMap::new();
            for victory in UncivGame::current().game_info.as_ref().unwrap().ruleset.victories.keys() {
                civ_win_rate.insert(victory.clone(), MutableInt::new(0));
                civ_win_turn.insert(victory.clone(), MutableInt::new(0));
            }
            win_rate_by_victory.insert(civ.clone(), civ_win_rate);
            win_turn_by_victory.insert(civ.clone(), civ_win_turn);
        }

        Self {
            new_game_info,
            simulations_per_thread,
            threads_number,
            max_turns,
            stat_turns,
            max_simulations,
            civilizations,
            major_civs,
            start_time: Instant::now(),
            steps: Arc::new(Mutex::new(Vec::new())),
            num_wins: Arc::new(Mutex::new(num_wins)),
            summary_stats: Arc::new(Mutex::new(summary_stats)),
            win_rate_by_victory: Arc::new(Mutex::new(win_rate_by_victory)),
            win_turn_by_victory: Arc::new(Mutex::new(win_turn_by_victory)),
            avg_speed: 0.0,
            avg_duration: Duration::from_secs(0),
            total_turns: 0,
            total_duration: Duration::from_secs(0),
            step_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Start the simulation
    pub fn start(&self) {
        self.start_time = Instant::now();
        let (tx, rx): (Sender<SimulationStep>, Receiver<SimulationStep>) = channel();
        let mut handles = vec![];

        println!(
            "Starting new game with major civs: {} and minor civs: {}",
            self.new_game_info
                .civilizations
                .iter()
                .filter(|c| c.is_major_civ())
                .map(|c| &c.civ_name)
                .collect::<Vec<_>>()
                .join(", "),
            self.new_game_info
                .civilizations
                .iter()
                .filter(|c| c.is_city_state())
                .map(|c| &c.civ_name)
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.new_game_info.game_parameters.shuffle_player_order = true;

        for thread_id in 1..=self.threads_number {
            let tx = tx.clone();
            let new_game_info = self.new_game_info.clone();
            let stat_turns = self.stat_turns.clone();
            let max_turns = self.max_turns;
            let step_counter = self.step_counter.clone();
            let steps = self.steps.clone();

            let handle = thread::spawn(move || {
                for _ in 0..self.simulations_per_thread {
                    let step = SimulationStep::new(&new_game_info, &stat_turns);
                    let game_info = GameStarter::start_new_game(GameSetupInfo::new(&new_game_info));
                    let mut game_info = game_info;
                    game_info.simulate_until_win = true;

                    for &turn in &stat_turns {
                        game_info.simulate_max_turns = turn;
                        game_info.next_turn();
                        step.update(&game_info);
                        if step.victory_type.is_some() {
                            break;
                        }
                        step.save_turn_stats(&game_info);
                    }

                    // Check if Victory
                    step.update(&game_info);
                    if step.victory_type.is_none() {
                        game_info.simulate_max_turns = max_turns;
                        game_info.next_turn();
                    }

                    step.update(&game_info); // Final game state

                    if let Some(victory_type) = &step.victory_type {
                        step.save_turn_stats(&game_info);
                        step.winner = Some(step.current_player.clone());
                        println!(
                            "{} won {} victory on turn {}",
                            step.winner.as_ref().unwrap(),
                            victory_type,
                            step.turns
                        );
                    } else {
                        println!("Max simulation {} turns reached: Draw", step.turns);
                    }

                    Self::update_counter(&step_counter, thread_id);
                    Self::add(&steps, step.clone(), thread_id);
                    tx.send(step).unwrap();
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to finish
        for handle in handles {
            handle.join().unwrap();
        }

        // Collect all results
        let mut all_steps = Vec::new();
        for _ in 0..self.max_simulations {
            if let Ok(step) = rx.recv() {
                all_steps.push(step);
            }
        }

        // Update the steps
        let mut steps = self.steps.lock().unwrap();
        *steps = all_steps;

        self.print();
    }

    /// Add a simulation step
    ///
    /// # Parameters
    ///
    /// * `steps` - The steps to add to
    /// * `step` - The step to add
    /// * `thread_id` - The thread ID
    fn add(steps: &Arc<Mutex<Vec<SimulationStep>>>, step: SimulationStep, _thread_id: usize) {
        let mut steps = steps.lock().unwrap();
        steps.push(step);
    }

    /// Update the step counter
    ///
    /// # Parameters
    ///
    /// * `step_counter` - The step counter
    /// * `thread_id` - The thread ID
    fn update_counter(step_counter: &Arc<AtomicUsize>, _thread_id: usize) {
        let count = step_counter.fetch_add(1, Ordering::SeqCst) + 1;
        println!("Simulation step ({}/{})", count, step_counter.load(Ordering::SeqCst));
    }

    /// Print the simulation results
    pub fn print(&self) {
        self.get_stats();
        println!("{}", self.text());
    }

    /// Get the simulation statistics
    pub fn get_stats(&self) {
        // Reset counters
        {
            let mut num_wins = self.num_wins.lock().unwrap();
            for wins in num_wins.values_mut() {
                wins.set(0);
            }
        }

        {
            let mut win_rate_by_victory = self.win_rate_by_victory.lock().unwrap();
            for civ_rates in win_rate_by_victory.values_mut() {
                for rate in civ_rates.values_mut() {
                    rate.set(0);
                }
            }
        }

        {
            let mut win_turn_by_victory = self.win_turn_by_victory.lock().unwrap();
            for civ_turns in win_turn_by_victory.values_mut() {
                for turns in civ_turns.values_mut() {
                    turns.set(0);
                }
            }
        }

        {
            let mut summary_stats = self.summary_stats.lock().unwrap();
            for civ_stats in summary_stats.values_mut() {
                for turn_stats in civ_stats.values_mut() {
                    for stat in turn_stats.values_mut() {
                        stat.set(0);
                    }
                }
            }
        }

        // Update statistics
        let steps = self.steps.lock().unwrap();
        for step in steps.iter() {
            if let Some(ref winner) = step.winner {
                {
                    let mut num_wins = self.num_wins.lock().unwrap();
                    num_wins.get_mut(winner).unwrap().inc();
                }

                {
                    let mut win_rate_by_victory = self.win_rate_by_victory.lock().unwrap();
                    let victory_type = step.victory_type.as_ref().unwrap();
                    win_rate_by_victory
                        .get_mut(winner)
                        .unwrap()
                        .get_mut(victory)
                        .unwrap()
                        .inc();
                }

                {
                    let mut win_turn_by_victory = self.win_turn_by_victory.lock().unwrap();
                    let victory_type = step.victory_type.as_ref().unwrap();
                    win_turn_by_victory
                        .get_mut(winner)
                        .unwrap()
                        .get_mut(victory)
                        .unwrap()
                        .add(step.turns as i32);
                }
            }

            for civ in &self.civilizations {
                for &turn in &self.stat_turns {
                    let value = step.turn_stats.get(civ).unwrap().get(&turn).unwrap().get();
                    if value != -1 {
                        {
                            let mut summary_stats = self.summary_stats.lock().unwrap();
                            summary_stats
                                .get_mut(civ)
                                .unwrap()
                                .get_mut(&turn)
                                .unwrap()
                                .get_mut(&Stat::Sum)
                                .unwrap()
                                .add(value);
                            summary_stats
                                .get_mut(civ)
                                .unwrap()
                                .get_mut(&turn)
                                .unwrap()
                                .get_mut(&Stat::Num)
                                .unwrap()
                                .inc();
                        }
                    }
                }

                // End of game stats
                let value = step.turn_stats.get(civ).unwrap().get(&-1).unwrap().get();
                {
                    let mut summary_stats = self.summary_stats.lock().unwrap();
                    summary_stats
                        .get_mut(civ)
                        .unwrap()
                        .get_mut(&-1)
                        .unwrap()
                        .get_mut(&Stat::Sum)
                        .unwrap()
                        .add(value);
                    summary_stats
                        .get_mut(civ)
                        .unwrap()
                        .get_mut(&-1)
                        .unwrap()
                        .get_mut(&Stat::Num)
                        .unwrap()
                        .inc();
                }
            }
        }

        // Calculate total turns and duration
        self.total_turns = steps.iter().map(|step| step.turns).sum();
        self.total_duration = self.start_time.elapsed();
        self.avg_speed = self.total_turns as f32 / self.total_duration.as_secs_f32();
        self.avg_duration = self.total_duration / steps.len();
    }

    /// Get the simulation results as text
    ///
    /// # Returns
    ///
    /// The simulation results as text
    pub fn text(&self) -> String {
        let mut out_string = String::new();
        let steps = self.steps.lock().unwrap();
        let num_wins = self.num_wins.lock().unwrap();
        let summary_stats = self.summary_stats.lock().unwrap();
        let win_rate_by_victory = self.win_rate_by_victory.lock().unwrap();
        let win_turn_by_victory = self.win_turn_by_victory.lock().unwrap();

        for civ in &self.civilizations {
            let num_steps = steps.len().max(1);
            let exp_win_rate = 1.0 / self.major_civs as f32;
            if num_wins.get(civ).unwrap().get() == 0 {
                continue;
            }
            let win_rate = format!("{:.1}", num_wins.get(civ).unwrap().get() as f32 * 100.0 / num_steps as f32);

            out_string.push_str(&format!("\n{}:\n", civ));
            out_string.push_str(&format!("{}% total win rate \n", win_rate));
            if (num_steps as f32 * exp_win_rate >= 10.0) && (num_steps as f32 * (1.0 - exp_win_rate) >= 10.0) {
                // Large enough sample size, binomial distribution approximates the Normal Curve
                let pval = self.binomial_test(
                    num_wins.get(civ).unwrap().get() as f64,
                    num_steps as f64,
                    exp_win_rate as f64,
                    "greater",
                );
                out_string.push_str(&format!("one-tail binomial pval = {}\n", pval));
            }

            for victory in UncivGame::current().game_info.as_ref().unwrap().ruleset.victories.keys() {
                let wins_victory = win_rate_by_victory
                    .get(civ)
                    .unwrap()
                    .get(victory)
                    .unwrap()
                    .get() as f32
                    * 100.0
                    / num_wins.get(civ).unwrap().get().max(1) as f32;
                out_string.push_str(&format!("{}: {:.0}%    ", victory, wins_victory));
            }
            out_string.push('\n');

            for victory in UncivGame::current().game_info.as_ref().unwrap().ruleset.victories.keys() {
                let wins_turns = win_turn_by_victory
                    .get(civ)
                    .unwrap()
                    .get(victory)
                    .unwrap()
                    .get() as f32
                    / win_rate_by_victory
                        .get(civ)
                        .unwrap()
                        .get(victory)
                        .unwrap()
                        .get()
                        .max(1) as f32;
                out_string.push_str(&format!("{}: {:.0}    ", victory, wins_turns));
            }
            out_string.push_str("avg turns\n");

            for &turn in &self.stat_turns {
                let turn_stats = summary_stats.get(civ).unwrap().get(&turn).unwrap();
                let sum = turn_stats.get(&Stat::Sum).unwrap().get() as f32;
                let num = turn_stats.get(&Stat::Num).unwrap().get() as f32;
                out_string.push_str(&format!(
                    "@{}: popsum avg={:.1} cnt={}\n",
                    turn,
                    sum / num,
                    num
                ));
            }

            // End of match stats
            let turn = -1;
            let turn_stats = summary_stats.get(civ).unwrap().get(&turn).unwrap();
            let sum = turn_stats.get(&Stat::Sum).unwrap().get() as f32;
            let num = turn_stats.get(&Stat::Num).unwrap().get() as f32;
            out_string.push_str(&format!(
                "@END: popsum avg={:.1} cnt={}\n",
                sum / num,
                num
            ));
        }

        out_string.push_str(&format!("\nAverage speed: {:.1} turns/s \n", self.avg_speed));
        out_string.push_str(&format!("Average game duration: {:?}\n", self.avg_duration));
        out_string.push_str(&format!("Total time: {:?}\n", self.total_duration));

        out_string
    }

    /// Perform a binomial test
    ///
    /// # Parameters
    ///
    /// * `successes` - The number of successes
    /// * `trials` - The number of trials
    /// * `p` - The probability of success
    /// * `alternative` - The alternative hypothesis
    ///
    /// # Returns
    ///
    /// The p-value
    fn binomial_test(&self, successes: f64, trials: f64, p: f64, alternative: &str) -> f64 {
        let q = 1.0 - p;
        let mean = trials * p;
        let variance = trials * p * q;
        let std_dev = variance.sqrt();
        let z = (successes - mean) / std_dev;
        let p_value = 1.0 - self.normal_cdf(z);
        match alternative {
            "greater" => p_value,
            "less" => 1.0 - p_value,
            _ => panic!("Alternative must be 'greater' or 'less'"),
        }
    }

    /// Calculate the cumulative distribution function of the normal distribution
    ///
    /// # Parameters
    ///
    /// * `z` - The z-score
    ///
    /// # Returns
    ///
    /// The cumulative distribution function value
    fn normal_cdf(&self, z: f64) -> f64 {
        0.5 * (1.0 + self.erf(z / (2.0_f64).sqrt()))
    }

    /// Approximate the error function
    ///
    /// # Parameters
    ///
    /// * `x` - The input value
    ///
    /// # Returns
    ///
    /// The error function value
    fn erf(&self, x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.5 * x.abs());
        let tau = t
            * (-x * x
                - 1.26551223
                + t * (1.00002368
                    + t * (0.37409196
                        + t * (0.09678418
                            + t * (-0.18628806
                                + t * (0.27886807
                                    + t * (-1.13520398
                                        + t * (1.48851587
                                            + t * (-0.82215223 + t * 0.17087277))))))))))
            .exp();
        if x >= 0.0 {
            1.0 - tau
        } else {
            tau - 1.0
        }
    }
}