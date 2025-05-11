// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/unit/AutoPlay.kt

use std::rc::Rc;
use std::cell::RefCell;
use crate::game::settings::GameSettings;
use crate::ui::screens::worldscreen::WorldScreen;
use crate::utils::concurrency::Concurrency;

/// Manages auto-play functionality for the game
pub struct AutoPlay {
    /// Settings for auto-play behavior
    auto_play_settings: Rc<RefCell<GameSettings>>,

    /// How many turns we should multiturn AutoPlay for.
    /// In the case that auto_play_settings.auto_play_until_end is true,
    /// the value should not be decremented after each turn.
    pub turns_to_auto_play: i32,

    /// Determines whether or not we are currently processing the viewing player's turn.
    /// This can be on the main thread or on a different thread.
    pub auto_play_turn_in_progress: bool,

    /// The current auto-play job
    pub auto_play_job: Option<Rc<RefCell<Concurrency>>>,
}

impl AutoPlay {
    /// Creates a new AutoPlay instance
    pub fn new(auto_play_settings: Rc<RefCell<GameSettings>>) -> Self {
        Self {
            auto_play_settings,
            turns_to_auto_play: 0,
            auto_play_turn_in_progress: false,
            auto_play_job: None,
        }
    }

    /// Starts multiturn auto-play
    pub fn start_multiturn_auto_play(&mut self) {
        self.auto_play_turn_in_progress = false;
        self.turns_to_auto_play = self.auto_play_settings.borrow().auto_play_max_turns;
    }

    /// Processes the end of the user's turn being AutoPlayed.
    /// Only decrements turns_to_auto_play if auto_play_settings.auto_play_until_end is false.
    pub fn end_turn_multiturn_auto_play(&mut self) {
        if !self.auto_play_settings.borrow().auto_play_until_end && self.turns_to_auto_play > 0 {
            self.turns_to_auto_play -= 1;
        }
    }

    /// Stops multiturn AutoPlay and sets auto_play_turn_in_progress to false
    pub fn stop_auto_play(&mut self) {
        self.turns_to_auto_play = 0;
        self.auto_play_turn_in_progress = false;
    }

    /// Does the provided job on a new thread if there isn't already an AutoPlay thread running.
    /// Will set auto_play_turn_in_progress to true for the duration of the job.
    ///
    /// # Arguments
    /// * `job_name` - Name of the job
    /// * `world_screen` - Reference to the world screen
    /// * `set_player_turn_after_end` - Keep this as the default (true) if it will still be the viewing player's turn after the job is finished.
    ///   Set it to false if the turn will end.
    /// * `job` - The job to run
    ///
    /// # Panics
    /// * If an AutoPlay job is currently running as this is called.
    pub fn run_auto_play_job_in_new_thread<F>(
        &mut self,
        job_name: String,
        world_screen: Rc<RefCell<WorldScreen>>,
        set_player_turn_after_end: bool,
        job: F
    ) where
        F: FnOnce() + Send + 'static,
    {
        if self.auto_play_turn_in_progress {
            panic!("Trying to start an AutoPlay job while a job is currently running");
        }

        self.auto_play_turn_in_progress = true;
        world_screen.borrow_mut().is_players_turn = false;

        let world_screen_clone = world_screen.clone();
        let auto_play = Rc::new(RefCell::new(self));

        self.auto_play_job = Some(Rc::new(RefCell::new(
            Concurrency::run_on_non_daemon_thread_pool(job_name, move || {
                job();
                auto_play.borrow_mut().auto_play_turn_in_progress = false;
                if set_player_turn_after_end {
                    world_screen_clone.borrow_mut().is_players_turn = true;
                }
            })
        )));
    }

    /// Checks if auto-play is currently active
    pub fn is_auto_playing(&self) -> bool {
        self.turns_to_auto_play > 0 || self.auto_play_turn_in_progress
    }

    /// Checks if auto-play is active and using full AI
    pub fn is_auto_playing_and_full_auto_play_ai(&self) -> bool {
        self.is_auto_playing() && self.auto_play_settings.borrow().full_auto_play_ai
    }

    /// Returns true if we should play at least 1 more turn and we are not currently processing any AutoPlay
    pub fn should_continue_auto_playing(&self) -> bool {
        !self.auto_play_turn_in_progress && self.turns_to_auto_play > 0
    }
}